use eframe::egui::{self, Color32};
pub fn source(name: &str) -> &'static [u8] {
    match name {
        "activity" => include_bytes!("../assets/icons/activity.svg"),
        "bell" => include_bytes!("../assets/icons/bell.svg"),
        "brand-adguard-home" => include_bytes!("../assets/icons/brand-adguard-home.svg"),
        "brand-apple" => include_bytes!("../assets/icons/brand-apple.svg"),
        "brand-caddy" => include_bytes!("../assets/icons/brand-caddy.svg"),
        "brand-debian" => include_bytes!("../assets/icons/brand-debian.svg"),
        "brand-dell" => include_bytes!("../assets/icons/brand-dell.svg"),
        "brand-docker" => include_bytes!("../assets/icons/brand-docker.svg"),
        "brand-emby" => include_bytes!("../assets/icons/brand-emby.svg"),
        "brand-esphome" => include_bytes!("../assets/icons/brand-esphome.svg"),
        "brand-frigate" => include_bytes!("../assets/icons/brand-frigate.svg"),
        "brand-grafana" => include_bytes!("../assets/icons/brand-grafana.svg"),
        "brand-home-assistant" => include_bytes!("../assets/icons/brand-home-assistant.svg"),
        "brand-immich" => include_bytes!("../assets/icons/brand-immich.svg"),
        "brand-jellyfin" => include_bytes!("../assets/icons/brand-jellyfin.svg"),
        "brand-kubernetes" => include_bytes!("../assets/icons/brand-kubernetes.svg"),
        "brand-linux" => include_bytes!("../assets/icons/brand-linux.svg"),
        "brand-nextcloud" => include_bytes!("../assets/icons/brand-nextcloud.svg"),
        "brand-nginx" => include_bytes!("../assets/icons/brand-nginx.svg"),
        "brand-opnsense" => include_bytes!("../assets/icons/brand-opnsense.svg"),
        "brand-pfsense" => include_bytes!("../assets/icons/brand-pfsense.svg"),
        "brand-pi-hole" => include_bytes!("../assets/icons/brand-pi-hole.svg"),
        "brand-plex" => include_bytes!("../assets/icons/brand-plex.svg"),
        "brand-portainer" => include_bytes!("../assets/icons/brand-portainer.svg"),
        "brand-prometheus" => include_bytes!("../assets/icons/brand-prometheus.svg"),
        "brand-proxmox" => include_bytes!("../assets/icons/brand-proxmox.svg"),
        "brand-qbittorrent" => include_bytes!("../assets/icons/brand-qbittorrent.svg"),
        "brand-qnap" => include_bytes!("../assets/icons/brand-qnap.svg"),
        "brand-radarr" => include_bytes!("../assets/icons/brand-radarr.svg"),
        "brand-raspberry-pi" => include_bytes!("../assets/icons/brand-raspberry-pi.svg"),
        "brand-sabnzbd" => include_bytes!("../assets/icons/brand-sabnzbd.svg"),
        "brand-sonarr" => include_bytes!("../assets/icons/brand-sonarr.svg"),
        "brand-synology" => include_bytes!("../assets/icons/brand-synology.svg"),
        "brand-tailscale" => include_bytes!("../assets/icons/brand-tailscale.svg"),
        "brand-traefik" => include_bytes!("../assets/icons/brand-traefik.svg"),
        "brand-truenas" => include_bytes!("../assets/icons/brand-truenas.svg"),
        "brand-ubiquiti" => include_bytes!("../assets/icons/brand-ubiquiti.svg"),
        "brand-ubuntu" => include_bytes!("../assets/icons/brand-ubuntu.svg"),
        "brand-unraid" => include_bytes!("../assets/icons/brand-unraid.svg"),
        "brand-vaultwarden" => include_bytes!("../assets/icons/brand-vaultwarden.svg"),
        "brand-windows" => include_bytes!("../assets/icons/brand-windows.svg"),
        "brand-wireguard" => include_bytes!("../assets/icons/brand-wireguard.svg"),
        "cable" => include_bytes!("../assets/icons/cable.svg"),
        "check" => include_bytes!("../assets/icons/check.svg"),
        "chevron-down" => include_bytes!("../assets/icons/chevron-down.svg"),
        "chevron-left" => include_bytes!("../assets/icons/chevron-left.svg"),
        "chevron-right" => include_bytes!("../assets/icons/chevron-right.svg"),
        "chevrons-up-down" => include_bytes!("../assets/icons/chevrons-up-down.svg"),
        "circle-help" => include_bytes!("../assets/icons/circle-help.svg"),
        "clock" => include_bytes!("../assets/icons/clock.svg"),
        "cloud" => include_bytes!("../assets/icons/cloud.svg"),
        "copy" => include_bytes!("../assets/icons/copy.svg"),
        "database" => include_bytes!("../assets/icons/database.svg"),
        "download" => include_bytes!("../assets/icons/download.svg"),
        "ellipsis-vertical" => include_bytes!("../assets/icons/ellipsis-vertical.svg"),
        "external-link" => include_bytes!("../assets/icons/external-link.svg"),
        "file-text" => include_bytes!("../assets/icons/file-text.svg"),
        "folder" => include_bytes!("../assets/icons/folder.svg"),
        "globe" => include_bytes!("../assets/icons/globe.svg"),
        "hard-drive" => include_bytes!("../assets/icons/hard-drive.svg"),
        "layout-grid" => include_bytes!("../assets/icons/layout-grid.svg"),
        "list" => include_bytes!("../assets/icons/list.svg"),
        "lock" => include_bytes!("../assets/icons/lock.svg"),
        "log-out" => include_bytes!("../assets/icons/log-out.svg"),
        "minus" => include_bytes!("../assets/icons/minus.svg"),
        "monitor" => include_bytes!("../assets/icons/monitor.svg"),
        "network" => include_bytes!("../assets/icons/network.svg"),
        "panels-top-left" => include_bytes!("../assets/icons/panels-top-left.svg"),
        "pin-off" => include_bytes!("../assets/icons/pin-off.svg"),
        "pin" => include_bytes!("../assets/icons/pin.svg"),
        "plus" => include_bytes!("../assets/icons/plus.svg"),
        "power" => include_bytes!("../assets/icons/power.svg"),
        "radar" => include_bytes!("../assets/icons/radar.svg"),
        "redo-2" => include_bytes!("../assets/icons/redo-2.svg"),
        "refresh-cw" => include_bytes!("../assets/icons/refresh-cw.svg"),
        "router" => include_bytes!("../assets/icons/router.svg"),
        "search" => include_bytes!("../assets/icons/search.svg"),
        "server" => include_bytes!("../assets/icons/server.svg"),
        "settings" => include_bytes!("../assets/icons/settings.svg"),
        "shield" => include_bytes!("../assets/icons/shield.svg"),
        "square" => include_bytes!("../assets/icons/square.svg"),
        "tag" => include_bytes!("../assets/icons/tag.svg"),
        "terminal" => include_bytes!("../assets/icons/terminal.svg"),
        "trash-2" => include_bytes!("../assets/icons/trash-2.svg"),
        "undo-2" => include_bytes!("../assets/icons/undo-2.svg"),
        "unplug" => include_bytes!("../assets/icons/unplug.svg"),
        "upload" => include_bytes!("../assets/icons/upload.svg"),
        "user" => include_bytes!("../assets/icons/user.svg"),
        "x" => include_bytes!("../assets/icons/x.svg"),
        _ => include_bytes!("../assets/icons/server.svg"),
    }
}
const MAX_CACHE_BYTES: usize = 16 * 1024 * 1024;
#[derive(Default)]
struct IconCache {
    entries: std::collections::VecDeque<(String, u32, egui::TextureHandle)>,
    bytes: usize,
}
impl IconCache {
    fn get(&mut self, name: &str, pixels: u32) -> Option<egui::TextureHandle> {
        let index = self
            .entries
            .iter()
            .position(|(key, size, _)| key == name && *size == pixels)?;
        let entry = self.entries.remove(index)?;
        let texture = entry.2.clone();
        self.entries.push_back(entry);
        Some(texture)
    }
    fn insert(&mut self, name: &str, pixels: u32, texture: egui::TextureHandle) {
        let bytes = pixels as usize * pixels as usize * 4;
        while self.bytes + bytes > MAX_CACHE_BYTES || self.entries.len() >= 192 {
            let Some((_, size, _)) = self.entries.pop_front() else {
                break;
            };
            self.bytes -= size as usize * size as usize * 4;
        }
        self.bytes += bytes;
        self.entries.push_back((name.into(), pixels, texture));
    }
}
pub fn paint(p: &egui::Painter, rect: egui::Rect, name: &str, tint: Option<Color32>) {
    let pixels = (rect.width().max(rect.height()) * p.ctx().pixels_per_point())
        .ceil()
        .clamp(16.0, 512.0) as u32;
    let pixels = pixels.next_power_of_two();
    let id = egui::Id::new("bounded_svg_cache");
    let cache = p.ctx().data_mut(|d| {
        d.get_temp_mut_or_default::<std::sync::Arc<std::sync::Mutex<IconCache>>>(id)
            .clone()
    });
    let texture = cache
        .lock()
        .ok()
        .and_then(|mut cache| cache.get(name, pixels))
        .or_else(|| {
            let text = std::str::from_utf8(source(name))
                .ok()?
                .replace("currentColor", "#ffffff");
            let tree = resvg::usvg::Tree::from_data(text.as_bytes(), &Default::default()).ok()?;
            let mut pixmap = resvg::tiny_skia::Pixmap::new(pixels, pixels)?;
            let size = tree.size();
            let scale = (pixels as f32 / size.width()).min(pixels as f32 / size.height());
            let transform = resvg::tiny_skia::Transform::from_translate(
                (pixels as f32 - size.width() * scale) / 2.0,
                (pixels as f32 - size.height() * scale) / 2.0,
            )
            .pre_scale(scale, scale);
            resvg::render(&tree, transform, &mut pixmap.as_mut());
            let image = egui::ColorImage::from_rgba_premultiplied(
                [pixels as usize, pixels as usize],
                pixmap.data(),
            );
            let texture = p.ctx().load_texture(
                format!("svg-{name}-{pixels}"),
                image,
                egui::TextureOptions::LINEAR,
            );
            if let Ok(mut cache) = cache.lock() {
                cache.insert(name, pixels, texture.clone());
            }
            Some(texture)
        });
    if let Some(texture) = texture {
        p.image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            tint.unwrap_or(Color32::WHITE),
        );
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn resized_icons_keep_cache_memory_bounded_and_evict_old_entries() {
        let ctx = eframe::egui::Context::default();
        let texture = ctx.load_texture(
            "cache-test",
            eframe::egui::ColorImage::new([1, 1], eframe::egui::Color32::WHITE),
            Default::default(),
        );
        let mut cache = super::IconCache::default();
        for index in 0..1000 {
            cache.insert(&format!("icon-{index}"), 512, texture.clone());
            assert!(cache.bytes <= super::MAX_CACHE_BYTES);
            assert!(cache.entries.len() <= 192);
        }
        assert!(cache.get("icon-0", 512).is_none());
        assert!(cache.get("icon-999", 512).is_some());
    }

    #[test]
    fn every_bundled_svg_parses_and_renders() {
        for (name, bytes) in [
            ("activity", super::source("activity")),
            ("bell", super::source("bell")),
            ("brand-adguard-home", super::source("brand-adguard-home")),
            ("brand-apple", super::source("brand-apple")),
            ("brand-caddy", super::source("brand-caddy")),
            ("brand-debian", super::source("brand-debian")),
            ("brand-dell", super::source("brand-dell")),
            ("brand-docker", super::source("brand-docker")),
            ("brand-emby", super::source("brand-emby")),
            ("brand-esphome", super::source("brand-esphome")),
            ("brand-frigate", super::source("brand-frigate")),
            ("brand-grafana", super::source("brand-grafana")),
            (
                "brand-home-assistant",
                super::source("brand-home-assistant"),
            ),
            ("brand-immich", super::source("brand-immich")),
            ("brand-jellyfin", super::source("brand-jellyfin")),
            ("brand-kubernetes", super::source("brand-kubernetes")),
            ("brand-linux", super::source("brand-linux")),
            ("brand-nextcloud", super::source("brand-nextcloud")),
            ("brand-nginx", super::source("brand-nginx")),
            ("brand-opnsense", super::source("brand-opnsense")),
            ("brand-pfsense", super::source("brand-pfsense")),
            ("brand-pi-hole", super::source("brand-pi-hole")),
            ("brand-plex", super::source("brand-plex")),
            ("brand-portainer", super::source("brand-portainer")),
            ("brand-prometheus", super::source("brand-prometheus")),
            ("brand-proxmox", super::source("brand-proxmox")),
            ("brand-qbittorrent", super::source("brand-qbittorrent")),
            ("brand-qnap", super::source("brand-qnap")),
            ("brand-radarr", super::source("brand-radarr")),
            ("brand-raspberry-pi", super::source("brand-raspberry-pi")),
            ("brand-sabnzbd", super::source("brand-sabnzbd")),
            ("brand-sonarr", super::source("brand-sonarr")),
            ("brand-synology", super::source("brand-synology")),
            ("brand-tailscale", super::source("brand-tailscale")),
            ("brand-traefik", super::source("brand-traefik")),
            ("brand-truenas", super::source("brand-truenas")),
            ("brand-ubiquiti", super::source("brand-ubiquiti")),
            ("brand-ubuntu", super::source("brand-ubuntu")),
            ("brand-unraid", super::source("brand-unraid")),
            ("brand-vaultwarden", super::source("brand-vaultwarden")),
            ("brand-windows", super::source("brand-windows")),
            ("brand-wireguard", super::source("brand-wireguard")),
            ("cable", super::source("cable")),
            ("check", super::source("check")),
            ("chevron-down", super::source("chevron-down")),
            ("chevron-left", super::source("chevron-left")),
            ("chevron-right", super::source("chevron-right")),
            ("chevrons-up-down", super::source("chevrons-up-down")),
            ("circle-help", super::source("circle-help")),
            ("clock", super::source("clock")),
            ("cloud", super::source("cloud")),
            ("copy", super::source("copy")),
            ("database", super::source("database")),
            ("download", super::source("download")),
            ("ellipsis-vertical", super::source("ellipsis-vertical")),
            ("external-link", super::source("external-link")),
            ("file-text", super::source("file-text")),
            ("folder", super::source("folder")),
            ("globe", super::source("globe")),
            ("hard-drive", super::source("hard-drive")),
            ("layout-grid", super::source("layout-grid")),
            ("list", super::source("list")),
            ("lock", super::source("lock")),
            ("log-out", super::source("log-out")),
            ("minus", super::source("minus")),
            ("monitor", super::source("monitor")),
            ("network", super::source("network")),
            ("panels-top-left", super::source("panels-top-left")),
            ("pin-off", super::source("pin-off")),
            ("pin", super::source("pin")),
            ("plus", super::source("plus")),
            ("power", super::source("power")),
            ("radar", super::source("radar")),
            ("redo-2", super::source("redo-2")),
            ("refresh-cw", super::source("refresh-cw")),
            ("router", super::source("router")),
            ("search", super::source("search")),
            ("server", super::source("server")),
            ("settings", super::source("settings")),
            ("shield", super::source("shield")),
            ("square", super::source("square")),
            ("tag", super::source("tag")),
            ("terminal", super::source("terminal")),
            ("trash-2", super::source("trash-2")),
            ("undo-2", super::source("undo-2")),
            ("unplug", super::source("unplug")),
            ("upload", super::source("upload")),
            ("user", super::source("user")),
            ("x", super::source("x")),
        ] {
            let text = std::str::from_utf8(bytes)
                .unwrap()
                .replace("currentColor", "#ffffff");
            let tree = resvg::usvg::Tree::from_data(text.as_bytes(), &Default::default())
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let mut pixmap = resvg::tiny_skia::Pixmap::new(64, 64).unwrap();
            let scale = 64.0 / tree.size().width().max(tree.size().height());
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(scale, scale),
                &mut pixmap.as_mut(),
            );
            assert!(
                pixmap.data().chunks(4).any(|pixel| pixel[3] > 0),
                "{name} rendered empty"
            );
        }
    }
}
