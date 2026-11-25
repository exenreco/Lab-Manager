use eframe::egui;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
    time::{Duration, Instant},
};

const API: &str = "https://api.github.com/repos/exenreco/Lab-Manager/releases?per_page=30";
const DOWNLOAD_PREFIX: &str = "https://github.com/exenreco/Lab-Manager/releases/download/";
const CURRENT: &str = env!("CARGO_PKG_VERSION");

fn asset_name(version: &Version) -> String {
    match std::env::consts::OS {
        "windows" => format!("Lab-Manager-{version}-windows-x64-setup.exe"),
        "macos" => format!("Lab-Manager-{version}-macos-{}.dmg", std::env::consts::ARCH),
        _ => format!(
            "Lab-Manager-{version}-linux-{}.tar.gz",
            std::env::consts::ARCH
        ),
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Preferences {
    automatic: bool,
}

#[derive(Clone, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
    size: u64,
}

#[derive(Clone, Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

fn candidate(releases: Vec<Release>, current: &str) -> Option<(Release, Asset)> {
    let current = Version::parse(current).ok()?;
    releases
        .into_iter()
        .filter_map(|release| {
            let version = Version::parse(release.tag_name.trim_start_matches('v')).ok()?;
            if release.draft
                || version <= current
                || ((release.prerelease || !version.pre.is_empty()) && current.pre.is_empty())
            {
                return None;
            }
            let expected = asset_name(&version);
            let asset = release
                .assets
                .iter()
                .find(|asset| {
                    asset.name == expected
                        && asset.browser_download_url.starts_with(DOWNLOAD_PREFIX)
                        && asset.digest.as_deref().is_some_and(valid_digest)
                        && asset.size > 0
                        && asset.size <= 200_000_000
                })?
                .clone();
            Some((version, release, asset))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, release, asset)| (release, asset))
}

fn valid_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn hidden(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    #[cfg(not(windows))]
    let _ = command;
}

fn curl(url: &str, destination: Option<&Path>, limit: u64) -> Result<Vec<u8>, String> {
    let mut command = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" });
    command.args([
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--connect-timeout",
        "10",
        "--max-time",
        "180",
        "--max-filesize",
        &limit.to_string(),
        "--user-agent",
        "Lab-Manager-Updater",
        "--header",
        "Accept: application/vnd.github+json",
    ]);
    if let Some(path) = destination {
        command.arg("--output").arg(path);
    }
    command.arg(url);
    hidden(&mut command);
    let output = command
        .output()
        .map_err(|e| format!("Cannot start Windows HTTPS client: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "GitHub request failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

fn verify(path: &Path, asset: &Asset) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    if bytes.len() as u64 != asset.size || asset.digest.as_deref() != Some(&digest) {
        return Err("Installer integrity verification failed. Nothing was installed.".into());
    }
    Ok(())
}

fn download(asset: &Asset, root: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let path = root.join("installer.download");
    let result = (|| {
        curl(&asset.browser_download_url, Some(&path), 200_000_000)?;
        verify(&path, asset)?;
        let ready = root.join(&asset.name);
        std::fs::rename(&path, &ready).map_err(|e| e.to_string())?;
        Ok(ready)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(path);
    }
    result
}

enum Event {
    Checked(Result<Option<(Release, Asset)>, String>),
    Downloaded(Result<PathBuf, String>),
}

#[derive(Default)]
pub struct UpdatePanel {
    preferences: Preferences,
    loaded: bool,
    next_check: Option<Instant>,
    receiver: Option<mpsc::Receiver<Event>>,
    available: Option<(Release, Asset)>,
    ready: Option<PathBuf>,
    message: String,
}

impl UpdatePanel {
    fn check(&mut self, ctx: &egui::Context) {
        if self.receiver.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.message = "Checking GitHub releases…".into();
        self.next_check = Some(Instant::now() + Duration::from_secs(6 * 60 * 60));
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = curl(API, None, 2_000_000).and_then(|bytes| {
                serde_json::from_slice::<Vec<Release>>(&bytes)
                    .map(|releases| candidate(releases, CURRENT))
                    .map_err(|e| format!("Invalid release response: {e}"))
            });
            let _ = tx.send(Event::Checked(result));
            ctx.request_repaint();
        });
    }

    fn start_download(&mut self, ctx: &egui::Context, root: &Path) {
        let Some((_, asset)) = self.available.clone() else {
            return;
        };
        if self.receiver.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.message = "Downloading and verifying installer…".into();
        let root = root.join("updates");
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Event::Downloaded(download(&asset, &root)));
            ctx.request_repaint();
        });
    }

    pub fn tick(&mut self, ctx: &egui::Context, root: &Path) {
        if !self.loaded {
            match std::fs::read(root.join("updates.json")) {
                Ok(bytes) => match serde_json::from_slice(&bytes) {
                    Ok(preferences) => self.preferences = preferences,
                    Err(e) => self.message = format!("Cannot read update preferences: {e}"),
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => self.message = format!("Cannot read update preferences: {e}"),
            }
            self.loaded = true;
        }
        if let Some(event) = self.receiver.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.receiver = None;
            match event {
                Event::Checked(Ok(Some(found))) => {
                    self.message = format!("{} is available.", found.0.tag_name);
                    self.available = Some(found);
                    if self.preferences.automatic {
                        self.start_download(ctx, root);
                    }
                }
                Event::Checked(Ok(None)) => {
                    self.available = None;
                    self.message = "You’re up to date.".into();
                }
                Event::Checked(Err(e)) | Event::Downloaded(Err(e)) => self.message = e,
                Event::Downloaded(Ok(path)) => {
                    self.ready = Some(path);
                    self.message = "Update verified and ready. Install when you’re ready to close Lab Manager.".into();
                }
            }
        }
        if self.preferences.automatic
            && self.receiver.is_none()
            && self.ready.is_none()
            && self.next_check.is_none_or(|at| Instant::now() >= at)
        {
            self.check(ctx);
        }
        if self.preferences.automatic {
            ctx.request_repaint_after(Duration::from_secs(60));
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, root: &Path) {
        ui.strong("Updates");
        ui.label(format!(
            "Lab Manager {CURRENT} · {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
        ui.small("Updates come from exenreco/Lab-Manager on GitHub. Installers are verified against GitHub’s SHA-256 digest before opening.");
        if ui
            .checkbox(&mut self.preferences.automatic, "Automatic updates")
            .changed()
        {
            let save = std::fs::create_dir_all(root).and_then(|()| {
                std::fs::write(
                    root.join("updates.json"),
                    serde_json::to_vec(&self.preferences).expect("serializable preferences"),
                )
            });
            if let Err(e) = save {
                self.message = format!("Cannot save update preferences: {e}");
            }
        }
        ui.small("Check at launch and every 6 hours while open; download automatically. You choose when to install and restart. Enabled for this Windows installation.");
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.receiver.is_none(),
                    egui::Button::new("Check for updates"),
                )
                .clicked()
            {
                self.check(ui.ctx());
            }
            if self.available.is_some()
                && self.ready.is_none()
                && ui
                    .add_enabled(
                        self.receiver.is_none(),
                        egui::Button::new("Download update"),
                    )
                    .clicked()
            {
                self.start_download(ui.ctx(), root);
            }
            if let Some(path) = &self.ready
                && ui.button(if cfg!(windows) { "Install update and close app" } else { "Open downloaded update" }).clicked()
            {
                let verified = self
                    .available
                    .as_ref()
                    .ok_or_else(|| "Check for updates again before installing.".to_string())
                    .and_then(|(_, asset)| verify(path, asset));
                if let Err(error) = verified {
                    self.message = error;
                    self.ready = None;
                    return;
                }
                let mut command = if cfg!(windows) { let mut command = Command::new(path); command.arg("/SP-"); command } else if cfg!(target_os = "macos") { let mut command = Command::new("open"); command.arg(path); command } else { let mut command = Command::new("xdg-open"); command.arg(path.parent().unwrap_or(root)); command };
                hidden(&mut command);
                match command.spawn() {
                    Ok(_) if cfg!(windows) => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                    Ok(_) => self.message = "Downloaded update opened. Quit Lab Manager before replacing the application; your data stays in your user data folder.".into(),
                    Err(e) => self.message = format!("Cannot open installer: {e}"),
                }
            }
            if self.receiver.is_some() {
                ui.spinner();
            }
        });
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn release(version: &str, prerelease: bool) -> Release {
        Release {
            tag_name: format!("v{version}"),
            draft: false,
            prerelease,
            assets: vec![Asset {
                name: asset_name(&Version::parse(version).unwrap()),
                browser_download_url: format!("{DOWNLOAD_PREFIX}v{version}/setup.exe"),
                digest: Some(format!("sha256:{}", "a".repeat(64))),
                size: 1024,
            }],
        }
    }
    #[test]
    fn update_selection_obeys_versions_channels_and_integrity() {
        assert!(candidate(vec![release("0.1.0-alpha.1", true)], "0.1.0-alpha.1").is_none());
        assert!(candidate(vec![release("0.1.0-alpha.2", true)], "0.1.0-alpha.1").is_some());
        assert!(candidate(vec![release("0.2.0-alpha.1", true)], "0.1.0").is_none());
        assert!(candidate(vec![release("0.2.0-alpha.1", false)], "0.1.0").is_none());
        let mut bad = release("0.2.0", false);
        bad.assets[0].digest = None;
        assert!(candidate(vec![bad], "0.1.0").is_none());
        let mut bad = release("0.2.0", false);
        bad.assets[0].browser_download_url = "https://example.com/setup.exe".into();
        assert!(candidate(vec![bad], "0.1.0").is_none());
        let chosen = candidate(
            vec![release("0.2.0", false), release("0.3.0", false)],
            "0.1.0",
        )
        .unwrap();
        assert_eq!(chosen.0.tag_name, "v0.3.0");
    }

    #[test]
    fn installer_verification_rejects_changed_bytes_and_wrong_size() {
        let path =
            std::env::temp_dir().join(format!("lab-update-integrity-{}", std::process::id()));
        std::fs::write(&path, b"verified installer fixture").unwrap();
        let mut asset = release("0.2.0", false).assets.remove(0);
        asset.digest = Some(format!(
            "sha256:{:x}",
            Sha256::digest(b"verified installer fixture")
        ));
        asset.size = std::fs::metadata(&path).unwrap().len();
        assert!(verify(&path, &asset).is_ok());
        asset.size += 1;
        assert!(verify(&path, &asset).is_err());
        asset.size -= 1;
        std::fs::write(&path, b"modified installer fixture").unwrap();
        assert!(verify(&path, &asset).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
