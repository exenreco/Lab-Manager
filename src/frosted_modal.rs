use eframe::egui::{self, Color32, Vec2};
use std::time::{Duration, Instant};

pub const TITLE_BAR_HEIGHT: f32 = 44.0;

#[derive(Clone, Default)]
pub struct FrostedModal {
    waiting: Option<Instant>,
    backdrop: Option<egui::TextureHandle>,
    pending: Option<std::sync::Arc<std::sync::Mutex<Option<egui::TextureHandle>>>>,
}
pub struct ModalResponse<R> {
    pub inner: R,
    pub close: bool,
}

pub fn body_rect(ctx: &egui::Context) -> egui::Rect {
    let mut rect = ctx.screen_rect();
    rect.min.y = (rect.top() + TITLE_BAR_HEIGHT).min(rect.bottom());
    rect
}

impl FrostedModal {
    pub fn open(&mut self, ctx: &egui::Context, id: egui::Id) {
        self.waiting = Some(Instant::now());
        self.backdrop = None;
        self.pending = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(id)));
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn show<R>(
        &mut self,
        ctx: &egui::Context,
        id: egui::Id,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<ModalResponse<R>> {
        self.show_with_frame(ctx, id, true, None, content)
    }
    pub fn show_gallery<R>(
        &mut self,
        ctx: &egui::Context,
        id: egui::Id,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<ModalResponse<R>> {
        self.show_with_frame(ctx, id, false, None, content)
    }
    pub fn show_gallery_in<R>(
        &mut self,
        ctx: &egui::Context,
        id: egui::Id,
        bounds: egui::Rect,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<ModalResponse<R>> {
        self.show_with_frame(ctx, id, false, Some(bounds), content)
    }
    fn show_with_frame<R>(
        &mut self,
        ctx: &egui::Context,
        id: egui::Id,
        framed: bool,
        scope: Option<egui::Rect>,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<ModalResponse<R>> {
        if let Some(pending) = &self.pending {
            let texture = pending.lock().ok().and_then(|mut result| result.take());
            if let Some(texture) = texture {
                self.backdrop = Some(texture);
                self.pending = None;
            }
        }
        if self.waiting.is_some() {
            let screenshot = ctx.input(|input| {
                input.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot {
                        user_data, image, ..
                    } = event
                        && user_data
                            .data
                            .as_ref()
                            .and_then(|data| data.downcast_ref::<egui::Id>())
                            == Some(&id)
                    {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(image) = screenshot {
                if scope.is_some() {
                    let result = std::sync::Arc::new(std::sync::Mutex::new(None));
                    self.pending = Some(result.clone());
                    let ctx = ctx.clone();
                    std::thread::spawn(move || {
                        let texture = blurred_backdrop(&ctx, &image);
                        if let Ok(mut result) = result.lock() {
                            *result = Some(texture);
                        }
                        ctx.request_repaint();
                    });
                } else {
                    self.backdrop = Some(blurred_backdrop(ctx, &image));
                }
                self.waiting = None;
            } else if self
                .waiting
                .is_some_and(|at| at.elapsed() >= Duration::from_millis(350))
            {
                self.waiting = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(16));
                scope?;
            }
        }
        let body = scope
            .unwrap_or_else(|| body_rect(ctx))
            .intersect(body_rect(ctx));
        // A body-only input shield leaves native window controls and dragging usable.
        let shield = egui::Area::new(id.with("backdrop"))
            .order(egui::Order::Foreground)
            .fixed_pos(body.min)
            .movable(false)
            .constrain(false)
            .show(ctx, |ui| {
                ui.set_clip_rect(body);
                let (_, response) =
                    ui.allocate_exact_size(body.size(), egui::Sense::click_and_drag());
                if let Some(backdrop) = &self.backdrop {
                    ui.painter().image(
                        backdrop.id(),
                        ctx.screen_rect(),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                ui.painter()
                    .rect_filled(body, 0.0, Color32::from_black_alpha(90));
                response
            });
        let frame = if framed {
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(28, 32, 32, 225))
                .corner_radius(20.0)
                .stroke(egui::Stroke::new(1.0_f32, Color32::from_white_alpha(65)))
                .inner_margin(22.0)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 12],
                    blur: 32,
                    spread: 0,
                    color: Color32::from_black_alpha(100),
                })
        } else {
            egui::Frame::new()
        };
        let panel_area = egui::Area::new(id.with("panel"))
            .order(egui::Order::Foreground)
            .constrain_to(body);
        let panel_area = if framed {
            panel_area.anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        } else {
            panel_area.fixed_pos(body.min).movable(false)
        };
        let panel = panel_area.show(ctx, |ui| {
            ui.set_clip_rect(body);
            frame
                .show(ui, |ui| {
                    ui.style_mut().interaction.selectable_labels = false;
                    content(ui)
                })
                .inner
        });
        // Resizing or interacting with the shield can raise it above sibling areas.
        // Keep modal content above its backdrop on every frame.
        ctx.move_to_top(panel.response.layer_id);
        let escape = ctx.memory(|memory| !memory.any_popup_open())
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        Some(ModalResponse {
            inner: panel.inner,
            close: shield.inner.clicked() || escape,
        })
    }
}

pub fn blurred_backdrop(ctx: &egui::Context, screenshot: &egui::ColorImage) -> egui::TextureHandle {
    let bytes = screenshot
        .pixels
        .iter()
        .flat_map(|pixel| pixel.to_array())
        .collect();
    let rgba =
        image::RgbaImage::from_raw(screenshot.width() as u32, screenshot.height() as u32, bytes)
            .expect("screenshot dimensions match its pixels");
    let blurred = image::DynamicImage::ImageRgba8(rgba)
        .resize(640, 640, image::imageops::FilterType::Triangle)
        .blur(5.0)
        .to_rgba8();
    ctx.load_texture(
        "frosted_modal_backdrop",
        egui::ColorImage::from_rgba_unmultiplied(
            [blurred.width() as usize, blurred.height() as usize],
            blurred.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scoped_gallery_covers_only_the_right_sidebar_and_leaves_title_bar_free() {
        let ctx = egui::Context::default();
        let mut modal = FrostedModal::default();
        let id = egui::Id::new("sidebar_gallery");
        let bounds = egui::Rect::from_min_max(egui::pos2(700.0, 44.0), egui::pos2(1100.0, 760.0));
        for frame in 0..2 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1100.0, 760.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    if frame == 0 {
                        modal.open(ctx, id);
                    }
                    assert!(
                        modal
                            .show_gallery_in(ctx, id, bounds, |ui| {
                                ui.set_min_size(bounds.size());
                                ui.heading("Choose a device");
                            })
                            .is_some()
                    );
                },
            );
        }
        assert_eq!(
            ctx.memory(|memory| memory.area_rect(id.with("backdrop"))),
            Some(bounds)
        );
        assert_eq!(
            ctx.memory(|memory| memory.area_rect(id.with("panel"))),
            Some(bounds)
        );
        assert_ne!(
            ctx.layer_id_at(egui::pos2(300.0, 300.0)),
            Some(egui::LayerId::new(
                egui::Order::Foreground,
                id.with("backdrop")
            ))
        );
        assert_ne!(
            ctx.layer_id_at(egui::pos2(900.0, 20.0)),
            Some(egui::LayerId::new(
                egui::Order::Foreground,
                id.with("panel")
            ))
        );
    }
    #[test]
    fn gallery_stays_above_backdrop_after_wide_resize() {
        let ctx = egui::Context::default();
        let mut modal = FrostedModal::default();
        let id = egui::Id::new("wide_gallery");
        for width in [900.0, 1749.0, 2560.0, 3440.0] {
            ctx.move_to_top(egui::LayerId::new(
                egui::Order::Foreground,
                id.with("backdrop"),
            ));
            for _ in 0..3 {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(width, 1200.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        modal.show_gallery(ctx, id, |ui| {
                            ui.set_width(width);
                            ui.set_min_height(600.0);
                            ui.label("Visible gallery");
                        });
                    },
                );
            }
            assert_eq!(
                ctx.layer_id_at(egui::pos2(width / 2.0, 600.0)),
                Some(egui::LayerId::new(
                    egui::Order::Foreground,
                    id.with("panel")
                ))
            );
        }
    }
    #[test]
    fn modal_shield_leaves_title_bar_clickable() {
        let ctx = egui::Context::default();
        let mut modal = FrostedModal::default();
        let mut clicked = false;
        let mut run = |events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(800.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::TopBottomPanel::top("title")
                        .exact_height(TITLE_BAR_HEIGHT)
                        .show(ctx, |ui| {
                            clicked = ui.button("Title action").clicked();
                        });
                    modal.show(ctx, egui::Id::new("test_modal"), |ui| {
                        ui.set_width(300.0);
                        ui.label("Modal body");
                    });
                },
            );
            clicked
        };
        run(vec![]);
        run(vec![]);
        let pos = egui::pos2(35.0, 15.0);
        run(vec![egui::Event::PointerMoved(pos)]);
        run(vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }]);
        assert!(run(vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default()
        }]));
        assert_eq!(body_rect(&ctx).top(), TITLE_BAR_HEIGHT);
    }
}
