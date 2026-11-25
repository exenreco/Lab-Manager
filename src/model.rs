use serde::{Deserialize, Serialize};
use std::{net::IpAddr, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum ServerIcon {
    #[default]
    Server,
    Database,
    Computer,
    Cloud,
    Gateway,
    DellIdrac,
    Proxmox,
    Ubuntu,
    Windows,
    MacOs,
    Linux,
    Docker,
    Nas,
    Router,
    Switch,
    Debian,
    Plex,
    PiHole,
    UniFi,
    Unraid,
    TrueNas,
    Synology,
    Qnap,
    RaspberryPi,
    HomeAssistant,
    Jellyfin,
    Emby,
    Nextcloud,
    Portainer,
    Kubernetes,
    Grafana,
    Prometheus,
    OpnSense,
    PfSense,
    Nginx,
    Caddy,
    Traefik,
    AdGuard,
    Tailscale,
    WireGuard,
    Vaultwarden,
    Immich,
    Sonarr,
    Radarr,
    Sabnzbd,
    Qbittorrent,
    Frigate,
    EspHome,
}

impl ServerIcon {
    pub const ALL: [Self; 48] = [
        Self::Server,
        Self::Database,
        Self::Computer,
        Self::Cloud,
        Self::Gateway,
        Self::DellIdrac,
        Self::Proxmox,
        Self::Ubuntu,
        Self::Windows,
        Self::MacOs,
        Self::Linux,
        Self::Docker,
        Self::Nas,
        Self::Router,
        Self::Switch,
        Self::Debian,
        Self::Plex,
        Self::PiHole,
        Self::UniFi,
        Self::Unraid,
        Self::TrueNas,
        Self::Synology,
        Self::Qnap,
        Self::RaspberryPi,
        Self::HomeAssistant,
        Self::Jellyfin,
        Self::Emby,
        Self::Nextcloud,
        Self::Portainer,
        Self::Kubernetes,
        Self::Grafana,
        Self::Prometheus,
        Self::OpnSense,
        Self::PfSense,
        Self::Nginx,
        Self::Caddy,
        Self::Traefik,
        Self::AdGuard,
        Self::Tailscale,
        Self::WireGuard,
        Self::Vaultwarden,
        Self::Immich,
        Self::Sonarr,
        Self::Radarr,
        Self::Sabnzbd,
        Self::Qbittorrent,
        Self::Frigate,
        Self::EspHome,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Server => "Server",
            Self::Database => "Database",
            Self::Computer => "Computer",
            Self::Cloud => "Cloud",
            Self::Gateway => "Gateway",
            Self::DellIdrac => "Dell iDRAC",
            Self::Proxmox => "Proxmox",
            Self::Ubuntu => "Ubuntu",
            Self::Windows => "Windows",
            Self::MacOs => "macOS",
            Self::Linux => "Linux",
            Self::Docker => "Docker",
            Self::Nas => "NAS",
            Self::Router => "Router",
            Self::Switch => "Network switch",
            Self::Debian => "Debian",
            Self::Plex => "Plex",
            Self::PiHole => "Pi-hole",
            Self::UniFi => "UniFi",
            Self::Unraid => "Unraid",
            Self::TrueNas => "TrueNAS",
            Self::Synology => "Synology",
            Self::Qnap => "QNAP",
            Self::RaspberryPi => "Raspberry Pi",
            Self::HomeAssistant => "Home Assistant",
            Self::Jellyfin => "Jellyfin",
            Self::Emby => "Emby",
            Self::Nextcloud => "Nextcloud",
            Self::Portainer => "Portainer",
            Self::Kubernetes => "Kubernetes",
            Self::Grafana => "Grafana",
            Self::Prometheus => "Prometheus",
            Self::OpnSense => "OPNsense",
            Self::PfSense => "pfSense",
            Self::Nginx => "NGINX",
            Self::Caddy => "Caddy",
            Self::Traefik => "Traefik",
            Self::AdGuard => "AdGuard Home",
            Self::Tailscale => "Tailscale",
            Self::WireGuard => "WireGuard",
            Self::Vaultwarden => "Vaultwarden",
            Self::Immich => "Immich",
            Self::Sonarr => "Sonarr",
            Self::Radarr => "Radarr",
            Self::Sabnzbd => "SABnzbd",
            Self::Qbittorrent => "qBittorrent",
            Self::Frigate => "Frigate",
            Self::EspHome => "ESPHome",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceOs {
    DebianUbuntu,
    FedoraRhel,
    Arch,
    Windows,
    MacOs,
    Linux,
    Unifi,
    Idrac,
    Other,
}
impl DeviceOs {
    pub const ALL: [Self; 9] = [
        Self::DebianUbuntu,
        Self::FedoraRhel,
        Self::Arch,
        Self::Windows,
        Self::MacOs,
        Self::Linux,
        Self::Unifi,
        Self::Idrac,
        Self::Other,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::DebianUbuntu => "Debian / Ubuntu",
            Self::FedoraRhel => "Fedora / RHEL",
            Self::Arch => "Arch Linux",
            Self::Windows => "Windows",
            Self::MacOs => "macOS",
            Self::Linux => "Linux (custom)",
            Self::Unifi => "UniFi OS",
            Self::Idrac => "Dell iDRAC",
            Self::Other => "Other / appliance",
        }
    }
    pub fn from_icon(icon: ServerIcon) -> Self {
        match icon {
            ServerIcon::Ubuntu
            | ServerIcon::Debian
            | ServerIcon::Proxmox
            | ServerIcon::RaspberryPi
            | ServerIcon::PiHole => Self::DebianUbuntu,
            ServerIcon::Windows => Self::Windows,
            ServerIcon::MacOs => Self::MacOs,
            ServerIcon::DellIdrac => Self::Idrac,
            ServerIcon::UniFi => Self::Unifi,
            ServerIcon::Linux | ServerIcon::Docker => Self::Linux,
            _ => Self::Other,
        }
    }
    pub fn update_command(self) -> Option<&'static str> {
        match self {
            Self::DebianUbuntu => Some("sudo apt update && sudo apt upgrade"),
            Self::FedoraRhel => Some("sudo dnf upgrade"),
            Self::Arch => Some("sudo pacman -Syu"),
            Self::MacOs => Some("softwareupdate --list"),
            Self::Idrac => Some("racadm getsysinfo"),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Server {
    pub ip: IpAddr,
    pub name: String,
    pub username: String,
    pub ssh_port: u16,
    pub web_port: u16,
    pub https: bool,
    #[serde(default)]
    pub icon: ServerIcon,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<DeviceOs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_icon: Option<String>,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub tags: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub links: String,
    #[serde(default)]
    pub reminder_at: Option<u64>,
    #[serde(default)]
    pub reminder_repeat_seconds: Option<u64>,
}

impl Server {
    pub fn effective_os(&self) -> DeviceOs {
        self.os.unwrap_or_else(|| DeviceOs::from_icon(self.icon))
    }
    pub fn suggested_update_command(&self) -> Option<String> {
        self.effective_os().update_command().map(|command| {
            if self.username == "root" {
                command.replace("sudo ", "")
            } else {
                command.into()
            }
        })
    }
    pub fn new(ip: IpAddr) -> Self {
        Self {
            ip,
            name: format!("Server {ip}"),
            username: "root".into(),
            ssh_port: 22,
            web_port: 80,
            https: false,
            icon: ServerIcon::default(),
            os: None,
            custom_icon: None,
            group: String::new(),
            tags: String::new(),
            notes: String::new(),
            links: String::new(),
            reminder_at: None,
            reminder_repeat_seconds: None,
        }
    }

    pub fn next_reminder(&self, now: u64) -> Option<u64> {
        let seconds = self
            .reminder_repeat_seconds
            .filter(|seconds| *seconds >= 60)?;
        let at = self.reminder_at?;
        at.checked_add((now.saturating_sub(at) / seconds + 1).checked_mul(seconds)?)
    }

    pub fn web_url(&self) -> String {
        let host = match self.ip {
            IpAddr::V4(ip) => ip.to_string(),
            IpAddr::V6(ip) => format!("[{ip}]"),
        };
        format!(
            "{}://{}:{}",
            if self.https { "https" } else { "http" },
            host,
            self.web_port
        )
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.group.len() > 128
            || self.tags.len() > 1024
            || self.notes.len() > 16384
            || self.links.len() > 4096
        {
            return Err("Device group, tags, notes or links exceed their size limit.".into());
        }
        if let Some(id) = &self.custom_icon {
            crate::icon_assets::validate_id(id)?;
        }
        if self.username.is_empty()
            || self.username.starts_with('-')
            || !self
                .username
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        {
            return Err("SSH username must contain letters, numbers, underscores or hyphens and cannot start with a hyphen.".into());
        }
        if self.ssh_port == 0 || self.web_port == 0 {
            return Err("Ports must be between 1 and 65535.".into());
        }
        Ok(())
    }
    pub fn tag_names(&self) -> impl Iterator<Item = &str> {
        self.tags
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }
    pub fn matches_query(&self, query: &str) -> bool {
        [
            &self.name,
            &self.ip.to_string(),
            &self.group,
            &self.tags,
            &self.notes,
        ]
        .iter()
        .any(|value| value.to_lowercase().contains(query))
    }
}

pub fn parse_addresses(input: &str) -> Result<Vec<IpAddr>, String> {
    let mut addresses = Vec::new();
    for token in input
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .filter(|s| !s.is_empty())
    {
        let ip = token.parse::<IpAddr>().map_err(|_| {
            format!("Invalid IP address: {token}. Enter IPv4 or IPv6 without a URL or port.")
        })?;
        if !addresses.contains(&ip) {
            addresses.push(ip);
        }
    }
    if addresses.is_empty() {
        return Err("Enter at least one IP address.".into());
    }
    Ok(addresses)
}

pub struct ServerStore {
    pub path: PathBuf,
}

impl ServerStore {
    pub fn for_platform(executable: &std::path::Path) -> Result<Self, String> {
        #[cfg(windows)]
        {
            Self::beside_executable(executable)
        }
        #[cfg(not(windows))]
        {
            let _ = executable;
            let directories = directories::ProjectDirs::from("com", "Exenreco", "LabManager")
                .ok_or("Cannot find local application data directory")?;
            Ok(Self {
                path: directories.data_local_dir().join("servers.json"),
            })
        }
    }
    pub fn for_user(root: &std::path::Path, username: &str) -> Result<Self, String> {
        if username.is_empty()
            || username.len() > 64
            || !username
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err("Invalid inventory owner.".into());
        }
        let key: String = username
            .to_ascii_lowercase()
            .bytes()
            .map(|b| format!("{b:02x}"))
            .collect();
        Ok(Self {
            path: root.join("users").join(key).join("servers.json"),
        })
    }
    /// Keep portable installs writable even when AppData is redirected by a launcher.
    pub fn beside_executable(executable: &std::path::Path) -> Result<Self, String> {
        let directory = executable.parent().ok_or("Executable path has no parent")?;
        Ok(Self {
            path: directory.join("data").join("servers.json"),
        })
    }

    pub fn load(&self) -> Result<Vec<Server>, String> {
        match std::fs::read(&self.path) {
            Ok(data) => {
                let servers: Vec<Server> = serde_json::from_slice(&data)
                    .map_err(|e| format!("Saved inventory is invalid: {e}"))?;
                for server in &servers {
                    server.validate()?;
                }
                Ok(servers)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(format!("Cannot read inventory: {e}")),
        }
    }

    pub fn save(&self, servers: &[Server]) -> Result<(), String> {
        for server in servers {
            server.validate()?;
        }
        let data = serde_json::to_vec_pretty(servers).map_err(|e| e.to_string())?;
        let parent = self.path.parent().ok_or("Inventory path has no parent")?;
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create {}: {e}", parent.display()))?;
        let temp = self.path.with_extension("json.tmp");
        use std::io::Write;
        let mut file = std::fs::File::create(&temp)
            .map_err(|e| format!("Cannot create {}: {e}", temp.display()))?;
        file.write_all(&data)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(temp, &self.path)
            .map_err(|e| format!("Cannot replace inventory at {}: {e}", self.path.display()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn os_follows_icon_until_overridden_and_survives_serialization() {
        use super::{DeviceOs, Server, ServerIcon};
        let mut server = Server::new("192.0.2.1".parse().unwrap());
        server.icon = ServerIcon::Proxmox;
        assert_eq!(server.effective_os(), DeviceOs::DebianUbuntu);
        assert_eq!(
            server.suggested_update_command().as_deref(),
            Some("apt update && apt upgrade")
        );
        server.icon = ServerIcon::DellIdrac;
        assert_eq!(server.effective_os(), DeviceOs::Idrac);
        assert_eq!(
            server.suggested_update_command().as_deref(),
            Some("racadm getsysinfo")
        );
        server.os = Some(DeviceOs::Arch);
        server.icon = ServerIcon::Ubuntu;
        assert_eq!(server.effective_os(), DeviceOs::Arch);
        server.username = "admin".into();
        assert_eq!(
            server.suggested_update_command().as_deref(),
            Some("sudo pacman -Syu")
        );
        let restored: Server =
            serde_json::from_str(&serde_json::to_string(&server).unwrap()).unwrap();
        assert_eq!(restored.os, Some(DeviceOs::Arch));
    }
    #[test]
    fn repeating_reminders_skip_missed_intervals_without_drifting() {
        let mut server = super::Server::new("192.0.2.1".parse().unwrap());
        server.reminder_at = Some(100);
        assert_eq!(server.next_reminder(100), None);
        server.reminder_repeat_seconds = Some(3600);
        assert_eq!(server.next_reminder(100), Some(3700));
        assert_eq!(server.next_reminder(7400), Some(10900));
        server.reminder_repeat_seconds = Some(0);
        assert_eq!(server.next_reminder(100), None);
    }

    #[test]
    fn metadata_is_backward_compatible_and_searchable() {
        let mut server = super::Server::new("192.0.2.1".parse().unwrap());
        server.group = "Rack A".into();
        server.tags = "DNS, core, ".into();
        server.notes = "Reboot after UPS maintenance".into();
        server.reminder_at = Some(100);
        assert!(server.matches_query("rack a"));
        assert!(server.matches_query("ups"));
        assert_eq!(server.tag_names().collect::<Vec<_>>(), vec!["DNS", "core"]);
        let saved = serde_json::to_value(&server).unwrap();
        assert_eq!(
            serde_json::from_value::<super::Server>(saved.clone()).unwrap(),
            server
        );
        let mut legacy = saved;
        for key in [
            "group",
            "tags",
            "notes",
            "links",
            "reminder_at",
            "reminder_repeat_seconds",
        ] {
            legacy.as_object_mut().unwrap().remove(key);
        }
        assert!(
            serde_json::from_value::<super::Server>(legacy)
                .unwrap()
                .group
                .is_empty()
        );
    }
    use super::*;
    #[test]
    fn accounts_have_separate_inventory_paths() {
        let root = std::env::temp_dir().join(format!("lab-isolation-{}", std::process::id()));
        let alice = ServerStore::for_user(&root, "Alice").unwrap();
        let bob = ServerStore::for_user(&root, "Bob").unwrap();
        assert_ne!(alice.path, bob.path);
        assert_eq!(
            alice.path,
            ServerStore::for_user(&root, "ALICE").unwrap().path
        );
        assert!(ServerStore::for_user(&root, "../Alice").is_err());
        let servers = vec![Server::new("192.0.2.10".parse().unwrap())];
        alice.save(&servers).unwrap();
        assert!(bob.load().unwrap().is_empty());
        bob.save(&[]).unwrap();
        assert_eq!(alice.load().unwrap(), servers);
        for store in [alice, bob] {
            std::fs::remove_file(&store.path).unwrap();
            std::fs::remove_dir(store.path.parent().unwrap()).unwrap();
        }
        std::fs::remove_dir(root.join("users")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn all_device_icons_survive_inventory_serialization() {
        for icon in ServerIcon::ALL {
            let mut server = Server::new("192.0.2.1".parse().unwrap());
            server.icon = icon;
            let saved = serde_json::to_string(&server).unwrap();
            let restored: Server = serde_json::from_str(&saved).unwrap();
            assert_eq!(restored.icon, icon);
        }
    }
    #[test]
    fn portable_inventory_creates_directory_and_survives_updates() {
        let root = std::env::temp_dir().join(format!("lab-portable-{}", std::process::id()));
        let store = ServerStore::beside_executable(&root.join("lab-manager.exe")).unwrap();
        assert_eq!(store.path, root.join("data").join("servers.json"));
        assert!(store.load().unwrap().is_empty());
        let mut servers = vec![Server::new("192.0.2.10".parse().unwrap())];
        store.save(&servers).unwrap();
        servers[0].name = "Updated device".into();
        store.save(&servers).unwrap();
        assert_eq!(store.load().unwrap(), servers);
        std::fs::remove_file(&store.path).unwrap();
        std::fs::remove_dir(root.join("data")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn batch_validation_and_duplicates() {
        assert_eq!(
            parse_addresses("192.168.1.2, ::1;192.168.1.2\n10.0.0.1")
                .unwrap()
                .len(),
            3
        );
        for input in [
            "",
            "10.0.0.1; bad",
            "https://10.0.0.1",
            "-oProxyCommand=evil",
        ] {
            assert!(parse_addresses(input).is_err());
        }
    }
    #[test]
    fn ipv6_browser_url_and_username_validation() {
        let mut server = Server::new("::1".parse().unwrap());
        assert_eq!(server.web_url(), "http://[::1]:80");
        for username in ["-option", "root;whoami", "user name", "user\nname", ""] {
            server.username = username.into();
            assert!(server.validate().is_err());
        }
        server.username = "lab-admin_2".into();
        assert!(server.validate().is_ok());
    }
    #[test]
    fn inventory_round_trip_and_corruption() {
        let path =
            std::env::temp_dir().join(format!("lab-manager-test-{}.json", std::process::id()));
        let store = ServerStore { path: path.clone() };
        let servers = vec![Server::new("10.0.0.1".parse().unwrap())];
        store.save(&servers).unwrap();
        store.save(&servers).unwrap();
        assert_eq!(store.load().unwrap(), servers);
        std::fs::write(&path, "broken").unwrap();
        assert!(store.load().is_err());
        std::fs::remove_file(path).unwrap();
    }
}
