#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui::{self, Color32, RichText, Vec2};
use lab_manager::{
    model::{Server, ServerStore},
    server_form::ServerForm,
    status::{Connection, Monitor},
};
mod account_removal;
mod action_dock;
mod auth_screen;
mod backup_ui;
mod design;
mod device_settings;
mod discovery_screen;
mod frosted_popup;
mod help_menu;
mod monitoring_page;
mod monitoring_selector;
mod monitoring_settings;
mod notification_center;
mod profile_ui;
mod server_table;
mod sidebar;
mod terminal_panel;
mod terminal_search;
#[cfg(test)]
mod ui_tests;
mod update_panel;

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Devices,
    Monitoring,
    Local,
    Storage,
}

impl Page {
    fn title(self) -> &'static str {
        match self {
            Self::Devices => "Devices",
            Self::Monitoring => "Monitoring",
            Self::Local => "Local shell",
            Self::Storage => "Settings",
        }
    }
    fn icon(self) -> usize {
        match self {
            Self::Devices => 1,
            Self::Monitoring => 7,
            Self::Local => 5,
            Self::Storage => 6,
        }
    }
    fn section(self) -> &'static str {
        match self {
            Self::Devices | Self::Monitoring => "Workspace",
            Self::Local => "Terminal",
            Self::Storage => "Settings",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
enum ViewMode {
    #[default]
    List,
    Grid,
}

struct LabManager {
    store: Option<ServerStore>,
    servers: Vec<Server>,
    history: lab_manager::history::InventoryHistory,
    page: Page,
    server_form: ServerForm,
    right_sidebar: sidebar::RightSidebar,
    account_removal: account_removal::AccountRemoval,
    search: String,
    message: String,
    writable: bool,
    editor: Option<(usize, Server)>,
    editor_ip: String,
    remote: Option<Server>,
    terminal: terminal_panel::TerminalTabs,
    local_terminal: terminal_panel::TerminalPanel,
    command: String,
    remove: Option<usize>,
    navigation_open: bool,
    navigation_hover: bool,
    view: ViewMode,
    table: server_table::TableState,
    pending_reorder: Option<(std::net::IpAddr, std::net::IpAddr, bool)>,
    monitor: Monitor,
    monitoring: lab_manager::monitoring::Monitoring,
    auth: auth_screen::AuthScreen,
    data_root: std::path::PathBuf,
    active_user: Option<String>,
    notifications: notification_center::NotificationCenter,
    discovery: discovery_screen::DiscoveryScreen,
    backup: backup_ui::BackupUi,
    updates: update_panel::UpdatePanel,
}

impl LabManager {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        design::configure(&cc.egui_ctx);
        let store = std::env::current_exe()
            .map_err(|e| e.to_string())
            .and_then(|exe| ServerStore::beside_executable(&exe))
            .ok();
        let data_root = store
            .as_ref()
            .and_then(|s| s.path.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("data"));
        let account_path = store
            .as_ref()
            .map(|s| s.path.with_file_name("account.enc"))
            .unwrap_or_else(|| std::path::PathBuf::from("data/account.enc"));
        let auth = auth_screen::AuthScreen::new(account_path);
        Self {
            store: None,
            servers: Vec::new(),
            history: Default::default(),
            message: String::new(),
            writable: false,
            data_root,
            active_user: None,
            notifications: notification_center::NotificationCenter::default(),
            discovery: discovery_screen::DiscoveryScreen::default(),
            backup: Default::default(),
            updates: Default::default(),
            page: Page::Devices,
            server_form: ServerForm::default(),
            right_sidebar: sidebar::RightSidebar::default(),
            account_removal: Default::default(),
            search: String::new(),
            editor: None,
            editor_ip: String::new(),
            remote: None,
            terminal: Default::default(),
            local_terminal: Default::default(),
            command: "sudo apt update && sudo apt upgrade".into(),
            remove: None,
            navigation_open: false,
            navigation_hover: false,
            view: ViewMode::List,
            table: server_table::TableState::default(),
            pending_reorder: None,
            monitor: Monitor::default(),
            monitoring: Default::default(),
            auth,
        }
    }

    fn load_signed_in_inventory(&mut self) {
        let Some(username) = self.auth.signed_in.clone() else {
            return;
        };
        if self.active_user.as_ref() == Some(&username) {
            return;
        }
        self.account_removal = Default::default();
        self.backup = Default::default();
        self.monitor.reset();
        self.terminal.clear();
        self.local_terminal.clear();
        self.monitoring.reset();
        self.history = Default::default();
        self.pending_reorder = None;
        self.view = ViewMode::List;
        self.table = server_table::TableState::default();
        self.notifications = notification_center::NotificationCenter::default();
        self.discovery.clear();
        self.servers.clear();
        self.store = None;
        self.writable = false;
        self.editor = None;
        self.remote = None;
        self.remove = None;
        self.server_form = ServerForm::default();
        self.right_sidebar.open = false;
        self.search.clear();
        self.message.clear();
        self.active_user = Some(username.clone());
        let result = ServerStore::for_user(&self.data_root, &username).and_then(|store| {
            if !store.path.exists() && self.auth.owns_legacy_inventory(&username)? {
                let legacy = ServerStore {
                    path: self.data_root.join("servers.json"),
                };
                if legacy.path.exists() {
                    store.save(&legacy.load()?)?;
                }
            }
            let servers = store.load()?;
            Ok((store, servers))
        });
        match result {
            Ok((store, servers)) => {
                if let Some(root) = store.path.parent()
                    && let Err(error) = lab_manager::command_history::recover_interrupted(
                        &root.join("commands.json"),
                    )
                {
                    self.notify(format!("Command history could not be recovered: {error}"));
                }
                self.monitoring
                    .load(store.path.with_file_name("monitoring.json"));
                self.notifications
                    .load(store.path.with_file_name("notifications.json"));
                self.store = Some(store);
                self.servers = servers;
                self.writable = true;
                self.notify(format!("Signed in as {username}."));
            }
            Err(error) => {
                self.message = format!("Cannot load your inventory: {error}. Saving is disabled.")
            }
        }
    }

    fn notify(&mut self, message: String) {
        self.notifications.record(message.clone());
        self.message = message;
    }

    fn open_add_panel(&mut self) {
        self.terminal.hide();
        self.monitoring.settings_open = false;
        self.remote = None;
        self.remove = None;
        self.editor = None;
        self.discovery.close();
        self.right_sidebar.open = true;
    }
    fn open_scan_panel(&mut self) {
        self.terminal.hide();
        self.monitoring.settings_open = false;
        self.remote = None;
        self.remove = None;
        self.editor = None;
        self.discovery.open();
        self.right_sidebar.open = true;
    }
    fn open_settings_panel(&mut self, index: usize, server: Server) {
        self.terminal.hide();
        self.monitoring.settings_open = false;
        self.remote = None;
        self.remove = None;
        if self
            .editor
            .as_ref()
            .is_some_and(|(_, current)| current.ip == server.ip)
        {
            self.right_sidebar.open = true;
            return;
        }
        self.discovery.close();
        self.editor_ip = server.ip.to_string();
        self.editor = Some((index, server));
        self.right_sidebar.open = true;
    }

    fn open_command_panel(&mut self, server: Server) {
        self.command = server.suggested_update_command().unwrap_or_default();
        self.terminal.hide();
        self.monitoring.settings_open = false;
        self.editor = None;
        self.remove = None;
        self.discovery.close();
        self.remote = Some(server);
        self.right_sidebar.open = true;
    }
    fn open_remove_panel(&mut self, index: usize) {
        self.terminal.hide();
        self.monitoring.settings_open = false;
        self.editor = None;
        self.remote = None;
        self.discovery.close();
        self.remove = Some(index);
        self.right_sidebar.open = true;
    }

    fn open_ssh_terminal(&mut self, server: Server, command: Option<String>) {
        self.monitoring.settings_open = false;
        self.editor = None;
        self.remote = None;
        self.remove = None;
        self.discovery.close();
        let config = self
            .monitoring
            .database
            .devices
            .get(&lab_manager::monitoring::key(&server))
            .map(|h| h.config.clone());
        self.terminal
            .open_ssh(server, config, self.monitoring.credential_root(), command);
        self.right_sidebar.open = true;
    }
    fn change_page(&mut self, page: Page) {
        if self.page == page {
            return;
        }
        self.page = page;
        self.right_sidebar.open = false;
        self.editor = None;
        self.remote = None;
        self.remove = None;
        self.discovery.close();
        self.monitoring.settings_open = false;
        self.monitoring.password_draft = Default::default();
        self.terminal.hide();
    }

    fn open_local_terminal(&mut self) {
        self.monitoring.settings_open = false;
        self.editor = None;
        self.remote = None;
        self.remove = None;
        self.discovery.close();
        self.terminal.hide();
        self.right_sidebar.open = false;
        if !self.local_terminal.active() {
            self.local_terminal.open_local();
        }
    }

    fn right_panel(&mut self, ctx: &egui::Context) {
        if self.monitoring.settings_open && self.page != Page::Monitoring {
            self.monitoring.settings_open = false;
            self.right_sidebar.open = false;
            self.monitoring.password_draft = Default::default();
        }
        let terminal = self.terminal.active();
        let monitoring_settings = self.monitoring.settings_open;
        if monitoring_settings {
            self.right_sidebar.open = true;
        }

        let settings = self.editor.is_some();
        let scan = self.discovery.open;
        let command = self.remote.is_some();
        let removing = self.remove.is_some();
        let mut panel = std::mem::take(&mut self.right_sidebar);
        panel.content_id = self
            .editor
            .as_ref()
            .map(|(_, s)| format!("settings-{}", s.ip))
            .unwrap_or_else(|| if scan { "scan".into() } else { "add".into() });
        if let Some(server) = &self.remote {
            panel.content_id = format!("command-{}", server.ip);
        }
        if let Some(index) = self.remove {
            panel.content_id = format!("remove-{index}");
        }
        if monitoring_settings {
            panel.content_id = format!("monitoring-{:?}", self.monitoring.selected);
        }
        if terminal {
            panel.content_id = format!("terminal-{}", self.terminal.identity());
        }
        let title = if terminal {
            if self.terminal.command_os().is_some() {
                "Update / command"
            } else {
                "Terminal"
            }
        } else if monitoring_settings {
            "Monitoring settings"
        } else if command {
            "Update / command"
        } else if removing {
            "Remove server"
        } else if settings {
            "Device settings"
        } else if scan {
            "Scan network"
        } else {
            "Add Device"
        };
        let mut save = false;
        let mut draft = None;
        let mut cancel_settings = false;
        panel.show(ctx, title, |ui| {
            if terminal {
                if let Some(server) = self.terminal.show(ui, &self.servers, &self.monitor) {
                    let config = self
                        .monitoring
                        .database
                        .devices
                        .get(&lab_manager::monitoring::key(&server))
                        .map(|history| history.config.clone());
                    self.terminal.open_new_ssh(
                        server,
                        config,
                        self.monitoring.credential_root(),
                        None,
                    );
                }
            } else if monitoring_settings {
                if let Some(server) = self
                    .servers
                    .iter()
                    .find(|s| Some(lab_manager::monitoring::key(s)) == self.monitoring.selected)
                {
                    monitoring_settings::show(ui, &mut self.monitoring, server);
                } else {
                    ui.label("Select a device to configure monitoring.");
                }
            } else if command {
                self.command_panel(ui);
            } else if removing {
                self.remove_panel(ui);
            } else if settings {
                if let Some((_, server)) = &mut self.editor {
                    (save, cancel_settings) =
                        device_settings::show(ui, server, &mut self.editor_ip);
                }
            } else if scan {
                draft = self.discovery.show(ui, &self.servers);
            } else {
                (save, cancel_settings) = self.server_form.show(ui, self.writable);
            }
        });
        if cancel_settings {
            panel.open = false;
        }
        if terminal && !self.terminal.active() {
            panel.open = false;
        }
        if !panel.open {
            self.terminal.hide();
            if terminal {
                ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("embedded_terminal_input")));
            }
            self.monitoring.settings_open = false;
            self.monitoring.password_draft = Default::default();
            self.remote = None;
            self.remove = None;
            self.editor = None;
            if self.discovery.open {
                self.discovery.close();
            }
        }
        if (removing && self.remove.is_none())
            || (command && self.remote.is_none() && !self.terminal.active())
            || (scan && !self.discovery.open)
        {
            panel.open = false;
        }
        self.right_sidebar = panel;
        if save {
            if settings {
                if let Some((index, mut server)) = self.editor.clone() {
                    let Ok(ip) = self.editor_ip.trim().parse() else {
                        self.notify("Enter a valid IPv4 or IPv6 address.".into());
                        return;
                    };
                    if self
                        .servers
                        .iter()
                        .enumerate()
                        .any(|(i, s)| i != index && s.ip == ip)
                    {
                        self.notify("A device with this IP address already exists.".into());
                        return;
                    }
                    server.ip = ip;
                    let mut servers = self.servers.clone();
                    servers[index] = server;
                    if self.persist(servers, "Server settings saved.") {
                        self.editor = None;
                        self.right_sidebar.open = false;
                    }
                }
            } else {
                self.save_new_servers();
            }
        }
        if let Some(form) = draft {
            self.server_form = form;
            self.page = Page::Devices;
            self.open_add_panel();
            self.notify("Review the discovered device settings, then choose Save to add them to your inventory.".into());
        }
    }

    fn persist(&mut self, servers: Vec<Server>, notice: &str) -> bool {
        if !self.writable {
            self.notify("Saving is disabled because the inventory could not be loaded.".into());
            return false;
        }
        match self
            .store
            .as_ref()
            .ok_or_else(|| "Missing inventory directory".to_owned())
            .and_then(|s| s.save(&servers))
        {
            Ok(()) => {
                if self.servers != servers {
                    self.history.record(self.servers.clone());
                }
                self.servers = servers;
                self.notify(notice.into());
                true
            }
            Err(error) => {
                self.notify(format!("Save failed: {error}"));
                false
            }
        }
    }

    fn restore_inventory(&mut self, redo: bool) {
        if !self.writable {
            return;
        }
        let Some(store) = &self.store else {
            return;
        };
        match self
            .history
            .restore(&self.servers, redo, |servers| store.save(servers))
        {
            Ok(Some(servers)) => {
                self.servers = servers;
                self.editor = None;
                self.remote = None;
                self.remove = None;
                self.right_sidebar.open = false;
                self.monitor.reset();
                self.terminal.clear();
                self.notify(
                    if redo {
                        "Inventory change redone."
                    } else {
                        "Inventory change undone."
                    }
                    .into(),
                );
            }
            Ok(None) => {}
            Err(error) => self.notify(format!(
                "Could not {} inventory change: {error}",
                if redo { "redo" } else { "undo" }
            )),
        }
    }

    fn save_new_servers(&mut self) {
        match self.server_form.build(&self.servers) {
            Err(error) => {
                self.server_form.error = error.clone();
                self.notify(format!("Could not add servers: {error}"));
            }
            Ok(new_servers) => {
                let added = new_servers.len();
                let mut servers = self.servers.clone();
                servers.extend(new_servers);
                if self.persist(servers, &format!("Added {added} server(s).")) {
                    self.server_form = ServerForm::default();
                    self.right_sidebar.open = false;
                    self.page = Page::Devices;
                } else {
                    self.server_form.error = self.message.clone();
                }
            }
        }
    }

    fn status_dot(&self, ui: &mut egui::Ui, server: &Server) {
        let status = self.monitor.get(server);
        let (color, label) = match status.connection {
            Connection::Online => (
                Color32::from_rgb(75, 190, 120),
                "Online: a configured browser or SSH port responded.",
            ),
            Connection::Offline => (
                Color32::from_rgb(220, 80, 85),
                "Offline / unreachable: neither configured port responded. A firewall may block the check.",
            ),
            Connection::Checking => (Color32::from_gray(115), "Checking connection..."),
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 4.0, color);
        let checked = status
            .checked
            .map(|t| format!(" Last checked {}s ago.", t.elapsed().as_secs()))
            .unwrap_or_default();
        response.on_hover_text(format!("{label}{checked} Checks refresh every 30 seconds."));
    }

    fn dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new(self.page.title()).size(30.0));
        ui.label(
            RichText::new("Connect, maintain, and manage your infrastructure.")
                .color(Color32::from_gray(150)),
        );
        ui.add_space(20.0);
        let mut grid = self.view == ViewMode::Grid;
        let add_active = self.right_sidebar.open
            && self.editor.is_none()
            && self.remote.is_none()
            && self.remove.is_none()
            && !self.discovery.open
            && !self.terminal.active()
            && !self.monitoring.settings_open;
        let (scan, add, terminals) = action_dock::search_controls(
            ui,
            &mut self.search,
            self.discovery.open,
            add_active,
            &mut grid,
            (self.terminal.count(), self.terminal.active()),
        );
        self.view = if grid { ViewMode::Grid } else { ViewMode::List };
        if scan {
            self.open_scan_panel();
        }
        if add {
            self.open_add_panel();
        }
        if terminals {
            if self.terminal.count() == 0 {
                if let Some(server) = self.servers.first().cloned() {
                    self.open_ssh_terminal(server, None);
                } else {
                    self.open_add_panel();
                }
            } else {
                self.editor = None;
                self.remote = None;
                self.remove = None;
                self.monitoring.settings_open = false;
                self.discovery.close();
                self.terminal.resume();
                self.right_sidebar.open = true;
            }
        }
        ui.add_space(20.0);
        let query = self.search.trim().to_lowercase();
        let matches: Vec<_> = self
            .servers
            .iter()
            .enumerate()
            .filter(|(_, s)| s.matches_query(&query))
            .map(|(i, s)| (i, s.clone()))
            .collect();
        self.server_collection(ui, &matches);
    }

    fn grid_rows(&mut self, ui: &mut egui::Ui, rows: &[&(usize, Server)]) {
        ui.scope(|ui| {
            ui.set_max_width(ui.available_width().min(1488.0));
            let columns = ((ui.available_width() + 16.0) / 330.0)
                .floor()
                .clamp(1.0, 4.0) as usize;
            ui.add_space(8.0);
            for row in rows.chunks(columns) {
                ui.columns(columns, |uis| {
                    for (column, (index, server)) in row.iter().copied().enumerate() {
                        let available = uis[column].available_width().min(360.0);
                        uis[column].scope_builder(
                            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                                uis[column].cursor().min,
                                Vec2::new(available, 0.0),
                            )),
                            |ui| {
                                self.server_card(ui, *index, server);
                            },
                        );
                    }
                });
                ui.add_space(16.0);
            }
        });
    }

    fn device_drag(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: egui::Rect,
        server: &Server,
        grid: bool,
    ) {
        #[cfg(test)]
        ui.ctx()
            .data_mut(|data| data.insert_temp(egui::Id::new(("device_rect", server.ip)), rect));
        if !self.writable {
            return;
        }
        if response.sense.senses_drag() {
            #[cfg(test)]
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    egui::Id::new(("device_drag_handle", server.ip)),
                    response.rect,
                )
            });
            response.dnd_set_drag_payload(server.ip);
        }
        if response.sense.senses_drag() && response.hovered() {
            ui.ctx()
                .set_cursor_icon(if response.is_pointer_button_down_on() {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::Grab
                });
        }
        let Some(source) = egui::DragAndDrop::payload::<std::net::IpAddr>(ui.ctx()) else {
            return;
        };
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        if *source == server.ip {
            server_table::active_outline(ui, rect, if grid { 12.0 } else { 4.0 });
            return;
        }
        let Some(origin) = self.servers.iter().find(|device| device.ip == *source) else {
            return;
        };
        if self.monitor.get(origin).connection != self.monitor.get(server).connection {
            return;
        }
        let Some(pointer) = ui
            .ctx()
            .pointer_interact_pos()
            .filter(|point| rect.contains(*point))
        else {
            return;
        };
        let after = if grid {
            pointer.x >= rect.center().x
        } else {
            pointer.y >= rect.center().y
        };
        let line = if grid {
            let x = if after { rect.right() } else { rect.left() };
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())]
        } else {
            let y = if after { rect.bottom() } else { rect.top() };
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)]
        };
        ui.painter().line_segment(
            line,
            egui::Stroke::new(2.0_f32, Color32::from_rgb(64, 184, 119)),
        );
        if ui.ctx().input(|input| input.pointer.any_released()) {
            self.pending_reorder = Some((*source, server.ip, after));
            egui::DragAndDrop::take_payload::<std::net::IpAddr>(ui.ctx());
        }
    }

    fn apply_reorder(&mut self) {
        let Some((source, target, after)) = self.pending_reorder.take() else {
            return;
        };
        let Some(servers) =
            lab_manager::device_order::reordered(&self.servers, source, target, after)
        else {
            return;
        };
        if self.persist(servers, "Device order saved.") {
            if let Some((index, draft)) = &mut self.editor
                && let Some(new_index) =
                    self.servers.iter().position(|device| device.ip == draft.ip)
            {
                *index = new_index;
            }
            self.remove = None;
        }
    }

    fn server_card(&mut self, ui: &mut egui::Ui, index: usize, server: &Server) {
        ui.push_id(server.ip, |ui| {
            let mut menu_open = false;
            let mut name_response = None;
            let card = ui.scope_builder(egui::UiBuilder::new().sense(egui::Sense::click()), |ui| {
                design::card()
                    .show(ui, |ui| {
                        ui.style_mut().interaction.selectable_labels = false;
                        ui.set_min_width((ui.available_width() - 40.0).max(80.0));
                        ui.horizontal(|ui| {
                            self.status_dot(ui, server);
                            let name_width = (ui.available_width() - 38.0).max(30.0);
                            let response = ui
                                .add_sized(
                                    [name_width, 28.0],
                                    egui::Label::new(
                                        RichText::new(&server.name).strong().size(15.0),
                                    )
                                    .truncate()
                                    .sense(egui::Sense::click_and_drag()),
                                )
                                .on_hover_text(&server.name);
                            if response.clicked() {
                                self.open_settings_panel(index, server.clone());
                            }
                            response.context_menu(|ui| self.device_actions_menu(ui, index, server));
                            menu_open |= response.context_menu_opened();
                            name_response = Some(response);
                            menu_open |= self.device_action_button(ui, index, server);
                        });
                        if !server.group.trim().is_empty() || !server.tags.trim().is_empty() {
                            ui.small(format!(
                                "{}{}{}",
                                server.group,
                                if server.group.is_empty() || server.tags.is_empty() {
                                    ""
                                } else {
                                    " · "
                                },
                                server.tags
                            ));
                        }
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            device_image(ui, server);
                            ui.vertical(|ui| {
                                let ip = ui
                                    .add(
                                        egui::Label::new(
                                            RichText::new(server.ip.to_string()).size(19.0),
                                        )
                                        .truncate()
                                        .sense(egui::Sense::click()),
                                    )
                                    .on_hover_text(format!("Open {}", server.web_url()))
                                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                                server_table::hover_outline(ui, ip.rect.expand(3.0), &ip, 5.0);
                                ip.context_menu(|ui| self.device_actions_menu(ui, index, server));
                                menu_open |= ip.context_menu_opened();
                                if ip.clicked()
                                    && let Err(error) = webbrowser::open(&server.web_url())
                                {
                                    self.notify(format!("Cannot open browser: {error}"));
                                }
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(server.web_url())
                                            .size(11.0)
                                            .color(Color32::from_gray(150)),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(server.web_url());
                                ui.add_space(5.0);
                                ui.horizontal_wrapped(|ui| {
                                    for tag in [
                                        if server.https { "HTTPS" } else { "HTTP" },
                                        "SSH",
                                        server.icon.label(),
                                    ] {
                                        egui::Frame::new()
                                            .fill(Color32::from_gray(43))
                                            .corner_radius(8.0)
                                            .inner_margin(egui::Margin::symmetric(7, 3))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(tag)
                                                        .size(11.0)
                                                        .color(Color32::from_gray(175)),
                                                );
                                            });
                                    }
                                });
                            });
                        });
                        ui.add_space(10.0);
                        ui.horizontal_wrapped(|ui| {
                            for (label, value) in [
                                ("Browser port", server.web_port.to_string()),
                                ("SSH port", server.ssh_port.to_string()),
                                ("Account", server.username.clone()),
                            ] {
                                ui.vertical(|ui| {
                                    ui.set_min_width(70.0);
                                    ui.label(
                                        RichText::new(label)
                                            .size(11.0)
                                            .color(Color32::from_gray(135)),
                                    );
                                    ui.label(value);
                                });
                            }
                        });
                    })
                    .response
                    .rect
            });
            if let Some(response) = &name_response {
                self.device_drag(ui, response, card.inner, server, true);
            }
            self.device_drag(ui, &card.response, card.inner, server, true);
            if card.response.clicked() {
                self.open_settings_panel(index, server.clone());
            }
            card.response
                .context_menu(|ui| self.device_actions_menu(ui, index, server));
            let selected = self.right_sidebar.open
                && self
                    .editor
                    .as_ref()
                    .is_some_and(|(_, device)| device.ip == server.ip);
            if selected || menu_open || card.response.context_menu_opened() {
                server_table::active_outline(ui, card.inner, 12.0);
            }
        });
    }

    fn navigation(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let active_width = if self.navigation_open || self.navigation_hover {
            230.0
        } else {
            60.0
        };
        let pointer_inside = ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| {
            p.x >= screen.left()
                && p.x < screen.left() + active_width
                && p.y >= screen.top()
                && p.y <= screen.bottom()
        });
        let leave_id = egui::Id::new("navigation_leave_time");
        let now = ctx.input(|i| i.time);
        if pointer_inside {
            self.navigation_hover = true;
            ctx.data_mut(|d| d.remove::<f64>(leave_id));
        } else if self.navigation_hover {
            let left_at = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(leave_id, || now));
            if now - left_at >= 0.12 {
                self.navigation_hover = false;
                ctx.data_mut(|d| d.remove::<f64>(leave_id));
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
        }
        let expanded = self.navigation_open || self.navigation_hover;
        let progress = ctx.animate_bool_with_time_and_easing(
            egui::Id::new("navigation_expansion"),
            expanded,
            0.24,
            egui::emath::easing::cubic_out,
        );
        let width = 60.0 + 170.0 * progress;
        egui::SidePanel::left("navigation_panel")
            .exact_width(width)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(design::BLACK))
            .show(ctx, |ui| {
                let visible = ui.max_rect();
                {
                    let rect = egui::Rect::from_min_size(
                        visible.min + Vec2::new(12.0, 16.0),
                        Vec2::new(206.0, (visible.height() - 32.0).max(0.0)),
                    );
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt("expanded_navigation")
                            .max_rect(rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );
                    child.set_clip_rect(visible);

                    let ui = &mut child;
                    if design::sidebar_pin(ui, self.navigation_open, expanded).clicked() {
                        self.navigation_open = !self.navigation_open;
                    }
                    ui.add_space(16.0);
                    ui.scope(|ui| {
                        ui.horizontal(|ui| {
                            design::lab_manager_icon(ui);
                            ui.scope(|ui| {
                                ui.set_opacity(progress);
                                ui.heading(RichText::new("Lab Manager").size(20.0));
                            });
                        });
                        ui.set_opacity(progress);
                        ui.label(
                            RichText::new("Your workspace")
                                .size(13.0)
                                .color(design::MUTED),
                        );
                    });
                    ui.add_space(12.0);
                    egui::ScrollArea::vertical()
                        .max_height((ui.available_height() - 85.0).max(100.0))
                        .show(ui, |ui| {
                            for (name, pages) in [
                                (
                                    "WORKSPACE",
                                    vec![Page::Devices, Page::Monitoring, Page::Local],
                                ),
                                ("PREFERENCES", vec![Page::Storage]),
                            ] {
                                let labels: Vec<_> = pages.iter().map(|p| p.title()).collect();
                                if let Some(i) = design::nav_group(
                                    ui,
                                    name,
                                    &labels,
                                    &pages.iter().map(|p| p.icon()).collect::<Vec<_>>(),
                                    pages.iter().position(|p| *p == self.page),
                                    progress,
                                ) {
                                    self.change_page(pages[i]);
                                }
                            }
                        });
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                        ui.set_opacity(progress);
                        ui.label(
                            RichText::new("Stored on this computer")
                                .size(11.0)
                                .color(design::MUTED),
                        );
                        ui.label(
                            RichText::new(format!("{} saved servers", self.servers.len()))
                                .size(12.0),
                        );
                    });
                }
            });
    }

    fn toolbar(&mut self, ctx: &egui::Context) {
        let mut profile_action = profile_ui::ProfileAction::None;
        egui::TopBottomPanel::top("browser_toolbar")
            .exact_height(lab_manager::frosted_modal::TITLE_BAR_HEIGHT)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(design::WORKSPACE)
                    .corner_radius(egui::CornerRadius {
                        nw: 22,
                        ne: 0,
                        sw: 0,
                        se: 0,
                    })
                    .inner_margin(egui::Margin::symmetric(20, 10)),
            )
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                // Paint only: no label widget or hit target can cover the drag surface.
                ui.painter().text(
                    egui::pos2(ctx.screen_rect().center().x, rect.center().y),
                    egui::Align2::CENTER_CENTER,
                    format!("{} / {}", self.page.section(), self.page.title()),
                    egui::FontId::proportional(12.0),
                    Color32::from_gray(175),
                );
                let (_, drag) = ui.allocate_exact_size(rect.size(), egui::Sense::click_and_drag());
                let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                if drag.double_clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                } else if drag.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                let signed_in = self.auth.signed_in.is_some();
                let profile_width = 32.0;
                let dock_width = if signed_in {
                    276.0 + profile_width + 4.0
                } else {
                    120.0
                };
                let screen = ctx.screen_rect();
                let dock_rect = egui::Rect::from_min_size(
                    egui::pos2(screen.right() - dock_width, screen.top()),
                    Vec2::new(dock_width, 44.0),
                );
                action_dock::paint_top_toolbar(ui, dock_rect);
                let mut controls = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("toolbar_controls")
                        .max_rect(dock_rect.shrink2(Vec2::new(8.0, 6.0)))
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                );
                controls.set_clip_rect(ui.clip_rect().union(dock_rect).intersect(screen));
                controls.spacing_mut().item_spacing.x = 4.0;
                if design::window_control(&mut controls, 2, maximized).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if design::window_control(&mut controls, 1, maximized).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                if design::window_control(&mut controls, 0, maximized).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                if self.auth.signed_in.is_some() {
                    action_dock::separator(&mut controls);
                    if let Some(username) = &self.auth.signed_in {
                        profile_action = profile_ui::menu(
                            &mut controls,
                            username,
                            &self.auth.profile,
                            profile_width,
                        );
                    }
                    help_menu::show(&mut controls);
                    if design::history_icon(
                        &mut controls,
                        true,
                        self.writable && self.history.can_redo(),
                    )
                    .clicked()
                    {
                        self.restore_inventory(true);
                    }
                    if design::history_icon(
                        &mut controls,
                        false,
                        self.writable && self.history.can_undo(),
                    )
                    .clicked()
                    {
                        self.restore_inventory(false);
                    }
                    action_dock::separator(&mut controls);
                    self.notifications.show(&mut controls);
                }
            });
        match profile_action {
            profile_ui::ProfileAction::None => {}
            profile_ui::ProfileAction::Logout => self.sign_out(ctx),
            profile_ui::ProfileAction::RemoveAccount => self.account_removal.open(ctx),
        }
    }

    fn sign_out(&mut self, ctx: &egui::Context) {
        match self.auth.sign_out() {
            Ok(()) => {
                self.notify("Signed out.".into());
                self.clear_workspace(ctx);
            }
            Err(error) => self.notify(format!("Could not revoke automatic sign-in: {error}")),
        }
        ctx.request_repaint();
    }

    fn clear_workspace(&mut self, ctx: &egui::Context) {
        lab_manager::desktop_alerts::clear();
        self.account_removal = Default::default();
        self.backup = Default::default();
        self.monitor.reset();
        self.terminal.clear();
        self.local_terminal.clear();
        ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("embedded_terminal_input")));
        self.monitoring.reset();
        self.history = Default::default();
        self.pending_reorder = None;
        self.view = ViewMode::List;
        self.table = server_table::TableState::default();
        self.notifications = notification_center::NotificationCenter::default();
        ctx.memory_mut(|memory| memory.close_popup());
        self.discovery.clear();
        self.servers.clear();
        self.store = None;
        self.active_user = None;
        self.writable = false;
        self.search.clear();
        self.editor = None;
        self.remote = None;
        self.remove = None;
        self.right_sidebar.open = false;
        self.server_form = ServerForm::default();
        self.message.clear();
        self.page = Page::Devices;
    }

    fn content(&mut self, ui: &mut egui::Ui) {
        match self.page {
            Page::Devices => self.dashboard(ui),
            Page::Monitoring => {
                let was_open = self.monitoring.settings_open;
                let shell = monitoring_page::show(ui, &mut self.monitoring, &self.servers);
                if !was_open && self.monitoring.settings_open {
                    self.terminal.hide();
                    self.editor = None;
                    self.remote = None;
                    self.remove = None;
                    self.discovery.close();
                    self.right_sidebar.open = true;
                }
                if was_open && !self.monitoring.settings_open && self.monitoring.paused {
                    self.right_sidebar.open = false;
                }
                if let Some(server) = shell {
                    self.open_ssh_terminal(server, None);
                }
            }
            Page::Local => {
                ui.heading(RichText::new("Local shell").size(28.0));
                ui.label("Run local commands in the embedded PowerShell terminal.");
                ui.add_space(20.0);
                self.open_local_terminal();
                self.local_terminal.show(ui);
            }
            Page::Storage => {
                ui.heading(RichText::new("Settings").size(28.0));
                ui.label("Your servers and connection settings are saved automatically.");
                ui.add_space(22.0);
                design::card().show(ui, |ui| self.updates.show(ui, &self.data_root));
                ui.add_space(16.0);
                if let Some(root) = self.store.as_ref().and_then(|store| store.path.parent()) {
                    design::card().show(ui, |ui| {
                        self.backup.controls(
                            ui,
                            root,
                            &self.servers,
                            &self.monitoring.database,
                            (
                                self.auth.signed_in.as_deref().unwrap_or(""),
                                self.auth.profile.avatar,
                            ),
                        )
                    });
                    ui.add_space(16.0);
                }
                design::card().show(ui, |ui| {
                    ui.strong("Inventory file");
                    if let Some(store) = &self.store {
                        ui.label(store.path.display().to_string());
                    }
                    ui.label(format!("{} saved servers", self.servers.len()));
                    ui.label(
                        "Passwords and keys are managed by OpenSSH, never stored in this file.",
                    );
                });
            }
        }
    }

    fn command_panel(&mut self, ui: &mut egui::Ui) {
        let Some(server) = self.remote.clone() else {
            return;
        };
        let endpoint = lab_manager::command_history::endpoint(&server);
        ui.strong(&server.name);
        ui.label(server.ip.to_string());
        ui.label("Review the command before running. Updates change the remote server.");
        lab_manager::settings_form::os_badge(ui, server.effective_os());
        if server.effective_os() == lab_manager::model::DeviceOs::Idrac {
            ui.small("iDRAC firmware updates require a specific package and reviewed RACADM command. The default command only reads system information.");
        } else if server.effective_os().update_command().is_none() {
            ui.small(
                "Enter a command appropriate for this OS. Change OS in Device settings if needed.",
            );
        }
        ui.add(
            egui::TextEdit::multiline(&mut self.command)
                .code_editor()
                .desired_rows(6)
                .desired_width(f32::INFINITY),
        );
        ui.label("Runs through the remote account's shell. Enter passwords and answer prompts in the terminal. Disconnect or close its tab to end the client; hiding the sidebar keeps tabs connected.");
        if ui
            .add_enabled(
                !self.command.trim().is_empty(),
                egui::Button::new("Run command in SSH terminal"),
            )
            .clicked()
        {
            self.open_ssh_terminal(server, Some(self.command.clone()));
        }
        egui::CollapsingHeader::new("Command history").show(ui, |ui| {
            ui.small("Reviewed commands only · custom text, terminal input and output are not recorded. Closing a client cannot confirm remote completion.");
            if let Some(root) = self.store.as_ref().and_then(|store| store.path.parent()) {
                match lab_manager::command_history::load(&root.join("commands.json")) {
                    Ok(entries) => {
                        let entries: Vec<_> = entries.iter().filter(|entry| entry.endpoint == endpoint).rev().take(30).collect();
                        if entries.is_empty() { ui.small("No reviewed commands recorded for this device."); }
                        for entry in entries { ui.label(&entry.command); ui.small(format!("{} · {}", lab_manager::notifications::age(entry.timestamp), entry.result)); ui.separator(); }
                    }
                    Err(error) => { ui.label(format!("Cannot read history: {error}")); }
                }
            }
        });
    }
    fn remove_panel(&mut self, ui: &mut egui::Ui) {
        let Some(index) = self.remove else {
            return;
        };
        let Some(server) = self.servers.get(index) else {
            self.remove = None;
            return;
        };
        ui.strong(&server.name);
        ui.label(server.ip.to_string());
        ui.add_space(10.0);
        ui.label("Remove this server from your inventory?");
        ui.label("This only removes the saved entry.");
        ui.add_space(12.0);
        if server_table::remove_button(ui).clicked() {
            let mut servers = self.servers.clone();
            servers.remove(index);
            if self.persist(servers, "Server removed.") {
                self.remove = None;
                self.editor = None;
            }
        }
        if ui.button("Cancel").clicked() {
            self.remove = None;
        }
    }
}

impl eframe::App for LabManager {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        design::BLACK.to_normalized_gamma_f32()
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.updates.tick(ctx, &self.data_root);
        if self.auth.signed_in.is_none() {
            self.toolbar(ctx);
            egui::TopBottomPanel::bottom("signed_out_footer")
                .show_separator_line(false)
                .frame(egui::Frame::new().fill(design::FOOTER).inner_margin(12.0))
                .show(ctx, |ui| {
                    ui.small("Local account · encrypted on this computer");
                });
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(design::WORKSPACE)
                        .inner_margin(24.0),
                )
                .show(ctx, |ui| self.auth.show(ui));
            design::resize_window(ctx);
            return;
        }
        self.load_signed_in_inventory();
        lab_manager::icon_assets::set_root(
            ctx,
            self.store
                .as_ref()
                .and_then(|store| store.path.parent())
                .map(|root| root.join("icons")),
        );
        let now = lab_manager::notifications::now();
        let due: Vec<_> = self
            .servers
            .iter()
            .filter(|s| s.reminder_at.is_some_and(|at| at <= now))
            .map(|s| s.name.clone())
            .collect();
        if !due.is_empty() && self.writable {
            let mut servers = self.servers.clone();
            for server in &mut servers {
                if server.reminder_at.is_some_and(|at| at <= now) {
                    server.reminder_at = server.next_reminder(now);
                }
            }
            if self
                .store
                .as_ref()
                .is_some_and(|store| store.save(&servers).is_ok())
            {
                self.servers = servers;
                for name in due {
                    self.notify(format!("Maintenance reminder: {name}"));
                }
            }
        }
        self.terminal.poll();
        self.monitor.update(&self.servers);
        let offline = self
            .servers
            .iter()
            .filter(|s| {
                let connection = self.monitor.get(s).connection;
                connection == Connection::Offline
                    || (connection == Connection::Checking
                        && self
                            .monitoring
                            .database
                            .devices
                            .get(&lab_manager::monitoring::key(s))
                            .and_then(|h| h.samples.last())
                            .is_some_and(|sample| !sample.online()))
            })
            .map(lab_manager::monitoring::key)
            .collect();
        self.monitoring.set_offline(offline);
        for notice in self.monitoring.update(&self.servers) {
            self.notify(notice);
        }
        for notice in std::mem::take(&mut self.monitoring.desktop_notices) {
            lab_manager::desktop_alerts::send(&notice);
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
        if !ctx.wants_keyboard_input() && !terminal_panel::TerminalPanel::focused(ctx) {
            let redo = ctx.input_mut(|input| {
                input.consume_key(egui::Modifiers::CTRL, egui::Key::Y)
                    || input
                        .consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)
            });
            let undo =
                ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Z));
            if redo {
                self.restore_inventory(true);
            } else if undo {
                self.restore_inventory(false);
            }
            ctx.input_mut(|input| {
                for (key, page) in [
                    (egui::Key::Num1, Page::Devices),
                    (egui::Key::Num2, Page::Local),
                    (egui::Key::Num3, Page::Storage),
                ] {
                    if input.consume_key(egui::Modifiers::CTRL, key) {
                        self.change_page(page);
                    }
                }
                if input.consume_key(egui::Modifiers::CTRL, egui::Key::B) {
                    self.navigation_open = !self.navigation_open;
                }
            });
        }
        self.navigation(ctx);
        self.toolbar(ctx);
        if self.auth.signed_in.is_none() {
            ctx.request_repaint();
            return;
        }
        self.right_panel(ctx);
        egui::TopBottomPanel::bottom("status")
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(design::FOOTER)
                    .inner_margin(egui::Margin::symmetric(20, 10)),
            )
            .show(ctx, |ui| {
                if !self.writable {
                    ui.colored_label(Color32::from_rgb(225, 150, 140), &self.message);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.small(format!(
                        "{} saved servers / {} HTTPS endpoints",
                        self.servers.len(),
                        self.servers.iter().filter(|s| s.https).count()
                    ));
                    if self.page == Page::Monitoring {
                        ui.separator();
                        if self.monitoring.busy() {
                            ui.spinner();
                        }
                        ui.small("Automatic checks while Lab Manager is open");
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(design::WORKSPACE)
                    .inner_margin(28.0),
            )
            .show(ctx, |ui| {
                ui.add_space(12.0);
                let dashboard = matches!(self.page, Page::Devices | Page::Monitoring | Page::Local);
                let width = if dashboard {
                    ui.available_width()
                } else {
                    ui.available_width().min(820.0)
                };
                let inset = 0.0;
                let rect = egui::Rect::from_min_size(
                    ui.next_widget_position() + Vec2::new(inset, 0.0),
                    Vec2::new(width, ui.available_height()),
                );
                let mut column = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("workspace_column")
                        .max_rect(rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                );
                self.content(&mut column);
            });

        if let Some(username) = self.auth.signed_in.clone()
            && self
                .account_removal
                .show(ctx, &username, &self.auth.profile)
        {
            match self.auth.remove_account() {
                Ok(()) => {
                    self.clear_workspace(ctx);
                    ctx.request_repaint();
                    return;
                }
                Err(error) => self.account_removal.error = error,
            }
        }
        if let Some((backup, replace)) = self.backup.show(ctx)
            && let Some(root) = self
                .store
                .as_ref()
                .and_then(|store| store.path.parent())
                .map(|path| path.to_owned())
        {
            let result = lab_manager::backup::restore(
                &root,
                &backup,
                &self.servers,
                &self.monitoring.database,
                replace,
            );
            match result {
                Ok((servers, database)) => {
                    self.terminal.clear();
                    self.local_terminal.clear();
                    self.monitor.reset();
                    self.monitoring.load(root.join("monitoring.json"));
                    self.monitoring.database = database;
                    self.servers = servers;
                    self.history = Default::default();
                    self.table = Default::default();
                    self.editor = None;
                    self.remote = None;
                    self.remove = None;
                    self.discovery.close();
                    self.right_sidebar.open = false;
                    self.backup.result(Ok(()));
                    self.notify("Backup restored to your account.".into());
                }
                Err(error) => self.backup.result(Err(error)),
            }
        }
        self.apply_reorder();
        for notice in std::mem::take(&mut self.discovery.notifications) {
            self.notify(notice);
        }
        design::resize_window(ctx);
    }
}

fn device_image(ui: &mut egui::Ui, server: &Server) {
    device_thumbnail(ui, server, 90.0);
}
fn device_thumbnail(ui: &mut egui::Ui, server: &Server, size: f32) {
    lab_manager::device_icons::thumbnail(ui, server.icon, server.custom_icon.as_deref(), size);
}

fn main() -> eframe::Result {
    if let Some(code) = lab_manager::monitoring_credentials::askpass_mode() {
        std::process::exit(code);
    }
    eframe::run_native(
        "Lab Manager",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_decorations(false)
                .with_resizable(true)
                .with_inner_size([1100.0, 760.0])
                .with_min_inner_size([760.0, 540.0]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(LabManager::new(cc)))),
    )
}
