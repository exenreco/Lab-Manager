use super::{LabManager, Server, ViewMode, device_thumbnail};
use eframe::egui::{self, Color32, RichText, Vec2};
use lab_manager::{
    status::Connection,
    table_layout::{Columns, page_range},
};

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Filter {
    #[default]
    All,
    Online,
    Offline,
}
pub struct TableState {
    filter: Filter,
    collapsed: [bool; 3],
    page: usize,
    page_size: usize,
    query: String,
    group: String,
    tag: String,
    by_group: bool,
    group_collapsed: std::collections::BTreeSet<String>,
}
impl Default for TableState {
    fn default() -> Self {
        Self {
            filter: Filter::All,
            collapsed: [false; 3],
            page: 0,
            page_size: 10,
            query: String::new(),
            group: String::new(),
            tag: String::new(),
            by_group: false,
            group_collapsed: Default::default(),
        }
    }
}

fn cell(ui: &mut egui::Ui, x: &mut f32, top: f32, width: f32, height: f32, id: &str) -> egui::Ui {
    let rect = egui::Rect::from_min_size(egui::pos2(*x, top), Vec2::new(width, height));
    *x += width + 8.0;
    let mut child = ui.new_child(egui::UiBuilder::new().id_salt(id).max_rect(rect).layout(
        egui::Layout::left_to_right(egui::Align::Center).with_main_align(
            if matches!(
                id,
                "Protocol" | "SSH account" | "Actions" | "protocol" | "account" | "actions"
            ) {
                egui::Align::Center
            } else {
                egui::Align::Min
            },
        ),
    ));
    child.set_clip_rect(ui.clip_rect().intersect(rect));
    child
}

fn centered_label(ui: &mut egui::Ui, label: &str, width: f32) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), egui::Sense::hover());
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(11.0),
        Color32::from_gray(180),
    );
    response.on_hover_text(label);
}

impl LabManager {
    pub(super) fn server_collection(&mut self, ui: &mut egui::Ui, servers: &[(usize, Server)]) {
        let grid = self.view == ViewMode::Grid;
        if self.table.query != self.search {
            self.table.query = self.search.clone();
            self.table.page = 0;
        }
        egui::Frame::new()
            .fill(Color32::from_gray(25))
            .corner_radius(12.0)
            .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(43)))
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.set_min_width(ui.available_width());
                self.collection_filters(ui);
                let filtered: Vec<_> = servers
                    .iter()
                    .filter(|(_, s)| match self.table.filter {
                        Filter::All => true,
                        Filter::Online => self.monitor.get(s).connection == Connection::Online,
                        Filter::Offline => self.monitor.get(s).connection == Connection::Offline,
                    })
                    .filter(|(_, s)| {
                        (self.table.group.is_empty() || s.group.trim() == self.table.group)
                            && (self.table.tag.is_empty()
                                || s.tag_names().any(|tag| tag == self.table.tag))
                    })
                    .collect();
                let total = filtered.len();
                let range = page_range(total, self.table.page_size, &mut self.table.page);
                let scroll = ui.spacing().scroll;
                let reserve = scroll.bar_width
                    + scroll.bar_inner_margin
                    + scroll.bar_outer_margin
                    + scroll.floating_allocated_width;
                let table_width = (ui.available_width() - reserve).max(160.0);
                let columns = Columns::for_width(table_width);
                ui.add_space(6.0);
                if !grid {
                    self.table_header(ui, &columns);
                }
                let footer_reserve = if ui.available_width() < 620.0 {
                    95.0
                } else {
                    65.0
                };
                let max_height = (ui.available_height() - footer_reserve).max(65.0);
                egui::ScrollArea::vertical()
                    .id_salt(("device_collection_body", grid))
                    .max_height(max_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.set_width(table_width);
                        if total == 0 {
                            ui.add_space(14.0);
                            if self.servers.is_empty() {
                                ui.heading("Add your first server");
                                ui.label(
                                    "Keep local and remote servers together in your workspace.",
                                );
                                if ui.button("+ Add servers").clicked() {
                                    self.open_add_panel();
                                }
                            } else {
                                ui.label("No servers match this search or connection status.");
                            }
                            ui.add_space(14.0);
                        }
                        let groups: Vec<(String, Option<Connection>)> = if self.table.by_group {
                            filtered
                                .iter()
                                .map(|(_, s)| s.group.trim().to_owned())
                                .collect::<std::collections::BTreeSet<_>>()
                                .into_iter()
                                .map(|label| (label, None))
                                .collect()
                        } else {
                            vec![
                                ("Online".into(), Some(Connection::Online)),
                                ("Offline / unreachable".into(), Some(Connection::Offline)),
                                ("Checking".into(), Some(Connection::Checking)),
                            ]
                        };
                        for (group, (label, status)) in groups.into_iter().enumerate() {
                            let in_group = |s: &Server| {
                                status.map_or_else(
                                    || s.group.trim() == label,
                                    |status| self.monitor.get(s).connection == status,
                                )
                            };
                            let rows: Vec<_> = filtered[range.clone()]
                                .iter()
                                .copied()
                                .filter(|(_, s)| in_group(s))
                                .collect();
                            if rows.is_empty() {
                                continue;
                            }
                            let group_total = filtered.iter().filter(|(_, s)| in_group(s)).count();
                            let count = if rows.len() == group_total {
                                rows.len().to_string()
                            } else {
                                format!("{} / {}", rows.len(), group_total)
                            };
                            if self.device_group_header(
                                ui,
                                group,
                                if label.is_empty() {
                                    "Ungrouped"
                                } else {
                                    &label
                                },
                                &count,
                            ) {
                                if grid {
                                    self.grid_rows(ui, &rows);
                                } else {
                                    for (index, server) in rows {
                                        ui.push_id(server.ip, |ui| {
                                            self.table_row(ui, *index, server, &columns)
                                        });
                                    }
                                }
                            }
                            ui.add_space(7.0);
                        }
                    });
                ui.separator();
                self.table_pagination(ui, total);
            });
    }

    fn collection_filters(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_width();
        let inline = available >= 640.0;
        let left_height = if available < 280.0 { 64.0 } else { 32.0 };
        let (row, _) = ui.allocate_exact_size(
            Vec2::new(available, if inline { 32.0 } else { left_height + 40.0 }),
            egui::Sense::hover(),
        );
        let left_rect = egui::Rect::from_min_size(
            row.min,
            Vec2::new(if inline { 280.0 } else { available }, left_height),
        );
        let right_width = available.min(332.0);
        let right_rect = egui::Rect::from_min_size(
            egui::pos2(
                row.right() - right_width,
                if inline {
                    row.top()
                } else {
                    row.top() + left_height + 8.0
                },
            ),
            Vec2::new(right_width, 32.0),
        );
        let mut left = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("connection_filters")
                .max_rect(left_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(!inline)),
        );
        left.spacing_mut().interact_size.y = 32.0;
        for (filter, label) in [
            (Filter::All, "All devices"),
            (Filter::Online, "Online"),
            (Filter::Offline, "Offline"),
        ] {
            let response = left.selectable_value(&mut self.table.filter, filter, label);
            #[cfg(test)]
            ui.ctx().data_mut(|data| {
                data.insert_temp(egui::Id::new(("status_filter", label)), response.rect)
            });
            if response.changed() {
                self.table.page = 0;
            }
        }
        let before = (
            self.table.group.clone(),
            self.table.tag.clone(),
            self.table.by_group,
        );
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("metadata_filters")
                .max_rect(right_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        right.spacing_mut().item_spacing.x = 8.0;
        right.spacing_mut().interact_size.y = 32.0;
        let field_width = ((right_width - 16.0) / 3.0).max(36.0);
        for (label, selected, options) in [
            (
                "Group",
                &mut self.table.group,
                self.servers
                    .iter()
                    .map(|s| s.group.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect::<std::collections::BTreeSet<_>>(),
            ),
            (
                "Tag",
                &mut self.table.tag,
                self.servers
                    .iter()
                    .flat_map(|s| s.tag_names().map(str::to_owned))
                    .collect::<std::collections::BTreeSet<_>>(),
            ),
        ] {
            egui::ComboBox::from_id_salt(label)
                .width(field_width)
                .wrap_mode(egui::TextWrapMode::Truncate)
                .selected_text(if selected.is_empty() {
                    format!("All {}s", label.to_lowercase())
                } else {
                    selected.clone()
                })
                .show_ui(&mut right, |ui| {
                    ui.selectable_value(selected, String::new(), "All");
                    for option in options {
                        ui.selectable_value(selected, option.clone(), &option);
                    }
                });
        }
        egui::ComboBox::from_id_salt("group_by")
            .width(field_width)
            .wrap_mode(egui::TextWrapMode::Truncate)
            .selected_text(if self.table.by_group {
                "By group"
            } else {
                "By status"
            })
            .show_ui(&mut right, |ui| {
                ui.selectable_value(&mut self.table.by_group, false, "By status");
                ui.selectable_value(&mut self.table.by_group, true, "By group");
            });
        if before
            != (
                self.table.group.clone(),
                self.table.tag.clone(),
                self.table.by_group,
            )
        {
            self.table.page = 0;
        }
        #[cfg(test)]
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::new("device_status_filters_rect"), left_rect);
            data.insert_temp(egui::Id::new("device_metadata_filters_rect"), right_rect);
        });
    }
    pub(super) fn device_group_header(
        &mut self,
        ui: &mut egui::Ui,
        group: usize,
        label: &str,
        count: &str,
    ) -> bool {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
        });
        ui.painter().rect_filled(
            rect,
            5.0,
            Color32::from_gray(if response.hovered() { 39 } else { 34 }),
        );
        let text = ui.painter().text(
            rect.left_center() + Vec2::new(10.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(12.0),
            Color32::from_gray(215),
        );
        ui.painter().text(
            egui::pos2(text.right() + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            count,
            egui::FontId::proportional(11.0),
            Color32::from_gray(120),
        );
        let c = rect.right_center() - Vec2::new(14.0, 0.0);
        let collapsed = if self.table.by_group {
            self.table.group_collapsed.contains(label)
        } else {
            self.table.collapsed[group]
        };
        lab_manager::svg_icons::paint(
            ui.painter(),
            egui::Rect::from_center_size(c, Vec2::splat(14.0)),
            if collapsed {
                "chevron-right"
            } else {
                "chevron-down"
            },
            Some(Color32::from_gray(140)),
        );
        if response.clicked() {
            if self.table.by_group {
                if collapsed {
                    self.table.group_collapsed.remove(label);
                } else {
                    self.table.group_collapsed.insert(label.into());
                }
            } else {
                self.table.collapsed[group] = !collapsed;
            }
        }
        let open = if response.clicked() {
            collapsed
        } else {
            !collapsed
        };
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
        open
    }

    fn table_pagination(&mut self, ui: &mut egui::Ui, total: usize) {
        page_range(total, self.table.page_size, &mut self.table.page);
        let width = ui.available_width();
        let one_row = width >= 620.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 32.0), egui::Sense::hover());
        let left_width = if one_row { 250.0 } else { width };
        let mut left = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("pagination_left")
                .max_rect(egui::Rect::from_min_size(
                    rect.min,
                    Vec2::new(left_width, 32.0),
                ))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        self.page_size_selector(&mut left, total);
        let right_rect = if one_row {
            egui::Rect::from_min_max(rect.min + Vec2::new(left_width + 12.0, 0.0), rect.max)
        } else {
            ui.allocate_exact_size(Vec2::new(width, 32.0), egui::Sense::hover())
                .0
        };
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("pagination_right")
                .max_rect(right_rect)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        self.page_navigation(&mut right, total, width < 500.0);
    }

    fn page_navigation(&mut self, ui: &mut egui::Ui, total: usize, compact: bool) {
        let pages = total.div_ceil(self.table.page_size).max(1);
        if crate::design::page_arrow(ui, true, self.table.page + 1 < pages, compact).clicked() {
            self.table.page += 1;
        }
        let count = if compact { 3 } else { 5 };
        let start = self
            .table
            .page
            .saturating_sub(count / 2)
            .min(pages.saturating_sub(count));
        for page in (start..(start + count).min(pages)).rev() {
            if ui
                .selectable_label(self.table.page == page, (page + 1).to_string())
                .clicked()
            {
                self.table.page = page;
            }
        }
        if crate::design::page_arrow(ui, false, self.table.page > 0, compact).clicked() {
            self.table.page -= 1;
        }
    }

    fn page_size_selector(&mut self, ui: &mut egui::Ui, total: usize) {
        let old_size = self.table.page_size;
        let range = page_range(total, self.table.page_size, &mut self.table.page);
        let compact = ui.available_width() < 220.0;
        ui.spacing_mut().item_spacing.x = 6.0;
        centered_label(ui, "Rows per page", if compact { 68.0 } else { 80.0 });
        egui::ComboBox::from_id_salt("table_page_size")
            .width(if compact { 45.0 } else { 55.0 })
            .selected_text(self.table.page_size.to_string())
            .show_ui(ui, |ui| {
                for size in [5, 10, 25, 50, 100] {
                    ui.selectable_value(&mut self.table.page_size, size, size.to_string());
                }
            });
        centered_label(
            ui,
            &format!(
                "{}–{} of {}",
                if total == 0 { 0 } else { range.start + 1 },
                range.end,
                total
            ),
            if compact { 50.0 } else { 68.0 },
        );
        if self.table.page_size != old_size {
            self.table.page = 0;
        }
    }

    fn table_header(&self, ui: &mut egui::Ui, columns: &Columns) {
        let (row, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), egui::Sense::hover());
        let mut x = row.left() + 10.0;
        for (label, width) in [
            ("Icon", Some(columns.icon)),
            ("Status", Some(columns.status)),
            ("Server", Some(columns.name)),
            ("IP address", columns.ip),
            ("Ports", columns.ports),
            ("Protocol", columns.protocol),
            ("SSH account", columns.account),
            ("Actions", Some(columns.actions)),
        ] {
            if let Some(width) = width {
                let mut part = cell(ui, &mut x, row.top(), width, row.height(), label);
                let label_widget = egui::Label::new(
                    RichText::new(label)
                        .size(10.0)
                        .color(Color32::from_gray(115)),
                )
                .truncate();
                if matches!(label, "Protocol" | "SSH account" | "Actions") {
                    part.add_sized(Vec2::new(width, row.height()), label_widget);
                } else {
                    part.add(label_widget);
                }
            }
        }
    }

    fn table_row(&mut self, ui: &mut egui::Ui, index: usize, server: &Server, columns: &Columns) {
        let height = if columns.ip.is_none() { 48.0 } else { 36.0 };
        let (row, row_response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), height),
            egui::Sense::click(),
        );
        if row_response.clicked() {
            self.open_settings_panel(index, server.clone());
        }
        row_response.context_menu(|ui| self.device_actions_menu(ui, index, server));
        let mut menu_open = row_response.context_menu_opened();
        let selected = self.editor.as_ref().is_some_and(|(_, s)| s.ip == server.ip);
        if selected {
            ui.painter().rect_filled(row, 4.0, Color32::from_gray(35));
        }
        if ui
            .ctx()
            .pointer_hover_pos()
            .is_some_and(|p| row.contains(p))
        {
            ui.painter().rect_filled(row, 4.0, Color32::from_gray(30));
        }
        let mut x = row.left() + 10.0;
        let mut icon = cell(ui, &mut x, row.top(), columns.icon, row.height(), "icon");
        device_thumbnail(&mut icon, server, columns.icon.min(26.0));
        icon.interact(
            icon.min_rect(),
            icon.id().with("tooltip"),
            egui::Sense::hover(),
        )
        .on_hover_text(server.icon.label());
        let mut status = cell(
            ui,
            &mut x,
            row.top(),
            columns.status,
            row.height(),
            "status",
        );
        self.status_dot(&mut status, server);
        let mut name = cell(ui, &mut x, row.top(), columns.name, row.height(), "name");
        let response = if columns.ip.is_none() {
            name.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let response = ui.add(
                    egui::Label::new(RichText::new(&server.name).size(13.0))
                        .truncate()
                        .sense(egui::Sense::click_and_drag()),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(server.ip.to_string())
                            .monospace()
                            .size(10.0)
                            .color(Color32::from_gray(140)),
                    )
                    .truncate(),
                );
                response
            })
            .inner
        } else {
            name.add(
                egui::Label::new(RichText::new(&server.name).size(13.0))
                    .truncate()
                    .sense(egui::Sense::click_and_drag()),
            )
        }
        .on_hover_text(format!(
            "{}\n{} · {}\nBrowser {} · SSH {}\nSSH account: {}\nClick to open",
            server.name,
            server.ip,
            server.icon.label(),
            server.web_port,
            server.ssh_port,
            server.username
        ));
        self.device_drag(ui, &response, row, server, false);
        response.context_menu(|ui| self.device_actions_menu(ui, index, server));
        menu_open |= response.context_menu_opened();
        if response.clicked() {
            self.open_settings_panel(index, server.clone());
        }
        if let Some(width) = columns.ip {
            let mut ip = cell(ui, &mut x, row.top(), width, row.height(), "ip");
            let ip_frame = egui::Frame::new()
                .fill(Color32::from_gray(31))
                .corner_radius(7.0)
                .inner_margin(egui::Margin::symmetric(7, 3))
                .show(&mut ip, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(server.ip.to_string())
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_gray(165)),
                        )
                        .truncate()
                        .sense(egui::Sense::click()),
                    )
                });
            let response = ip_frame
                .inner
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(format!("Open {}", server.web_url()));
            hover_outline(&ip, ip_frame.response.rect, &response, 7.0);
            response.context_menu(|ui| self.device_actions_menu(ui, index, server));
            menu_open |= response.context_menu_opened();
            if response.clicked()
                && let Err(error) = webbrowser::open(&server.web_url())
            {
                self.notify(format!("Cannot open browser: {error}"));
            }
        }
        if let Some(width) = columns.ports {
            let mut part = cell(ui, &mut x, row.top(), width, row.height(), "ports");
            part.add(
                egui::Label::new(
                    RichText::new(format!("{} / {}", server.web_port, server.ssh_port))
                        .size(11.0)
                        .color(Color32::from_gray(140)),
                )
                .truncate(),
            )
            .on_hover_text(format!(
                "Browser {} · SSH {}",
                server.web_port, server.ssh_port
            ));
        }
        if let Some(width) = columns.protocol {
            let mut part = cell(ui, &mut x, row.top(), width, row.height(), "protocol");
            part.add_sized(
                Vec2::new(width, row.height()),
                egui::Label::new(
                    RichText::new(if server.https { "HTTPS" } else { "HTTP" })
                        .size(10.0)
                        .color(Color32::from_gray(135)),
                ),
            );
        }
        if let Some(width) = columns.account {
            let mut part = cell(ui, &mut x, row.top(), width, row.height(), "account");
            part.add_sized(
                Vec2::new(width, row.height()),
                egui::Label::new(
                    RichText::new(&server.username)
                        .size(11.0)
                        .color(Color32::from_gray(145)),
                )
                .truncate(),
            )
            .on_hover_text(&server.username);
        }
        let mut menu = cell(
            ui,
            &mut x,
            row.top(),
            columns.actions,
            row.height(),
            "actions",
        );
        menu.add_space((columns.actions - 28.0) / 2.0);
        menu_open |= self.device_action_button(&mut menu, index, server);
        if menu_open {
            active_outline(ui, row, 4.0);
        }
        self.device_drag(ui, &row_response, row, server, false);
    }

    pub(super) fn device_action_button(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        server: &Server,
    ) -> bool {
        ui.spacing_mut().button_padding = Vec2::new(6.0, 2.0);
        let mut open = false;
        let menu = egui::menu::menu_custom_button(
            ui,
            egui::Button::new("")
                .fill(Color32::from_gray(38))
                .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(65)))
                .corner_radius(4.0)
                .min_size(Vec2::new(28.0, 22.0)),
            |ui| {
                open = true;
                self.device_actions_menu(ui, index, server);
            },
        );
        let response = menu
            .response
            .on_hover_text("Device actions")
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Device actions")
        });
        lab_manager::svg_icons::paint(
            ui.painter(),
            response.rect.shrink(3.0),
            "ellipsis-vertical",
            Some(Color32::from_gray(190)),
        );
        hover_outline(ui, response.rect, &response, 4.0);
        if open {
            active_outline(ui, response.rect, 4.0);
        }
        open
    }

    pub(super) fn device_actions_menu(&mut self, ui: &mut egui::Ui, index: usize, server: &Server) {
        if action_item(ui, "↗", "Open in browser").clicked() {
            if let Err(error) = webbrowser::open(&server.web_url()) {
                self.notify(format!("Cannot open browser: {error}"));
            }
            ui.close_menu();
        }
        if action_item(ui, ">_", "SSH shell").clicked() {
            self.open_ssh_terminal(server.clone(), None);
            ui.close_menu();
        }
        if action_item(ui, "↑", "Update / command").clicked() {
            self.open_command_panel(server.clone());
            ui.close_menu();
        }
        if action_item(ui, "⚙", "Device settings").clicked() {
            self.open_settings_panel(index, server.clone());
            ui.close_menu();
        }
        ui.separator();
        if remove_button(ui).clicked() {
            self.open_remove_panel(index);
            ui.close_menu();
        }
    }
}

fn action_item(ui: &mut egui::Ui, icon: &str, label: &str) -> egui::Response {
    let response = ui.add(egui::Button::new("").min_size(Vec2::new(190.0, 26.0)));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(response.rect.left() + 15.0, response.rect.center().y),
        Vec2::splat(22.0),
    );
    ui.painter().rect_stroke(
        icon_rect,
        5.0,
        egui::Stroke::new(1.0_f32, Color32::from_gray(75)),
        egui::StrokeKind::Inside,
    );
    let _ = icon;
    let name = match label {
        "Remove server" => "trash-2",
        "Open in browser" => "external-link",
        "SSH shell" => "terminal",
        "Update / command" => "terminal",
        _ => "settings",
    };
    lab_manager::svg_icons::paint(
        ui.painter(),
        icon_rect.shrink(3.0),
        name,
        Some(ui.visuals().text_color()),
    );
    ui.painter().text(
        egui::pos2(response.rect.left() + 36.0, response.rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::TextStyle::Button.resolve(ui.style()),
        ui.visuals().text_color(),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(super) fn hover_outline(
    ui: &egui::Ui,
    rect: egui::Rect,
    response: &egui::Response,
    radius: f32,
) {
    if response.hovered() || response.has_focus() || response.context_menu_opened() {
        ui.painter().rect_stroke(
            rect,
            radius,
            egui::Stroke::new(1.0_f32, Color32::from_rgb(64, 184, 119)),
            egui::StrokeKind::Inside,
        );
    }
}

pub(super) fn remove_button(ui: &mut egui::Ui) -> egui::Response {
    action_item(ui, "", "Remove server")
}

pub(super) fn paint_trash(ui: &egui::Ui, rect: egui::Rect) {
    lab_manager::svg_icons::paint(
        ui.painter(),
        rect.shrink(4.0),
        "trash-2",
        Some(ui.visuals().text_color()),
    );
}

pub(super) fn active_outline(ui: &egui::Ui, rect: egui::Rect, radius: f32) {
    ui.painter().rect_stroke(
        rect,
        radius,
        egui::Stroke::new(1.0_f32, Color32::from_rgb(64, 184, 119)),
        egui::StrokeKind::Inside,
    );
}

#[cfg(test)]
mod grouping_tests {
    use super::*;
    #[test]
    fn more_than_three_named_groups_render_in_both_views() {
        for view in [ViewMode::List, ViewMode::Grid] {
            let mut app = crate::ui_tests::fixture();
            app.servers = (1..=7)
                .map(|number| {
                    let mut server = Server::new(format!("192.0.2.{number}").parse().unwrap());
                    server.group = format!("Group {number}");
                    server
                })
                .collect();
            app.view = view;
            app.table.by_group = true;
            let ctx = egui::Context::default();
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1100.0, 900.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.dashboard(ui));
                },
            );
        }
    }
}
