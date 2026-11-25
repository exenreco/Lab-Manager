use eframe::egui::{self, Color32, Vec2};
use lab_manager::{
    frosted_modal::FrostedModal,
    model::Server,
    status::{Connection, Monitor},
};

#[derive(Default)]
pub struct DeviceSearch {
    pub open: bool,
    query: String,
    focus: bool,
    highlighted: usize,
    filter: Option<Connection>,
    needs_open: bool,
    modal: FrostedModal,
}
impl DeviceSearch {
    pub fn toggle(&mut self) {
        self.open = !self.open;
        self.needs_open = self.open;
        if self.open {
            self.query.clear();
            self.highlighted = 0;
            self.filter = None;
            self.focus = true;
        }
    }
    pub fn close(&mut self) {
        self.open = false;
        self.focus = false;
        self.modal.clear();
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        bounds: egui::Rect,
        _trigger: egui::Rect,
        servers: &[Server],
        monitor: &Monitor,
    ) -> Option<Server> {
        if !self.open {
            return None;
        }
        let id = egui::Id::new("terminal_device_search");
        let modal_id = id.with("modal");
        let bounds = bounds.intersect(lab_manager::frosted_modal::body_rect(ctx));
        if self.needs_open {
            self.modal.open(ctx, modal_id);
            self.needs_open = false;
        }
        let navigation = ctx.input_mut(|input| {
            [
                egui::Key::Escape,
                egui::Key::ArrowDown,
                egui::Key::ArrowUp,
                egui::Key::Enter,
            ]
            .map(|key| input.consume_key(egui::Modifiers::NONE, key))
        });
        let mut selected = None;
        let mut close = navigation[0];
        let mut modal = std::mem::take(&mut self.modal);
        let response = modal.show_gallery_in(ctx, modal_id, bounds, |ui| {
            ui.set_min_size(bounds.size());
            ui.set_max_size(bounds.size());
            ui.painter().rect_filled(
                bounds,
                0.0,
                Color32::from_rgba_unmultiplied(20, 22, 22, 220),
            );
            let close_rect = egui::Rect::from_min_size(
                bounds.right_top() + Vec2::new(-54.0, 10.0),
                Vec2::splat(44.0),
            );
            let close_response = ui
                .interact(close_rect, id.with("close"), egui::Sense::click())
                .on_hover_text("Close device search")
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            close_response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Close device search")
            });
            ui.painter().rect_filled(
                close_rect,
                8.0,
                if close_response.hovered() {
                    Color32::from_rgb(150, 40, 45)
                } else {
                    Color32::from_gray(30)
                },
            );
            ui.painter().rect_stroke(
                close_rect,
                8.0,
                egui::Stroke::new(1.0_f32, Color32::from_gray(100)),
                egui::StrokeKind::Inside,
            );
            lab_manager::svg_icons::paint(
                ui.painter(),
                close_rect.shrink(9.0),
                "x",
                Some(Color32::from_gray(235)),
            );
            close |= close_response.clicked();
            let content = bounds.shrink(24.0);
            let mut column = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("device_gallery")
                    .max_rect(content)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );
            column.set_width(content.width());
            column.set_clip_rect(bounds);
            column.allocate_ui_with_layout(
                Vec2::new((content.width() - 48.0).max(40.0), 30.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.heading("Choose a device");
                },
            );
            column.label(
                egui::RichText::new("Open a new SSH terminal")
                    .size(12.0)
                    .color(Color32::from_gray(165)),
            );
            column.add_space(16.0);
            let mut changed = false;
            column.horizontal_wrapped(|ui| {
                for (filter, label) in [
                    (None, "All devices"),
                    (Some(Connection::Online), "Online"),
                    (Some(Connection::Offline), "Offline"),
                ] {
                    changed |= ui
                        .selectable_value(&mut self.filter, filter, label)
                        .changed();
                }
            });
            column.add_space(8.0);
            let query_response = column.add(
                egui::TextEdit::singleline(&mut self.query)
                    .id(id.with("input"))
                    .hint_text(
                        eframe::egui::RichText::new("Search name, IP, group or tag…")
                            .color(eframe::egui::Color32::from_gray(100)),
                    )
                    .char_limit(256)
                    .desired_width(f32::INFINITY),
            );
            if self.focus || !query_response.has_focus() {
                query_response.request_focus();
                self.focus = false;
            }
            column.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    id.with("input"),
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                        tab: false,
                    },
                )
            });
            changed |= query_response.changed();
            if changed {
                self.highlighted = 0;
            }
            column.add_space(10.0);
            let query = self.query.trim().to_lowercase();
            let matches: Vec<_> = servers
                .iter()
                .filter(|server| {
                    server.matches_query(&query)
                        && self
                            .filter
                            .is_none_or(|filter| monitor.get(server).connection == filter)
                })
                .collect();
            self.highlighted = self.highlighted.min(matches.len().saturating_sub(1));
            if navigation[1] {
                self.highlighted = (self.highlighted + 1).min(matches.len().saturating_sub(1));
            }
            if navigation[2] {
                self.highlighted = self.highlighted.saturating_sub(1);
            }
            if navigation[3]
                && let Some(server) = matches.get(self.highlighted)
            {
                selected = Some((*server).clone());
            }
            column.small(format!("{} devices", matches.len()));
            column.add_space(8.0);
            egui::ScrollArea::vertical()
                .id_salt(id.with("results"))
                .auto_shrink([false, false])
                .max_height(column.available_height().max(40.0))
                .show(&mut column, |ui| {
                    ui.set_width(content.width());
                    if matches.is_empty() {
                        ui.label(if servers.is_empty() {
                            "No saved devices yet. Add devices from the Devices page."
                        } else {
                            "No matching devices."
                        });
                    }
                    for (index, server) in matches.iter().enumerate() {
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 62.0),
                            egui::Sense::click(),
                        );
                        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                format!("Open SSH: {} ({})", server.name, server.ip),
                            )
                        });
                        let highlighted = response.hovered() || index == self.highlighted;
                        ui.painter().rect_filled(
                            rect,
                            10.0,
                            Color32::from_gray(if highlighted { 34 } else { 25 }),
                        );
                        ui.painter().rect_stroke(
                            rect,
                            10.0,
                            egui::Stroke::new(
                                1.0_f32,
                                if highlighted {
                                    Color32::from_rgb(58, 94, 74)
                                } else {
                                    Color32::from_gray(47)
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        let icon = egui::Rect::from_center_size(
                            rect.left_center() + Vec2::new(30.0, 0.0),
                            Vec2::splat(36.0),
                        );
                        let mut icon_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .id_salt(("result_icon", server.ip))
                                .max_rect(icon)
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        lab_manager::device_icons::thumbnail(
                            &mut icon_ui,
                            server.icon,
                            server.custom_icon.as_deref(),
                            36.0,
                        );
                        let status = match monitor.get(server).connection {
                            Connection::Online => Color32::from_rgb(65, 188, 122),
                            Connection::Offline => Color32::from_rgb(230, 80, 90),
                            Connection::Checking => Color32::from_gray(120),
                        };
                        ui.painter().circle_filled(
                            icon.right_bottom() - Vec2::splat(1.0),
                            4.0,
                            status,
                        );
                        let text_rect = egui::Rect::from_min_max(
                            rect.min + Vec2::new(58.0, 9.0),
                            rect.max - Vec2::new(12.0, 9.0),
                        );
                        for (text, y, size, color) in [
                            (&server.name, 1.0, 14.0, Color32::from_gray(230)),
                            (
                                &format!("{}@{}:{}", server.username, server.ip, server.ssh_port),
                                24.0,
                                11.0,
                                Color32::from_gray(150),
                            ),
                        ] {
                            let galley =
                                egui::WidgetText::from(egui::RichText::new(text).size(size))
                                    .into_galley(
                                        ui,
                                        Some(egui::TextWrapMode::Truncate),
                                        text_rect.width(),
                                        egui::FontId::proportional(size),
                                    );
                            ui.painter()
                                .with_clip_rect(ui.clip_rect().intersect(text_rect))
                                .galley(text_rect.min + Vec2::new(0.0, y), galley, color);
                        }
                        if (changed || navigation[1] || navigation[2]) && self.highlighted == index
                        {
                            response.scroll_to_me(Some(egui::Align::Center));
                        }
                        if response.clicked() {
                            selected = Some((*server).clone());
                        }
                        #[cfg(test)]
                        ctx.data_mut(|data| {
                            data.insert_temp(
                                egui::Id::new(("terminal_search_result", server.ip)),
                                rect,
                            )
                        });
                        ui.add_space(6.0);
                    }
                });
            #[cfg(test)]
            ctx.data_mut(|data| {
                data.insert_temp(egui::Id::new("terminal_search_input"), query_response.rect);
                data.insert_temp(
                    egui::Id::new("terminal_search_results_count"),
                    matches.len(),
                );
                data.insert_temp(egui::Id::new("terminal_search_bounds"), bounds);
            });
        });
        self.modal = modal;
        if close || response.is_some_and(|response| response.close) || selected.is_some() {
            self.close();
        }
        selected
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        search: &mut DeviceSearch,
        ctx: &egui::Context,
        servers: &[Server],
        time: f64,
        events: Vec<egui::Event>,
    ) -> Option<Server> {
        let mut selected = None;
        let _ = ctx.run(
            egui::RawInput {
                time: Some(time),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(800.0, 760.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                selected = search.show(
                    ctx,
                    egui::Rect::from_min_max(egui::pos2(100.0, 100.0), egui::pos2(700.0, 700.0)),
                    egui::Rect::from_min_size(egui::pos2(650.0, 70.0), Vec2::splat(30.0)),
                    servers,
                    &Monitor::default(),
                );
            },
        );
        selected
    }
    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn devices() -> Vec<Server> {
        ["Pi-hole", "Proxmox", "Ubuntu"]
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let mut server = Server::new(format!("192.0.2.{}", index + 1).parse().unwrap());
                server.name = (*name).into();
                server
            })
            .collect()
    }
    #[test]
    fn search_filters_as_you_type_and_results_stay_inside_sidebar_modal() {
        let ctx = egui::Context::default();
        let mut search = DeviceSearch::default();
        let servers = devices();
        search.toggle();
        for time in [0.0, 0.15, 0.4, 0.6] {
            frame(&mut search, &ctx, &servers, time, vec![]);
        }
        assert_eq!(
            ctx.data(|data| data.get_temp::<usize>(egui::Id::new("terminal_search_results_count"))),
            Some(3)
        );
        frame(
            &mut search,
            &ctx,
            &servers,
            0.8,
            vec![egui::Event::Text("pi".into())],
        );
        assert_eq!(search.query, "pi");
        assert_eq!(
            ctx.data(|data| data.get_temp::<usize>(egui::Id::new("terminal_search_results_count"))),
            Some(1)
        );
        let input = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("terminal_search_input")))
            .unwrap();
        let result = ctx
            .data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new((
                    "terminal_search_result",
                    servers[0].ip,
                )))
            })
            .unwrap();
        assert!(result.top() > input.bottom());
        let bounds = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("terminal_search_bounds")))
            .unwrap();
        assert!(bounds.contains_rect(input));
        assert!(result.left() >= bounds.left() && result.right() <= bounds.right());
        assert_eq!(
            frame(
                &mut search,
                &ctx,
                &servers,
                1.0,
                vec![key(egui::Key::Enter)]
            )
            .unwrap()
            .ip,
            servers[0].ip
        );
        assert!(!search.open);
    }
    #[test]
    fn arrows_choose_result_escape_dismisses_and_no_match_is_safe() {
        let ctx = egui::Context::default();
        let mut search = DeviceSearch::default();
        let servers = devices();
        search.toggle();
        for time in [0.0, 0.4] {
            frame(&mut search, &ctx, &servers, time, vec![]);
        }
        frame(
            &mut search,
            &ctx,
            &servers,
            0.5,
            vec![key(egui::Key::ArrowDown)],
        );
        assert_eq!(
            frame(
                &mut search,
                &ctx,
                &servers,
                0.6,
                vec![key(egui::Key::Enter)]
            )
            .unwrap()
            .ip,
            servers[1].ip
        );
        search.toggle();
        search.query = "no-such-device".into();
        assert!(
            frame(
                &mut search,
                &ctx,
                &servers,
                1.0,
                vec![key(egui::Key::Enter)]
            )
            .is_none()
        );
        assert!(search.open);
        frame(
            &mut search,
            &ctx,
            &servers,
            1.1,
            vec![key(egui::Key::Escape)],
        );
        assert!(!search.open);
    }
}
