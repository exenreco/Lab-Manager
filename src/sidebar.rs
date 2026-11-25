use crate::design;
use eframe::egui;

#[derive(Default)]
pub struct RightSidebar {
    pub open: bool,
    pub content_id: String,
}

impl RightSidebar {
    pub fn show<R>(
        &mut self,
        ctx: &egui::Context,
        title: &str,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<R> {
        if !self.open {
            return None;
        }
        let available = ctx.available_rect().width();
        let terminal = title == "Terminal";
        let max_width = if terminal {
            1000.0
        } else if matches!(
            title,
            "Device settings" | "Monitoring settings" | "Add Device" | "Scan network"
        ) {
            440.0
        } else {
            360.0
        };
        let width = (available * if terminal { 0.62 } else { 0.48 })
            .clamp(280.0, max_width)
            .min((available - 260.0).max(180.0));
        let panel = egui::SidePanel::right(if terminal {
            "terminal_sidebar"
        } else {
            "details_sidebar"
        });
        let panel = if terminal {
            panel
                .default_width(width)
                .width_range(280.0..=(available - 220.0).max(280.0))
                .resizable(true)
        } else {
            panel.exact_width(width).resizable(false)
        };
        panel
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(design::BLACK).inner_margin(20.0))
            .show(ctx, |ui| {
                let bounds = ui
                    .max_rect()
                    .expand(20.0)
                    .intersect(lab_manager::frosted_modal::body_rect(ctx));
                ctx.data_mut(|data| {
                    data.insert_temp(egui::Id::new("right_sidebar_bounds"), bounds)
                });
                ui.horizontal(|ui| {
                    ui.heading(title);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if design::window_control(ui, 2, false)
                            .on_hover_text("Close sidebar")
                            .clicked()
                        {
                            self.open = false;
                        }
                    });
                });
                ui.add_space(12.0);
                if terminal {
                    content(ui)
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt(("details_scroll", &self.content_id))
                        .show(ui, content)
                        .inner
                }
            })
            .inner
            .into()
    }
}
