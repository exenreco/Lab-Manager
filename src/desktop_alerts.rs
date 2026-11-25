use std::sync::{
    OnceLock,
    mpsc::{self, SyncSender},
};
static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub fn clear() {
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}
static SENDER: OnceLock<SyncSender<(u64, String)>> = OnceLock::new();
pub fn send(message: &str) {
    let sender = SENDER.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel::<(u64, String)>(8);
        std::thread::spawn(move || {
            for (generation, message) in receiver {
                if generation != GENERATION.load(std::sync::atomic::Ordering::Relaxed) { continue; }
                #[cfg(windows)] {
                    use std::os::windows::process::CommandExt;
                    let _ = std::process::Command::new("powershell.exe")
                        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-STA", "-Command", "Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $n=New-Object System.Windows.Forms.NotifyIcon; try {$n.Icon=[System.Drawing.SystemIcons]::Information; $n.Text='Lab Manager'; $n.Visible=$true; $n.ShowBalloonTip(8000,'Lab Manager',$env:LAB_MANAGER_NOTICE,[System.Windows.Forms.ToolTipIcon]::Info); Start-Sleep -Seconds 8} finally {$n.Dispose()}"])
                        .env("LAB_MANAGER_NOTICE", message).creation_flags(0x08000000)
                        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
                }
                #[cfg(target_os = "linux")]
                { let _ = std::process::Command::new("notify-send").args(["--app-name=Lab Manager", "--", "Lab Manager", &message]).status(); }
                #[cfg(target_os = "macos")]
                { let _ = std::process::Command::new("osascript").args(["-e", "on run argv\ndisplay notification (item 1 of argv) with title \"Lab Manager\"\nend run", "--", &message]).status(); }
            }
        });
        sender
    });
    let _ = sender.try_send((
        GENERATION.load(std::sync::atomic::Ordering::Relaxed),
        message.chars().take(250).collect(),
    ));
}
