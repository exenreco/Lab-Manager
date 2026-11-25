use crate::model::ServerIcon;
use eframe::egui::{self, Color32, Vec2};

pub fn thumbnail(ui: &mut egui::Ui, icon: ServerIcon, custom: Option<&str>, size: f32) {
    if let Some(id) = custom
        && let Some(texture) = crate::icon_assets::texture(ui.ctx(), id)
    {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, (size * 0.15).min(14.0), Color32::from_gray(39));
        egui::Image::from_texture(&texture)
            .fit_to_exact_size(Vec2::splat((size - 6.0).max(8.0)))
            .paint_at(ui, rect.shrink(3.0));
        response.on_hover_text(crate::icon_assets::label(id));
        return;
    }
    system_thumbnail(ui, icon, size);
}

pub fn system_thumbnail(ui: &mut egui::Ui, icon: ServerIcon, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, (size * 0.15).min(14.0), Color32::from_gray(39));
    let name = match icon {
        ServerIcon::Server => "server",
        ServerIcon::Database => "database",
        ServerIcon::Computer => "monitor",
        ServerIcon::Cloud => "cloud",
        ServerIcon::Gateway => "router",
        ServerIcon::DellIdrac => "brand-dell",
        ServerIcon::Proxmox => "brand-proxmox",
        ServerIcon::Ubuntu => "brand-ubuntu",
        ServerIcon::Windows => "brand-windows",
        ServerIcon::MacOs => "brand-apple",
        ServerIcon::Linux => "brand-linux",
        ServerIcon::Docker => "brand-docker",
        ServerIcon::Nas => "hard-drive",
        ServerIcon::Router => "router",
        ServerIcon::Switch => "network",
        ServerIcon::Debian => "brand-debian",
        ServerIcon::Plex => "brand-plex",
        ServerIcon::PiHole => "brand-pi-hole",
        ServerIcon::UniFi => "brand-ubiquiti",
        ServerIcon::Unraid => "brand-unraid",
        ServerIcon::TrueNas => "brand-truenas",
        ServerIcon::Synology => "brand-synology",
        ServerIcon::Qnap => "brand-qnap",
        ServerIcon::RaspberryPi => "brand-raspberry-pi",
        ServerIcon::HomeAssistant => "brand-home-assistant",
        ServerIcon::Jellyfin => "brand-jellyfin",
        ServerIcon::Emby => "brand-emby",
        ServerIcon::Nextcloud => "brand-nextcloud",
        ServerIcon::Portainer => "brand-portainer",
        ServerIcon::Kubernetes => "brand-kubernetes",
        ServerIcon::Grafana => "brand-grafana",
        ServerIcon::Prometheus => "brand-prometheus",
        ServerIcon::OpnSense => "brand-opnsense",
        ServerIcon::PfSense => "brand-pfsense",
        ServerIcon::Nginx => "brand-nginx",
        ServerIcon::Caddy => "brand-caddy",
        ServerIcon::Traefik => "brand-traefik",
        ServerIcon::AdGuard => "brand-adguard-home",
        ServerIcon::Tailscale => "brand-tailscale",
        ServerIcon::WireGuard => "brand-wireguard",
        ServerIcon::Vaultwarden => "brand-vaultwarden",
        ServerIcon::Immich => "brand-immich",
        ServerIcon::Sonarr => "brand-sonarr",
        ServerIcon::Radarr => "brand-radarr",
        ServerIcon::Sabnzbd => "brand-sabnzbd",
        ServerIcon::Qbittorrent => "brand-qbittorrent",
        ServerIcon::Frigate => "brand-frigate",
        ServerIcon::EspHome => "brand-esphome",
    };
    crate::svg_icons::paint(
        ui.painter(),
        rect.shrink(size * 0.16),
        name,
        if name.starts_with("brand-") {
            None
        } else {
            Some(Color32::from_gray(195))
        },
    );
}
