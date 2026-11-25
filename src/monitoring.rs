use crate::model::Server;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

pub const HISTORY_LIMIT: usize = 1440;
#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Debug)]
pub enum Platform {
    #[default]
    Linux,
    Windows,
    MacOs,
    Unifi,
    Idrac,
}
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum AuthMode {
    Key,
    Password,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub enabled: bool,
    pub host_metrics: bool,
    pub platform: Platform,
    pub platform_detected: bool,
    pub interval: u64,
    pub cpu_limit: f64,
    pub memory_limit: f64,
    pub disk_limit: f64,
    pub alerts: bool,
    pub password_id: Option<String>,
    pub identity_file: String,
    pub auth_mode: Option<AuthMode>,
    pub maintenance_until: u64,
    pub desktop_alerts: bool,
    pub services: Vec<crate::service_checks::ServiceCheck>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            host_metrics: false,
            platform: Platform::Linux,
            platform_detected: false,
            interval: 60,
            cpu_limit: 90.0,
            memory_limit: 90.0,
            disk_limit: 90.0,
            alerts: true,
            password_id: None,
            identity_file: String::new(),
            auth_mode: None,
            maintenance_until: 0,
            desktop_alerts: false,
            services: Vec::new(),
        }
    }
}
impl Config {
    pub fn maintenance_active(&self) -> bool {
        self.maintenance_until > crate::notifications::now()
    }
    pub fn ssh_identity(&self) -> Result<Option<std::path::PathBuf>, String> {
        let value = self.identity_file.trim();
        if value.is_empty() {
            return Ok(None);
        }
        if value.len() > 4096 || value.contains(['\0', '\r', '\n']) {
            return Err("Enter a valid SSH private key file path.".into());
        }
        let path = if let Some(suffix) = value
            .strip_prefix("~/")
            .or_else(|| value.strip_prefix("~\\"))
        {
            directories::BaseDirs::new()
                .ok_or("Home directory unavailable")?
                .home_dir()
                .join(suffix)
        } else {
            std::path::PathBuf::from(value)
        };
        Ok(Some(path))
    }

    pub fn uses_password(&self) -> bool {
        self.auth_mode.unwrap_or(if self.password_id.is_some() {
            AuthMode::Password
        } else {
            AuthMode::Key
        }) == AuthMode::Password
    }
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Sample {
    pub timestamp: u64,
    pub web: bool,
    pub ssh: bool,
    pub latency_ms: Option<f64>,
    pub http_status: Option<u16>,
    pub certificate_days: Option<i64>,
    pub cpu: Option<f64>,
    pub memory: Option<f64>,
    pub disk: Option<f64>,
    pub uptime: Option<f64>,
    pub received: Option<f64>,
    pub sent: Option<f64>,
    pub error: Option<String>,
    #[serde(default)]
    pub services: Vec<crate::service_checks::ServiceResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware: Option<crate::idrac::Hardware>,
}
impl Sample {
    pub fn online(&self) -> bool {
        self.web || self.ssh
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DeviceHistory {
    pub config: Config,
    pub samples: Vec<Sample>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Database {
    pub devices: BTreeMap<String, DeviceHistory>,
}
pub fn key(server: &Server) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        server.ip, server.web_port, server.ssh_port, server.username, server.https
    )
}

#[derive(Default)]
pub struct Monitoring {
    pub database: Database,
    pub error: String,
    pub selected: Option<String>,
    pub paused: bool,
    pub settings_open: bool,
    pub password_draft: zeroize::Zeroizing<String>,
    pub password_target: Option<String>,
    pub credential_message: String,
    pub offline: HashSet<String>,
    pub desktop_notices: Vec<String>,
    path: Option<PathBuf>,
    last: BTreeMap<String, Instant>,
    receiver: Option<mpsc::Receiver<(String, Sample)>>,
    busy: HashSet<String>,
    cancel: Arc<AtomicBool>,
    raised: BTreeMap<String, Vec<String>>,
    history_dirty: bool,
}
impl Monitoring {
    pub fn load(&mut self, path: PathBuf) {
        self.reset();
        self.path = Some(path.clone());
        match std::fs::read(path) {
            Ok(bytes) => match serde_json::from_slice::<Database>(&bytes) {
                Ok(mut data) => {
                    for h in data.devices.values_mut() {
                        let keep = h.samples.len().saturating_sub(HISTORY_LIMIT);
                        h.samples.drain(..keep);
                        h.config.interval = h.config.interval.clamp(15, 3600);
                    }
                    self.database = data;
                }
                Err(e) => self.error = format!("Cannot load monitoring history: {e}"),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => self.error = format!("Cannot load monitoring history: {e}"),
        }
    }
    pub fn reset(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        *self = Self::default();
    }
    pub fn credential_root(&self) -> Option<PathBuf> {
        self.path
            .as_ref()
            .map(|p| p.with_file_name("monitoring-credentials"))
    }
    pub fn busy(&self) -> bool {
        !self.busy.is_empty()
    }
    pub fn set_offline(&mut self, offline: HashSet<String>) {
        if self.offline == offline {
            return;
        }
        if self.busy.iter().any(|k| offline.contains(k)) {
            self.cancel.store(true, Ordering::Relaxed);
            self.receiver = None;
            self.busy.clear();
            self.cancel = Arc::new(AtomicBool::new(false));
            self.last.clear();
        }
        for recovered in self.offline.difference(&offline) {
            self.last.remove(recovered);
        }
        self.offline = offline;
    }
    pub fn set_paused(&mut self, paused: bool) {
        if self.paused == paused {
            return;
        }
        self.paused = paused;
        self.cancel.store(true, Ordering::Relaxed);
        self.receiver = None;
        self.busy.clear();
        self.cancel = Arc::new(AtomicBool::new(false));
        self.last.clear();
        if paused {
            self.settings_open = false;
            self.password_draft = Default::default();
        }
    }
    pub fn refresh(&mut self) {
        self.last.clear();
    }
    pub fn save(&mut self) {
        if let Err(e) = self.save_inner() {
            self.error = e;
        }
    }
    fn save_inner(&self) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let bytes = serde_json::to_vec(&self.database).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(path.parent().ok_or("Missing monitoring directory")?)
            .map_err(|e| e.to_string())?;
        let temp = path.with_extension("json.tmp");
        use std::io::Write;
        let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(temp, path).map_err(|e| format!("Cannot save monitoring history: {e}"))
    }
    pub fn update(&mut self, servers: &[Server]) -> Vec<String> {
        let names: BTreeMap<_, _> = servers.iter().map(|s| (key(s), s.name.clone())).collect();
        let mut changed = false;
        let before = self.database.devices.len();
        self.database.devices.retain(|k, _| names.contains_key(k));
        changed |= before != self.database.devices.len();
        self.last.retain(|k, _| names.contains_key(k));
        self.raised.retain(|k, _| names.contains_key(k));
        for s in servers {
            let history = self.database.devices.entry(key(s)).or_default();
            if !history.config.platform_detected {
                if s.icon == crate::model::ServerIcon::DellIdrac
                    && history.config.platform == Platform::Linux
                {
                    history.config.platform = Platform::Idrac;
                }
                history.config.platform_detected = true;
                changed = true;
            }
        }
        let mut notices = Vec::new();
        if !self.paused {
            for server in servers
                .iter()
                .filter(|server| self.offline.contains(&key(server)))
            {
                let k = key(server);
                let config = &self.database.devices[&k].config;
                if config.enabled && config.alerts && !config.maintenance_active() {
                    let previous = self.raised.entry(k).or_default();
                    let condition = "device is offline".to_owned();
                    if !previous.contains(&condition) {
                        let message =
                            format!("Monitoring warning: {} — device is offline", server.name);
                        if config.desktop_alerts {
                            self.desktop_notices.push(message.clone());
                        }
                        notices.push(message);
                        previous.push(condition);
                    }
                }
            }
        }
        let mut finished = false;
        if let Some(receiver) = &self.receiver {
            while let Ok((key, sample)) = receiver.try_recv() {
                self.busy.remove(&key);
                let Some(history) = self.database.devices.get_mut(&key) else {
                    continue;
                };
                if !history.config.enabled {
                    continue;
                }
                let conditions = conditions(&sample, &history.config);
                let previous = self.raised.entry(key.clone()).or_default();
                let notice_start = notices.len();
                if history.config.alerts && !history.config.maintenance_active() {
                    for condition in &conditions {
                        if !previous.contains(condition) {
                            notices.push(format!(
                                "Monitoring warning: {} — {condition}",
                                names.get(&key).map(String::as_str).unwrap_or("Device")
                            ));
                        }
                    }
                    if !previous.is_empty() && conditions.is_empty() {
                        notices.push(format!(
                            "Monitoring restored: {} is healthy.",
                            names.get(&key).map(String::as_str).unwrap_or("Device")
                        ));
                    }
                }
                if history.config.desktop_alerts {
                    self.desktop_notices
                        .extend(notices[notice_start..].iter().cloned());
                }
                *previous = conditions;
                if let Some(hardware) = history
                    .samples
                    .last_mut()
                    .and_then(|sample| sample.hardware.as_mut())
                {
                    hardware.compact();
                }
                history.samples.push(sample);
                if history.samples.len() > HISTORY_LIMIT {
                    history.samples.remove(0);
                }
                changed = true;
            }
            // Each queued device produces exactly one result.
            if self.busy.is_empty() {
                finished = true;
            }
        }
        if finished {
            self.receiver = None;
        }
        self.history_dirty |= changed;
        if self.history_dirty && self.receiver.is_none() {
            self.save();
            self.history_dirty = false;
        }
        if self.paused || self.receiver.is_some() || !self.error.is_empty() {
            return notices;
        }
        let due: Vec<_> = servers
            .iter()
            .filter_map(|server| {
                let k = key(server);
                let config = &self.database.devices[&k].config;
                (config.enabled
                    && !self.offline.contains(&k)
                    && self.last.get(&k).is_none_or(|t| {
                        t.elapsed()
                            >= Duration::from_secs(config.interval.clamp(
                                if config.platform == Platform::Idrac {
                                    60
                                } else {
                                    15
                                },
                                3600,
                            ))
                    }))
                .then(|| (server.clone(), k, config.clone()))
            })
            .collect();
        if due.is_empty() {
            return notices;
        }
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        for (_, k, _) in &due {
            self.busy.insert(k.clone());
            self.last.insert(k.clone(), Instant::now());
        }
        let cancel = self.cancel.clone();
        let credential_root = self.credential_root();
        std::thread::spawn(move || {
            use std::sync::atomic::AtomicUsize;
            let next = AtomicUsize::new(0);
            std::thread::scope(|scope| {
                for _ in 0..4.min(due.len()) {
                    let (due, next, tx, cancel, credential_root) =
                        (&due, &next, tx.clone(), &cancel, &credential_root);
                    scope.spawn(move || {
                        loop {
                            if cancel.load(Ordering::Relaxed) {
                                break;
                            }
                            let Some((server, key, config)) =
                                due.get(next.fetch_add(1, Ordering::Relaxed))
                            else {
                                break;
                            };
                            let sample = crate::monitoring_probe::probe_with_credentials(
                                server,
                                config,
                                cancel,
                                credential_root.as_deref(),
                            );
                            if cancel.load(Ordering::Relaxed)
                                || tx.send((key.clone(), sample)).is_err()
                            {
                                break;
                            }
                        }
                    });
                }
            });
        });
        notices
    }
}
impl Drop for Monitoring {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if self.history_dirty {
            let _ = self.save_inner();
        }
    }
}
pub fn conditions(s: &Sample, c: &Config) -> Vec<String> {
    if c.maintenance_active() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if !s.online() {
        out.push("device is offline".into());
    } else {
        if !s.web {
            out.push("web port is unavailable".into());
        }
        if !s.ssh {
            out.push("SSH port is unavailable".into());
        }
    }
    if let Some(hardware) = &s.hardware {
        out.extend(
            hardware
                .issues
                .iter()
                .map(|issue| format!("iDRAC hardware: {issue}")),
        );
    }
    for (label, value, limit) in [
        ("CPU", s.cpu, c.cpu_limit),
        ("Memory", s.memory, c.memory_limit),
        ("Disk", s.disk, c.disk_limit),
    ] {
        if value.is_some_and(|v| v >= limit) {
            out.push(format!("{label} exceeds {limit:.0}%"));
        }
    }
    if s.http_status.is_some_and(|v| v >= 400) {
        out.push(format!("HTTP returned {}", s.http_status.unwrap()));
    }
    if s.certificate_days.is_some_and(|v| v <= 30) {
        out.push(format!(
            "TLS certificate expires in {} days",
            s.certificate_days.unwrap()
        ));
    }
    if let Some(error) = &s.error {
        out.push(error.clone());
    }
    for service in &s.services {
        if !service.online {
            out.push(format!("{}: {}", service.name, service.detail));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn idrac_platform_is_detected_once_without_overwriting_credentials_or_later_choices() {
        let mut server = super::Server::new("192.0.2.9".parse().unwrap());
        server.icon = crate::model::ServerIcon::DellIdrac;
        let mut monitoring = super::Monitoring::default();
        monitoring.paused = true;
        let history = monitoring
            .database
            .devices
            .entry(super::key(&server))
            .or_default();
        history.config.password_id = Some("existing-credential".into());
        monitoring.update(std::slice::from_ref(&server));
        let config = &mut monitoring
            .database
            .devices
            .get_mut(&super::key(&server))
            .unwrap()
            .config;
        assert_eq!(config.platform, super::Platform::Idrac);
        assert_eq!(config.password_id.as_deref(), Some("existing-credential"));
        config.platform = super::Platform::Linux;
        monitoring.update(std::slice::from_ref(&server));
        assert_eq!(
            monitoring.database.devices[&super::key(&server)]
                .config
                .platform,
            super::Platform::Linux
        );
    }
    #[test]
    fn monitoring_history_is_saved_once_when_the_batch_finishes() {
        use super::*;
        let servers = [
            Server::new("192.0.2.1".parse().unwrap()),
            Server::new("192.0.2.2".parse().unwrap()),
        ];
        let path = std::env::temp_dir().join(format!(
            "lab-history-batch-{}.json",
            rand_core::RngCore::next_u64(&mut rand_core::OsRng)
        ));
        let (tx, rx) = mpsc::channel();
        let mut monitoring = Monitoring::default();
        monitoring.path = Some(path.clone());
        monitoring.paused = true;
        monitoring.receiver = Some(rx);
        for server in &servers {
            monitoring.busy.insert(key(server));
        }
        tx.send((key(&servers[0]), Sample::default())).unwrap();
        monitoring.update(&servers);
        assert!(
            !path.exists(),
            "partial batches should not force a disk sync"
        );
        tx.send((key(&servers[1]), Sample::default())).unwrap();
        monitoring.update(&servers);
        let saved: Database = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved.devices.len(), 2);
        assert!(
            saved
                .devices
                .values()
                .all(|history| history.samples.len() == 1)
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn maintenance_suppresses_alerts_and_service_failures_raise_conditions() {
        let sample = super::Sample {
            web: true,
            ssh: true,
            services: vec![crate::service_checks::ServiceResult {
                name: "DNS".into(),
                online: false,
                detail: "Timed out".into(),
            }],
            ..Default::default()
        };
        let mut config = super::Config::default();
        assert_eq!(super::conditions(&sample, &config), vec!["DNS: Timed out"]);
        config.maintenance_until = crate::notifications::now() + 60;
        assert!(super::conditions(&sample, &config).is_empty());
        config.maintenance_until = 1;
        assert_eq!(super::conditions(&sample, &config).len(), 1);
    }
    #[test]
    fn offline_alert_is_deduplicated_without_running_device_probes() {
        let server = super::Server::new("192.0.2.44".parse().unwrap());
        let mut monitor = super::Monitoring::default();
        monitor.set_offline([super::key(&server)].into_iter().collect());
        assert_eq!(monitor.update(std::slice::from_ref(&server)).len(), 1);
        assert!(monitor.update(&[server]).is_empty());
        assert!(!monitor.busy());
    }
    use super::*;
    #[test]
    fn offline_devices_are_not_scheduled_and_recovery_releases_cooldown() {
        let s = Server::new("192.0.2.100".parse().unwrap());
        let k = key(&s);
        let mut m = Monitoring::default();
        m.set_offline([k.clone()].into_iter().collect());
        m.update(std::slice::from_ref(&s));
        assert!(!m.busy());
        assert!(m.receiver.is_none());
        assert!(m.database.devices[&k].samples.is_empty());
        m.last.insert(k.clone(), Instant::now());
        m.set_offline(HashSet::new());
        assert!(!m.last.contains_key(&k));
    }
    #[test]
    fn explicit_auth_mode_preserves_legacy_passwords_and_supports_key_switching() {
        let mut c: Config =
            serde_json::from_str(r#"{"password_id":"encrypted-record-id"}"#).unwrap();
        assert!(c.uses_password());
        c.auth_mode = Some(AuthMode::Key);
        assert!(!c.uses_password());
        assert!(c.password_id.is_some());
        let mut restored: Config =
            serde_json::from_slice(&serde_json::to_vec(&c).unwrap()).unwrap();
        assert!(!restored.uses_password());
        restored.auth_mode = Some(AuthMode::Password);
        assert!(restored.uses_password());
    }
    #[test]
    fn pause_cancels_pending_work_and_resume_uses_fresh_cycle() {
        let mut m = Monitoring::default();
        let cancel = m.cancel.clone();
        m.busy.insert("pending".into());
        m.last.insert("pending".into(), Instant::now());
        m.settings_open = true;
        m.set_paused(true);
        assert!(cancel.load(Ordering::Relaxed));
        assert!(m.paused);
        assert!(!m.busy());
        assert!(m.last.is_empty());
        assert!(!m.settings_open);
        m.set_paused(false);
        assert!(!m.paused);
        assert!(!m.cancel.load(Ordering::Relaxed));
    }
    fn deliver(m: &mut Monitoring, s: &Server, sample: Sample) -> Vec<String> {
        let k = key(s);
        let (tx, rx) = mpsc::channel();
        m.receiver = Some(rx);
        m.busy.insert(k.clone());
        tx.send((k, sample)).unwrap();
        drop(tx);
        m.update(std::slice::from_ref(s))
    }
    #[test]
    fn bounded_history_alert_transitions_and_account_reset() {
        let server = Server::new("192.0.2.20".parse().unwrap());
        let mut m = Monitoring::default();
        m.paused = true;
        m.database.devices.entry(key(&server)).or_default().samples =
            vec![Sample::default(); HISTORY_LIMIT];
        let first = deliver(&mut m, &server, Sample::default());
        assert_eq!(first.len(), 1);
        assert!(deliver(&mut m, &server, Sample::default()).is_empty());
        let recovered = deliver(
            &mut m,
            &server,
            Sample {
                web: true,
                ssh: true,
                ..Default::default()
            },
        );
        assert!(recovered[0].contains("restored"));
        assert_eq!(
            m.database.devices[&key(&server)].samples.len(),
            HISTORY_LIMIT
        );
        let cancellation = m.cancel.clone();
        m.reset();
        assert!(cancellation.load(Ordering::Relaxed));
        assert!(m.database.devices.is_empty());
        assert!(!m.busy());
    }
    #[test]
    fn monitoring_config_and_history_survive_reload_in_account_directory() {
        let root = std::env::temp_dir().join(format!("lab-monitoring-{}", std::process::id()));
        let alice = crate::model::ServerStore::for_user(&root, "alice")
            .unwrap()
            .path
            .with_file_name("monitoring.json");
        let bob = crate::model::ServerStore::for_user(&root, "bob")
            .unwrap()
            .path
            .with_file_name("monitoring.json");
        let server = Server::new("192.0.2.30".parse().unwrap());
        let mut m = Monitoring::default();
        m.load(alice.clone());
        m.paused = true;
        m.database
            .devices
            .entry(key(&server))
            .or_default()
            .config
            .interval = 120;
        deliver(
            &mut m,
            &server,
            Sample {
                web: true,
                ssh: true,
                cpu: Some(12.0),
                ..Default::default()
            },
        );
        assert!(m.error.is_empty());
        let mut reloaded = Monitoring::default();
        reloaded.load(alice);
        assert_eq!(
            reloaded.database.devices[&key(&server)]
                .samples
                .last()
                .unwrap()
                .cpu,
            Some(12.0)
        );
        assert_eq!(
            reloaded.database.devices[&key(&server)].config.interval,
            120
        );
        reloaded.load(bob);
        assert!(reloaded.database.devices.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }
    #[test]
    fn thresholds_and_offline_alerts_are_real_conditions() {
        let mut s = Sample {
            web: true,
            ssh: true,
            cpu: Some(95.0),
            certificate_days: Some(10),
            ..Default::default()
        };
        assert_eq!(conditions(&s, &Config::default()).len(), 2);
        s.cpu = Some(4.0);
        s.certificate_days = Some(80);
        assert!(conditions(&s, &Config::default()).is_empty());
        s.web = false;
        s.ssh = false;
        assert!(conditions(&s, &Config::default())[0].contains("offline"));
    }
    #[test]
    fn endpoint_configuration_changes_do_not_reuse_old_history() {
        let mut s = Server::new("127.0.0.1".parse().unwrap());
        let old = key(&s);
        s.ssh_port = 2222;
        assert_ne!(old, key(&s));
    }
}
