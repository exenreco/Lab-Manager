use eframe::egui::{self, Color32, Vec2};
use lab_manager::{
    model::Server,
    monitoring::{Monitoring, key},
};

const HEADER: f32 = 56.0;
const ROW: f32 = 56.0;

#[cfg(test)]
pub fn show(ui: &mut egui::Ui, monitoring: &mut Monitoring, servers: &[Server]) {
    show_sized(ui, monitoring, servers, 320.0);
}

pub fn show_sized(ui: &mut egui::Ui, monitoring: &mut Monitoring, servers: &[Server], width: f32) {
    if servers.is_empty() {
        monitoring.selected = None;
        ui.add_enabled(false, egui::Button::new("No devices"));
        return;
    }
    if monitoring
        .selected
        .as_ref()
        .is_none_or(|k| !servers.iter().any(|s| key(s) == *k))
    {
        monitoring.selected = Some(key(&servers[0]));
    }
    let id = ui.make_persistent_id("monitoring_device_dropdown");
    if !ui.is_enabled() {
        ui.memory_mut(|m| m.close_popup());
    }
    let width = width.min(ui.max_rect().width()).max(80.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, HEADER), egui::Sense::hover());
    let trigger = ui.interact(rect, id.with("trigger"), egui::Sense::click());
    trigger.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Choose monitoring device")
    });
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new("monitoring_device_trigger"), rect));
    if trigger.clicked() {
        ui.memory_mut(|m| m.toggle_popup(id));
    }
    let open = ui.memory(|m| m.is_popup_open(id));
    let progress = ui.ctx().animate_bool_with_time_and_easing(
        id.with("expand"),
        open,
        0.32,
        egui::emath::easing::cubic_out,
    );
    if progress < 0.001 {
        paint_header(ui, rect, monitoring, servers, 0.0);
        return;
    }
    let screen = ui.ctx().screen_rect();
    let list_height =
        (servers.len() as f32 * ROW + 12.0).min((screen.bottom() - rect.bottom() - 12.0).max(ROW));
    let height = HEADER + list_height * progress;
    let panel = egui::Area::new(id.with("expanding_panel"))
        .order(egui::Order::Foreground)
        .fixed_pos(rect.min)
        .movable(false)
        .constrain(false)
        .show(ui.ctx(), |ui| {
            let panel = egui::Rect::from_min_size(rect.min, Vec2::new(width, height));
            ui.set_min_size(panel.size());
            ui.set_clip_rect(panel.intersect(screen));
            ui.painter()
                .rect_filled(panel, 24.0, Color32::from_gray(20));
            #[cfg(test)]
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("monitoring_device_panel"), panel));
            paint_header(ui, rect, monitoring, servers, progress);
            let arrow = egui::Rect::from_center_size(
                rect.right_center() - Vec2::new(58.0, 0.0),
                Vec2::splat(32.0),
            );
            let close = egui::Rect::from_center_size(
                rect.right_center() - Vec2::new(24.0, 0.0),
                Vec2::splat(32.0),
            );
            let arrow_response = ui.interact(arrow, id.with("collapse"), egui::Sense::click());
            let close_response = ui.interact(close, id.with("close"), egui::Sense::click());
            if arrow_response.on_hover_text("Collapse devices").clicked()
                || close_response.on_hover_text("Close devices").clicked()
            {
                ui.memory_mut(|m| m.close_popup());
            }
            let body = egui::Rect::from_min_max(
                rect.left_bottom() + Vec2::new(12.0, 0.0),
                panel.right_bottom() - Vec2::new(12.0, 8.0),
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("device_rows")
                    .max_rect(egui::Rect::from_min_size(
                        body.min,
                        Vec2::new(body.width(), list_height - 8.0),
                    ))
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );
            child.set_clip_rect(body.intersect(panel).intersect(screen));
            child.set_opacity(progress);
            if !open {
                child.disable();
            }
            egui::ScrollArea::vertical()
                .max_height(list_height - 8.0)
                .id_salt(id.with("scroll"))
                .show(&mut child, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for server in servers {
                        let k = key(server);
                        let (row, _) = ui.allocate_exact_size(
                            Vec2::new(body.width(), ROW),
                            egui::Sense::hover(),
                        );
                        if ui.rect_contains_pointer(row) {
                            ui.painter().rect_filled(
                                row.shrink2(Vec2::new(0.0, 3.0)),
                                12.0,
                                Color32::from_gray(30),
                            );
                        }
                        paint_device(ui, row, server, monitoring, true);
                        let response =
                            ui.interact(row, id.with(("option", &k)), egui::Sense::click());
                        #[cfg(test)]
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(
                                egui::Id::new(("monitoring_device_option", k.clone())),
                                row,
                            )
                        });
                        if response
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            monitoring.selected = Some(k);
                            ui.memory_mut(|m| m.close_popup());
                        }
                    }
                });
        });
    let outside = ui.ctx().input(|i| {
        i.key_pressed(egui::Key::Escape)
            || (i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !panel.response.rect.contains(p)))
    });
    if outside {
        ui.memory_mut(|m| m.close_popup());
    }
}

fn paint_header(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    monitoring: &Monitoring,
    servers: &[Server],
    progress: f32,
) {
    ui.painter().rect_filled(rect, 24.0, Color32::from_gray(20));
    if let Some(server) = servers.iter().find(|s| Some(key(s)) == monitoring.selected) {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.set_opacity(1.0 - progress);
        paint_device(&mut child, rect, server, monitoring, false);
    }
    let color = Color32::from_white_alpha((progress * 235.0) as u8);
    ui.painter().text(
        rect.left_center() + Vec2::new(20.0, 0.0),
        egui::Align2::LEFT_CENTER,
        "Devices",
        egui::FontId::proportional(16.0),
        color,
    );
    if progress > 0.001 {
        ui.painter().line_segment(
            [
                rect.left_bottom() + Vec2::new(18.0, -1.0),
                rect.right_bottom() + Vec2::new(-18.0, -1.0),
            ],
            egui::Stroke::new(1.0_f32, Color32::from_white_alpha((progress * 22.0) as u8)),
        );
    }
    let c = rect.right_center() - Vec2::new(24.0 + 34.0 * progress, 0.0);
    lab_manager::svg_icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(c, Vec2::splat(18.0)),
        if progress > 0.5 {
            "chevron-right"
        } else {
            "chevron-down"
        },
        Some(Color32::from_gray(210)),
    );
    let c = rect.right_center() - Vec2::new(24.0, 0.0);
    lab_manager::svg_icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(c, Vec2::splat(18.0)),
        "x",
        Some(Color32::from_white_alpha((progress * 210.0) as u8)),
    );
}

fn paint_device(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    server: &Server,
    monitoring: &Monitoring,
    list: bool,
) {
    let history = monitoring.database.devices.get(&key(server));
    let color = if history.is_some_and(|h| !h.config.enabled) {
        Color32::from_gray(115)
    } else if let Some(s) = history.and_then(|h| h.samples.last()) {
        if s.online() {
            Color32::from_rgb(65, 188, 122)
        } else {
            Color32::from_rgb(230, 80, 90)
        }
    } else {
        Color32::from_rgb(230, 164, 70)
    };
    ui.painter()
        .circle_filled(rect.left_center() + Vec2::new(15.0, 0.0), 3.5, color);
    let icon =
        egui::Rect::from_center_size(rect.left_center() + Vec2::new(43.0, 0.0), Vec2::splat(32.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(icon));
    child.set_clip_rect(icon.intersect(ui.clip_rect()));
    lab_manager::device_icons::thumbnail(
        &mut child,
        server.icon,
        server.custom_icon.as_deref(),
        32.0,
    );
    let text = egui::Rect::from_min_max(
        rect.min + Vec2::new(70.0, 0.0),
        rect.max - Vec2::new(if list { 80.0 } else { 42.0 }, 0.0),
    );
    let galley = ui.painter().layout_no_wrap(
        server.name.clone(),
        egui::FontId::proportional(13.0),
        Color32::from_gray(225),
    );
    ui.painter()
        .with_clip_rect(text.intersect(ui.clip_rect()))
        .galley(
            text.left_center() - Vec2::new(0.0, 16.0),
            galley,
            Color32::from_gray(225),
        );
    ui.painter()
        .with_clip_rect(text.intersect(ui.clip_rect()))
        .text(
            text.left_center() + Vec2::new(0.0, 5.0),
            egui::Align2::LEFT_TOP,
            server.ip.to_string(),
            egui::FontId::proportional(11.0),
            Color32::from_gray(140),
        );
    if list {
        let selected = monitoring.selected.as_ref() == Some(&key(server));
        let pill = egui::Rect::from_center_size(
            rect.right_center() - Vec2::new(37.0, 0.0),
            Vec2::new(66.0, 25.0),
        );
        if selected {
            ui.painter().rect_filled(pill, 12.0, Color32::from_gray(38));
        } else {
            ui.painter().rect_stroke(
                pill,
                12.0,
                egui::Stroke::new(1.0_f32, Color32::from_gray(48)),
                egui::StrokeKind::Inside,
            );
        }
        ui.painter().text(
            pill.center(),
            egui::Align2::CENTER_CENTER,
            if selected { "Selected" } else { "View" },
            egui::FontId::proportional(11.0),
            Color32::from_gray(200),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dropdown_defaults_selects_device_and_recovers_after_removal() {
        let ctx = egui::Context::default();
        let mut monitoring = Monitoring::default();
        let mut servers = vec![
            Server::new("192.0.2.1".parse().unwrap()),
            Server::new("192.0.2.2".parse().unwrap()),
        ];
        let run = |monitoring: &mut Monitoring, servers: &[Server], events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1000.0, 800.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| show(ui, monitoring, servers));
                },
            );
        };
        for _ in 0..2 {
            run(&mut monitoring, &servers, vec![]);
        }
        assert_eq!(monitoring.selected, Some(key(&servers[0])));
        let trigger = ctx
            .data(|d| {
                d.get_temp::<egui::Rect>(egui::Id::new("monitoring_device_trigger"))
                    .unwrap()
            })
            .center();
        for pressed in [true, false] {
            run(
                &mut monitoring,
                &servers,
                vec![
                    egui::Event::PointerMoved(trigger),
                    egui::Event::PointerButton {
                        pos: trigger,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        run(&mut monitoring, &servers, vec![]);
        let expanding = ctx.data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new("monitoring_device_panel"))
                .unwrap()
        });
        for _ in 0..30 {
            run(&mut monitoring, &servers, vec![]);
        }
        let expanded = ctx.data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new("monitoring_device_panel"))
                .unwrap()
        });
        let origin = ctx.data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new("monitoring_device_trigger"))
                .unwrap()
        });
        assert_eq!(expanded.min, origin.min);
        assert!(expanded.height() > expanding.height());
        let option = ctx
            .data(|d| {
                d.get_temp::<egui::Rect>(egui::Id::new((
                    "monitoring_device_option",
                    key(&servers[1]),
                )))
                .unwrap()
            })
            .center();
        for pressed in [true, false] {
            run(
                &mut monitoring,
                &servers,
                vec![
                    egui::Event::PointerMoved(option),
                    egui::Event::PointerButton {
                        pos: option,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        assert_eq!(monitoring.selected, Some(key(&servers[1])));
        servers.pop();
        run(&mut monitoring, &servers, vec![]);
        assert_eq!(monitoring.selected, Some(key(&servers[0])));
        run(&mut monitoring, &[], vec![]);
        assert!(monitoring.selected.is_none());
    }
}
