use eframe::egui::{self, Color32, RichText, Vec2};

pub const BLACK: Color32 = Color32::from_gray(15);
pub const WORKSPACE: Color32 = Color32::from_gray(28);
pub const FOOTER: Color32 = Color32::from_gray(23);
pub const MUTED: Color32 = Color32::from_gray(130);

pub fn lab_manager_icon(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::hover());
    let ink = Color32::from_rgb(75, 193, 131);
    ui.painter()
        .rect_filled(rect, 7.0, Color32::from_rgb(24, 55, 40));
    lab_manager::svg_icons::paint(ui.painter(), rect.shrink(4.0), "server", Some(ink));
}

pub fn sidebar_pin(ui: &mut egui::Ui, pinned: bool, label: bool) -> egui::Response {
    let size = Vec2::new(if label { ui.available_width() } else { 32.0 }, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.hovered() || pinned {
        ui.painter().rect_filled(rect, 6.0, Color32::from_gray(34));
    }
    let c = rect.left_center() + Vec2::new(16.0, 0.0);
    let ink = if pinned {
        Color32::from_rgb(75, 193, 131)
    } else {
        Color32::from_gray(185)
    };
    lab_manager::svg_icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(c, Vec2::splat(20.0)),
        if pinned { "pin" } else { "pin-off" },
        Some(ink),
    );
    if label {
        ui.painter().text(
            c + Vec2::new(18.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "Pin Sidebar",
            egui::FontId::proportional(13.0),
            ink,
        );
    }
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Pin Sidebar"));
    response
        .on_hover_text(if pinned {
            "Unpin Sidebar (Ctrl+B)"
        } else {
            "Pin Sidebar (Ctrl+B)"
        })
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn history_icon(ui: &mut egui::Ui, redo: bool, enabled: bool) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::click());
        let label = if redo {
            "Redo inventory change"
        } else {
            "Undo inventory change"
        };
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
        if response.hovered() || response.has_focus() {
            ui.painter().rect_filled(rect, 5.0, Color32::from_gray(45));
        }
        lab_manager::svg_icons::paint(
            ui.painter(),
            rect.shrink(5.0),
            if redo { "redo-2" } else { "undo-2" },
            Some(Color32::from_gray(if enabled { 190 } else { 75 })),
        );
        response
            .on_hover_text(label)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
}

pub fn page_arrow(
    ui: &mut egui::Ui,
    forward: bool,
    enabled: bool,
    compact: bool,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let label = if compact {
            ""
        } else if forward {
            "Next"
        } else {
            "Previous"
        };
        let ink = Color32::from_gray(if enabled { 190 } else { 78 });
        let galley =
            ui.painter()
                .layout_no_wrap(label.into(), egui::FontId::proportional(12.0), ink);
        let width = if compact {
            25.0
        } else {
            galley.size().x + 26.0
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), egui::Sense::click());
        let title = if forward {
            "Next page"
        } else {
            "Previous page"
        };
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, title));
        let arrow_x = if forward {
            rect.right() - 8.0
        } else {
            rect.left() + 8.0
        };
        let c = egui::pos2(arrow_x, rect.center().y);
        lab_manager::svg_icons::paint(
            ui.painter(),
            egui::Rect::from_center_size(c, Vec2::splat(16.0)),
            if forward {
                "chevron-right"
            } else {
                "chevron-left"
            },
            Some(ink),
        );
        if !compact {
            let x = if forward {
                rect.left() + 2.0
            } else {
                rect.left() + 20.0
            };
            ui.painter().galley(
                egui::pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                ink,
            );
        }
        response
            .on_hover_text(title)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
}

pub fn notification_bell(ui: &mut egui::Ui, unread: usize) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Notifications")
    });
    let c = rect.center();
    let p = ui.painter();
    lab_manager::svg_icons::paint(p, rect.shrink(4.0), "bell", Some(Color32::from_gray(190)));
    if unread > 0 {
        p.circle_filled(
            c + Vec2::new(8.0, -8.0),
            3.0,
            Color32::from_rgb(180, 130, 106),
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn logout_menu_item(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(180.0, 36.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Log out")
    });
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, 5.0, Color32::from_gray(45));
    }
    let icon_rect =
        egui::Rect::from_center_size(rect.left_center() + Vec2::new(18.0, 0.0), Vec2::splat(26.0));
    ui.painter().rect_stroke(
        icon_rect,
        7.0,
        egui::Stroke::new(1.0_f32, Color32::from_gray(75)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.left_center() + Vec2::new(40.0, 0.0),
        egui::Align2::LEFT_CENTER,
        "Log out",
        egui::FontId::proportional(13.0),
        Color32::from_gray(220),
    );
    lab_manager::svg_icons::paint(
        ui.painter(),
        icon_rect.shrink(4.0),
        "log-out",
        Some(Color32::from_gray(190)),
    );
    response
        .on_hover_text("Sign out and disable automatic sign-in")
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn paint_nav_icon(ui: &egui::Ui, c: egui::Pos2, kind: usize, active: bool) {
    let ink = if active {
        Color32::from_gray(235)
    } else {
        Color32::from_gray(145)
    };
    let name = match kind {
        0 => "globe",
        1 => "server",
        2 => "plus",
        3..=5 => "terminal",
        7 => "activity",
        _ => "hard-drive",
    };
    lab_manager::svg_icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(c, Vec2::splat(22.0)),
        name,
        Some(ink),
    );
}

pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_gray(35))
        .corner_radius(18.0)
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(47)))
        .inner_margin(20.0)
}

/// Native decorations are disabled, so explicitly hand edge drags to Windows.
pub fn resize_window(ctx: &egui::Context) {
    use egui::{CursorIcon, ResizeDirection};
    let (maximized, pointer, pressed) = ctx.input(|input| {
        (
            input.viewport().maximized.unwrap_or(false),
            input.pointer.hover_pos(),
            input.pointer.primary_pressed(),
        )
    });
    if maximized {
        return;
    }
    let Some(pointer) = pointer else {
        return;
    };
    let rect = ctx.screen_rect();
    if !rect.contains(pointer) {
        return;
    }
    let left = pointer.x - rect.left();
    let right = rect.right() - pointer.x;
    let top = pointer.y - rect.top();
    let bottom = rect.bottom() - pointer.y;
    let (direction, cursor) = if left < 14.0 && top < 14.0 {
        (ResizeDirection::NorthWest, CursorIcon::ResizeNwSe)
    } else if right < 14.0 && top < 14.0 {
        (ResizeDirection::NorthEast, CursorIcon::ResizeNeSw)
    } else if left < 14.0 && bottom < 14.0 {
        (ResizeDirection::SouthWest, CursorIcon::ResizeNeSw)
    } else if right < 14.0 && bottom < 14.0 {
        (ResizeDirection::SouthEast, CursorIcon::ResizeNwSe)
    } else if left < 5.0 {
        (ResizeDirection::West, CursorIcon::ResizeHorizontal)
    } else if right < 5.0 {
        (ResizeDirection::East, CursorIcon::ResizeHorizontal)
    } else if top < 5.0 {
        (ResizeDirection::North, CursorIcon::ResizeVertical)
    } else if bottom < 5.0 {
        (ResizeDirection::South, CursorIcon::ResizeVertical)
    } else {
        return;
    };
    ctx.set_cursor_icon(cursor);
    if pressed {
        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
    }
}

pub fn window_control(ui: &mut egui::Ui, kind: u8, maximized: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(32.0, 28.0), egui::Sense::click());
    let label = match kind {
        0 => "Minimize",
        1 if maximized => "Restore",
        1 => "Maximize",
        _ => "Close",
    };
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(
            rect,
            4.0,
            if kind == 2 {
                Color32::from_rgb(180, 45, 50)
            } else {
                Color32::from_gray(48)
            },
        );
    }
    let name = match kind {
        0 => "minus",
        1 if maximized => "panels-top-left",
        1 => "square",
        _ => "x",
    };
    lab_manager::svg_icons::paint(
        ui.painter(),
        rect.shrink2(Vec2::new(8.0, 6.0)),
        name,
        Some(Color32::from_gray(205)),
    );
    response
        .on_hover_text(label)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn configure(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.menu_corner_radius = egui::CornerRadius::same(8);
    style.visuals.window_stroke = egui::Stroke::new(1.0_f32, Color32::from_gray(65));
    style.visuals.panel_fill = BLACK;
    style.visuals.window_fill = WORKSPACE;
    style.visuals.extreme_bg_color = Color32::from_gray(23);
    style.visuals.faint_bg_color = Color32::from_gray(33);
    style.visuals.override_text_color = Some(Color32::from_gray(225));
    style.visuals.selection.bg_fill = Color32::from_gray(70);
    style.visuals.selection.stroke.color = Color32::from_gray(50);
    style.visuals.hyperlink_color = Color32::from_gray(220);
    style.visuals.widgets.inactive.bg_fill = Color32::from_gray(38);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_gray(38);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_gray(60));
    style.visuals.widgets.hovered.bg_fill = Color32::from_gray(50);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_gray(50);
    style.visuals.widgets.active.bg_fill = Color32::from_gray(65);
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(12.0, 8.0);
    for (text, size) in [
        (egui::TextStyle::Body, 14.0),
        (egui::TextStyle::Button, 14.0),
        (egui::TextStyle::Small, 12.0),
    ] {
        style
            .text_styles
            .insert(text, egui::FontId::proportional(size));
    }
    ctx.set_style(style);
}

/// Left-aligned links with the dotted, animated branch from the navigation reference.
pub fn nav_group(
    ui: &mut egui::Ui,
    name: &str,
    labels: &[&str],
    icons: &[usize],
    selected: Option<usize>,
    label_opacity: f32,
) -> Option<usize> {
    ui.add_space(20.0);
    ui.label(
        RichText::new(name)
            .size(10.0)
            .color(MUTED.linear_multiply(label_opacity)),
    );
    ui.add_space(6.0);
    let top = ui.cursor().top();
    let mut chosen = None;
    let mut hovered = None;
    let mut centers = Vec::new();
    for (index, label) in labels.iter().enumerate() {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), egui::Sense::click());
        if response.hovered() {
            hovered = Some(index);
        }
        let color = if response.hovered() || response.has_focus() || selected == Some(index) {
            Color32::from_gray(240)
        } else {
            MUTED
        };
        paint_nav_icon(
            ui,
            rect.left_center() + Vec2::new(18.0, 0.0),
            icons[index],
            selected == Some(index) || response.hovered(),
        );
        ui.painter().text(
            rect.left_center() + Vec2::new(48.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(14.0),
            color.linear_multiply(label_opacity),
        );
        if response.clicked() {
            chosen = Some(index);
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
        centers.push(rect.center().y);
    }
    // Separate lanes and animation IDs keep hover from displacing selection.
    for (kind, target, offset, line, tip) in [
        (
            "active",
            selected,
            2.0,
            Color32::from_gray(155),
            Color32::from_rgb(180, 130, 106),
        ),
        (
            "hover",
            hovered,
            8.0,
            Color32::from_gray(90),
            Color32::from_gray(215),
        ),
    ] {
        let Some(index) = target else {
            continue;
        };
        let y = ui
            .ctx()
            .animate_value_with_time(ui.id().with((name, kind)), centers[index], 0.16);
        let x = ui.min_rect().left() + offset;
        let start = top + 4.0;
        let radius = 8.0_f32.min((y - start).max(0.0));
        let vertical = (y - start - radius).max(0.0);
        let arc = radius * std::f32::consts::FRAC_PI_2;
        let length = vertical + arc + 12.0 - radius;
        let mut distance = 0.0;
        while distance <= length {
            let point = if distance <= vertical {
                egui::pos2(x, start + distance)
            } else if distance < vertical + arc {
                let angle = std::f32::consts::PI - (distance - vertical) / radius;
                egui::pos2(
                    x + radius + radius * angle.cos(),
                    y - radius + radius * angle.sin(),
                )
            } else {
                egui::pos2(x + radius + distance - vertical - arc, y)
            };
            ui.painter().circle_filled(point, 0.75, line);
            distance += 3.5;
        }
        ui.painter()
            .circle_filled(egui::pos2(x + 15.0, y), 1.5, tip);
    }
    chosen
}
