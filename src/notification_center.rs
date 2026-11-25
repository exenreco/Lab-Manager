use crate::design;
use eframe::egui::{self, Color32, RichText};
use lab_manager::notifications::{NotificationLog, age};

#[derive(Default)]
pub struct NotificationCenter {
    log: Option<NotificationLog>,
    error: String,
    glass: crate::frosted_popup::Glass,
}
impl NotificationCenter {
    pub fn load(&mut self, path: std::path::PathBuf) {
        *self = Self::default();
        match NotificationLog::load(path) {
            Ok(log) => self.log = Some(log),
            Err(error) => self.error = error,
        }
    }
    pub fn record(&mut self, message: String) {
        if let Some(log) = &mut self.log
            && let Err(error) = log.record(message)
        {
            self.error = format!("History could not be saved: {error}");
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let unread = self.log.as_ref().map_or(0, NotificationLog::unread);
        let response = design::notification_bell(ui, unread);
        let id = ui.make_persistent_id("notifications_dropdown");
        if response.clicked() {
            ui.memory_mut(|m| m.toggle_popup(id));
            if ui.memory(|m| m.is_popup_open(id)) {
                self.glass.capture(ui.ctx(), id);
            }
            if let Some(log) = &mut self.log
                && let Err(error) = log.mark_read()
            {
                self.error = error;
            }
        }
        if !ui.memory(|m| m.is_popup_open(id)) || !self.glass.ready(ui.ctx(), id) {
            return;
        }
        let screen = ui.ctx().screen_rect();
        let width = 350.0_f32.min(screen.width() - 68.0);
        let x = screen.right() - width - 28.0 - 12.0;
        let popup = egui::Area::new(id.with("glass_panel"))
            .order(egui::Order::Foreground)
            .movable(false)
            .default_width(width + 30.0)
            .constrain_to(lab_manager::frosted_modal::body_rect(ui.ctx()))
            .fixed_pos(egui::pos2(x, response.rect.bottom() + 12.0))
            .show(ui.ctx(), |ui| {
                self.glass.show(ui, |ui| {
                    ui.set_width(width);
                    ui.horizontal(|ui| {
                        ui.strong("Notifications");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let has_entries =
                                self.log.as_ref().is_some_and(|log| !log.entries.is_empty());
                            if ui
                                .add_enabled(
                                    has_entries,
                                    egui::Button::new("Clear All").min_size(egui::vec2(88.0, 34.0)),
                                )
                                .clicked()
                                && let Some(log) = &mut self.log
                                && let Err(error) = log.clear()
                            {
                                self.error = error;
                            }
                        });
                    });
                    ui.separator();
                    if !self.error.is_empty() {
                        ui.label(&self.error);
                    }
                    if let Some(log) = &self.log {
                        if log.entries.is_empty() {
                            ui.add_space(14.0);
                            ui.label("You're all caught up.");
                            ui.small("New activity will appear here.");
                            ui.add_space(14.0);
                        }
                        egui::ScrollArea::vertical()
                            .max_height(
                                (screen.bottom() - response.rect.bottom() - 130.0)
                                    .clamp(80.0, 360.0),
                            )
                            .show(ui, |ui| {
                                for item in log.entries.iter().rev() {
                                    ui.horizontal_top(|ui| {
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(10.0, 18.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().circle_filled(
                                            rect.center(),
                                            4.0,
                                            message_color(&item.message),
                                        );
                                        ui.add(egui::Label::new(&item.message).wrap());
                                    });
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {}",
                                            age(item.timestamp),
                                            item.local_time
                                        ))
                                        .size(11.0)
                                        .color(Color32::from_gray(130)),
                                    );
                                    ui.add_space(5.0);
                                    ui.separator();
                                }
                            });
                    }
                });
            });
        if ui.ctx().input(|i| {
            i.key_pressed(egui::Key::Escape)
                || (i.pointer.any_pressed()
                    && i.pointer.interact_pos().is_some_and(|p| {
                        !popup.response.rect.contains(p) && !response.rect.contains(p)
                    }))
        }) {
            ui.memory_mut(|m| m.close_popup());
        }
    }
}

fn message_color(message: &str) -> Color32 {
    let text = message.to_lowercase();
    if ["error", "failed", "cannot", "denied", "invalid"]
        .iter()
        .any(|word| text.contains(word))
    {
        Color32::from_rgb(231, 82, 91)
    } else if ["warning", "disabled", "review", "unreachable", "no devices"]
        .iter()
        .any(|word| text.contains(word))
    {
        Color32::from_rgb(236, 159, 66)
    } else if [
        "added", "saved", "removed", "success", "updated", "restored",
    ]
    .iter()
    .any(|word| text.contains(word))
    {
        Color32::from_rgb(66, 188, 122)
    } else {
        Color32::from_rgb(78, 151, 232)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_notifications_stay_inside_fixed_panel_and_open_without_waiting() {
        for width in [360.0, 1100.0] {
            let ctx = egui::Context::default();
            let mut center = NotificationCenter {
                log: Some(NotificationLog {
                    path: Default::default(),
                    entries: vec![lab_manager::notifications::Notification {
                        message: "Monitoring warning: UniFi Gateway — SSH port is unavailable. "
                            .repeat(12),
                        timestamp: 0,
                        local_time: "2026-11-16 21:10 (local)".into(),
                        read: true,
                    }],
                }),
                ..Default::default()
            };
            let mut id = egui::Id::NULL;
            for frame in 0..3 {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 760.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    id = ui.make_persistent_id("notifications_dropdown");
                                    if frame == 0 {
                                        ui.memory_mut(|memory| memory.open_popup(id));
                                        center.glass.capture(ctx, id);
                                    }
                                    center.show(ui);
                                },
                            );
                        });
                    },
                );
                if frame == 0 {
                    assert!(!output.shapes.is_empty());
                    assert!(center.glass.ready(&ctx, id));
                }
            }
            let rect = ctx
                .memory(|memory| memory.area_rect(id.with("glass_panel")))
                .unwrap();
            assert!(rect.width() <= 381.0, "panel stretched: {rect:?}");
            assert!(
                rect.left() >= 0.0 && rect.right() <= width + 1.0,
                "panel outside viewport: {rect:?}"
            );
        }
    }
    #[test]
    fn notification_status_colors_include_errors_before_success_words() {
        assert_eq!(
            message_color("Save failed: cannot update"),
            Color32::from_rgb(231, 82, 91)
        );
        assert_eq!(
            message_color("Review discovered devices"),
            Color32::from_rgb(236, 159, 66)
        );
        assert_eq!(
            message_color("Added 1 server(s)."),
            Color32::from_rgb(66, 188, 122)
        );
        assert_eq!(
            message_color("Scan started"),
            Color32::from_rgb(78, 151, 232)
        );
    }
}
