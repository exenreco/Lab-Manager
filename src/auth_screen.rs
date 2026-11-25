use crate::design;
use eframe::egui::{self, Color32, RichText};
use lab_manager::auth::{AccountStore, QUESTIONS};
use zeroize::Zeroize;

#[derive(PartialEq)]
enum Mode {
    SignIn,
    Register,
    Recover,
    Blocked,
}

pub struct AuthScreen {
    store: AccountStore,
    mode: Mode,
    pub signed_in: Option<String>,
    pub profile: lab_manager::profile::Profile,
    registration_profile: lab_manager::profile::Profile,
    profile_picker: crate::profile_ui::PhotoPicker,
    username: String,
    password: String,
    confirmation: String,
    answers: [String; 2],
    question_ids: [usize; 2],
    recovery_questions: [String; 2],
    message: String,
    automatic_sign_in: bool,
}

impl AuthScreen {
    pub fn new(path: std::path::PathBuf) -> Self {
        let store = AccountStore { path };
        let (mode, username, mut message) = match store.username() {
            Ok(Some(username)) => (Mode::SignIn, username, String::new()),
            Ok(None) => (Mode::Register, String::new(), String::new()),
            Err(error) => (Mode::Blocked, String::new(), error),
        };
        let signed_in = if mode == Mode::SignIn {
            match store.automatic_sign_in() {
                Ok(user) => user,
                Err(error) => {
                    message = format!("Automatic sign-in unavailable: {error}");
                    None
                }
            }
        } else {
            None
        };
        let profile = signed_in
            .as_deref()
            .and_then(|username| store.profile(username).ok())
            .unwrap_or_default();
        Self {
            profile,
            registration_profile: Default::default(),
            profile_picker: Default::default(),
            store,
            mode,
            signed_in,
            username,
            password: String::new(),
            confirmation: String::new(),
            answers: [String::new(), String::new()],
            question_ids: [0, 1],
            recovery_questions: [String::new(), String::new()],
            message,
            automatic_sign_in: false,
        }
    }

    fn clear_secrets(&mut self) {
        self.password.zeroize();
        self.confirmation.zeroize();
        for answer in &mut self.answers {
            answer.zeroize();
        }
    }
    pub fn owns_legacy_inventory(&self, username: &str) -> Result<bool, String> {
        self.store.owns_legacy_inventory(username)
    }

    pub fn sign_out(&mut self) -> Result<(), String> {
        self.store.sign_out()?;
        self.automatic_sign_in = false;
        self.clear_secrets();
        self.signed_in = None;
        self.profile = Default::default();
        self.registration_profile = Default::default();
        self.profile_picker = Default::default();
        self.mode = Mode::SignIn;
        self.message = "Signed out.".into();
        Ok(())
    }

    pub fn remove_account(&mut self) -> Result<(), String> {
        let username = self
            .signed_in
            .as_deref()
            .ok_or("Sign in before removing an account.")?;
        self.store.remove_account(username)?;
        self.automatic_sign_in = false;
        self.clear_secrets();
        self.signed_in = None;
        self.profile = Default::default();
        self.registration_profile = Default::default();
        self.profile_picker = Default::default();
        self.username.clear();
        self.mode = Mode::SignIn;
        self.message = "Account removed.".into();
        Ok(())
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width().min(430.0);
        ui.add_space((ui.available_height() * 0.07).min(45.0));
        let rect = egui::Rect::from_min_size(
            ui.next_widget_position()
                + egui::vec2(((ui.available_width() - width) / 2.0).max(0.0), 0.0),
            egui::vec2(width, ui.available_height()),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("local_authentication")
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
        );
        egui::ScrollArea::vertical().show(&mut child,|ui| {
            design::card().show(ui,|ui| {
                ui.set_width((width-40.0).max(200.0));
                ui.label(RichText::new("Lab Manager").color(Color32::from_gray(150)).size(13.0));
                ui.heading(RichText::new(match self.mode {Mode::SignIn=>"Welcome back",Mode::Register=>"Create your local account",Mode::Recover=>"Recover your account",Mode::Blocked=>"Account unavailable"}).size(25.0));
                ui.label(match self.mode {Mode::Register=>"Protect this workspace with a password and two recovery questions.",Mode::Recover=>"Answer both security questions to set a new password.",Mode::Blocked=>"The account file could not be decrypted or read. It has not been overwritten.",Mode::SignIn=>"Sign in to your local server workspace."});
                ui.add_space(12.0);
                if self.mode==Mode::Blocked {
                    ui.label(&self.message);
                    ui.label("Use the same computer account that created this local account. Restore a valid backup if the file is damaged.");
                    ui.small(self.store.path.display().to_string());
                    return;
                }
                if self.mode==Mode::Register {
                    self.profile_picker.show(ui,&mut self.registration_profile);
                    ui.separator();
                }
                ui.label("Username");
                ui.add(egui::TextEdit::singleline(&mut self.username).desired_width(f32::INFINITY).hint_text(RichText::new("Your username").color(Color32::from_gray(100))));
                if self.mode==Mode::Recover {
                    for i in 0..2 {
                        ui.label(&self.recovery_questions[i]);
                        ui.add(egui::TextEdit::singleline(&mut self.answers[i]).password(true).desired_width(f32::INFINITY));
                    }
                }
                ui.label(if self.mode==Mode::Recover {"New password"} else {"Password"});
                let password=ui.add(egui::TextEdit::singleline(&mut self.password).password(true).desired_width(f32::INFINITY));
                if self.mode!=Mode::SignIn {
                    ui.small("At least 12 characters. A long, unique passphrase works well.");
                    ui.label("Confirm password");
                    ui.add(egui::TextEdit::singleline(&mut self.confirmation).password(true).desired_width(f32::INFINITY));
                }
                if self.mode==Mode::Register {
                    ui.add_space(10.0);
                    ui.strong("Account recovery");
                    ui.small("Choose answers only you know. Both are required to reset your password; case and extra spaces are ignored.");
                    for i in 0..2 {
                        ui.label(format!("Security question {}",i+1));
                        egui::ComboBox::from_id_salt(("security_question",i)).width(ui.available_width()).selected_text(QUESTIONS[self.question_ids[i]]).show_ui(ui,|ui| {
                            for (index,question) in QUESTIONS.iter().enumerate() { ui.selectable_value(&mut self.question_ids[i],index,*question); }
                        });
                        ui.label("Answer");
                        ui.add(egui::TextEdit::singleline(&mut self.answers[i]).password(true).desired_width(f32::INFINITY));
                    }
                }
                ui.add_space(12.0);
                if self.mode == Mode::SignIn || self.mode == Mode::Register {
                    ui.checkbox(&mut self.automatic_sign_in, "Automatically sign in on this computer");
                    ui.small("For 30 days. Signing out or resetting your password turns this off.");
                }
                if !self.message.is_empty() { ui.label(RichText::new(&self.message).color(Color32::from_rgb(225,170,150))); }
                let submit=ui.add_sized([ui.available_width(),36.0],egui::Button::new(match self.mode {Mode::Register=>"Create local account",Mode::Recover=>"Reset password",_=>"Sign in"}).corner_radius(16.0)).clicked()
                    || (self.mode==Mode::SignIn && password.lost_focus() && ui.input(|i|i.key_pressed(egui::Key::Enter)));
                if submit && !self.profile_picker.pending() { self.submit(); }
                if self.mode==Mode::SignIn && ui.link("Create another local account").clicked() {
                    self.clear_secrets(); self.username.clear(); self.message.clear(); self.automatic_sign_in=false; self.registration_profile=Default::default(); self.profile_picker=Default::default(); self.mode=Mode::Register;
                }
                if self.mode==Mode::Register && ui.link("Already registered? Sign in").clicked() { self.clear_secrets(); self.message.clear(); self.mode=Mode::SignIn; }
                if self.mode==Mode::SignIn && ui.link("Forgot your password?").clicked() {
                    match self.store.questions(&self.username) {
                        Ok(questions)=> { self.clear_secrets(); self.recovery_questions=questions; self.message.clear(); self.mode=Mode::Recover; }
                        Err(error)=>self.message=error,
                    }
                }
                if self.mode==Mode::Recover && ui.link("Back to sign in").clicked() { self.clear_secrets(); self.mode=Mode::SignIn; self.message.clear(); }
                ui.add_space(12.0);
                ui.small("Local only. Account data is encrypted for this computer user. No account details are sent online.");
            });
        });
    }

    fn submit(&mut self) {
        if self.mode != Mode::SignIn && self.password != self.confirmation {
            self.message = "Passwords do not match.".into();
            return;
        }
        let result = match self.mode {
            Mode::Register => self
                .store
                .register_with_profile(
                    &self.username,
                    &self.password,
                    self.question_ids,
                    [&self.answers[0], &self.answers[1]],
                    self.registration_profile.clone(),
                )
                .and_then(|()| {
                    if self.automatic_sign_in {
                        self.mode = Mode::SignIn;
                        self.store
                            .sign_in_with_options(&self.username, &self.password, true)
                            .map(|username| self.signed_in = Some(username))
                    } else {
                        Ok(())
                    }
                }),
            Mode::Recover => self.store.reset(
                &self.username,
                [&self.answers[0], &self.answers[1]],
                &self.password,
            ),
            Mode::SignIn => self
                .store
                .sign_in_with_options(&self.username, &self.password, self.automatic_sign_in)
                .map(|username| {
                    self.signed_in = Some(username);
                }),
            Mode::Blocked => return,
        };
        match result {
            Ok(()) => {
                if let Some(username) = &self.signed_in {
                    self.profile = self.store.profile(username).unwrap_or_default();
                }
                self.message = match self.mode {
                    Mode::Register => "Account created. Sign in to continue.",
                    Mode::Recover => "Password reset. Sign in with your new password.",
                    _ => "Signed in.",
                }
                .into();
                self.clear_secrets();
                self.mode = Mode::SignIn;
            }
            Err(error) => {
                self.message = error;
                self.password.zeroize();
                self.confirmation.zeroize();
            }
        }
    }
}

impl Drop for AuthScreen {
    fn drop(&mut self) {
        self.clear_secrets();
    }
}
