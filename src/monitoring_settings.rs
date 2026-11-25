use eframe::egui::{self, Color32};
use lab_manager::settings_form::row;
use lab_manager::{
    model::Server,
    monitoring::{AuthMode, Monitoring, Platform, key},
};

pub fn show(ui: &mut egui::Ui, monitoring: &mut Monitoring, server: &Server) {
    lab_manager::settings_form::configure(ui);
    row(ui, "Device", 0, |ui| {
        ui.label(&server.name);
    });
    row(ui, "IP address", 3, |ui| {
        ui.label(server.ip.to_string());
    });
    ui.small("Changes are saved automatically.");
    ui.add_space(14.0);
    let credential_root = monitoring.credential_root();
    let selected_key = key(server);
    if monitoring.password_target.as_ref() != Some(&selected_key) {
        monitoring.password_draft = Default::default();
        monitoring.password_target = Some(selected_key);
        monitoring.credential_message.clear();
    }
    let h = monitoring.database.devices.entry(key(server)).or_default();
    let mut config_changed = false;
    ui.separator();
    row(ui, "Monitoring", 0, |ui| {
        config_changed |= ui.checkbox(&mut h.config.enabled, "Enabled").changed();
    });
    row(ui, "Notifications", 0, |ui| {
        config_changed |= ui.checkbox(&mut h.config.alerts, "Enabled").changed();
    });
    row(ui, "Check interval", 0, |ui| {
        config_changed |= ui
            .add(
                egui::DragValue::new(&mut h.config.interval)
                    .range(
                        (if h.config.platform == Platform::Idrac {
                            60
                        } else {
                            15
                        })..=3600,
                    )
                    .suffix(" sec"),
            )
            .changed();
    });
    row(ui, "Host metrics", 0, |ui| {
        config_changed |= ui
            .checkbox(&mut h.config.host_metrics, "Collect over SSH")
            .changed();
    });
    ui.separator();
    egui::CollapsingHeader::new("SSH Setting").default_open(true).show(ui, |ui| {
        row(ui, "SSH user", 2, |ui| { ui.label(&server.username); });
        row(ui, "SSH port", 3, |ui| { ui.label(server.ssh_port.to_string()); });
        let before = h.config.platform;
        row(ui, "Platform", 0, |ui| {
        egui::ComboBox::from_id_salt("monitoring_platform")
            .selected_text(platform_label(h.config.platform))
            .show_ui(ui, |ui| {
                for p in [
                    Platform::Linux,
                    Platform::Unifi,
                    Platform::Idrac,
                    Platform::Windows,
                    Platform::MacOs,
                ] {
                    ui.selectable_value(&mut h.config.platform, p, platform_label(p));
                }
            });
        });
        config_changed |= before != h.config.platform;
        if h.config.platform == Platform::Idrac {
            ui.small("iDRAC hardware monitoring uses read-only RACADM commands over SSH, at least 60 seconds apart. Enable Collect over SSH above. The account needs permission to read hardware sensors and storage. No server changes are made.");
        }
        if h.config.platform == Platform::Unifi {
            ui.small("UniFi consoles use their console SSH credentials; APs and switches use their configured device SSH credentials. SSH must be enabled on the device. Metrics depend on firmware access to /proc.");
        }
        ui.small("Use an SSH key/agent, or save a password encrypted for this Windows user. Verify the host key through SSH Shell first.");
        let mut auth = if h.config.uses_password() {
            AuthMode::Password
        } else {
            AuthMode::Key
        };
        let before = auth;
        row(ui, "Authentication", 2, |ui| {
            let mut password = auth == AuthMode::Password;
            let response = lab_manager::segmented_switch::show(ui, "ssh_auth", ["SSH Key", "Password"], &mut password, 216.0);
            auth = if password { AuthMode::Password } else { AuthMode::Key };
            #[cfg(test)]
            ui.ctx().data_mut(|d| {
                d.insert_temp(egui::Id::new("monitoring_auth_key"), response.options[0]);
                d.insert_temp(egui::Id::new("monitoring_auth_password"), response.options[1]);
            });
            #[cfg(not(test))]
            let _ = response;
        });
        if auth != before {
            h.config.auth_mode = Some(auth);
            config_changed = true;
            monitoring.password_draft = Default::default();
            monitoring.credential_message.clear();
        }
        if auth == AuthMode::Key {
            row(ui, "Private key", 0, |ui| {
                config_changed |= ui.add(egui::TextEdit::singleline(&mut h.config.identity_file)
                    .hint_text(egui::RichText::new("~/.ssh/id_ed25519").color(Color32::from_gray(100)))
                    .char_limit(4096).margin(egui::vec2(10.0,9.0)).desired_width(f32::INFINITY)).changed();
            });
            ui.small("Enter the private key file path, not the public key or key contents. Leave empty for OpenSSH defaults/agent. Passphrase-protected keys need an unlocked SSH agent for background checks.");
            if let Err(error) = h.config.ssh_identity() { ui.colored_label(Color32::from_rgb(230,95,95),error); }

        } else {
            ui.label(if h.config.password_id.is_some() {
                "Password saved securely"
            } else {
                "No password saved"
            });
            row(ui, "Password", 0, |ui| { ui.add(
                egui::TextEdit::singleline(&mut *monitoring.password_draft)
                    .password(true)
                    .hint_text(
                        eframe::egui::RichText::new(if h.config.password_id.is_some() {
                            "Replace saved SSH password"
                        } else {
                            "SSH password"
                        })
                        .color(eframe::egui::Color32::from_gray(100)),
                    )
                    .margin(egui::vec2(10.0,9.0)).desired_width(f32::INFINITY),
            ); });
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        credential_root.is_some() && !monitoring.password_draft.is_empty(),
                        egui::Button::new("Save password & retry"),
                    )
                    .clicked()
                    && let Some(root) = &credential_root
                {
                    match lab_manager::monitoring_credentials::save(
                        root,
                        server,
                        &monitoring.password_draft,
                        h.config.password_id.as_deref(),
                    ) {
                        Ok(id) => {
                            h.config.password_id = Some(id);
                            monitoring.password_draft = Default::default();
                            monitoring.credential_message =
                                "SSH password saved encrypted. Retrying monitoring.".into();
                            config_changed = true;
                        }
                        Err(error) => monitoring.credential_message = error,
                    }
                }
                if h.config.password_id.is_some()
                    && ui.button("Remove saved password").clicked()
                    && let (Some(root), Some(id)) = (&credential_root, &h.config.password_id)
                {
                    match lab_manager::monitoring_credentials::remove(root, id) {
                        Ok(()) => {
                            h.config.password_id = None;
                            monitoring.credential_message =
                                "Saved password removed. Enter a password or switch to SSH key."
                                    .into();
                            config_changed = true;
                        }
                        Err(error) => monitoring.credential_message = error,
                    }
                }
            });
            if !monitoring.credential_message.is_empty() {
                ui.small(&monitoring.credential_message);
            }
        }
    });
    egui::CollapsingHeader::new("Alert thresholds").show(ui, |ui| {
        for (label, value) in [
            ("CPU", &mut h.config.cpu_limit),
            ("Memory", &mut h.config.memory_limit),
            ("Disk", &mut h.config.disk_limit),
        ] {
            row(ui, label, 0, |ui| {
                config_changed |= ui
                    .add(egui::DragValue::new(value).range(1.0..=100.0).suffix("%"))
                    .changed();
            });
        }
    });
    egui::CollapsingHeader::new("Maintenance & alerts").default_open(false).show(ui, |ui| {
        config_changed |= ui.checkbox(&mut h.config.desktop_alerts, "Windows desktop notifications").changed();
        ui.small("Desktop alerts follow Notifications and alert thresholds below. Windows notification settings may suppress them.");
        row(ui, "Maintenance", 0, |ui| {
            config_changed |= maintenance_menu(ui, &mut h.config);
        });
        if h.config.maintenance_active() {
            ui.small(format!("Alerts silenced for {} more minutes. Checks and history continue.", (h.config.maintenance_until - lab_manager::notifications::now()).div_ceil(60)));
        } else { ui.small("Choose a maintenance duration to silence alerts during planned work."); }

    });
    config_changed |= service_settings(ui, &mut h.config.services);
    ui.small("History stores the latest 1,440 checks per device. Alerts are sent on changes; certificates warn at 30 days. Linux collectors also support Proxmox hosts. Select Dell iDRAC / RACADM for iDRAC hardware health, fans, temperatures, power and storage. Other appliances use port/web checks unless they expose a compatible collector.");
    if config_changed {
        monitoring.save();
        monitoring.refresh();
    }
    if !monitoring.error.is_empty() {
        ui.colored_label(Color32::from_rgb(230, 95, 95), &monitoring.error);
    }
}

fn maintenance_menu(ui: &mut egui::Ui, config: &mut lab_manager::monitoring::Config) -> bool {
    let now = lab_manager::notifications::now();
    let remaining = config.maintenance_until.saturating_sub(now);
    let selected = if remaining == 0 {
        0
    } else if remaining <= 3600 {
        1
    } else if remaining <= 14400 {
        4
    } else {
        24
    };
    let mut picked = None;
    let menu = egui::ComboBox::from_id_salt("maintenance_duration")
        .width(ui.available_width())
        .selected_text(match selected {
            1 => "1h Maintenance",
            4 => "4h Maintenance",
            24 => "24h Maintenance",
            _ => "Maintenance",
        })
        .show_ui(ui, |ui| {
            for (hours, label) in [
                (0, "Maintenance"),
                (1, "1h Maintenance"),
                (4, "4h Maintenance"),
                (24, "24h Maintenance"),
            ] {
                let option = ui.selectable_label(selected == hours, label);
                #[cfg(test)]
                ui.ctx().data_mut(|data| {
                    data.insert_temp(egui::Id::new(("maintenance_option", hours)), option.rect)
                });
                if option.clicked() {
                    picked = Some(hours);
                }
            }
        });
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(egui::Id::new("maintenance_trigger"), menu.response.rect)
    });
    #[cfg(not(test))]
    let _ = menu;
    if let Some(hours) = picked {
        config.maintenance_until = if hours == 0 { 0 } else { now + hours * 3600 };
        true
    } else {
        false
    }
}

fn service_add_menu(
    ui: &mut egui::Ui,
    checks: &mut Vec<lab_manager::service_checks::ServiceCheck>,
) -> bool {
    use lab_manager::service_checks::{Kind, ServiceCheck};
    let mut changed = false;
    ui.add_enabled_ui(checks.len() < 8, |ui| {
        let response = ui.menu_button("Add", |ui| {
            for (index, (label, kind, port, target)) in [
                ("+ TCP", Kind::Tcp, 443, ""),
                ("+ Pi-hole DNS", Kind::Dns, 53, "example.com"),
                ("+ Proxmox", Kind::Tcp, 8006, ""),
                ("+ Linux service", Kind::Systemd, 22, "ssh"),
                ("+ Container", Kind::Docker, 22, "container-name"),
            ]
            .into_iter()
            .enumerate()
            {
                let response = ui.button(label);
                #[cfg(test)]
                ui.ctx().data_mut(|data| {
                    data.insert_temp(egui::Id::new(("service_add_option", index)), response.rect)
                });
                #[cfg(not(test))]
                let _ = index;
                if response.clicked() {
                    checks.push(ServiceCheck {
                        name: label.trim_start_matches("+ ").into(),
                        kind,
                        port,
                        target: target.into(),
                    });
                    changed = true;
                    ui.close_menu();
                }
            }
        });
        #[cfg(test)]
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::new("service_add_trigger"), response.response.rect)
        });
        #[cfg(not(test))]
        let _ = response;
    });
    changed
}

fn service_settings(
    ui: &mut egui::Ui,
    checks: &mut Vec<lab_manager::service_checks::ServiceCheck>,
) -> bool {
    use lab_manager::service_checks::Kind;
    let mut changed = false;
    egui::CollapsingHeader::new("Service checks").show(ui, |ui| {
        ui.small("Up to 8 checks per device. Linux services and containers use the SSH authentication below. Checks never start or stop a service.");
        changed |= service_add_menu(ui, checks);
        let mut remove = None;
        for (index, check) in checks.iter_mut().enumerate() {
            ui.push_id(index, |ui| {
                crate::design::card().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.small(check.kind.label());
                        if ui.small_button("×").on_hover_text("Remove check").clicked() { remove = Some(index); }
                    });
                    changed |= ui.add(egui::TextEdit::singleline(&mut check.name).hint_text(eframe::egui::RichText::new("Service label").color(eframe::egui::Color32::from_gray(100))).char_limit(80).desired_width(f32::INFINITY)).changed();
                    if matches!(check.kind, Kind::Tcp | Kind::Dns) {
                        ui.horizontal(|ui| { ui.label("Port"); changed |= ui.add(egui::DragValue::new(&mut check.port).range(1..=65535)).changed(); });
                    }
                    if check.kind != Kind::Tcp {
                        changed |= ui.add(egui::TextEdit::singleline(&mut check.target).hint_text(eframe::egui::RichText::new(if check.kind == Kind::Dns { "DNS name" } else { "Exact service/container name" }).color(eframe::egui::Color32::from_gray(100))).char_limit(253).desired_width(f32::INFINITY)).changed();
                    }
                    if let Err(error) = check.validate() { ui.colored_label(Color32::from_rgb(235, 160, 70), error); }
                });
            });
        }
        if let Some(index) = remove { checks.remove(index); changed = true; }
    });
    changed
}

fn platform_label(p: Platform) -> &'static str {
    match p {
        Platform::Linux => "Linux / Proxmox",
        Platform::Windows => "Windows",
        Platform::MacOs => "macOS",
        Platform::Unifi => "UniFi OS / AP / Switch",
        Platform::Idrac => "Dell iDRAC / RACADM",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maintenance_menu_applies_each_duration_and_default_clears_it() {
        for hours in [0u64, 1, 4, 24] {
            let ctx = egui::Context::default();
            let mut config = lab_manager::monitoring::Config {
                maintenance_until: if hours == 0 {
                    lab_manager::notifications::now() + 7200
                } else {
                    0
                },
                ..Default::default()
            };
            let before = lab_manager::notifications::now();
            let mut frame = |events| {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(500.0, 600.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            maintenance_menu(ui, &mut config);
                        });
                    },
                );
            };
            frame(vec![]);
            frame(vec![]);
            for id in [
                egui::Id::new("maintenance_trigger"),
                egui::Id::new(("maintenance_option", hours)),
            ] {
                let pos = ctx
                    .data(|data| data.get_temp::<egui::Rect>(id).unwrap())
                    .center();
                for pressed in [true, false] {
                    frame(vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ]);
                }
                frame(vec![]);
                frame(vec![]);
            }
            if hours == 0 {
                assert_eq!(config.maintenance_until, 0);
            } else {
                assert!(
                    config.maintenance_until >= before + hours * 3600
                        && config.maintenance_until
                            <= lab_manager::notifications::now() + hours * 3600
                );
            }
        }
    }
    #[test]
    fn service_add_menu_keeps_all_five_presets_available() {
        for (index, port) in [443, 53, 8006, 22, 22].into_iter().enumerate() {
            let ctx = egui::Context::default();
            let mut checks = Vec::new();
            let mut frame = |events| {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(500.0, 600.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            service_add_menu(ui, &mut checks);
                        });
                    },
                );
            };
            frame(vec![]);
            frame(vec![]);
            for id in [
                egui::Id::new("service_add_trigger"),
                egui::Id::new(("service_add_option", index)),
            ] {
                let pos = ctx
                    .data(|data| data.get_temp::<egui::Rect>(id).unwrap())
                    .center();
                for pressed in [true, false] {
                    frame(vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ]);
                }
                frame(vec![]);
                frame(vec![]);
            }
            assert_eq!(checks.len(), 1);
            assert_eq!(checks[0].port, port);
            assert!(checks[0].validate().is_ok());
        }
    }
    #[test]
    fn auth_toggle_selects_mode_without_deleting_saved_password() {
        let ctx = egui::Context::default();
        let mut monitoring = Monitoring::default();
        let server = Server::new("192.0.2.1".parse().unwrap());
        let config = &mut monitoring
            .database
            .devices
            .entry(key(&server))
            .or_default()
            .config;
        config.host_metrics = true;
        config.password_id = Some("test-id-never-read".into());
        let run = |m: &mut Monitoring, events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(440.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| show(ui, m, &server));
                },
            );
        };
        for _ in 0..2 {
            run(&mut monitoring, vec![]);
        }
        for (id, password) in [
            ("monitoring_auth_key", false),
            ("monitoring_auth_password", true),
        ] {
            let pos = ctx
                .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(id)).unwrap())
                .center();
            for pressed in [true, false] {
                run(
                    &mut monitoring,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ],
                );
            }
            let c = &monitoring.database.devices[&key(&server)].config;
            assert_eq!(c.uses_password(), password);
            assert!(c.password_id.is_some());
        }
    }
}
