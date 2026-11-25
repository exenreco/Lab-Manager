use eframe::egui::{self, Color32, RichText};
use lab_manager::{
    discovery::{self, Device, Event, Scan},
    model::{Server, ServerIcon},
    server_form::{EntryMode, ServerForm},
};

#[derive(Default)]
pub struct DiscoveryScreen {
    pub open: bool,
    range: String,
    scan: Option<Scan>,
    results: Vec<(Device, bool)>,
    completed: usize,
    total: usize,
    message: String,
    pub notifications: Vec<String>,
}
impl DiscoveryScreen {
    pub fn open(&mut self) {
        self.open = true;
        if self.range.is_empty() {
            self.range = discovery::suggested_range();
        }
    }
    pub fn clear(&mut self) {
        if let Some(scan) = &self.scan {
            scan.cancel();
        }
        *self = Self::default();
    }
    pub fn close(&mut self) {
        self.open = false;
        if let Some(scan) = self.scan.take() {
            scan.cancel();
            self.notifications.push("Network discovery stopped.".into());
        }
    }
    /// Returns a draft only after the user explicitly selects Configure; nothing is saved.
    pub fn show(&mut self, ui: &mut egui::Ui, existing: &[Server]) -> Option<ServerForm> {
        if !self.open {
            return None;
        }
        let mut done = false;
        if let Some(scan) = &self.scan {
            while let Ok(event) = scan.receiver.try_recv() {
                match event {
                    Event::Found(device) => self.results.push((device, false)),
                    Event::Progress => self.completed += 1,
                    Event::Done => done = true,
                }
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(200));
        }
        if done {
            self.scan = None;
            self.results.sort_by_key(|(d, _)| d.ip);
            self.message = format!(
                "Scan finished: {} devices with an open service.",
                self.results.len()
            );
            self.notifications.push(self.message.clone());
        }
        lab_manager::settings_form::configure(ui);
        use egui::Vec2;
        use lab_manager::settings_form::row;
        let mut draft = None;
        ui.small("Discover devices, then review the ones you want to add.");
        ui.add_enabled_ui(self.scan.is_none(), |ui| {
            row(ui, "Network", 3, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.range)
                        .hint_text(
                            eframe::egui::RichText::new("192.168.1.0/24")
                                .color(eframe::egui::Color32::from_gray(100)),
                        )
                        .margin(Vec2::new(10.0, 9.0))
                        .desired_width(f32::INFINITY),
                );
                ui.small("Confirm the suggested range for your network.");
            });
        });
        row(ui, "TCP ports", 1, |ui| {
            ui.label("22, 80, 443, 8006, 8080, 8443");
            ui.small("Service checks only; no login or device configuration.");
        });
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 36.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                if let Some(scan) = &self.scan {
                    if ui
                        .add_sized([110.0, 36.0], egui::Button::new("Stop scan").frame(false))
                        .clicked()
                    {
                        scan.cancel();
                        self.message = "Stopping scan…".into();
                    }
                } else if ui
                    .add_sized(
                        [130.0, 36.0],
                        egui::Button::new("Scan network").fill(Color32::from_rgb(36, 96, 63)),
                    )
                    .clicked()
                {
                    match Scan::start(&self.range) {
                        Ok(scan) => {
                            self.total = scan.total;
                            self.completed = 0;
                            self.results.clear();
                            self.message.clear();
                            self.scan = Some(scan);
                            self.notifications
                                .push(format!("Network discovery started for {}.", self.range));
                        }
                        Err(error) => {
                            self.notifications.push(error.clone());
                            self.message = error;
                        }
                    }
                }
            },
        );
        if self.scan.is_some() {
            row(ui, "Progress", 0, |ui| {
                ui.add(
                    egui::ProgressBar::new(self.completed as f32 / self.total.max(1) as f32)
                        .text(format!("{} / {}", self.completed, self.total)),
                );
            });
        }
        if !self.message.is_empty() {
            ui.label(
                RichText::new(&self.message)
                    .size(12.0)
                    .color(Color32::from_gray(165)),
            );
        }
        ui.separator();
        egui::CollapsingHeader::new(format!("Discovered devices ({})", self.results.len()))
            .id_salt("scan_results")
            .default_open(true)
            .show(ui, |ui| {
                if self.results.is_empty() {
                    ui.small(if self.scan.is_some() {
                        "Scanning for devices…"
                    } else {
                        "Scan a network to find devices."
                    });
                }
                egui::ScrollArea::vertical()
                    .id_salt("scan_devices")
                    .max_height(280.0)
                    .show(ui, |ui| {
                        for (device, selected) in &mut self.results {
                            let saved = existing
                                .iter()
                                .any(|server| server.ip == std::net::IpAddr::V4(device.ip));
                            egui::Frame::new()
                                .fill(Color32::from_gray(24))
                                .corner_radius(7.0)
                                .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(47)))
                                .inner_margin(10.0)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.add_enabled(
                                        !saved,
                                        egui::Checkbox::new(selected, device.ip.to_string()),
                                    );
                                    ui.small(format!(
                                        "Open TCP: {}",
                                        device
                                            .ports
                                            .iter()
                                            .map(u16::to_string)
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    ));
                                    if saved {
                                        ui.small("Already saved");
                                    }
                                });
                            ui.add_space(6.0);
                        }
                    });
                ui.small(
                    "Devices blocking these ports will not appear. Nothing is added automatically.",
                );
            });
        let chosen: Vec<_> = self
            .results
            .iter()
            .filter(|(device, selected)| {
                *selected
                    && !existing
                        .iter()
                        .any(|server| server.ip == std::net::IpAddr::V4(device.ip))
            })
            .map(|(device, _)| device.clone())
            .collect();
        ui.add_space(4.0);
        let mut configure = false;
        let mut cancel = false;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            configure = ui
                .add_enabled(
                    self.scan.is_none() && !chosen.is_empty(),
                    egui::Button::new(format!("Configure ({})", chosen.len()))
                        .fill(Color32::from_rgb(36, 96, 63))
                        .min_size(Vec2::new(120.0, 36.0)),
                )
                .clicked();
            cancel = ui
                .add_sized([90.0, 36.0], egui::Button::new("× Cancel").frame(false))
                .clicked();
        });
        if configure {
            let mut form = ServerForm::default();
            if chosen.len() == 1 {
                let device = &chosen[0];
                form.ip = device.ip.to_string();
                form.name = format!("Device {}", device.ip);
                if let Some(port) = device.ports.iter().find(|port| **port != 22) {
                    form.web_port = *port;
                    form.https = [443, 8006, 8443].contains(port);
                }
            } else {
                form.mode = EntryMode::Many;
                form.addresses = chosen
                    .iter()
                    .map(|device| device.ip.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
            }
            form.icon = ServerIcon::Server;
            draft = Some(form);
            self.open = false;
        }
        if cancel {
            self.close();
        }
        draft
    }
}
