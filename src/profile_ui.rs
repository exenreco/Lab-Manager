use crate::design;
use eframe::egui::{self, Color32, Vec2};
use lab_manager::profile::Profile;
use std::{
    hash::{Hash, Hasher},
    sync::mpsc,
    time::Duration,
};

#[derive(Default)]
pub struct PhotoPicker {
    pending: Option<mpsc::Receiver<Result<Option<std::path::PathBuf>, String>>>,
    error: String,
}
impl PhotoPicker {
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn show(&mut self, ui: &mut egui::Ui, profile: &mut Profile) {
        if let Some(receiver) = &self.pending {
            if let Ok(result) = receiver.try_recv() {
                self.pending = None;
                match result {
                    Ok(Some(path)) => match Profile::from_photo(&path) {
                        Ok(photo) => {
                            *profile = photo;
                            self.error.clear();
                        }
                        Err(error) => self.error = error,
                    },
                    Ok(None) => {}
                    Err(error) => self.error = error,
                }
            } else {
                ui.ctx().request_repaint_after(Duration::from_millis(100));
            }
        }
        ui.strong("Profile picture");
        ui.small("Choose an avatar or add your own photo. Avatar 1 is the default.");
        ui.horizontal_wrapped(|ui| {
            for avatar in 0..5 {
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(56.0, 66.0), egui::Sense::click());
                #[cfg(test)]
                ui.ctx().data_mut(|data| {
                    data.insert_temp(egui::Id::new(("avatar_option", avatar)), rect)
                });
                let circle = egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, rect.top() + 23.0),
                    Vec2::splat(42.0),
                );
                paint_avatar(ui, circle, avatar);
                if profile.photo.is_none() && profile.avatar == avatar {
                    ui.painter().circle_stroke(
                        circle.center(),
                        23.0,
                        egui::Stroke::new(2.0_f32, Color32::from_rgb(64, 184, 119)),
                    );
                }
                let label = format!("Avatar {}", avatar + 1);
                ui.painter().text(
                    egui::pos2(rect.center().x, rect.bottom() - 9.0),
                    egui::Align2::CENTER_CENTER,
                    &label,
                    egui::FontId::proportional(10.0),
                    Color32::from_gray(180),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
                });
                if response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    profile.avatar = avatar;
                    profile.photo = None;
                }
            }
        });
        ui.horizontal(|ui| {
            if profile.photo.is_some() {
                portrait(ui, profile, 48.0);
            }
            if ui
                .add_enabled(
                    self.pending.is_none(),
                    egui::Button::new("Add your photo..."),
                )
                .clicked()
            {
                let (sender, receiver) = mpsc::channel();
                self.pending = Some(receiver);
                let ctx = ui.ctx().clone();
                std::thread::spawn(move || {
                    let _ = sender.send(lab_manager::file_dialog::choose_image(
                        "Choose profile photo",
                        true,
                    ));
                    ctx.request_repaint();
                });
            }
            if profile.photo.is_some() && ui.small_button("Use avatar instead").clicked() {
                profile.photo = None;
            }
        });
        ui.small("PNG or JPEG · up to 5 MB · cropped to fit the round picture");
        if !self.error.is_empty() {
            ui.colored_label(Color32::from_rgb(240, 145, 145), &self.error);
        }
    }
}

pub fn portrait(ui: &mut egui::Ui, profile: &Profile, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    paint_portrait(ui, rect, profile);
}

fn paint_portrait(ui: &egui::Ui, rect: egui::Rect, profile: &Profile) {
    if let Some(bytes) = &profile.photo {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hash);
        let id = egui::Id::new(("profile_photo", hash.finish()));
        let texture = ui
            .ctx()
            .data(|data| data.get_temp::<egui::TextureHandle>(id))
            .or_else(|| {
                let image = image::load_from_memory(bytes).ok()?.to_rgba8();
                let texture = ui.ctx().load_texture(
                    format!("profile-{}", hash.finish()),
                    egui::ColorImage::from_rgba_unmultiplied(
                        [image.width() as usize, image.height() as usize],
                        image.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                );
                ui.ctx()
                    .data_mut(|data| data.insert_temp(id, texture.clone()));
                Some(texture)
            });
        if let Some(texture) = texture {
            egui::Image::from_texture(&texture)
                .corner_radius(rect.height() / 2.0)
                .paint_at(ui, rect);
            return;
        }
    }
    paint_avatar(ui, rect, profile.avatar.min(4));
}

fn paint_avatar(ui: &egui::Ui, rect: egui::Rect, avatar: u8) {
    let p = ui.painter();
    let c = rect.center();
    let r = rect.width() / 2.0;
    let backgrounds = [
        Color32::from_rgb(53, 78, 86),
        Color32::from_rgb(83, 66, 89),
        Color32::from_rgb(73, 89, 66),
        Color32::from_rgb(106, 78, 56),
        Color32::from_rgb(62, 70, 103),
    ];
    let skins = [
        Color32::from_rgb(218, 172, 134),
        Color32::from_rgb(170, 115, 82),
        Color32::from_rgb(238, 195, 158),
        Color32::from_rgb(129, 85, 65),
        Color32::from_rgb(205, 154, 118),
    ];
    let index = avatar as usize;
    p.circle_filled(c, r, backgrounds[index]);
    let mut body = vec![
        c + Vec2::new(-0.3 * r, 0.28 * r),
        c + Vec2::new(0.3 * r, 0.28 * r),
        c + Vec2::new(0.8 * r, 0.6 * r),
    ];
    for step in 0..=12 {
        let angle = (36.87 + step as f32 * 106.26 / 12.0).to_radians();
        body.push(c + Vec2::angled(angle) * r);
    }
    p.add(egui::Shape::convex_polygon(
        body,
        Color32::from_gray(185 + avatar * 10),
        egui::Stroke::NONE,
    ));
    p.rect_filled(
        egui::Rect::from_center_size(c + Vec2::new(0.0, 0.18 * r), Vec2::new(0.23 * r, 0.35 * r)),
        2.0,
        skins[index],
    );
    let face = c + Vec2::new(0.0, -0.2 * r);
    let hair = if avatar == 2 {
        Color32::from_rgb(152, 91, 51)
    } else {
        Color32::from_gray(25 + avatar * 8)
    };
    p.circle_filled(face + Vec2::new(0.0, -0.08 * r), 0.43 * r, hair);
    p.circle_filled(face, 0.34 * r, skins[index]);
    p.add(egui::Shape::convex_polygon(
        vec![
            face + Vec2::new(-0.4 * r, -0.1 * r),
            face + Vec2::new(-0.2 * r, -0.38 * r),
            face + Vec2::new(0.24 * r, -0.38 * r),
            face + Vec2::new(0.4 * r, -0.02 * r),
            face + Vec2::new(0.08 * r, -0.18 * r),
        ],
        hair,
        egui::Stroke::NONE,
    ));
    for x in [-0.12, 0.12] {
        p.circle_filled(
            face + Vec2::new(x * r, 0.01 * r),
            0.025 * r,
            Color32::from_gray(30),
        );
    }
    if avatar == 1 || avatar == 4 {
        for x in [-0.14, 0.14] {
            p.circle_stroke(
                face + Vec2::new(x * r, 0.01 * r),
                0.105 * r,
                egui::Stroke::new(0.8_f32, Color32::from_gray(35)),
            );
        }
        p.line_segment(
            [
                face + Vec2::new(-0.04 * r, 0.0),
                face + Vec2::new(0.04 * r, 0.0),
            ],
            egui::Stroke::new(0.8_f32, Color32::from_gray(35)),
        );
    }
    p.line_segment(
        [
            face + Vec2::new(-0.08 * r, 0.18 * r),
            face + Vec2::new(0.08 * r, 0.18 * r),
        ],
        egui::Stroke::new(0.8_f32, Color32::from_rgb(125, 75, 65)),
    );
}

#[derive(Clone, Default)]
struct MenuState {
    glass: crate::frosted_popup::Glass,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ProfileAction {
    #[default]
    None,
    Logout,
    RemoveAccount,
}

pub fn menu(ui: &mut egui::Ui, username: &str, profile: &Profile, width: f32) -> ProfileAction {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            format!("Profile menu: {username}"),
        )
    });
    let picture = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 16.0, rect.center().y),
        Vec2::splat(28.0),
    );
    paint_portrait(ui, picture, profile);
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    #[cfg(test)]
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new("profile_trigger"), rect));
    let id = ui.make_persistent_id(("profile_menu", username));
    let state_id = id.with("state");
    let mut state = ui
        .ctx()
        .data(|data| data.get_temp::<MenuState>(state_id))
        .unwrap_or_default();
    let was_open = ui.memory(|m| m.is_popup_open(id));
    if response.clicked() {
        ui.memory_mut(|memory| memory.toggle_popup(id));
    }
    let mut action = ProfileAction::None;
    let is_open = ui.memory(|m| m.is_popup_open(id));
    if is_open && !was_open {
        state.glass.capture(ui.ctx(), id);
    }
    if is_open && state.glass.ready(ui.ctx(), id) {
        let screen = ui.ctx().screen_rect();
        let x = (response.rect.right() - 210.0).clamp(screen.left() + 12.0, screen.right() - 222.0);
        let panel = egui::Area::new(id.with("glass_panel"))
            .order(egui::Order::Foreground)
            .movable(false)
            .default_width(210.0)
            .constrain_to(lab_manager::frosted_modal::body_rect(ui.ctx()))
            .fixed_pos(egui::pos2(x, response.rect.bottom() + 12.0))
            .show(ui.ctx(), |ui| {
                state.glass.show(ui, |ui| {
                    ui.set_min_width(180.0);
                    ui.label(
                        egui::RichText::new(format!("Signed in as {username}"))
                            .size(12.0)
                            .color(Color32::from_gray(160)),
                    );
                    ui.separator();
                    let logout_response = design::logout_menu_item(ui);
                    #[cfg(test)]
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(egui::Id::new("profile_logout"), logout_response.rect)
                    });
                    if logout_response.clicked() {
                        action = ProfileAction::Logout;
                        ui.memory_mut(|memory| memory.close_popup());
                    }
                    let remove_response = remove_account_button(ui);
                    #[cfg(test)]
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(
                            egui::Id::new("profile_remove_account"),
                            remove_response.rect,
                        )
                    });
                    if remove_response.clicked() {
                        action = ProfileAction::RemoveAccount;
                        ui.memory_mut(|memory| memory.close_popup());
                    }
                });
            });
        if ui.ctx().input(|i| {
            i.key_pressed(egui::Key::Escape)
                || (i.pointer.any_pressed()
                    && i.pointer.interact_pos().is_some_and(|p| {
                        !panel.response.rect.contains(p) && !response.rect.contains(p)
                    }))
        }) {
            ui.memory_mut(|m| m.close_popup());
        }
    }
    ui.ctx().data_mut(|data| data.insert_temp(state_id, state));
    action
}

pub fn remove_account_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(180.0, 36.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Remove account")
    });
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, 5.0, Color32::from_gray(45));
    }
    let icon =
        egui::Rect::from_center_size(rect.left_center() + Vec2::new(18.0, 0.0), Vec2::splat(26.0));
    ui.painter().rect_stroke(
        icon,
        7.0,
        egui::Stroke::new(1.0_f32, Color32::from_gray(75)),
        egui::StrokeKind::Inside,
    );
    crate::server_table::paint_trash(ui, icon);
    ui.painter().text(
        rect.left_center() + Vec2::new(40.0, 0.0),
        egui::Align2::LEFT_CENTER,
        "Remove account",
        egui::FontId::proportional(13.0),
        Color32::from_gray(220),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        }
    }
    #[test]
    fn profile_dropdown_ignores_hover_opens_on_click_and_dispatches_actions() {
        let ctx = egui::Context::default();
        let profile = Profile::default();
        let mut logout = ProfileAction::None;
        let mut run = |events: Vec<egui::Event>| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(600.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        logout = menu(ui, "test-user", &profile, 180.0);
                    });
                },
            );
            logout
        };
        run(vec![]);
        let trigger = ctx
            .data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new("profile_trigger"))
                    .unwrap()
            })
            .center();
        run(vec![egui::Event::PointerMoved(trigger)]);
        run(vec![]);
        assert!(!ctx.memory(|memory| memory.any_popup_open()));
        for pressed in [true, false] {
            run(vec![button(trigger, pressed)]);
        }
        run(vec![egui::Event::PointerMoved(egui::pos2(500.0, 500.0))]);
        assert!(ctx.memory(|memory| memory.any_popup_open()));
        let item = ctx
            .data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new("profile_logout"))
                    .unwrap()
            })
            .center();
        run(vec![egui::Event::PointerMoved(item)]);
        run(vec![button(item, true)]);
        assert_eq!(run(vec![button(item, false)]), ProfileAction::Logout);
        assert!(!ctx.memory(|memory| memory.any_popup_open()));
        run(vec![egui::Event::PointerMoved(trigger)]);
        run(vec![]);
        assert!(!ctx.memory(|memory| memory.any_popup_open()));
        for pressed in [true, false] {
            run(vec![button(trigger, pressed)]);
        }
        run(vec![]);
        let remove = ctx
            .data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new("profile_remove_account"))
                    .unwrap()
            })
            .center();
        run(vec![egui::Event::PointerMoved(remove)]);
        run(vec![button(remove, true)]);
        assert_eq!(
            run(vec![button(remove, false)]),
            ProfileAction::RemoveAccount
        );
    }
    #[test]
    fn registration_offers_five_avatars_and_selects_the_fifth() {
        let ctx = egui::Context::default();
        let mut picker = PhotoPicker::default();
        let mut profile = Profile::default();
        let mut run = |events: Vec<egui::Event>, profile: &mut Profile| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(600.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| picker.show(ui, profile));
                },
            );
        };
        run(vec![], &mut profile);
        for avatar in 0u8..5 {
            assert!(
                ctx.data(
                    |data| data.get_temp::<egui::Rect>(egui::Id::new(("avatar_option", avatar)))
                )
                .is_some()
            );
        }
        let pos = ctx
            .data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new(("avatar_option", 4u8)))
                    .unwrap()
            })
            .center();
        run(vec![egui::Event::PointerMoved(pos)], &mut profile);
        for pressed in [true, false] {
            run(vec![button(pos, pressed)], &mut profile);
        }
        assert_eq!(profile.avatar, 4);
        assert!(profile.photo.is_none());
    }
}
