use eframe::egui::{self, Color32, RichText, Vec2};
pub fn configure(ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing = Vec2::new(10.0, 14.0);
    ui.spacing_mut().interact_size.y = 34.0;
    ui.visuals_mut().widgets.inactive.corner_radius = 7.into();
    ui.visuals_mut().widgets.hovered.corner_radius = 7.into();
    ui.visuals_mut().widgets.active.corner_radius = 7.into();
    ui.visuals_mut().extreme_bg_color = Color32::from_gray(24);
}
pub fn os_selector(
    ui: &mut egui::Ui,
    os: &mut Option<crate::model::DeviceOs>,
    icon: crate::model::ServerIcon,
) {
    let inferred = crate::model::DeviceOs::from_icon(icon);
    row(ui, "OS", 0, |ui| {
        egui::ComboBox::from_id_salt("device_os")
            .width(ui.available_width())
            .selected_text(os.unwrap_or(inferred).label())
            .show_ui(ui, |ui| {
                ui.selectable_value(os, None, format!("Automatic ({})", inferred.label()));
                for option in crate::model::DeviceOs::ALL {
                    ui.selectable_value(os, Some(option), option.label());
                }
            });
        if os.is_none() {
            ui.small("Chosen from the device icon");
        }
    });
}
pub fn os_badge(ui: &mut egui::Ui, os: crate::model::DeviceOs) {
    egui::Frame::new()
        .fill(Color32::from_gray(30))
        .corner_radius(12.0)
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(48)))
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(os.label())
                    .size(12.0)
                    .color(Color32::from_gray(190)),
            );
        });
}
pub fn row(ui: &mut egui::Ui, label: &str, kind: usize, content: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();
    let label_width = (width * 0.35).clamp(78.0, 120.0);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(label_width, 34.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(label_width);
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                crate::svg_icons::paint(
                    ui.painter(),
                    rect,
                    match kind {
                        1 => "tag",
                        2 => "user",
                        3 => "globe",
                        _ => "file-text",
                    },
                    Some(Color32::from_gray(130)),
                );
                ui.label(
                    RichText::new(label)
                        .size(12.0)
                        .color(Color32::from_gray(170)),
                );
            },
        );
        ui.allocate_ui_with_layout(
            Vec2::new((width - label_width - 10.0).max(50.0), 34.0),
            egui::Layout::top_down(egui::Align::LEFT),
            content,
        );
    });
}

pub fn port_input(ui: &mut egui::Ui, id: &str, port: &mut u16) {
    let id = ui.make_persistent_id(id);
    let mut text = if ui.memory(|memory| memory.has_focus(id)) {
        ui.ctx()
            .data(|data| data.get_temp::<String>(id))
            .unwrap_or_else(|| port.to_string())
    } else {
        port.to_string()
    };
    let response = ui.add(
        egui::TextEdit::singleline(&mut text)
            .id(id)
            .horizontal_align(egui::Align::LEFT)
            .margin(Vec2::new(10.0, 9.0))
            .desired_width(f32::INFINITY),
    );
    if response.changed()
        && let Ok(value) = text.parse::<u16>()
        && value > 0
    {
        *port = value;
    }
    if !response.has_focus() {
        text = port.to_string();
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, text));
}

pub fn organization(
    ui: &mut egui::Ui,
    group: &mut String,
    tags: &mut String,
    notes: &mut String,
    links: &mut String,
    reminder_at: &mut Option<u64>,
    repeat_seconds: &mut Option<u64>,
) {
    egui::CollapsingHeader::new("Organization & notes").show(ui, |ui| {
        row(ui, "Group", 0, |ui| {
            ui.add(
                egui::TextEdit::singleline(group)
                    .hint_text(
                        eframe::egui::RichText::new("e.g. Home lab")
                            .color(eframe::egui::Color32::from_gray(100)),
                    )
                    .char_limit(128)
                    .margin(Vec2::new(10.0, 9.0))
                    .desired_width(f32::INFINITY),
            );
        });
        row(ui, "Tags", 1, |ui| {
            ui.add(
                egui::TextEdit::singleline(tags)
                    .hint_text(
                        eframe::egui::RichText::new("production, DNS, upstairs")
                            .color(eframe::egui::Color32::from_gray(100)),
                    )
                    .char_limit(1024)
                    .margin(Vec2::new(10.0, 9.0))
                    .desired_width(f32::INFINITY),
            );
        });
        row(ui, "Notes", 0, |ui| {
            ui.add(
                egui::TextEdit::multiline(notes)
                    .hint_text(
                        eframe::egui::RichText::new("Maintenance notes and useful details")
                            .color(eframe::egui::Color32::from_gray(100)),
                    )
                    .char_limit(16384)
                    .desired_rows(4)
                    .margin(Vec2::new(10.0, 9.0))
                    .desired_width(f32::INFINITY),
            );
        });
        row(ui, "Links", 3, |ui| {
            ui.add(
                egui::TextEdit::multiline(links)
                    .hint_text(
                        eframe::egui::RichText::new("One HTTP/HTTPS URL per line")
                            .color(eframe::egui::Color32::from_gray(100)),
                    )
                    .char_limit(4096)
                    .desired_rows(2)
                    .margin(Vec2::new(10.0, 9.0))
                    .desired_width(f32::INFINITY),
            );
            for link in links
                .lines()
                .map(str::trim)
                .filter(|link| link.starts_with("https://") || link.starts_with("http://"))
            {
                ui.hyperlink_to(link, link);
            }
        });
        row(ui, "Reminder", 0, |ui| {
            let id = ui.make_persistent_id("reminder_choice");
            let saved_at = ui
                .ctx()
                .data(|d| d.get_temp::<Option<u64>>(id.with("saved_at")));
            if saved_at != Some(*reminder_at) {
                ui.ctx().data_mut(|d| {
                    d.remove::<usize>(id);
                    d.remove::<[u64; 3]>(id.with("duration"));
                });
            }
            let mut choice = ui
                .ctx()
                .data(|d| d.get_temp::<usize>(id))
                .unwrap_or(if reminder_at.is_some() { 4 } else { 0 });
            let labels = ["None", "In 1 hour", "Tomorrow", "In 7 days", "Custom"];
            let old = choice;
            egui::ComboBox::from_id_salt("reminder_menu")
                .selected_text(labels[choice])
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for (index, label) in labels.iter().enumerate() {
                        ui.selectable_value(&mut choice, index, *label);
                    }
                });
            if choice != old {
                ui.ctx()
                    .data_mut(|d| d.remove::<[u64; 3]>(id.with("duration")));
                *repeat_seconds = None;
                *reminder_at = match choice {
                    1 => Some(crate::notifications::now() + 3600),
                    2 => Some(crate::notifications::now() + 86400),
                    3 => Some(crate::notifications::now() + 604800),
                    4 => Some(crate::notifications::now() + 3600),
                    _ => None,
                };
            }
            ui.ctx().data_mut(|d| d.insert_temp(id, choice));
            if choice == 4 {
                ui.small("Custom time from now");
                let duration_id = id.with("duration");
                let mut duration = ui
                    .ctx()
                    .data(|d| d.get_temp::<[u64; 3]>(duration_id))
                    .unwrap_or_else(|| {
                        let seconds = repeat_seconds.unwrap_or_else(|| {
                            reminder_at
                                .unwrap_or(0)
                                .saturating_sub(crate::notifications::now())
                        });
                        [seconds / 86400, seconds / 3600 % 24, seconds / 60 % 60]
                    });
                let mut changed = false;
                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut duration[0])
                                .range(0..=3650)
                                .suffix(" days"),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut duration[1])
                                .range(0..=23)
                                .suffix(" h"),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut duration[2])
                                .range(0..=59)
                                .suffix(" min"),
                        )
                        .changed();
                });
                let seconds = (duration[0] * 86400 + duration[1] * 3600 + duration[2] * 60).max(60);
                let mut repeat = repeat_seconds.is_some();
                let repeat_changed = ui
                    .checkbox(&mut repeat, "Repeat reminder at this interval")
                    .changed();
                if changed {
                    *reminder_at = Some(crate::notifications::now() + seconds);
                }
                if changed || repeat_changed {
                    *repeat_seconds = repeat.then_some(seconds);
                }
                ui.ctx().data_mut(|d| d.insert_temp(duration_id, duration));
            }
            ui.ctx()
                .data_mut(|d| d.insert_temp(id.with("saved_at"), *reminder_at));
            if let Some(at) = *reminder_at {
                let remaining = at.saturating_sub(crate::notifications::now());
                ui.small(format!(
                    "Due in {}d {}h {}m",
                    remaining / 86400,
                    remaining / 3600 % 24,
                    remaining / 60 % 60
                ));
                ui.small("Notified while Lab Manager is open");
            }
        });
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn https_port_change_is_not_overwritten_by_cached_input() {
        let ctx = eframe::egui::Context::default();
        let mut port: u16;
        for expected in [80, 443, 8080] {
            port = expected;
            let _ = ctx.run(Default::default(), |ctx| {
                eframe::egui::CentralPanel::default()
                    .show(ctx, |ui| super::port_input(ui, "web", &mut port));
            });
            assert_eq!(port, expected);
        }
    }
}
