use crate::model::{Server, ServerIcon, parse_addresses};
use eframe::egui;

#[derive(Default, PartialEq)]
pub enum EntryMode {
    #[default]
    One,
    Many,
}

pub struct ServerForm {
    pub mode: EntryMode,
    pub name: String,
    pub ip: String,
    pub addresses: String,
    pub prefix: String,
    pub icon: ServerIcon,
    pub os: Option<crate::model::DeviceOs>,
    pub custom_icon: Option<String>,
    pub web_port: u16,
    pub https: bool,
    pub username: String,
    pub ssh_port: u16,
    pub error: String,
    pub group: String,
    pub tags: String,
    pub notes: String,
    pub links: String,
    pub reminder_at: Option<u64>,
    pub reminder_repeat_seconds: Option<u64>,
}

impl Default for ServerForm {
    fn default() -> Self {
        Self {
            mode: EntryMode::One,
            name: String::new(),
            ip: String::new(),
            addresses: String::new(),
            prefix: String::new(),
            icon: ServerIcon::default(),
            os: None,
            custom_icon: None,
            web_port: 80,
            https: false,
            username: "root".into(),
            ssh_port: 22,
            error: String::new(),
            group: String::new(),
            tags: String::new(),
            notes: String::new(),
            links: String::new(),
            reminder_at: None,
            reminder_repeat_seconds: None,
        }
    }
}

impl ServerForm {
    pub fn build(&self, existing: &[Server]) -> Result<Vec<Server>, String> {
        let addresses = if self.mode == EntryMode::One {
            if self.name.trim().is_empty() {
                return Err("Enter a server name.".into());
            }
            let ip = self.ip.trim().parse().map_err(|_| {
                "Enter a valid IPv4 or IPv6 address without a URL or port.".to_owned()
            })?;
            if existing.iter().any(|s| s.ip == ip) {
                return Err("This IP address is already saved.".into());
            }
            vec![ip]
        } else {
            parse_addresses(&self.addresses)?
        };
        let mut servers = Vec::new();
        for ip in addresses {
            if existing.iter().any(|s| s.ip == ip) {
                continue;
            }
            let mut server = Server::new(ip);
            server.name = if self.mode == EntryMode::One {
                self.name.trim().into()
            } else if self.prefix.trim().is_empty() {
                format!("Server {ip}")
            } else {
                format!("{} {ip}", self.prefix.trim())
            };
            server.icon = self.icon;
            server.custom_icon = self.custom_icon.clone();
            server.web_port = self.web_port;
            server.https = self.https;
            server.username = self.username.trim().into();
            server.ssh_port = self.ssh_port;
            server.group = self.group.trim().into();
            server.tags = self.tags.clone();
            server.notes = self.notes.clone();
            server.links = self.links.clone();
            server.os = self.os;
            server.reminder_at = self.reminder_at;
            server.reminder_repeat_seconds = self.reminder_repeat_seconds;
            server.validate()?;
            servers.push(server);
        }
        if servers.is_empty() {
            return Err("All entered IP addresses are already saved.".into());
        }
        Ok(servers)
    }

    /// Shared form for the full page and the reusable right sidebar.
    pub fn show(&mut self, ui: &mut egui::Ui, writable: bool) -> (bool, bool) {
        crate::settings_form::configure(ui);
        let mut many = self.mode == EntryMode::Many;
        if crate::segmented_switch::show(ui, "entry_mode", ["One", "Many"], &mut many, 168.0)
            .changed
        {
            self.mode = if many {
                EntryMode::Many
            } else {
                EntryMode::One
            };
            self.error.clear();
        }
        ui.add_space(12.0);
        use crate::settings_form::{port_input, row};
        use egui::{Color32, Vec2};
        if self.mode == EntryMode::One {
            row(ui, "Device", 0, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.name)
                        .hint_text(
                            eframe::egui::RichText::new("Production server")
                                .color(eframe::egui::Color32::from_gray(100)),
                        )
                        .margin(Vec2::new(10.0, 9.0))
                        .desired_width(f32::INFINITY),
                );
            });
            row(ui, "IP address", 3, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.ip)
                        .hint_text(
                            eframe::egui::RichText::new("IPv4 or IPv6 address")
                                .color(eframe::egui::Color32::from_gray(100)),
                        )
                        .margin(Vec2::new(10.0, 9.0))
                        .desired_width(f32::INFINITY),
                );
            });
        } else {
            row(ui, "IP addresses", 3, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.addresses)
                        .hint_text(
                            eframe::egui::RichText::new("192.168.1.10\n192.168.1.11")
                                .color(eframe::egui::Color32::from_gray(100)),
                        )
                        .desired_rows(5)
                        .margin(Vec2::new(10.0, 9.0))
                        .desired_width(f32::INFINITY),
                );
                ui.small("Separate IPs with spaces, commas, semicolons or new lines. Duplicates are skipped.");
            });
            row(ui, "Name prefix", 0, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.prefix)
                        .hint_text(
                            eframe::egui::RichText::new("Lab (optional)")
                                .color(eframe::egui::Color32::from_gray(100)),
                        )
                        .margin(Vec2::new(10.0, 9.0))
                        .desired_width(f32::INFINITY),
                );
            });
            ui.small("The settings below apply to every new device.");
        }
        row(ui, "Web port", 3, |ui| {
            port_input(ui, "add_web_port", &mut self.web_port)
        });
        row(ui, "Protocol", 1, |ui| {
            if ui.checkbox(&mut self.https, "Use HTTPS").changed() {
                if self.https && self.web_port == 80 {
                    self.web_port = 443;
                } else if !self.https && self.web_port == 443 {
                    self.web_port = 80;
                }
            }
        });
        row(ui, "Icon", 1, |ui| {
            icon_picker(ui, &mut self.icon, &mut self.custom_icon)
        });
        ui.separator();
        egui::CollapsingHeader::new("SSH Setting")
            .id_salt("add_device_ssh_settings")
            .default_open(false)
            .show(ui, |ui| {
                row(ui, "SSH user", 2, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.username)
                            .hint_text(
                                eframe::egui::RichText::new("Username")
                                    .color(eframe::egui::Color32::from_gray(100)),
                            )
                            .margin(Vec2::new(10.0, 9.0))
                            .desired_width(f32::INFINITY),
                    );
                });
                row(ui, "SSH port", 3, |ui| {
                    port_input(ui, "add_ssh_port", &mut self.ssh_port)
                });
                ui.label(
                    egui::RichText::new("SSH uses your OpenSSH keys, agent, or password prompt.")
                        .size(11.0)
                        .color(Color32::from_gray(130)),
                );
            });
        crate::settings_form::os_selector(ui, &mut self.os, self.icon);
        crate::settings_form::organization(
            ui,
            &mut self.group,
            &mut self.tags,
            &mut self.notes,
            &mut self.links,
            &mut self.reminder_at,
            &mut self.reminder_repeat_seconds,
        );
        ui.add_space(4.0);
        if !self.error.is_empty() {
            ui.colored_label(egui::Color32::from_rgb(245, 145, 145), &self.error);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let save = ui
                .add_enabled(
                    writable,
                    egui::Button::new("Save Device/s")
                        .fill(Color32::from_rgb(36, 96, 63))
                        .min_size(Vec2::new(100.0, 36.0)),
                )
                .clicked();
            let cancel = ui
                .add_sized([90.0, 36.0], egui::Button::new("× Cancel").frame(false))
                .clicked();
            (save, cancel)
        })
        .inner
    }
}

pub use crate::icon_picker::icon_picker;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn organization_fields_are_saved_for_one_and_many_devices() {
        for mode in [EntryMode::One, EntryMode::Many] {
            let form = ServerForm {
                mode,
                name: "DNS".into(),
                ip: "192.0.2.1".into(),
                addresses: "192.0.2.1\n192.0.2.2".into(),
                group: " Home lab ".into(),
                tags: "DNS, core".into(),
                notes: "Check backups".into(),
                links: "https://example.com/docs".into(),
                reminder_at: Some(1234567890),
                reminder_repeat_seconds: Some(3600),
                ..Default::default()
            };
            let servers = form.build(&[]).unwrap();
            for server in servers {
                assert_eq!(server.group, "Home lab");
                assert_eq!(server.tags, "DNS, core");
                assert_eq!(server.notes, "Check backups");
                assert_eq!(server.links, "https://example.com/docs");
                assert_eq!(server.reminder_at, Some(1234567890));
                assert_eq!(server.reminder_repeat_seconds, Some(3600));
            }
        }
    }
    #[test]
    fn single_server_preserves_details_and_rejects_duplicates() {
        let form = ServerForm {
            name: "Production".into(),
            ip: "::1".into(),
            icon: ServerIcon::Cloud,
            web_port: 8443,
            https: true,
            ..Default::default()
        };
        let servers = form.build(&[]).unwrap();
        assert_eq!(servers[0].name, "Production");
        assert_eq!(servers[0].icon, ServerIcon::Cloud);
        assert_eq!(servers[0].web_url(), "https://[::1]:8443");
        assert!(form.build(&servers).is_err());
        assert!(ServerForm::default().build(&[]).is_err());
    }
    #[test]
    fn batch_is_validated_before_saving_and_uses_shared_settings() {
        let mut form = ServerForm {
            mode: EntryMode::Many,
            addresses: "10.0.0.1,10.0.0.2;10.0.0.2".into(),
            prefix: "Lab".into(),
            ssh_port: 2222,
            username: "admin".into(),
            ..Default::default()
        };
        let existing = vec![Server::new("10.0.0.1".parse().unwrap())];
        let servers = form.build(&existing).unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "Lab 10.0.0.2");
        assert_eq!(servers[0].ssh_port, 2222);
        form.addresses.push_str(" invalid");
        assert!(form.build(&existing).is_err());
    }
    #[test]
    fn legacy_inventory_gets_default_icon() {
        let mut value = serde_json::to_value(Server::new("10.0.0.1".parse().unwrap())).unwrap();
        value.as_object_mut().unwrap().remove("icon");
        let server: Server = serde_json::from_value(value).unwrap();
        assert_eq!(server.icon, ServerIcon::Server);
        assert!(server.custom_icon.is_none());
    }
    #[test]
    fn imported_icon_is_preserved_in_drafts_and_inventory() {
        let mut form = ServerForm {
            name: "Lab".into(),
            ip: "192.168.1.20".into(),
            custom_icon: Some("MyIcon-123.png".into()),
            ..Default::default()
        };
        let servers = form.build(&[]).unwrap();
        assert_eq!(servers[0].custom_icon, form.custom_icon);
        let decoded: Vec<Server> =
            serde_json::from_slice(&serde_json::to_vec(&servers).unwrap()).unwrap();
        assert_eq!(decoded, servers);
        form.custom_icon = Some("../another-user.png".into());
        assert!(form.build(&[]).is_err());
    }
}
