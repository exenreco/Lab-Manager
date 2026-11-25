use crate::design;
use eframe::egui::{self, Color32, Vec2};

pub fn width(available: f32) -> f32 {
    if available < 360.0 {
        166.0
    } else if available < 700.0 {
        306.0
    } else {
        484.0
    }
}

pub fn search_controls(
    ui: &mut egui::Ui,
    search: &mut String,
    scan_active: bool,
    add_active: bool,
    grid: &mut bool,
    terminals: (usize, bool),
) -> (bool, bool, bool) {
    let available = ui.available_width();
    let dock_width = width(available);
    let one_row = available >= dock_width + 156.0;
    let (row, _) = ui.allocate_exact_size(Vec2::new(available, 46.0), egui::Sense::hover());
    let search_width = if one_row {
        (available - dock_width - 12.0).min(644.0)
    } else {
        available
    };
    let search_rect = egui::Rect::from_min_size(row.min, Vec2::new(search_width, 46.0));
    let mut field = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("device_search")
            .max_rect(search_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    field.add(
        egui::TextEdit::singleline(search)
            .hint_text(egui::RichText::new("Search name or IP...").color(Color32::from_gray(100)))
            .desired_width((search_width - 24.0).max(20.0))
            .min_size(search_rect.size())
            .vertical_align(egui::Align::Center)
            .margin(egui::Margin::symmetric(12, 8)),
    );
    let dock_row = if one_row {
        row
    } else {
        ui.add_space(8.0);
        ui.allocate_exact_size(Vec2::new(available, 46.0), egui::Sense::hover())
            .0
    };
    let rect = egui::Rect::from_min_size(
        egui::pos2(dock_row.right() - dock_width, dock_row.top()),
        Vec2::new(dock_width, 46.0),
    );
    let mut dock = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("aligned_dashboard_controls")
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(egui::Id::new("device_search_rect"), search_rect);
        data.insert_temp(egui::Id::new("device_actions_rect"), rect);
    });
    show(
        &mut dock,
        available,
        scan_active,
        add_active,
        grid,
        terminals,
    )
}

pub fn show(
    ui: &mut egui::Ui,
    available: f32,
    scan_active: bool,
    add_active: bool,
    grid: &mut bool,
    terminals: (usize, bool),
) -> (bool, bool, bool) {
    let compact = available < 700.0;
    let icon_only = available < 360.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width(available), 46.0), egui::Sense::hover());
    paint_capsule(ui, rect);
    let mut dock = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("dashboard_action_dock")
            .max_rect(rect.shrink(5.0))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    dock.spacing_mut().item_spacing.x = 4.0;
    let scan = action(
        &mut dock,
        0,
        "Scan network",
        if compact { 36.0 } else { 126.0 },
        scan_active,
        compact,
    )
    .clicked();
    let add = action(
        &mut dock,
        1,
        "Add Device",
        if compact { 36.0 } else { 124.0 },
        add_active,
        compact,
    )
    .clicked();
    view_toggle(&mut dock, grid, icon_only);
    let terminal = action(
        &mut dock,
        2,
        &format!("Terminals ({})", terminals.0),
        if icon_only { 36.0 } else { 128.0 },
        terminals.1,
        icon_only,
    )
    .clicked();

    (scan, add, terminal)
}

fn action(
    ui: &mut egui::Ui,
    kind: u8,
    label: &str,
    width: f32,
    active: bool,
    compact: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 36.0), egui::Sense::click());
    #[cfg(test)]
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new(("collection_action", kind)), rect));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    surface(ui, rect, &response, active);
    let c = egui::pos2(rect.left() + 18.0, rect.center().y);
    let ink = Color32::from_gray(if active { 245 } else { 190 });
    lab_manager::svg_icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(c, Vec2::splat(20.0)),
        match kind {
            0 => "radar",
            1 => "plus",
            _ => "terminal",
        },
        Some(ink),
    );
    if !compact {
        ui.painter().text(
            egui::pos2(rect.left() + 36.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(12.0),
            ink,
        );
    }
    response
        .on_hover_text(label)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn surface(ui: &egui::Ui, rect: egui::Rect, response: &egui::Response, active: bool) {
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.15);
    let fill = if active {
        65
    } else {
        (28.0 + 15.0 * hover) as u8
    };
    ui.painter().rect_filled(
        rect.shrink(if response.is_pointer_button_down_on() {
            1.0
        } else {
            0.0
        }),
        18.0,
        Color32::from_gray(fill),
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            18.0,
            egui::Stroke::new(1.0_f32, Color32::from_gray(140)),
            egui::StrokeKind::Inside,
        );
    }
}

/// A single control whose four blocks animate into four list rows and back.
pub fn view_toggle(ui: &mut egui::Ui, grid: &mut bool, icon_only: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(if icon_only { 36.0 } else { 84.0 }, 36.0),
        egui::Sense::click(),
    );
    #[cfg(test)]
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new("collection_view_toggle"), rect));
    if response.clicked() {
        *grid = !*grid;
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            if *grid {
                "Grid view; switch to list"
            } else {
                "List view; switch to grid"
            },
        )
    });
    surface(ui, rect, &response, true);
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id.with("grid_to_list"), !*grid, 0.45);
    let c = egui::pos2(rect.left() + 18.0, rect.center().y);
    let ink = Color32::from_gray(230);
    let p = ui.painter();
    lab_manager::svg_icons::paint(
        p,
        egui::Rect::from_center_size(c, Vec2::splat(22.0)),
        "layout-grid",
        Some(ink.gamma_multiply(1.0 - t)),
    );
    lab_manager::svg_icons::paint(
        p,
        egui::Rect::from_center_size(c, Vec2::splat(22.0)),
        "list",
        Some(ink.gamma_multiply(t)),
    );
    if !icon_only {
        let text_rect = egui::Rect::from_min_max(
            egui::pos2(rect.left() + 36.0, rect.top() + 6.0),
            rect.max - Vec2::new(8.0, 6.0),
        );
        let painter = p.with_clip_rect(text_rect);
        painter.text(
            egui::pos2(text_rect.left(), text_rect.center().y - 12.0 * t),
            egui::Align2::LEFT_CENTER,
            "Grid",
            egui::FontId::proportional(12.0),
            ink.gamma_multiply(1.0 - t),
        );
        painter.text(
            egui::pos2(text_rect.left(), text_rect.center().y + 12.0 * (1.0 - t)),
            egui::Align2::LEFT_CENTER,
            "List",
            egui::FontId::proportional(12.0),
            ink.gamma_multiply(t),
        );
    }
    response
        .on_hover_text(if *grid {
            "Switch to list view"
        } else {
            "Switch to grid view"
        })
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn paint_capsule(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter()
        .rect_filled(rect, rect.height() / 2.0, design::BLACK);
    ui.painter().rect_stroke(
        rect,
        rect.height() / 2.0,
        egui::Stroke::new(1.0_f32, Color32::from_gray(42)),
        egui::StrokeKind::Inside,
    );
}

pub fn paint_top_toolbar(ui: &egui::Ui, rect: egui::Rect) {
    let corners = if cfg!(target_os = "macos") {
        egui::CornerRadius {
            nw: 0,
            sw: 0,
            ne: 22,
            se: 22,
        }
    } else {
        egui::CornerRadius {
            nw: 22,
            sw: 22,
            ne: 0,
            se: 0,
        }
    };
    let painter = ui
        .painter()
        .with_clip_rect(ui.clip_rect().union(rect).intersect(ui.ctx().screen_rect()));
    painter.rect_filled(rect, corners, design::BLACK);
    painter.rect_stroke(
        rect,
        corners,
        egui::Stroke::new(1.0_f32, Color32::from_gray(42)),
        egui::StrokeKind::Inside,
    );
}

pub fn separator(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 28.0), egui::Sense::hover());
    ui.painter().line_segment(
        [
            rect.center() - Vec2::new(0.0, 7.0),
            rect.center() + Vec2::new(0.0, 7.0),
        ],
        egui::Stroke::new(1.0_f32, Color32::from_gray(60)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_terminal_control_is_after_view_toggle_and_clickable() {
        let ctx = egui::Context::default();
        let mut grid = false;
        let mut search = String::new();
        let mut clicked = false;
        let mut frame = |events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1100.0, 760.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        clicked =
                            search_controls(ui, &mut search, false, false, &mut grid, (0, false)).2;
                    });
                },
            );
        };
        frame(vec![]);
        let terminal = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("collection_action", 2u8))))
            .unwrap();
        let view = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("collection_view_toggle")))
            .unwrap();
        assert!(terminal.left() > view.right());
        assert_eq!(terminal.center().y, view.center().y);
        frame(vec![egui::Event::PointerMoved(terminal.center())]);
        for pressed in [true, false] {
            frame(vec![egui::Event::PointerButton {
                pos: terminal.center(),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
        assert!(clicked);
    }
    #[test]
    fn single_toggle_changes_view_in_both_directions() {
        let ctx = egui::Context::default();
        let mut grid = false;
        let mut rect = egui::Rect::NOTHING;
        let mut run = |events: Vec<egui::Event>, grid: &mut bool| {
            let _ = ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        rect = view_toggle(ui, grid, false).rect;
                    });
                },
            );
            rect.center()
        };
        let pos = run(vec![], &mut grid);
        for expected in [true, false] {
            run(vec![egui::Event::PointerMoved(pos)], &mut grid);
            for pressed in [true, false] {
                run(
                    vec![egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    }],
                    &mut grid,
                );
            }
            assert_eq!(grid, expected);
        }
    }
    #[test]
    fn search_and_controls_are_spaced_to_opposite_edges() {
        for available in [220.0, 600.0, 1400.0] {
            let ctx = egui::Context::default();
            let mut search = String::new();
            let mut grid = false;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(available + 16.0, 800.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        search_controls(ui, &mut search, false, false, &mut grid, (0, false));
                    });
                },
            );
            let search = ctx.data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new("device_search_rect"))
                    .unwrap()
            });
            let controls = ctx.data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new("device_actions_rect"))
                    .unwrap()
            });
            assert!((search.left() - 8.0).abs() < 0.1);
            assert!((controls.right() - (available + 8.0)).abs() < 0.1);
            if available > 400.0 {
                assert!((search.center().y - controls.center().y).abs() < 0.1);
            } else {
                assert!(controls.top() > search.bottom());
            }
        }
    }
}
