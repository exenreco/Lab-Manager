use crate::{
    command_history::write_json,
    model::{Server, ServerStore},
    monitoring::{Config, Database, DeviceHistory},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::Path,
};
const MAX_BYTES: u64 = 32 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
pub struct Backup {
    pub version: u32,
    pub created: u64,
    pub devices: Vec<Server>,
    pub monitoring: BTreeMap<String, Config>,
    pub icons: BTreeMap<String, Vec<u8>>,
}
impl Backup {
    pub fn create(root: &Path, devices: &[Server], database: &Database) -> Result<Self, String> {
        let mut icons = BTreeMap::new();
        for id in devices
            .iter()
            .filter_map(|server| server.custom_icon.as_ref())
        {
            crate::icon_assets::validate_id(id)?;
            icons.insert(
                id.clone(),
                std::fs::read(root.join("icons").join(id))
                    .map_err(|e| format!("Cannot back up icon {id}: {e}"))?,
            );
        }
        let monitoring = devices
            .iter()
            .filter_map(|server| {
                let key = crate::monitoring::key(server);
                database.devices.get(&key).map(|history| {
                    let mut config = history.config.clone();
                    config.password_id = None;
                    config.identity_file.clear();
                    config.auth_mode = Some(crate::monitoring::AuthMode::Key);
                    config.maintenance_until = 0;
                    (key, config)
                })
            })
            .collect();
        let backup = Self {
            version: 1,
            created: crate::notifications::now(),
            devices: devices.to_vec(),
            monitoring,
            icons,
        };
        backup.validate()?;
        Ok(backup)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.devices.len() > 4096 || self.icons.len() > 512 {
            return Err("Unsupported backup version or too many devices/icons.".into());
        }
        let mut ips = BTreeSet::new();
        for server in &self.devices {
            server.validate()?;
            if !ips.insert(server.ip) {
                return Err("Backup contains duplicate IP addresses.".into());
            }
            if let Some(id) = &server.custom_icon
                && !self.icons.contains_key(id)
            {
                return Err(format!("Backup is missing icon {id}."));
            }
        }
        let keys: BTreeSet<_> = self.devices.iter().map(crate::monitoring::key).collect();
        for (key, config) in &self.monitoring {
            if !keys.contains(key)
                || config.password_id.is_some()
                || !config.identity_file.is_empty()
                || config.services.len() > 8
                || !(15..=3600).contains(&config.interval)
                || [config.cpu_limit, config.memory_limit, config.disk_limit]
                    .iter()
                    .any(|limit| !limit.is_finite() || !(1.0..=100.0).contains(limit))
            {
                return Err(
                    "Backup has invalid monitoring configuration or credential references.".into(),
                );
            }
            for check in &config.services {
                check.validate()?;
            }
        }
        let mut total = 0;
        for (id, bytes) in &self.icons {
            crate::icon_assets::validate_id(id)?;
            total += bytes.len();
            if bytes.len() > 1024 * 1024 || total > 8 * 1024 * 1024 {
                return Err("Backup icons exceed the size limit.".into());
            }
            let mut reader = image::ImageReader::with_format(
                std::io::Cursor::new(bytes),
                image::ImageFormat::Png,
            );
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(256);
            limits.max_image_height = Some(256);
            limits.max_alloc = Some(1024 * 1024);
            reader.limits(limits);
            reader
                .decode()
                .map_err(|_| "Backup contains an invalid icon".to_owned())?;
        }
        Ok(())
    }
    pub fn read(path: &Path) -> Result<Self, String> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("Backup is larger than 32 MB.".into());
        }
        let backup: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("Invalid backup: {e}"))?;
        backup.validate()?;
        Ok(backup)
    }
    pub fn write(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("Backup exceeds 32 MB.".into());
        }
        write_json(path, self)
    }
}
pub fn restore(
    root: &Path,
    backup: &Backup,
    current: &[Server],
    database: &Database,
    replace: bool,
) -> Result<(Vec<Server>, Database), String> {
    backup.validate()?;
    let rollback = Backup::create(root, current, database)?;
    use rand_core::RngCore;
    let rollback_path = root.join("backups").join(format!(
        "before-restore-{}-{:016x}.json",
        crate::notifications::now(),
        rand_core::OsRng.next_u64()
    ));
    rollback.write(&rollback_path)?;
    let mut incoming = backup.devices.clone();
    std::fs::create_dir_all(root.join("icons")).map_err(|e| e.to_string())?;
    for (id, bytes) in &backup.icons {
        let mut new_id = id.clone();
        if root.join("icons").join(id).exists()
            && std::fs::read(root.join("icons").join(id)).map_err(|e| e.to_string())? != *bytes
        {
            new_id = format!("restore-{:016x}.png", rand_core::OsRng.next_u64());
            for server in &mut incoming {
                if server.custom_icon.as_deref() == Some(id) {
                    server.custom_icon = Some(new_id.clone());
                }
            }
        }
        let path = root.join("icons").join(new_id);
        if !path.exists() {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|e| e.to_string())?;
            file.write_all(bytes)
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
        }
    }
    let mut servers = if replace {
        Vec::new()
    } else {
        current.to_vec()
    };
    for server in incoming {
        if let Some(existing) = servers.iter_mut().find(|s| s.ip == server.ip) {
            *existing = server;
        } else {
            servers.push(server);
        }
    }
    let mut next = if replace {
        Database::default()
    } else {
        database.clone()
    };
    for (key, config) in &backup.monitoring {
        next.devices.insert(
            key.clone(),
            DeviceHistory {
                config: config.clone(),
                samples: Vec::new(),
            },
        );
    }
    let keys: BTreeSet<_> = servers.iter().map(crate::monitoring::key).collect();
    next.devices.retain(|key, _| keys.contains(key));
    let store = ServerStore {
        path: root.join("servers.json"),
    };
    store.save(&servers)?;
    if let Err(error) = write_json(&root.join("monitoring.json"), &next) {
        let rollback_error = store.save(current).err();
        return Err(format!(
            "Restore failed: {error}. Inventory rollback: {}",
            rollback_error.unwrap_or_else(|| "completed".into())
        ));
    }
    Ok((servers, next))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicates_and_credential_references() {
        let server = Server::new("127.0.0.1".parse().unwrap());
        let mut backup = Backup {
            version: 1,
            created: 0,
            devices: vec![server.clone(), server.clone()],
            monitoring: BTreeMap::new(),
            icons: BTreeMap::new(),
        };
        assert!(backup.validate().is_err());
        backup.devices.pop();
        backup.monitoring.insert(
            crate::monitoring::key(&server),
            Config {
                password_id: Some("secret".into()),
                ..Default::default()
            },
        );
        assert!(backup.validate().is_err());
    }
    #[test]
    fn qr_backups_exclude_passwords_and_machine_key_paths() {
        let server = Server::new("192.0.2.1".parse().unwrap());
        let mut database = Database::default();
        let history = database
            .devices
            .entry(crate::monitoring::key(&server))
            .or_default();
        history.config.password_id = Some("secret-record".into());
        history.config.identity_file = "C:/private/id_ed25519".into();
        let backup = Backup::create(std::path::Path::new("."), &[server], &database).unwrap();
        let json = serde_json::to_string(&backup).unwrap();
        assert!(!json.contains("secret-record"));
        assert!(!json.contains("C:/private"));
        assert!(
            backup
                .monitoring
                .values()
                .all(|config| config.password_id.is_none() && config.identity_file.is_empty())
        );
    }

    #[test]
    fn restore_merges_accounts_and_preserves_recovery_copy() {
        let root = std::env::temp_dir().join(format!(
            "lab-backup-{}",
            rand_core::RngCore::next_u64(&mut rand_core::OsRng)
        ));
        let mut current = Server::new("192.0.2.1".parse().unwrap());
        current.group = "Lab".into();
        current.tags = "DNS, core".into();
        current.notes = "Maintenance note".into();
        let backup = Backup::create(&root, &[current.clone()], &Database::default()).unwrap();
        let other = Server::new("192.0.2.2".parse().unwrap());
        let (merged, _) = restore(&root, &backup, &[other], &Database::default(), false).unwrap();
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[1].notes, "Maintenance note");
        assert_eq!(std::fs::read_dir(root.join("backups")).unwrap().count(), 1);
        assert_eq!(
            ServerStore {
                path: root.join("servers.json")
            }
            .load()
            .unwrap(),
            merged
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
