use eframe::egui::{self, Color32, Vec2};
use lab_manager::{model::Server, monitoring::Config, terminal::Session};
use std::{path::PathBuf, sync::mpsc, time::Duration};

fn auth_signature(config: Option<&Config>) -> (bool, String, Option<String>) {
    config
        .map(|config| {
            (
                config.uses_password(),
                config.identity_file.clone(),
                config.password_id.clone(),
            )
        })
        .unwrap_or_default()
}
fn connection_hint(output: &str, running: bool) -> Option<&'static str> {
    let text = output.to_lowercase();
    if text.contains("remote host identification has changed") {
        Some(
            "The device host key changed. Verify the device identity before updating its trusted key.",
        )
    } else if text.contains("host key verification failed") {
        Some(
            "Host identity is not trusted by OpenSSH. Use Log in manually to verify it, or verify the host in an OpenSSH terminal.",
        )
    } else if !running && text.contains("permission denied") {
        Some(
            "Authentication was rejected. Check the SSH username and password/key in this device's Monitoring settings.",
        )
    } else if !running
        && (text.contains("connection timed out") || text.contains("connection refused"))
    {
        Some(
            "The SSH port could not be reached. Check the device address, SSH port and whether SSH is enabled.",
        )
    } else if running && text.trim_end().ends_with("password:") {
        Some("Waiting for your SSH password. Type in the terminal and press Enter.")
    } else {
        None
    }
}

const TERMINAL_FOCUS_FILTER: egui::EventFilter = egui::EventFilter {
    tab: true,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: true,
};

#[derive(Clone)]
enum Target {
    Ssh {
        server: Box<Server>,
        config: Option<Config>,
        root: Option<PathBuf>,
        command: Option<String>,
    },
    Local,
}
#[derive(Default)]
pub struct TerminalPanel {
    target: Option<Target>,
    session: Option<Session>,
    pending: Option<mpsc::Receiver<Result<Session, String>>>,
    error: String,
    focus: bool,
    cancelled: bool,
    scroll: usize,
    selection: Option<((u16, u16), (u16, u16))>,
    history: Option<(PathBuf, String)>,
}
impl TerminalPanel {
    pub fn active(&self) -> bool {
        self.target.is_some()
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn open_ssh(
        &mut self,
        server: Server,
        config: Option<Config>,
        root: Option<PathBuf>,
        command: Option<String>,
    ) {
        if let Some(Target::Ssh {
            server: current,
            command: cmd,
            ..
        }) = &self.target
            && current.ip == server.ip
            && current.ssh_port == server.ssh_port
            && current.username == server.username
            && cmd == &command
        {
            let changed = match &self.target {
                Some(Target::Ssh { config: old, .. }) => {
                    auth_signature(old.as_ref()) != auth_signature(config.as_ref())
                }
                _ => false,
            };
            self.target = Some(Target::Ssh {
                server: Box::new(server),
                config,
                root,
                command,
            });
            self.focus = true;
            if self.pending.is_none()
                && (changed
                    || self
                        .session
                        .as_ref()
                        .is_some_and(|session| !session.running)
                    || self.cancelled
                    || !self.error.is_empty())
            {
                self.start();
            }
            return;
        }
        self.clear();
        self.target = Some(Target::Ssh {
            server: Box::new(server),
            config,
            root,
            command,
        });
        self.start();
    }
    pub fn open_local(&mut self) {
        self.clear();
        self.target = Some(Target::Local);
        self.start();
    }
    fn start(&mut self) {
        self.cancelled = false;
        self.finish_history("Session restarted · remote completion unknown");
        self.session = None;
        self.pending = None;
        self.error.clear();
        self.scroll = 0;
        self.selection = None;
        self.focus = true;
        let Some(target) = self.target.clone() else {
            return;
        };
        if let Target::Ssh {
            server,
            command: Some(command),
            root: Some(root),
            ..
        } = &target
            && let Some(parent) = root.parent()
        {
            let path = parent.join("commands.json");
            match lab_manager::command_history::start(&path, server, command) {
                Ok(id) => self.history = Some((path, id)),
                Err(error) => self.error = format!("Command history unavailable: {error}"),
            }
        }
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        std::thread::spawn(move || {
            let command = match target {
                Target::Local => Ok(lab_manager::terminal::local_command()),
                Target::Ssh {
                    server,
                    config,
                    root,
                    command,
                } => lab_manager::terminal::ssh_command(
                    &server,
                    config.as_ref(),
                    root.as_deref(),
                    command.as_deref(),
                ),
            };
            let result = command.and_then(|c| Session::spawn(c, 24, 80));
            let _ = tx.send(result);
        });
    }
    pub fn focused(ctx: &egui::Context) -> bool {
        ctx.memory(|m| m.has_focus(egui::Id::new("embedded_terminal_input")))
    }
    pub fn identity(&self) -> String {
        match &self.target {
            Some(Target::Ssh { server, .. }) => {
                format!("{}@{}:{}", server.username, server.ip, server.ssh_port)
            }
            Some(Target::Local) => "Local PowerShell".into(),
            None => "Terminal".into(),
        }
    }
    fn finish_history(&mut self, status: &str) {
        if let Some((path, id)) = self.history.take()
            && let Err(error) = lab_manager::command_history::finish(&path, &id, status)
        {
            self.error = format!("Could not update command history: {error}");
        }
    }
    pub fn poll(&mut self) {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(Ok(session)) => {
                    self.session = Some(session);
                    self.pending = None;
                    self.focus = true;
                }
                Ok(Err(error)) => {
                    self.error = error;
                    self.pending = None;
                    self.finish_history("Could not start SSH client");
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.error = "Terminal startup stopped.".into();
                    self.pending = None;
                }
                _ => {}
            }
        }
        if let Some(session) = &mut self.session {
            session.poll();
            if !session.running {
                let status = session.status.clone();
                self.finish_history(&status);
            }
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.poll();
        let manual = matches!(&self.target, Some(Target::Ssh { command: None, .. }));
        let actions = terminal_toolbar(
            ui,
            &self.identity(),
            self.pending.is_some() || self.session.as_ref().is_some_and(|session| session.running),
            self.session.is_some(),
            self.scroll > 0,
            self.pending.is_some(),
            manual,
        );
        if actions[0] {
            self.start();
        }
        if actions[1] {
            self.cancelled = true;
            self.pending = None;
            if let Some(session) = &mut self.session {
                session.disconnect();
            }
            self.finish_history("Disconnected · remote completion unknown");
        }
        if actions[2] {
            self.copy(ui.ctx());
        }
        if actions[3] {
            self.scroll = 0;
            self.selection = None;
        }
        if actions[4] {
            if let Some(Target::Ssh { config, .. }) = &mut self.target {
                let config = config.get_or_insert_with(Config::default);
                config.password_id = None;
                config.auth_mode = Some(lab_manager::monitoring::AuthMode::Password);
            }
            self.start();
        }
        ui.small("Click terminal to type · Ctrl+Shift+C/V copy/paste · scroll for history");
        if !self.error.is_empty() {
            ui.colored_label(Color32::from_rgb(230, 90, 90), &self.error);
        }
        if self.pending.is_some() {
            ui.spinner();
            ui.label("Starting embedded terminal…");
            ui.ctx().request_repaint_after(Duration::from_millis(30));
            return;
        }
        let Some(s) = &mut self.session else { return };
        s.repaint_on_output(ui.ctx());
        s.poll();
        ui.small(&s.status);
        if let Ok(parser) = s.parser.lock() {
            let text = parser.screen().contents();
            if let Some(hint) = connection_hint(&text, s.running) {
                ui.small(hint);
            }
        }
        let font = egui::FontId::monospace(13.0);
        let cell_width = ui.fonts(|f| f.glyph_width(&font, 'M')).max(6.0);
        let cell_height = 18.0;
        let size = Vec2::new(
            ui.available_width().max(100.0),
            (ui.available_height() - 6.0).clamp(80.0, 3000.0),
        );
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let id = egui::Id::new("embedded_terminal_input");
        let input = ui.interact(rect, id, egui::Sense::click_and_drag());
        if input.clicked() || input.secondary_clicked() || self.focus {
            input.request_focus();
            self.focus = false;
        }
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(id, TERMINAL_FOCUS_FILTER);
        });
        let rows = ((rect.height() - 12.0) / cell_height)
            .floor()
            .clamp(2.0, 160.0) as u16;
        let cols = ((rect.width() - 12.0) / cell_width)
            .floor()
            .clamp(10.0, 320.0) as u16;
        if let Err(error) = s.resize(rows, cols) {
            self.error = error;
        }
        if input.hovered() {
            let delta = ui.input(|i| i.smooth_scroll_delta.y);
            if delta != 0.0 {
                self.scroll = (self.scroll as i32 + (delta / 8.0) as i32)
                    .clamp(0, lab_manager::terminal::SCROLLBACK as i32)
                    as usize;
                self.selection = None;
            }
        }
        let origin = rect.min + Vec2::splat(6.0);
        let position = |p: egui::Pos2| {
            (
                ((p.y - origin.y) / cell_height)
                    .floor()
                    .clamp(0.0, (rows - 1) as f32) as u16,
                ((p.x - origin.x) / cell_width)
                    .floor()
                    .clamp(0.0, (cols - 1) as f32) as u16,
            )
        };
        if input.drag_started()
            && let Some(p) = input.interact_pointer_pos()
        {
            let pos = position(p);
            self.selection = Some((pos, pos));
        }
        if input.dragged()
            && let (Some((start, _)), Some(p)) = (self.selection, input.interact_pointer_pos())
        {
            self.selection = Some((start, position(p)));
        }
        if input.clicked() {
            self.selection = None;
        }
        let focused = input.has_focus();
        let mut copy = false;
        if focused && s.running {
            let mut events = ui.input(|i| i.events.clone());
            if input.secondary_clicked() {
                match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
                    Ok(text) => events.push(egui::Event::Paste(text)),
                    Err(error) => self.error = format!("Cannot paste clipboard text: {error}"),
                }
            }
            let modifiers = ui.input(|i| i.modifiers);
            let parser = s.parser.lock().ok();
            let application = parser
                .as_ref()
                .is_some_and(|p| p.screen().application_cursor());
            let bracketed = parser
                .as_ref()
                .is_some_and(|p| p.screen().bracketed_paste());
            drop(parser);
            let has_copy = events.iter().any(|e| matches!(e, egui::Event::Copy));
            let has_cut = events.iter().any(|e| matches!(e, egui::Event::Cut));
            for event in events {
                let bytes = match event {
                    egui::Event::Cut => Some(vec![24]),
                    egui::Event::Copy => {
                        if self.selection.is_some() || modifiers.shift {
                            copy = true;
                            None
                        } else {
                            Some(vec![3])
                        }
                    }
                    egui::Event::Text(text) => Some(if modifiers.alt && !modifiers.ctrl {
                        let mut v = vec![27];
                        v.extend(text.as_bytes());
                        v
                    } else {
                        text.into_bytes()
                    }),
                    egui::Event::Paste(text) => Some(paste_bytes(&text, bracketed)),
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        if modifiers.ctrl && modifiers.shift && key == egui::Key::C {
                            copy = true;
                            None
                        } else if modifiers.ctrl
                            && ((has_copy && key == egui::Key::C)
                                || (has_cut && key == egui::Key::X))
                        {
                            None
                        } else {
                            key_bytes(key, modifiers, application)
                        }
                    }
                    _ => None,
                };
                if let Some(bytes) = bytes {
                    self.scroll = 0;
                    self.selection = None;
                    if let Err(error) = s.send(bytes) {
                        self.error = error;
                    }
                }
            }
        }
        let painter = ui.painter().with_clip_rect(rect);
        let background = Color32::from_rgb(8, 10, 11);
        painter.rect_filled(rect, 5.0, background);
        painter.rect_stroke(
            rect,
            5.0,
            egui::Stroke::new(
                1.0_f32,
                if focused {
                    Color32::from_rgb(60, 140, 95)
                } else {
                    Color32::from_gray(50)
                },
            ),
            egui::StrokeKind::Inside,
        );
        if let Ok(mut parser) = s.parser.lock() {
            parser.set_scrollback(self.scroll);
            self.scroll = parser.screen().scrollback();
            let screen = parser.screen();
            let selection = self
                .selection
                .map(|(a, b)| if a <= b { (a, b) } else { (b, a) });
            for row in 0..rows {
                for col in 0..cols {
                    let Some(cell) = screen.cell(row, col) else {
                        continue;
                    };
                    if cell.is_wide_continuation() {
                        continue;
                    }
                    let mut foreground =
                        color(cell.fgcolor(), Color32::from_gray(220), cell.bold());
                    let mut bg = color(cell.bgcolor(), background, false);
                    if cell.inverse() {
                        std::mem::swap(&mut foreground, &mut bg);
                    }
                    if selection.is_some_and(|(a, b)| (row, col) >= a && (row, col) <= b) {
                        bg = Color32::from_rgb(40, 75, 60);
                    }
                    let pos = origin + Vec2::new(col as f32 * cell_width, row as f32 * cell_height);
                    let width = if cell.is_wide() {
                        cell_width * 2.0
                    } else {
                        cell_width
                    };
                    if bg != background {
                        painter.rect_filled(
                            egui::Rect::from_min_size(pos, Vec2::new(width, cell_height)),
                            0.0,
                            bg,
                        );
                    }
                    let text = cell.contents();
                    if !text.is_empty() {
                        painter.text(pos, egui::Align2::LEFT_TOP, text, font.clone(), foreground);
                    }
                    if cell.underline() {
                        painter.line_segment(
                            [
                                pos + Vec2::new(0.0, cell_height - 2.0),
                                pos + Vec2::new(width, cell_height - 2.0),
                            ],
                            egui::Stroke::new(1.0_f32, foreground),
                        );
                    }
                }
            }
            if focused && self.scroll == 0 && !screen.hide_cursor() && s.running {
                let (r, c) = screen.cursor_position();
                if r < rows && c < cols {
                    painter.rect_stroke(
                        egui::Rect::from_min_size(
                            origin + Vec2::new(c as f32 * cell_width, r as f32 * cell_height),
                            Vec2::new(cell_width, cell_height),
                        ),
                        0.0,
                        egui::Stroke::new(1.2_f32, Color32::from_gray(220)),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        if copy {
            self.copy(ui.ctx());
        }
    }
    fn copy(&self, ctx: &egui::Context) {
        if let (Some(s), Some((a, b))) = (&self.session, self.selection)
            && let Ok(parser) = s.parser.lock()
        {
            let (a, b) = if a <= b { (a, b) } else { (b, a) };
            ctx.copy_text(
                parser
                    .screen()
                    .contents_between(a.0, a.1, b.0, b.1.saturating_add(1)),
            );
        }
    }
}
impl Drop for TerminalPanel {
    fn drop(&mut self) {
        self.finish_history("Client disconnected · remote completion unknown");
    }
}

#[derive(Default)]
pub struct TerminalTabs {
    sessions: Vec<TerminalPanel>,
    selected: usize,
    visible: bool,
    error: String,
    scroll_to_selected: bool,
    search: crate::terminal_search::DeviceSearch,
}
impl TerminalTabs {
    pub fn command_os(&self) -> Option<lab_manager::model::DeviceOs> {
        match self
            .sessions
            .get(self.selected)
            .and_then(|panel| panel.target.as_ref())
        {
            Some(Target::Ssh {
                server,
                command: Some(_),
                ..
            }) => Some(server.effective_os()),
            _ => None,
        }
    }

    pub fn active(&self) -> bool {
        self.visible && !self.sessions.is_empty()
    }
    pub fn count(&self) -> usize {
        self.sessions.len()
    }
    pub fn hide(&mut self) {
        self.search.close();
        self.visible = false;
    }
    pub fn resume(&mut self) {
        self.visible = !self.sessions.is_empty();
        self.scroll_to_selected = true;
        if let Some(panel) = self.sessions.get_mut(self.selected) {
            panel.focus = true;
        }
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn poll(&mut self) {
        for panel in &mut self.sessions {
            panel.poll();
        }
    }
    pub fn identity(&self) -> String {
        self.sessions
            .get(self.selected)
            .map_or("Terminal".into(), TerminalPanel::identity)
    }
    pub fn open_ssh(
        &mut self,
        server: Server,
        config: Option<Config>,
        root: Option<PathBuf>,
        command: Option<String>,
    ) {
        if command.is_none() && let Some(index) = self.sessions.iter().position(|panel| matches!(&panel.target, Some(Target::Ssh { server: current, command: None, .. }) if current.ip == server.ip && current.ssh_port == server.ssh_port && current.username == server.username)) {
            self.sessions[index].open_ssh(server, config, root, command);
            self.selected = index; self.resume(); return;
        }
        self.open_new_ssh(server, config, root, command);
    }
    pub fn open_new_ssh(
        &mut self,
        server: Server,
        config: Option<Config>,
        root: Option<PathBuf>,
        command: Option<String>,
    ) {
        self.search.close();
        if self.sessions.len() >= 8 {
            self.error = "Up to 8 sessions. Close a tab before opening another.".into();
            self.resume();
            return;
        }
        let mut panel = TerminalPanel::default();
        panel.open_ssh(server, config, root, command);
        self.sessions.push(panel);
        self.selected = self.sessions.len() - 1;
        self.visible = true;
        self.scroll_to_selected = true;
        self.error.clear();
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        servers: &[Server],
        monitor: &lab_manager::status::Monitor,
    ) -> Option<Server> {
        self.poll();
        let command_os = self.command_os();
        let allow_add = command_os.is_none();
        if let Some(os) = command_os {
            self.search.close();
            lab_manager::settings_form::os_badge(ui, os);
            ui.add_space(6.0);
        }

        let was_searching = self.search.open;
        let mut close = None;
        let previous_selected = self.selected;
        let mut new_shell = false;
        let mut search_trigger = egui::Rect::NOTHING;
        let body_background = ui.painter().add(egui::Shape::Noop);
        let strip_background = ui.painter().add(egui::Shape::Noop);
        let old_spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        let available = ui.available_width();
        let tab_width = ((available - if allow_add { 48.0 } else { 12.0 })
            / self.sessions.len().clamp(1, 3) as f32)
            .clamp(148.0, 220.0);
        let strip = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add_space(12.0);
            ui.allocate_ui_with_layout(
                Vec2::new(
                    (available - if allow_add { 54.0 } else { 12.0 }).max(100.0),
                    44.0,
                ),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt("terminal_tabs_scroll")
                        .max_height(44.0)
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                for (index, panel) in self.sessions.iter().enumerate() {
                                    if let Some(Target::Ssh { server, .. }) = &panel.target {
                                        let (response, closed) = terminal_tab(
                                            ui,
                                            index,
                                            server,
                                            self.selected == index,
                                            tab_width,
                                        );
                                        if self.scroll_to_selected && self.selected == index {
                                            response.scroll_to_me(Some(egui::Align::Center));
                                        }
                                        if closed {
                                            close = Some(index);
                                        } else if response.clicked() {
                                            self.selected = index;
                                        }
                                    }
                                }
                            });
                        });
                },
            );
            if allow_add {
                let add = ui
                    .add_enabled(
                        self.sessions.len() < 8,
                        egui::Button::new("")
                            .frame(false)
                            .min_size(Vec2::splat(32.0)),
                    )
                    .on_hover_text("Search devices to open a terminal");
                lab_manager::svg_icons::paint(
                    ui.painter(),
                    add.rect.shrink(6.0),
                    "plus",
                    Some(Color32::from_gray(190)),
                );
                new_shell = add.clicked();
                search_trigger = add.rect;
                #[cfg(test)]
                ui.ctx().data_mut(|data| {
                    data.insert_temp(egui::Id::new("terminal_add_button"), add.rect)
                });
            }
        });
        ui.painter().set(
            strip_background,
            egui::Shape::rect_filled(
                egui::Rect::from_min_size(
                    strip.response.rect.min,
                    Vec2::new(available, strip.response.rect.height()),
                ),
                egui::CornerRadius {
                    nw: 12,
                    ne: 12,
                    sw: 0,
                    se: 0,
                },
                Color32::from_gray(21),
            ),
        );
        self.scroll_to_selected = false;
        if new_shell {
            self.search.toggle();
            ui.memory_mut(|memory| {
                memory.surrender_focus(egui::Id::new("embedded_terminal_input"))
            });
        }
        if let Some(index) = close {
            self.sessions.remove(index);
            if index < self.selected {
                self.selected -= 1;
            }
            self.selected = self.selected.min(self.sessions.len().saturating_sub(1));
        }
        if (previous_selected != self.selected || close.is_some())
            && let Some(panel) = self.sessions.get_mut(self.selected)
        {
            panel.focus = true;
            self.scroll_to_selected = true;
        }
        let body = egui::Frame::new().inner_margin(12.0).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = old_spacing;
            ui.set_min_height(ui.available_height());
            if !self.error.is_empty() {
                ui.colored_label(Color32::from_rgb(235, 160, 70), &self.error);
            }
            if let Some(panel) = self.sessions.get_mut(self.selected) {
                if self.search.open {
                    panel.focus = false;
                    ui.memory_mut(|memory| {
                        memory.surrender_focus(egui::Id::new("embedded_terminal_input"))
                    });
                }
                panel.show(ui);
            } else {
                self.visible = false;
            }
        });
        #[cfg(test)]
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::new("terminal_content_rect"), body.response.rect)
        });
        ui.painter().set(
            body_background,
            egui::Shape::Rect(egui::epaint::RectShape::new(
                body.response.rect,
                12.0,
                Color32::from_gray(28),
                egui::Stroke::new(1.0_f32, Color32::from_gray(60)),
                egui::StrokeKind::Inside,
            )),
        );
        ui.spacing_mut().item_spacing.y = old_spacing;
        let selected = self.search.show(
            ui.ctx(),
            ui.ctx()
                .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("right_sidebar_bounds")))
                .unwrap_or(body.response.rect),
            search_trigger,
            servers,
            monitor,
        );
        if was_searching
            && !self.search.open
            && let Some(panel) = self.sessions.get_mut(self.selected)
        {
            panel.focus = true;
        }
        selected
    }
}

fn terminal_toolbar(
    ui: &mut egui::Ui,
    identity: &str,
    running: bool,
    can_copy: bool,
    history: bool,
    starting: bool,
    manual: bool,
) -> [bool; 5] {
    let width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(Vec2::new(width, 34.0), egui::Sense::hover());
    let count = 3 + usize::from(history) + usize::from(manual);
    let controls_width = count as f32 * 30.0 + (count - 1) as f32 * 6.0;
    let label_rect = egui::Rect::from_min_max(
        row.min,
        egui::pos2(
            (row.right() - controls_width - 12.0).max(row.left()),
            row.bottom(),
        ),
    );
    let text = egui::WidgetText::from(egui::RichText::new(identity).strong().size(13.0))
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            label_rect.width(),
            egui::FontId::proportional(13.0),
        );
    ui.painter()
        .with_clip_rect(ui.clip_rect().intersect(label_rect))
        .galley(
            egui::pos2(
                label_rect.left(),
                label_rect.center().y - text.size().y / 2.0,
            ),
            text,
            Color32::from_gray(220),
        );
    ui.interact(
        label_rect,
        ui.id().with("terminal_connection_label"),
        egui::Sense::hover(),
    )
    .on_hover_text(identity);
    let mut actions = [false; 5];
    for (position, (kind, label, enabled)) in [
        (2, "Copy selection", can_copy),
        (0, "Reconnect", !starting),
        (4, "Log in manually", manual && !starting),
        (1, "Disconnect", running),
        (3, "Live output", history),
    ]
    .into_iter()
    .filter(|(kind, _, _)| (*kind != 4 || manual) && (*kind != 3 || history))
    .enumerate()
    {
        let rect = egui::Rect::from_min_size(
            egui::pos2(
                row.right() - controls_width + position as f32 * 36.0,
                row.center().y - 15.0,
            ),
            Vec2::splat(30.0),
        );
        let mut button_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("terminal_toolbar_button", kind))
                .max_rect(rect),
        );
        if !enabled {
            button_ui.disable();
        }
        let response = button_ui
            .interact(rect, button_ui.id(), egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(label);
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
        #[cfg(test)]
        ui.ctx()
            .data_mut(|data| data.insert_temp(egui::Id::new(("terminal_action", kind)), rect));
        let painter = button_ui.painter();
        if response.hovered() {
            painter.rect_filled(rect, 7.0, Color32::from_gray(40));
        }
        lab_manager::svg_icons::paint(
            button_ui.painter(),
            rect.shrink(5.0),
            match kind {
                0 => "unplug",
                1 => "power",
                2 => "copy",
                4 => "user",
                _ => "chevrons-up-down",
            },
            Some(Color32::from_gray(if enabled { 190 } else { 80 })),
        );
        actions[kind] = response.clicked();
    }
    actions
}
fn terminal_tab(
    ui: &mut egui::Ui,
    index: usize,
    server: &Server,
    selected: bool,
    width: f32,
) -> (egui::Response, bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 44.0), egui::Sense::hover());
    let select_rect =
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.right() - 50.0, rect.bottom()));
    let response = ui.interact(
        select_rect,
        ui.id().with(("terminal_tab_select", index)),
        egui::Sense::click(),
    );
    let response = response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(format!(
            "{}@{}:{}",
            server.username, server.ip, server.ssh_port
        ));
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            &server.name,
        )
    });
    if selected {
        paint_connected_tab(ui, rect, Color32::from_gray(28));
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect.shrink2(Vec2::new(3.0, 5.0)),
            9.0,
            Color32::from_gray(24),
        );
    }
    let icon_rect =
        egui::Rect::from_center_size(rect.left_center() + Vec2::new(25.0, 0.0), Vec2::splat(22.0));
    let mut icon_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("terminal_tab_icon", index))
            .max_rect(icon_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    lab_manager::device_icons::thumbnail(
        &mut icon_ui,
        server.icon,
        server.custom_icon.as_deref(),
        22.0,
    );
    let close_rect = egui::Rect::from_center_size(
        rect.right_center() - Vec2::new(34.0, 0.0),
        Vec2::splat(26.0),
    );
    let name_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 43.0, rect.top()),
        egui::pos2(close_rect.left() - 6.0, rect.bottom()),
    );
    let label_color = if selected {
        Color32::from_gray(235)
    } else {
        Color32::from_gray(150)
    };
    let galley = egui::WidgetText::from(
        egui::RichText::new(&server.name)
            .size(13.0)
            .color(label_color),
    )
    .into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        name_rect.width(),
        egui::FontId::proportional(13.0),
    );
    let label_position = egui::pos2(
        name_rect.left(),
        name_rect.center().y - galley.size().y / 2.0,
    );
    ui.painter()
        .with_clip_rect(ui.clip_rect().intersect(name_rect))
        .galley(label_position, galley, label_color);
    let close = ui
        .interact(
            close_rect,
            ui.id().with(("terminal_tab_close", index)),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Disconnect and close this tab");
    close.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            format!("Close terminal tab {}", server.name),
        )
    });
    if close.hovered() {
        ui.painter()
            .rect_filled(close_rect, 7.0, Color32::from_rgb(108, 40, 43));
    }
    lab_manager::svg_icons::paint(
        ui.painter(),
        close_rect.shrink(7.0),
        "x",
        Some(Color32::from_gray(190)),
    );
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(egui::Id::new(("terminal_tab", index)), rect);
        data.insert_temp(egui::Id::new(("terminal_tab_close", index)), close_rect);
        data.insert_temp(egui::Id::new(("terminal_tab_icon", index)), icon_rect);
    });
    (response, close.clicked())
}

fn paint_connected_tab(ui: &egui::Ui, rect: egui::Rect, fill: Color32) {
    // Rounded top corners and concave feet join the active tab to its content panel.
    let left = rect.left() + 10.0;
    let right = rect.right() - 10.0;
    let top = rect.top() + 3.0;
    let bottom = rect.bottom() + 1.0;
    let radius = 10.0;
    let mut points = Vec::new();
    let mut arc = |center: egui::Pos2, start: f32, end: f32| {
        for step in 0..=8 {
            let angle = start + (end - start) * step as f32 / 8.0;
            points.push(center + Vec2::angled(angle) * radius);
        }
    };
    use std::f32::consts::{FRAC_PI_2, PI};
    arc(egui::pos2(left - radius, bottom - radius), FRAC_PI_2, 0.0);
    arc(egui::pos2(left + radius, top + radius), PI, PI * 1.5);
    arc(egui::pos2(right - radius, top + radius), PI * 1.5, PI * 2.0);
    arc(egui::pos2(right + radius, bottom - radius), PI, FRAC_PI_2);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(egui::pos2(rect.center().x, bottom), fill);
    for point in &points {
        mesh.colored_vertex(*point, fill);
    }
    for index in 0..points.len() {
        mesh.add_triangle(0, index as u32 + 1, ((index + 1) % points.len()) as u32 + 1);
    }
    let painter = ui.painter().with_clip_rect(ui.clip_rect().expand(2.0));
    painter.add(egui::Shape::mesh(mesh));
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(1.0_f32, Color32::from_gray(60)),
    ));
}
fn color(color: vt100::Color, default: Color32, bold: bool) -> Color32 {
    match color {
        vt100::Color::Default => default,
        vt100::Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
        vt100::Color::Idx(mut i) => {
            if bold && i < 8 {
                i += 8;
            }
            let palette = [
                [0, 0, 0],
                [205, 70, 80],
                [65, 185, 110],
                [205, 180, 80],
                [90, 135, 220],
                [175, 110, 200],
                [65, 175, 190],
                [210, 210, 210],
                [100, 100, 100],
                [245, 100, 110],
                [110, 225, 150],
                [245, 225, 130],
                [140, 180, 255],
                [215, 155, 245],
                [110, 225, 235],
                [255, 255, 255],
            ];
            if i < 16 {
                let [r, g, b] = palette[i as usize];
                Color32::from_rgb(r, g, b)
            } else if i >= 232 {
                Color32::from_gray(8 + 10 * (i - 232))
            } else {
                let n = i - 16;
                let component = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
                Color32::from_rgb(component(n / 36), component((n / 6) % 6), component(n % 6))
            }
        }
    }
}
fn key_bytes(key: egui::Key, m: egui::Modifiers, application: bool) -> Option<Vec<u8>> {
    use egui::Key;
    if m.ctrl && !m.alt {
        let text = match key {
            Key::A => Some(1),
            Key::B => Some(2),
            Key::C => Some(3),
            Key::D => Some(4),
            Key::E => Some(5),
            Key::F => Some(6),
            Key::G => Some(7),
            Key::H => Some(8),
            Key::K => Some(11),
            Key::L => Some(12),
            Key::N => Some(14),
            Key::O => Some(15),
            Key::P => Some(16),
            Key::R => Some(18),
            Key::S => Some(19),
            Key::T => Some(20),
            Key::U => Some(21),
            Key::W => Some(23),
            Key::X => Some(24),
            Key::Y => Some(25),
            Key::Z => Some(26),
            _ => None,
        };
        if let Some(c) = text {
            return Some(vec![c]);
        }
    }
    let prefix = if application { "\x1bO" } else { "\x1b[" };
    let bytes = match key {
        Key::Enter => "\r".into(),
        Key::Backspace => "\x7f".into(),
        Key::Tab => if m.shift { "\x1b[Z" } else { "\t" }.into(),
        Key::Escape => "\x1b".into(),
        Key::ArrowUp => format!("{prefix}A"),
        Key::ArrowDown => format!("{prefix}B"),
        Key::ArrowRight => format!("{prefix}C"),
        Key::ArrowLeft => format!("{prefix}D"),
        Key::Home => "\x1b[H".into(),
        Key::End => "\x1b[F".into(),
        Key::Delete => "\x1b[3~".into(),
        Key::Insert => "\x1b[2~".into(),
        Key::PageUp => "\x1b[5~".into(),
        Key::PageDown => "\x1b[6~".into(),
        Key::F1 => "\x1bOP".into(),
        Key::F2 => "\x1bOQ".into(),
        Key::F3 => "\x1bOR".into(),
        Key::F4 => "\x1bOS".into(),
        Key::F5 => "\x1b[15~".into(),
        Key::F6 => "\x1b[17~".into(),
        Key::F7 => "\x1b[18~".into(),
        Key::F8 => "\x1b[19~".into(),
        Key::F9 => "\x1b[20~".into(),
        Key::F10 => "\x1b[21~".into(),
        Key::F11 => "\x1b[23~".into(),
        Key::F12 => "\x1b[24~".into(),
        _ => return None,
    };
    Some(bytes.into_bytes())
}
fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    if bracketed {
        // Bracketed paste retains newlines as text for the shell/editor to handle.
        let mut bytes = b"\x1b[200~".to_vec();
        bytes.extend(text.as_bytes());
        bytes.extend(b"\x1b[201~");
        bytes
    } else {
        text.replace('\n', "\r").into_bytes()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_terminal_uses_shared_tabs_without_add_button() {
        for command in [None, Some("racadm getsysinfo".to_owned())] {
            let ctx = egui::Context::default();
            let mut server = Server::new("192.0.2.1".parse().unwrap());
            server.icon = lab_manager::model::ServerIcon::DellIdrac;
            let mut panel = TerminalPanel::default();
            panel.target = Some(Target::Ssh {
                server: Box::new(server),
                config: None,
                root: None,
                command: command.clone(),
            });
            let mut tabs = TerminalTabs::default();
            tabs.sessions.push(panel);
            tabs.visible = true;
            for _ in 0..2 {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(480.0, 700.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            tabs.show(ui, &[], &Default::default());
                        });
                    },
                );
            }
            assert_eq!(tabs.command_os().is_some(), command.is_some());
            assert_eq!(
                ctx.data(|data| data.get_temp::<egui::Rect>(egui::Id::new("terminal_add_button")))
                    .is_some(),
                command.is_none()
            );
            assert!(
                ctx.data(
                    |data| data.get_temp::<egui::Rect>(egui::Id::new(("terminal_tab", 0usize)))
                )
                .is_some()
            );
        }
    }
    #[test]
    fn manual_login_icon_is_between_reconnect_and_disconnect() {
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                terminal_toolbar(ui, "root@device:22", true, true, false, false, true);
            });
        });
        let rect = |kind: usize| {
            ctx.data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new(("terminal_action", kind)))
                    .unwrap()
            })
        };
        assert!(rect(2).right() < rect(0).left());
        assert!(rect(0).right() < rect(4).left());
        assert!(rect(4).right() < rect(1).left());
    }
    #[test]
    fn ssh_hints_distinguish_trust_authentication_and_network_failures() {
        assert!(
            connection_hint("Host key verification failed.", false)
                .unwrap()
                .contains("trust")
        );
        assert!(
            connection_hint("root@host: Permission denied (password).", false)
                .unwrap()
                .contains("Authentication")
        );
        assert!(
            connection_hint("Connection timed out", false)
                .unwrap()
                .contains("port")
        );
        assert!(
            connection_hint("root@host password:", true)
                .unwrap()
                .contains("Waiting")
        );
        assert!(
            connection_hint("REMOTE HOST IDENTIFICATION HAS CHANGED", false)
                .unwrap()
                .contains("changed")
        );
        assert!(connection_hint("root@host:~# ", true).is_none());
    }

    #[test]
    fn reopening_an_existing_tab_updates_credentials_without_duplicating_it() {
        let server = Server::new("192.0.2.1".parse().unwrap());
        let old_config = Config {
            interval: 60,
            ..Default::default()
        };
        let new_config = Config {
            interval: 120,
            ..Default::default()
        };
        let mut panel = TerminalPanel::default();
        panel.target = Some(Target::Ssh {
            server: Box::new(server.clone()),
            config: Some(old_config),
            root: None,
            command: None,
        });
        let mut tabs = TerminalTabs::default();
        tabs.sessions.push(panel);
        tabs.open_ssh(server, Some(new_config), None, None);
        assert_eq!(tabs.count(), 1);
        assert!(tabs.active());
        match &tabs.sessions[0].target {
            Some(Target::Ssh {
                config: Some(config),
                ..
            }) => assert_eq!(config.interval, 120),
            _ => panic!("SSH configuration was lost"),
        }
    }

    #[test]
    fn tabs_contain_icon_and_close_and_keep_active_session_when_earlier_tab_closes() {
        let mut tabs = TerminalTabs::default();
        for octet in 1..=3 {
            let mut panel = TerminalPanel::default();
            panel.target = Some(Target::Ssh {
                server: Box::new(Server::new(format!("192.0.2.{octet}").parse().unwrap())),
                config: None,
                root: None,
                command: None,
            });
            tabs.sessions.push(panel);
        }
        tabs.selected = 2;
        tabs.resume();
        let ctx = egui::Context::default();
        let mut frame = |events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(900.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        tabs.show(ui, &[], &lab_manager::status::Monitor::default())
                    });
                },
            );
        };
        for _ in 0..3 {
            frame(vec![]);
        }
        let (tab, close, icon) = ctx.data(|data| {
            (
                data.get_temp::<egui::Rect>(egui::Id::new(("terminal_tab", 0)))
                    .unwrap(),
                data.get_temp::<egui::Rect>(egui::Id::new(("terminal_tab_close", 0)))
                    .unwrap(),
                data.get_temp::<egui::Rect>(egui::Id::new(("terminal_tab_icon", 0)))
                    .unwrap(),
            )
        });
        assert!(tab.contains_rect(icon) && tab.contains_rect(close));
        assert!(
            close.right() <= tab.right() - 20.0,
            "close control must stay inside the tab's curved edge"
        );
        let body = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("terminal_content_rect")))
            .unwrap();
        assert!(
            tab.left() >= body.left() + 10.0,
            "first tab must clear the content panel's rounded corner"
        );
        assert!(icon.right() < close.left());
        frame(vec![egui::Event::PointerMoved(close.center())]);
        for pressed in [true, false] {
            frame(vec![egui::Event::PointerButton {
                pos: close.center(),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
        assert_eq!(tabs.count(), 2);
        assert_eq!(tabs.selected, 1);
        assert!(tabs.identity().contains("192.0.2.3"));
    }
    #[test]
    fn terminal_tabs_hide_without_disconnecting_and_reselect_existing_device() {
        let mut tabs = TerminalTabs::default();
        for octet in [1, 2] {
            let server = Server::new(format!("192.0.2.{octet}").parse().unwrap());
            let mut panel = TerminalPanel::default();
            panel.target = Some(Target::Ssh {
                server: Box::new(server),
                config: None,
                root: None,
                command: None,
            });
            tabs.sessions.push(panel);
        }
        tabs.resume();
        assert!(tabs.active());
        tabs.hide();
        assert!(!tabs.active());
        assert_eq!(tabs.count(), 2);
        tabs.open_ssh(Server::new("192.0.2.2".parse().unwrap()), None, None, None);
        assert_eq!(tabs.selected, 1);
        assert_eq!(tabs.count(), 2);
        assert!(tabs.active());
        tabs.clear();
        assert_eq!(tabs.count(), 0);
    }
    #[test]
    fn terminal_keeps_focus_for_navigation_and_editor_keys() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("terminal_focus_test");
        let keys = [
            egui::Key::ArrowLeft,
            egui::Key::ArrowRight,
            egui::Key::ArrowUp,
            egui::Key::ArrowDown,
            egui::Key::Tab,
            egui::Key::Escape,
        ];
        for frame in 0..keys.len() + 3 {
            let events = if frame >= 3 {
                vec![egui::Event::Key {
                    key: keys[frame - 3],
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }]
            } else {
                vec![]
            };
            let _ = ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let _ = ui.button("Other control");
                        let response = ui.interact(
                            egui::Rect::from_min_size(egui::pos2(20.0, 80.0), Vec2::splat(200.0)),
                            id,
                            egui::Sense::click_and_drag(),
                        );
                        if frame == 0 {
                            response.request_focus();
                        }
                        ui.memory_mut(|memory| {
                            memory.set_focus_lock_filter(id, TERMINAL_FOCUS_FILTER)
                        });
                    });
                },
            );
            assert!(
                ctx.memory(|memory| memory.has_focus(id)),
                "focus lost at frame {frame}"
            );
        }
    }
    #[test]
    fn paste_preserves_editor_newlines_and_shell_enter_keys() {
        assert_eq!(paste_bytes("one\r\ntwo", false), b"one\rtwo");
        assert_eq!(
            paste_bytes("one\r\ntwo", true),
            b"\x1b[200~one\ntwo\x1b[201~"
        );
    }
    #[test]
    fn terminal_keys_keep_interrupt_and_cursor_modes() {
        assert_eq!(
            key_bytes(egui::Key::X, egui::Modifiers::CTRL, false),
            Some(vec![24])
        );
        assert_eq!(
            key_bytes(egui::Key::ArrowLeft, egui::Modifiers::NONE, false),
            Some(b"\x1b[D".to_vec())
        );
        assert_eq!(
            key_bytes(egui::Key::ArrowRight, egui::Modifiers::NONE, true),
            Some(b"\x1bOC".to_vec())
        );
        assert_eq!(
            key_bytes(egui::Key::C, egui::Modifiers::CTRL, false),
            Some(vec![3])
        );
        assert_eq!(
            key_bytes(egui::Key::ArrowUp, egui::Modifiers::NONE, true),
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            key_bytes(egui::Key::ArrowUp, egui::Modifiers::NONE, false),
            Some(b"\x1b[A".to_vec())
        );
    }
}
