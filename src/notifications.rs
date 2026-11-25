use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Notification {
    pub message: String,
    pub timestamp: u64,
    pub local_time: String,
    pub read: bool,
}

pub struct NotificationLog {
    pub path: PathBuf,
    pub entries: Vec<Notification>,
}
impl NotificationLog {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let entries = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| format!("Cannot read notification history: {e}"))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(format!("Cannot read notification history: {e}")),
        };
        Ok(Self { path, entries })
    }
    pub fn unread(&self) -> usize {
        self.entries.iter().filter(|n| !n.read).count()
    }
    pub fn record(&mut self, message: String) -> Result<(), String> {
        self.entries.push(Notification {
            message,
            timestamp: now(),
            local_time: local_time(),
            read: false,
        });
        self.save(&self.entries)
    }
    pub fn mark_read(&mut self) -> Result<(), String> {
        let mut updated = self.entries.clone();
        for item in &mut updated {
            item.read = true;
        }
        self.save(&updated)?;
        self.entries = updated;
        Ok(())
    }
    pub fn clear(&mut self) -> Result<(), String> {
        self.save(&[])?;
        self.entries.clear();
        Ok(())
    }
    fn save(&self, entries: &[Notification]) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(entries).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(
            self.path
                .parent()
                .ok_or("Missing notifications directory")?,
        )
        .map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(tmp, &self.path).map_err(|e| e.to_string())
    }
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn age(timestamp: u64) -> String {
    let seconds = now().saturating_sub(timestamp);
    match seconds {
        0..=59 => "Just now".into(),
        60..=3599 => format!("{}m ago", seconds / 60),
        3600..=86399 => format!("{}h ago", seconds / 3600),
        _ => format!("{}d ago", seconds / 86400),
    }
}
fn local_time() -> String {
    #[cfg(windows)]
    {
        let mut time = windows_sys::Win32::Foundation::SYSTEMTIME {
            wYear: 0,
            wMonth: 0,
            wDayOfWeek: 0,
            wDay: 0,
            wHour: 0,
            wMinute: 0,
            wSecond: 0,
            wMilliseconds: 0,
        };
        // SAFETY: GetLocalTime writes a complete SYSTEMTIME to the valid pointer.
        unsafe {
            windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut time);
        }
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} (local)",
            time.wYear, time.wMonth, time.wDay, time.wHour, time.wMinute, time.wSecond
        )
    }
    #[cfg(not(windows))]
    {
        let t = now();
        format!("{:02}:{:02}:{:02} UTC", t / 3600 % 24, t / 60 % 60, t % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_keeps_repeated_events_and_clear_does_not_affect_other_accounts() {
        let root = std::env::temp_dir().join(format!("lab-notifications-{}", std::process::id()));
        let mut alice = NotificationLog::load(root.join("alice.json")).unwrap();
        let bob = NotificationLog::load(root.join("bob.json")).unwrap();
        alice.record("Added 1 server(s).".into()).unwrap();
        alice.record("Added 1 server(s).".into()).unwrap();
        assert_eq!(alice.unread(), 2);
        assert!(bob.entries.is_empty());
        let restored = NotificationLog::load(alice.path.clone()).unwrap();
        assert_eq!(restored.entries.len(), 2);
        assert!(!restored.entries[0].local_time.is_empty());
        alice.mark_read().unwrap();
        assert_eq!(alice.unread(), 0);
        assert_eq!(
            NotificationLog::load(alice.path.clone()).unwrap().unread(),
            0
        );
        alice.clear().unwrap();
        assert!(
            NotificationLog::load(alice.path.clone())
                .unwrap()
                .entries
                .is_empty()
        );
        std::fs::remove_file(&alice.path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
