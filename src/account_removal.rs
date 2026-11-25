use crate::{design, profile_ui};
use eframe::egui::{self, Color32};
use lab_manager::{frosted_modal::FrostedModal, profile::Profile};

#[derive(Default)]
pub struct AccountRemoval {
    pub open: bool,
    pub error: String,
    frost: FrostedModal,
}
impl AccountRemoval {
    pub fn open(&mut self, ctx: &egui::Context) {
        self.open = true;
        self.error.clear();
        self.frost.open(ctx, egui::Id::new("remove_account_modal"));
    }
    pub fn show(&mut self, ctx: &egui::Context, username: &str, profile: &Profile) -> bool {
        if !self.open {
            return false;
        }
        let mut confirm = false;
        let mut frost = std::mem::take(&mut self.frost);
        let response=frost.show(ctx,egui::Id::new("remove_account_modal"),|ui| {
            ui.set_width((ctx.screen_rect().width()-100.0).clamp(260.0,460.0));
            ui.horizontal(|ui| {
                ui.heading("Remove account?");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                    if design::window_control(ui,2,false).on_hover_text("Cancel account removal").clicked() {self.open=false;}
                });
            });
            ui.horizontal(|ui| {profile_ui::portrait(ui,profile,48.0);ui.strong(username);});
            ui.add_space(10.0);
            ui.label("This removes your local sign-in and profile. Your saved devices and imported icons will be archived on this computer, and won't be assigned to a new account.");
            ui.label("Other accounts and their inventories stay available.");
            if !self.error.is_empty() {ui.colored_label(Color32::from_rgb(240,145,145),&self.error);}
            ui.add_space(16.0);
            ui.horizontal_wrapped(|ui| {
                confirm=profile_ui::remove_account_button(ui).clicked();
                if ui.button("Cancel").clicked() {self.open=false;}
            });
        });
        if response.is_some_and(|response| response.close) {
            self.open = false;
        }
        if !self.open {
            frost.clear();
        }
        self.frost = frost;
        confirm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escape_cancels_account_removal_without_confirming() {
        let ctx = egui::Context::default();
        let mut removal = AccountRemoval {
            open: true,
            ..Default::default()
        };
        for escape in [false, false, true] {
            let events = if escape {
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                }]
            } else {
                vec![]
            };
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    assert!(!removal.show(ctx, "test-user", &Profile::default()));
                },
            );
        }
        assert!(!removal.open);
    }
}
