use eframe::egui::{self, Color32, RichText, Vec2};

const PAGE_SIZE: usize = 5;
const SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+1", "Devices"),
    ("Ctrl+2", "Local shell"),
    ("Ctrl+3", "Settings"),
    ("Ctrl+B", "Pin or unpin sidebar"),
    ("Ctrl+Z", "Undo inventory change"),
    ("Ctrl+Y / Ctrl+Shift+Z", "Redo inventory change"),
    ("Ctrl+Shift+C", "Copy terminal selection"),
    ("Ctrl+Shift+V / Right click", "Paste into terminal"),
    ("Left / Right arrows", "Move terminal cursor"),
    ("Ctrl+C", "Interrupt terminal command"),
    ("Ctrl+X", "Exit nano in terminal"),
    ("Escape", "Close dropdown or dialog"),
];

#[derive(Clone, Default)]
struct State {
    search: String,
    page: usize,
    glass: crate::frosted_popup::Glass,
}

fn matching(search: &str) -> Vec<(&'static str, &'static str)> {
    let query = search.trim().to_lowercase();
    SHORTCUTS
        .iter()
        .copied()
        .filter(|(key, description)| {
            key.to_lowercase().contains(&query) || description.to_lowercase().contains(&query)
        })
        .collect()
}

pub fn show(ui: &mut egui::Ui) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::click());
    let response = response
        .on_hover_text("Help and keyboard shortcuts")
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Help"));
    lab_manager::svg_icons::paint(
        ui.painter(),
        rect.shrink(4.0),
        "circle-help",
        Some(Color32::from_gray(190)),
    );
    let id = ui.make_persistent_id("help_menu");
    let state_id = id.with("state");
    let mut state = ui
        .ctx()
        .data(|d| d.get_temp::<State>(state_id))
        .unwrap_or_default();
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(id));
        if ui.memory(|m| m.is_popup_open(id)) {
            state.glass.capture(ui.ctx(), id);
        }
    }
    if ui.memory(|m| m.is_popup_open(id)) && state.glass.ready(ui.ctx(), id) {
        let body = lab_manager::frosted_modal::body_rect(ui.ctx());
        let width = 360.0_f32.min(body.width() - 48.0);
        let x = (rect.right() - width - 28.0).clamp(body.left() + 8.0, body.right() - width - 36.0);
        let popup = egui::Area::new(id.with("panel"))
            .order(egui::Order::Foreground)
            .movable(false)
            .constrain_to(body)
            .fixed_pos(egui::pos2(x, rect.bottom() + 12.0))
            .show(ui.ctx(), |ui| {
                state.glass.show(ui, |ui| {
                    ui.set_width(width);
                    ui.strong("Help · Keyboard shortcuts");
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut state.search)
                                .hint_text(
                                    RichText::new("Search shortcuts…")
                                        .color(Color32::from_gray(100)),
                                )
                                .desired_width(f32::INFINITY)
                                .margin(Vec2::new(10.0, 8.0)),
                        )
                        .changed()
                    {
                        state.page = 0;
                    }
                    ui.small("App shortcuts apply outside text fields and terminals.");
                    let entries = matching(&state.search);
                    let pages = entries.len().div_ceil(PAGE_SIZE).max(1);
                    state.page = state.page.min(pages - 1);
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt(("help_results", state.page, state.search.clone()))
                        .auto_shrink([false, false])
                        .max_height((body.height() - 180.0).clamp(60.0, 250.0))
                        .show(ui, |ui| {
                            ui.set_min_height(160.0_f32.min((body.height() - 180.0).max(0.0)));
                            if entries.is_empty() {
                                ui.label("No matching shortcuts.");
                            }
                            for (keys, description) in
                                entries.iter().skip(state.page * PAGE_SIZE).take(PAGE_SIZE)
                            {
                                ui.label(*description);
                                ui.label(
                                    RichText::new(*keys)
                                        .monospace()
                                        .color(Color32::from_gray(150)),
                                );
                                ui.add_space(5.0);
                            }
                        });
                    ui.separator();
                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_width(), 32.0),
                        egui::Layout::left_to_right(egui::Align::Center)
                            .with_main_align(egui::Align::Center),
                        |ui| {
                            if ui
                                .add_enabled(
                                    state.page > 0,
                                    egui::Button::new("‹ Previous").frame(false),
                                )
                                .clicked()
                            {
                                state.page -= 1;
                            }
                            ui.label(format!("{} / {}", state.page + 1, pages));
                            if ui
                                .add_enabled(
                                    state.page + 1 < pages,
                                    egui::Button::new("Next ›").frame(false),
                                )
                                .clicked()
                            {
                                state.page += 1;
                            }
                        },
                    );
                });
            });
        if ui.ctx().input(|i| {
            i.key_pressed(egui::Key::Escape)
                || (i.pointer.any_pressed()
                    && i.pointer
                        .interact_pos()
                        .is_some_and(|p| !popup.response.rect.contains(p) && !rect.contains(p)))
        }) {
            ui.memory_mut(|m| m.close_popup());
        }
    }
    ui.ctx().data_mut(|d| d.insert_temp(state_id, state));
}

#[cfg(test)]
mod tests {
    #[test]
    fn search_matches_keys_and_descriptions_and_results_are_paginated() {
        assert_eq!(super::matching("CTRL+B").len(), 1);
        assert_eq!(super::matching("nano")[0].0, "Ctrl+X");
        assert!(super::matching("unrecognized shortcut").is_empty());
        assert!(super::matching("").len() > super::PAGE_SIZE);
    }
}
