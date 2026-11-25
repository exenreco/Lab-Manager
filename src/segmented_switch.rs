use eframe::egui::{self, Color32, Vec2};

pub struct SwitchResponse {
    pub changed: bool,
    pub options: [egui::Rect; 2],
}

/// Shared pill selector with a sliding thumb and a short stretch while pressed.
pub fn show(
    ui: &mut egui::Ui,
    id: &str,
    labels: [&str; 2],
    second: &mut bool,
    width: f32,
) -> SwitchResponse {
    let width = width.min(ui.available_width()).max(100.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 38.0), egui::Sense::hover());
    let id = ui.make_persistent_id(id);
    let options = [
        egui::Rect::from_min_max(rect.min, rect.center_bottom()),
        egui::Rect::from_min_max(rect.center_top(), rect.max),
    ];
    let before = *second;
    let mut held = false;
    let mut focused = false;
    for (index, option) in options.iter().enumerate() {
        let response = ui.interact(*option, id.with(index), egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                ui.is_enabled(),
                usize::from(*second) == index,
                labels[index],
            )
        });
        if response.clicked() {
            *second = index == 1;
        }
        held |= response.is_pointer_button_down_on();
        focused |= response.has_focus();
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    let progress = ui.ctx().animate_bool_with_time_and_easing(
        id.with("slide"),
        *second,
        0.28,
        egui::emath::easing::cubic_out,
    );
    let press = ui
        .ctx()
        .animate_bool_with_time(id.with("stretch"), held, 0.12);
    let inner = rect.shrink(4.0);
    let thumb_width = inner.width() * 0.5 + 9.0 * press;
    let center = egui::pos2(
        egui::lerp(
            (inner.left() + thumb_width / 2.0)..=(inner.right() - thumb_width / 2.0),
            progress,
        ),
        inner.center().y,
    );
    let thumb =
        egui::Rect::from_center_size(center, Vec2::new(thumb_width, inner.height() - 2.0 * press));
    ui.painter().rect_filled(rect, 19.0, Color32::from_gray(22));
    ui.painter().rect_stroke(
        rect,
        19.0,
        egui::Stroke::new(1.0_f32, Color32::from_gray(if focused { 110 } else { 55 })),
        egui::StrokeKind::Inside,
    );
    ui.painter().rect_filled(
        thumb.translate(Vec2::new(0.0, 1.0)),
        16.0,
        Color32::from_black_alpha(35),
    );
    ui.painter()
        .rect_filled(thumb, 16.0, Color32::from_gray(61));
    ui.painter().rect_stroke(
        thumb,
        16.0,
        egui::Stroke::new(1.0_f32, Color32::from_gray(74)),
        egui::StrokeKind::Inside,
    );
    for (index, option) in options.iter().enumerate() {
        ui.painter().text(
            option.center(),
            egui::Align2::CENTER_CENTER,
            labels[index],
            egui::FontId::proportional(13.0),
            Color32::from_gray(if usize::from(*second) == index {
                235
            } else {
                150
            }),
        );
    }
    SwitchResponse {
        changed: before != *second,
        options,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn both_switch_options_remain_clickable_during_animation() {
        let ctx = eframe::egui::Context::default();
        let mut second = false;
        let mut options = [eframe::egui::Rect::NOTHING; 2];
        for index in [1, 0, 1] {
            let _ = ctx.run(Default::default(), |ctx| {
                eframe::egui::CentralPanel::default().show(ctx, |ui| {
                    options = super::show(ui, "test", ["One", "Many"], &mut second, 180.0).options;
                });
            });
            let pos = options[index].center();
            for pressed in [true, false] {
                let _ = ctx.run(
                    eframe::egui::RawInput {
                        events: vec![
                            eframe::egui::Event::PointerMoved(pos),
                            eframe::egui::Event::PointerButton {
                                pos,
                                button: eframe::egui::PointerButton::Primary,
                                pressed,
                                modifiers: Default::default(),
                            },
                        ],
                        ..Default::default()
                    },
                    |ctx| {
                        eframe::egui::CentralPanel::default().show(ctx, |ui| {
                            super::show(ui, "test", ["One", "Many"], &mut second, 180.0);
                        });
                    },
                );
            }
            assert_eq!(second, index == 1);
        }
    }
}
