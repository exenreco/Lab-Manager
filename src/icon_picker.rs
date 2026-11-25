use crate::{device_icons, icon_assets, model::ServerIcon};
use eframe::egui::{self, Color32, Vec2};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

#[derive(Clone, Copy, Default, PartialEq)]
enum Tab {
    #[default]
    Icons,
    Import,
}
type FileResult = Arc<Mutex<mpsc::Receiver<Result<Option<PathBuf>, String>>>>;
#[derive(Clone, Default)]
struct Picker {
    open: bool,
    tab: Tab,
    query: String,
    error: String,
    imported: Vec<String>,
    file: Option<FileResult>,
    frost: crate::frosted_modal::FrostedModal,
    selected: bool,
}

/// Shared by the Add page, Add sidebar, and Device settings sidebar.
pub fn icon_picker(ui: &mut egui::Ui, icon: &mut ServerIcon, custom: &mut Option<String>) {
    picker_field(ui, icon, custom, false);
}

pub fn icon_picker_selected(ui: &mut egui::Ui, icon: &mut ServerIcon, custom: &mut Option<String>) {
    picker_field(ui, icon, custom, true);
}

fn picker_field(
    ui: &mut egui::Ui,
    icon: &mut ServerIcon,
    custom: &mut Option<String>,
    existing: bool,
) {
    let root = icon_assets::root(ui.ctx());
    let id = ui.make_persistent_id(("device_icon_picker", &root));
    let mut picker = ui
        .ctx()
        .data(|data| data.get_temp::<Picker>(id))
        .unwrap_or_default();
    ui.horizontal(|ui| {
        device_icons::thumbnail(ui, *icon, custom.as_deref(), 56.0);
        ui.vertical(|ui| {
            let chosen =
                existing || picker.selected || custom.is_some() || *icon != ServerIcon::Server;
            if ui
                .button(if chosen { "Change Icon" } else { "Choose icon" })
                .clicked()
            {
                picker.open = true;
                picker.error.clear();
                picker.query.clear();
                picker.imported = root.as_deref().map(icon_assets::list).unwrap_or_default();
                picker.frost.open(ui.ctx(), id.with("modal"));
            }
            ui.label(
                egui::RichText::new(
                    custom
                        .as_deref()
                        .map(icon_assets::label)
                        .unwrap_or(icon.label()),
                )
                .size(12.0)
                .color(Color32::from_gray(155)),
            );
        });
    });
    if picker.open {
        picker.show(ui.ctx(), id, root.as_deref(), icon, custom);
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, picker));
}

impl Picker {
    fn show(
        &mut self,
        ctx: &egui::Context,
        id: egui::Id,
        root: Option<&std::path::Path>,
        icon: &mut ServerIcon,
        custom: &mut Option<String>,
    ) {
        if let Some(receiver) = &self.file {
            let result = receiver
                .lock()
                .ok()
                .and_then(|receiver| receiver.try_recv().ok());
            if let Some(result) = result {
                self.file = None;
                match result {
                    Ok(Some(path)) => match root
                        .ok_or_else(|| "Sign in to import icons.".to_owned())
                        .and_then(|root| icon_assets::import(root, &path))
                    {
                        Ok(id) => {
                            *custom = Some(id);
                            self.imported = root.map(icon_assets::list).unwrap_or_default();
                            self.error.clear();
                        }
                        Err(error) => self.error = error,
                    },
                    Ok(None) => {}
                    Err(error) => self.error = error,
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
        let mut frost = std::mem::take(&mut self.frost);
        let response = frost.show_gallery(ctx, id.with("modal"), |ui| {
            let body = crate::frosted_modal::body_rect(ctx);
            ui.set_min_size(body.size());
            ui.set_max_size(body.size());
            ui.style_mut().interaction.selectable_labels = false;
            let gutter = (body.width() * 0.25)
                .clamp(210.0, 320.0)
                .min(body.width() * 0.48);
            let sidebar =
                egui::Rect::from_min_max(egui::pos2(body.right() - gutter, body.top()), body.max);
            let gallery =
                egui::Rect::from_min_max(body.min, egui::pos2(sidebar.left(), body.bottom()))
                    .shrink(24.0);
            ui.painter().rect_filled(
                sidebar,
                0.0,
                Color32::from_rgba_unmultiplied(20, 22, 22, 245),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(gallery)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
                |ui| {
                    ui.set_width(gallery.width());
                    ui.heading("Choose a device icon");
                    ui.label(
                        egui::RichText::new("Personalize this device")
                            .size(12.0)
                            .color(Color32::from_gray(165)),
                    );
                    ui.add_space(16.0);
                    ui.horizontal_wrapped(|ui| {
                        if tab_button(ui, "Icons", self.tab == Tab::Icons, false).clicked() {
                            self.tab = Tab::Icons;
                            self.query.clear();
                        }
                        if tab_button(ui, "Imported", self.tab == Tab::Import, true).clicked() {
                            self.tab = Tab::Import;
                            self.query.clear();
                        }
                    });
                    ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text(
                                eframe::egui::RichText::new(if self.tab == Tab::Icons {
                                    "Search system icons..."
                                } else {
                                    "Search imported icons..."
                                })
                                .color(eframe::egui::Color32::from_gray(100)),
                            )
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(8.0);
                    egui::ScrollArea::vertical()
                        .id_salt(id.with("icons_scroll"))
                        .auto_shrink([false, false])
                        .max_height(ui.available_height().max(0.0))
                        .show(ui, |ui| self.gallery(ui, icon, custom));
                },
            );
            let close_rect = egui::Rect::from_min_size(
                sidebar.right_top() + Vec2::new(-54.0, 10.0),
                Vec2::splat(44.0),
            );
            let close = ui
                .interact(close_rect, id.with("close"), egui::Sense::click())
                .on_hover_text("Close icon picker")
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            ui.painter().rect_filled(
                close_rect,
                8.0,
                if close.hovered() {
                    Color32::from_rgb(190, 42, 48)
                } else {
                    Color32::from_gray(27)
                },
            );
            ui.painter().rect_stroke(
                close_rect,
                8.0,
                egui::Stroke::new(1.0_f32, Color32::from_gray(100)),
                egui::StrokeKind::Inside,
            );
            crate::svg_icons::paint(
                ui.painter(),
                close_rect.shrink(9.0),
                "x",
                Some(Color32::from_gray(235)),
            );
            if close.clicked() {
                self.open = false;
            }
            let settings = egui::Rect::from_min_max(
                sidebar.min + Vec2::new(20.0, 60.0),
                sidebar.max - Vec2::splat(20.0),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(settings)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
                |ui| {
                    ui.set_width(settings.width());
                    egui::ScrollArea::vertical()
                        .id_salt(id.with("import_settings"))
                        .max_height(settings.height())
                        .show(ui, |ui| {
                            self.import_settings(ui, ctx, root, *icon, custom.as_deref());
                        });
                },
            );
        });
        if response.is_some_and(|response| response.close) {
            self.open = false;
        }
        if !self.open {
            frost.clear();
        }
        self.frost = frost;
    }
    fn gallery(&mut self, ui: &mut egui::Ui, icon: &mut ServerIcon, custom: &mut Option<String>) {
        let columns = ((ui.available_width() + 8.0) / 110.0)
            .floor()
            .clamp(1.0, 12.0) as usize;
        let query = self.query.to_lowercase();
        if self.tab == Tab::Icons {
            let icons: Vec<_> = ServerIcon::ALL
                .into_iter()
                .filter(|choice| choice.label().to_lowercase().contains(&query))
                .collect();
            for row in icons.chunks(columns) {
                ui.columns(columns, |uis| {
                    for (column, choice) in row.iter().enumerate() {
                        if icon_tile(
                            &mut uis[column],
                            choice.label(),
                            *choice,
                            None,
                            custom.is_none() && *icon == *choice,
                        ) {
                            *icon = *choice;
                            *custom = None;
                            self.selected = true;
                            self.open = false;
                        }
                    }
                });
                ui.add_space(8.0);
            }
            if icons.is_empty() {
                ui.label("No icons match your search.");
            }
        } else {
            let icons: Vec<_> = self
                .imported
                .iter()
                .filter(|id| icon_assets::label(id).to_lowercase().contains(&query))
                .cloned()
                .collect();
            for row in icons.chunks(columns) {
                ui.columns(columns, |uis| {
                    for (column, id) in row.iter().enumerate() {
                        if icon_tile(
                            &mut uis[column],
                            icon_assets::label(id),
                            ServerIcon::Server,
                            Some(id),
                            custom.as_ref() == Some(id),
                        ) {
                            *custom = Some(id.clone());
                            self.selected = true;
                            self.open = false;
                        }
                    }
                });
                ui.add_space(8.0);
            }
            if icons.is_empty() {
                ui.label(if self.imported.is_empty() {
                    "No imported icons yet. Add one using the import panel."
                } else {
                    "No imported icons match your search."
                });
            }
        }
    }
    fn import_settings(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        root: Option<&std::path::Path>,
        icon: ServerIcon,
        custom: Option<&str>,
    ) {
        ui.heading("Import icon");
        ui.label("Add images to your local icon library.");
        ui.add_space(14.0);
        device_icons::thumbnail(ui, icon, custom, 64.0);
        ui.label(
            egui::RichText::new(custom.map(icon_assets::label).unwrap_or(icon.label()))
                .size(12.0)
                .color(Color32::from_gray(180)),
        );
        ui.separator();
        if ui
            .add_enabled(
                root.is_some() && self.file.is_none(),
                egui::Button::new("Import icon..."),
            )
            .clicked()
        {
            self.tab = Tab::Import;
            self.query.clear();
            let (sender, receiver) = mpsc::channel();
            self.file = Some(Arc::new(Mutex::new(receiver)));
            let repaint = ctx.clone();
            std::thread::spawn(move || {
                let _ = sender.send(choose_file());
                repaint.request_repaint();
            });
        }
        if self.file.is_some() {
            ui.label("Choose an image in the file window...");
        }
        ui.add_space(8.0);
        ui.small("PNG with transparency");
        ui.small("File size: up to 5 MB");
        ui.small("Dimensions: up to 4096 × 4096");
        ui.separator();
        ui.label(format!("{} imported icons", self.imported.len()));
        if !self.error.is_empty() {
            ui.colored_label(Color32::from_rgb(240, 145, 145), &self.error);
        }
    }
}

fn icon_tile(
    ui: &mut egui::Ui,
    label: &str,
    system: ServerIcon,
    custom: Option<&str>,
    selected: bool,
) -> bool {
    ui.push_id((label, custom), |ui| {
        let tile = ui.scope_builder(egui::UiBuilder::new().sense(egui::Sense::click()), |ui| {
            egui::Frame::new()
                .fill(if selected {
                    Color32::from_white_alpha(16)
                } else {
                    Color32::TRANSPARENT
                })
                .corner_radius(10.0)
                .stroke(if selected {
                    egui::Stroke::new(1.0_f32, Color32::from_rgb(64, 184, 119))
                } else {
                    egui::Stroke::NONE
                })
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        device_icons::thumbnail(ui, system, custom, 52.0);
                        ui.add_sized(
                            [ui.available_width(), 28.0],
                            egui::Label::new(egui::RichText::new(label).size(11.0)).wrap(),
                        );
                    });
                });
        });
        #[cfg(test)]
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::new(("icon_tile", label)), tile.response.rect)
        });
        tile.response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(label)
            .clicked()
    })
    .inner
}

fn tab_button(ui: &mut egui::Ui, label: &str, selected: bool, import: bool) -> egui::Response {
    let response = ui.selectable_label(selected, format!("     {label}"));
    let c = response.rect.left_center() + Vec2::new(12.0, 0.0);
    crate::svg_icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(c, Vec2::splat(16.0)),
        if import { "upload" } else { "layout-grid" },
        Some(Color32::from_gray(190)),
    );
    response
}

fn choose_file() -> Result<Option<PathBuf>, String> {
    crate::file_dialog::choose_image("Import device icon", false)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn render(
        ctx: &egui::Context,
        picker: &mut Picker,
        icon: &mut ServerIcon,
        custom: &mut Option<String>,
        events: Vec<egui::Event>,
    ) {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(900.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label("Form behind the modal");
                });
                if picker.open {
                    picker.show(ctx, egui::Id::new("picker_test"), None, icon, custom);
                }
            },
        );
    }
    #[test]
    fn selecting_system_icon_replaces_custom_and_closes_modal() {
        let ctx = egui::Context::default();
        let mut picker = Picker {
            open: true,
            ..Default::default()
        };
        let mut icon = ServerIcon::Server;
        let mut custom = Some("previous.png".into());
        render(&ctx, &mut picker, &mut icon, &mut custom, vec![]);
        render(&ctx, &mut picker, &mut icon, &mut custom, vec![]);
        let pos = ctx
            .data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new(("icon_tile", "Cloud")))
                    .unwrap()
            })
            .center();
        render(
            &ctx,
            &mut picker,
            &mut icon,
            &mut custom,
            vec![egui::Event::PointerMoved(pos)],
        );
        for pressed in [true, false] {
            render(
                &ctx,
                &mut picker,
                &mut icon,
                &mut custom,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                }],
            );
        }
        assert_eq!(icon, ServerIcon::Cloud);
        assert!(custom.is_none());
        assert!(!picker.open);
    }
}
