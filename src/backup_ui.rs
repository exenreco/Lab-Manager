use eframe::egui;
use lab_manager::{backup::Backup, model::Server, monitoring::Database};
use std::{path::Path, sync::mpsc};
enum ResultItem {
    Exported,
    Preview(Backup),
    Cancelled,
}
#[derive(Default)]
pub struct BackupUi {
    qr: Vec<String>,
    qr_page: usize,
    qr_error: String,
    pending: Option<mpsc::Receiver<Result<ResultItem, String>>>,
    preview: Option<Backup>,
    replace: bool,
    error: String,
    message: String,
    modal: lab_manager::frosted_modal::FrostedModal,
}
impl BackupUi {
    pub fn controls(
        &mut self,
        ui: &mut egui::Ui,
        root: &Path,
        servers: &[Server],
        database: &Database,
        user: (&str, u8),
    ) {
        if ui.available_width() >= 620.0 {
            ui.columns(2, |columns| {
                self.backup_controls(&mut columns[0], root, servers, database);
                mobile_description(&mut columns[0]);
                self.mobile_trigger(&mut columns[0], root, servers, database, user);
                self.mobile_controls(&mut columns[1]);
            });
        } else {
            self.backup_controls(ui, root, servers, database);
            mobile_description(ui);
            self.mobile_trigger(ui, root, servers, database, user);
            ui.separator();
            self.mobile_controls(ui);
        }
    }
    fn mobile_trigger(
        &mut self,
        ui: &mut egui::Ui,
        root: &Path,
        servers: &[Server],
        database: &Database,
        user: (&str, u8),
    ) {
        if ui
            .button(if self.qr.is_empty() {
                "Show QR code"
            } else {
                "Refresh QR snapshot"
            })
            .clicked()
        {
            let result = Backup::create(root, servers, database).and_then(|backup| {
                lab_manager::mobile_transfer::encode(
                    &lab_manager::mobile_transfer::MobileSettings {
                        format: "labmanager.mobile".into(),
                        version: 1,
                        username: user.0.into(),
                        avatar: user.1,
                        backup,
                    },
                )
            });
            match result {
                Ok(parts) => {
                    self.qr = parts;
                    self.qr_page = 0;
                    self.qr_error.clear();
                }
                Err(error) => {
                    self.qr.clear();
                    self.qr_error = error;
                }
            }
        }
    }
    fn mobile_controls(&mut self, ui: &mut egui::Ui) {
        if !self.qr.is_empty() {
            match qrcode::QrCode::with_error_correction_level(
                self.qr[self.qr_page].as_bytes(),
                qrcode::EcLevel::M,
            ) {
                Ok(code) => {
                    let modules = code.width();
                    let size = ui.available_width().min(300.0);
                    let (rect, _) =
                        ui.allocate_exact_size(egui::Vec2::splat(size), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, egui::Color32::WHITE);
                    let cell = size / (modules + 8) as f32;
                    for y in 0..modules {
                        for x in 0..modules {
                            if code[(x, y)] == qrcode::Color::Dark {
                                let min = rect.min
                                    + egui::vec2((x + 4) as f32 * cell, (y + 4) as f32 * cell);
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(min, egui::Vec2::splat(cell)),
                                    0.0,
                                    egui::Color32::BLACK,
                                );
                            }
                        }
                    }
                }
                Err(error) => {
                    self.qr_error = format!("Cannot encode QR: {error}");
                }
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.qr_page > 0,
                        egui::Button::new("< Previous").frame(false),
                    )
                    .clicked()
                {
                    self.qr_page -= 1;
                }
                ui.label(format!("{} / {}", self.qr_page + 1, self.qr.len()));
                if ui
                    .add_enabled(
                        self.qr_page + 1 < self.qr.len(),
                        egui::Button::new("Next >").frame(false),
                    )
                    .clicked()
                {
                    self.qr_page += 1;
                }
            });
            ui.small("Scan all pages. This is a snapshot; refresh after changes.");
            if ui.button("Hide QR code").clicked() {
                self.qr.clear();
            }
        }
        if !self.qr_error.is_empty() {
            ui.colored_label(egui::Color32::from_rgb(230, 95, 95), &self.qr_error);
        }
    }
    fn backup_controls(
        &mut self,
        ui: &mut egui::Ui,
        root: &Path,
        servers: &[Server],
        database: &Database,
    ) {
        if let Some(receiver) = &self.pending
            && let Ok(result) = receiver.try_recv()
        {
            self.pending = None;
            match result {
                Ok(ResultItem::Exported) => self.message = "Backup exported.".into(),
                Ok(ResultItem::Preview(backup)) => {
                    self.preview = Some(backup);
                    self.replace = false;
                    self.error.clear();
                    self.modal.open(ui.ctx(), egui::Id::new("restore_backup"));
                }
                Ok(ResultItem::Cancelled) => {}
                Err(error) => self.error = error,
            }
        }
        ui.strong("Backup & restore");
        ui.small("Includes devices, groups, notes, reminders, imported icons and monitoring settings. Login credentials, SSH passwords, profile photos, command history and monitoring history are excluded. Backups are readable JSON; keep them private.");
        ui.add_enabled_ui(self.pending.is_none(), |ui| {
            ui.horizontal_wrapped(|ui| {
                for (label, export) in [("Export backup", true), ("Restore backup…", false)] {
                    if ui.button(label).clicked() {
                        let root = root.to_owned();
                        let servers = servers.to_vec();
                        let database = database.clone();
                        let (sender, receiver) = mpsc::channel();
                        self.pending = Some(receiver);
                        self.error.clear();
                        self.message.clear();
                        let ctx = ui.ctx().clone();
                        std::thread::spawn(move || {
                            let result = (|| {
                                let Some(path) = lab_manager::file_dialog::choose_backup(export)?
                                else {
                                    return Ok(ResultItem::Cancelled);
                                };
                                if export {
                                    Backup::create(&root, &servers, &database)?.write(&path)?;
                                    Ok(ResultItem::Exported)
                                } else {
                                    Backup::read(&path).map(ResultItem::Preview)
                                }
                            })();
                            let _ = sender.send(result);
                            ctx.request_repaint();
                        });
                    }
                }
            });
        });
        if self.pending.is_some() {
            ui.spinner();
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
        if !self.error.is_empty() {
            ui.colored_label(egui::Color32::from_rgb(235, 100, 100), &self.error);
        }
    }
    pub fn show(&mut self, ctx: &egui::Context) -> Option<(Backup, bool)> {
        let backup = self.preview.as_ref()?;
        let mut confirm = false;
        let mut cancel = false;
        let response = self.modal.show(ctx, egui::Id::new("restore_backup"), |ui| {
            ui.set_width((ctx.screen_rect().width() - 100.0).clamp(280.0, 460.0));
            ui.heading("Restore backup");
            ui.label(format!("{} devices · {} monitoring configurations · {} imported icons", backup.devices.len(), backup.monitoring.len(), backup.icons.len()));
            ui.small("Restores into your signed-in account only. SSH passwords must be configured again for imported devices.");
            ui.checkbox(&mut self.replace, "Replace my inventory instead of merging");
            ui.label(if self.replace { "Existing devices will be replaced. A recovery backup is saved first." } else { "New IPs are added; matching IPs receive the imported device settings. A recovery backup is saved first." });
            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                for server in backup.devices.iter().take(100) { ui.label(format!("{} · {}", server.name, server.ip)); }
            });
            ui.small("Recovery copies are in your account's backups folder. Monitoring samples for imported endpoints start fresh.");
            ui.horizontal(|ui| {
                confirm = ui.button("Restore to my account").clicked();
                cancel = ui.button("Cancel").clicked();
            });
            if !self.error.is_empty() { ui.label(&self.error); }
        });
        if response.is_some_and(|response| response.close) || cancel {
            self.preview = None;
        }
        if confirm {
            self.preview.take().map(|backup| (backup, self.replace))
        } else {
            None
        }
    }
    pub fn result(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.message =
                    "Backup restored. Recovery copy saved in your account's backups folder.".into()
            }
            Err(error) => self.error = error,
        }
    }
}

fn mobile_description(ui: &mut egui::Ui) {
    ui.add_space(16.0);
    ui.strong("Transfer to mobile");
    ui.small("Scan with the planned Lab Manager mobile app.");
    ui.add_space(6.0);
    ui.small("Contains your devices, notes, monitoring settings and avatar choice. Excludes passwords, private keys, key file paths and photos. Anyone who scans it can read this data.");
}
