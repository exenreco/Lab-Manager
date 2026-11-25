use eframe::egui::{self, Color32};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct Glass {
    texture: Option<egui::TextureHandle>,
    pending: Option<Arc<Mutex<Option<egui::TextureHandle>>>>,
}

impl Glass {
    pub fn capture(&mut self, ctx: &egui::Context, id: egui::Id) {
        self.texture = None;
        self.pending = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(id)));
        ctx.request_repaint();
    }
    pub fn ready(&mut self, ctx: &egui::Context, id: egui::Id) -> bool {
        if let Some(pending) = &self.pending {
            let texture = pending.lock().ok().and_then(|mut result| result.take());
            if let Some(texture) = texture {
                self.texture = Some(texture);
                self.pending = None;
            }
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|e| {
                if let egui::Event::Screenshot {
                    user_data, image, ..
                } = e
                    && user_data
                        .data
                        .as_ref()
                        .and_then(|v| v.downcast_ref::<egui::Id>())
                        == Some(&id)
                {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = image {
            let result = Arc::new(Mutex::new(None));
            self.pending = Some(result.clone());
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let texture = lab_manager::frosted_modal::blurred_backdrop(&ctx, &image);
                if let Ok(mut result) = result.lock() {
                    *result = Some(texture);
                }
                ctx.request_repaint();
            });
        }
        // Show immediately; the blurred backdrop arrives asynchronously.
        true
    }
    pub fn show<R>(
        &self,
        ui: &mut egui::Ui,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        let background = ui.painter().add(egui::Shape::Noop);
        let response = frame().show(ui, content);
        let rect = response.response.rect;
        let mut shapes = Vec::new();
        if let Some(texture) = &self.texture {
            let screen = ui.ctx().screen_rect();
            let uv = egui::Rect::from_min_max(
                egui::pos2(
                    (rect.left() - screen.left()) / screen.width(),
                    (rect.top() - screen.top()) / screen.height(),
                ),
                egui::pos2(
                    (rect.right() - screen.left()) / screen.width(),
                    (rect.bottom() - screen.top()) / screen.height(),
                ),
            );
            shapes.push(egui::Shape::Rect(
                egui::epaint::RectShape::filled(rect, 12.0, Color32::WHITE)
                    .with_texture(texture.id(), uv),
            ));
        }
        shapes.push(egui::Shape::rect_filled(
            rect,
            12.0,
            if self.texture.is_some() {
                Color32::from_rgba_unmultiplied(18, 22, 22, 205)
            } else {
                Color32::from_rgb(23, 27, 27)
            },
        ));
        ui.painter().set(background, egui::Shape::Vec(shapes));
        response
    }
}

pub fn frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::TRANSPARENT)
        .corner_radius(12.0)
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(45)))
        .inner_margin(14.0)
}
