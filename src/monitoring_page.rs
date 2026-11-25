use eframe::egui::{self, Color32, RichText, Vec2};
use lab_manager::{
    model::Server,
    monitoring::{Monitoring, Sample, conditions, key},
};

pub fn show(ui: &mut egui::Ui, monitoring: &mut Monitoring, servers: &[Server]) -> Option<Server> {
    let shell = monitoring_header(ui, monitoring, servers);
    ui.scope(|ui| {
        if monitoring.paused {
            ui.disable();
            ui.set_opacity(0.5);
        }
        monitoring_body(ui, monitoring, servers)
    })
    .inner
    .or(shell)
}

fn monitoring_body(
    ui: &mut egui::Ui,
    monitoring: &mut Monitoring,
    servers: &[Server],
) -> Option<Server> {
    if monitoring
        .selected
        .as_ref()
        .is_none_or(|k| !servers.iter().any(|s| key(s) == *k))
    {
        monitoring.selected = servers.first().map(key);
    }
    ui.label(
        RichText::new("Device health, service availability, and performance history.")
            .color(crate::design::MUTED),
    );
    let mut online = 0;
    let mut offline = 0;
    let mut attention = 0;
    for server in servers {
        if let Some(h) = monitoring.database.devices.get(&key(server))
            && h.config.enabled
            && let Some(s) = h.samples.last()
        {
            if s.online() && !monitoring.offline.contains(&key(server)) {
                online += 1;
            } else {
                offline += 1;
            }
            if !conditions(s, &h.config).is_empty() {
                attention += 1;
            }
        }
    }
    ui.add_space(12.0);
    let totals = [
        ("Online", online, green()),
        ("Offline", offline, red()),
        ("Needs attention", attention, amber()),
    ];
    let available = ui.available_width();
    let summary_columns = if available >= 540.0 {
        3
    } else if available >= 360.0 {
        2
    } else {
        1
    };
    let summary_width =
        available.min(summary_columns as f32 * 220.0 + (summary_columns - 1) as f32 * 8.0);
    ui.allocate_ui_with_layout(
        Vec2::new(summary_width, 0.0),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            for chunk in totals.chunks(summary_columns) {
                ui.columns(summary_columns, |columns| {
                    for ((label, value, color), ui) in chunk.iter().zip(columns) {
                        let card = egui::Frame::new()
                            .fill(Color32::from_gray(24))
                            .corner_radius(10.0)
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(RichText::new(value.to_string()).size(24.0).color(*color));
                                ui.small(*label);
                            });
                        #[cfg(test)]
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(
                                egui::Id::new(("monitoring_summary", *label)),
                                card.response.rect,
                            )
                        });
                        #[cfg(not(test))]
                        let _ = card;
                    }
                });
                ui.add_space(6.0);
            }
        },
    );
    if !monitoring.error.is_empty() {
        ui.colored_label(Color32::from_rgb(230, 95, 95), &monitoring.error);
    }
    if servers.is_empty() {
        ui.add_space(24.0);
        ui.label("Add devices to start monitoring their configured web and SSH ports.");
        return None;
    }
    ui.add_space(16.0);
    egui::ScrollArea::vertical().id_salt("monitoring_page_scroll").show(ui,|ui| {
        let Some(server)=servers.iter().find(|s|Some(key(s))==monitoring.selected) else {return;};
        let h=monitoring.database.devices.entry(key(server)).or_default();
        let offline=monitoring.offline.contains(&key(server));
        let fallback=Sample::default();
        let latest=h.samples.last().unwrap_or(&fallback);
        if h.samples.is_empty() && !offline {
            ui.label(if h.config.enabled {"Waiting for the first check…"}else{"Monitoring is disabled for this device."});return;
        }
        let mut current=latest.clone();
        if offline {current.web=false;current.ssh=false;current.http_status=None;}
        let latest=&current;
        status_line(ui,latest,h.samples.len());
        if h.config.maintenance_active() { badge(ui, "Maintenance · alerts silenced", amber()); }
        let alerts=conditions(latest,&h.config);
        for alert in &alerts {
            if alert!="device is offline" && !alert.contains("port is unavailable") {ui.colored_label(amber(),alert);}
        }
        ui.add_space(8.0);
        let good=h.samples.iter().filter(|s|s.online()).count();
        let availability=if h.samples.is_empty(){0.0}else{100.0*good as f64/h.samples.len() as f64};
        ui.horizontal_wrapped(|ui| {
            service(ui,"Web port",server.web_port,latest.web);
            service(ui,"SSH port",server.ssh_port,latest.ssh);
            for check in &latest.services { badge(ui, &format!("{}: {}", check.name, if offline { "Unavailable" } else { &check.detail }), if check.online && !offline { green() } else { red() }); }
            badge(ui,&format!("Availability {availability:.1}% of recorded checks · HTTP {}",latest.http_status.map_or("—".into(),|s|s.to_string())),Color32::from_gray(180));
            if let Some(days)=latest.certificate_days {badge(ui,&format!("Certificate: {days} days remaining"),Color32::from_gray(180));}
        });
        ui.add_space(14.0);
        let idrac = h.config.platform == lab_manager::monitoring::Platform::Idrac;
        if idrac {
            if let Some(hardware) = &latest.hardware { hardware_summary(ui, hardware); }
            else { ui.small("iDRAC hardware data unavailable. Select Dell iDRAC / RACADM, enable Collect over SSH and check the saved SSH credentials and host trust."); }
        }
        let gap=ui.spacing().item_spacing.x;
        let columns=(((ui.available_width()+gap)/(240.0+gap)).floor() as usize).clamp(1,4);
        let grid_width=ui.available_width().min(columns as f32*360.0+(columns-1) as f32*gap);
        ui.allocate_ui_with_layout(Vec2::new(grid_width,0.0),egui::Layout::top_down(egui::Align::LEFT),|ui| {
            let metrics: &[Metric] = if idrac { &[Metric::Temperature,Metric::Power,Metric::Fan,Metric::Latency] } else { &[Metric::Cpu,Metric::Memory,Metric::Disk,Metric::Latency,Metric::Rx,Metric::Tx] };
            for chunk in metrics.chunks(columns) {
                ui.columns(columns,|uis| {
                    for (metric,ui) in chunk.iter().zip(uis) {metric_card(ui,*metric,&h.samples);}
                });
                ui.add_space(12.0);
            }
        });
        if let Some(seconds)=latest.uptime {ui.label(format!("Host uptime: {} days {:02}h {:02}m",(seconds/86400.0) as u64,(seconds/3600.0) as u64%24,(seconds/60.0) as u64%60));}
        if !h.config.host_metrics {ui.small("Use the gear button to enable SSH host metrics for CPU, memory, disk, uptime, and network counters.");}
        ui.add_space(14.0);
        if idrac && let Some(hardware) = &latest.hardware { hardware_details(ui, hardware); }
        egui::CollapsingHeader::new("Recent checks").show(ui,|ui| {
            for sample in h.samples.iter().rev().take(20) {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(if sample.online(){green()}else{red()},if sample.online(){"Online"}else{"Offline"});
                    ui.label(lab_manager::notifications::age(sample.timestamp));
                    if let Some(v)=sample.latency_ms {ui.label(format!("{v:.1} ms"));}
                    if let Some(error)=&sample.error {ui.small(error);}
                });
                ui.separator();
            }
        });
    });
    None
}
fn green() -> Color32 {
    Color32::from_rgb(65, 188, 122)
}
fn hardware_summary(ui: &mut egui::Ui, hardware: &lab_manager::idrac::Hardware) {
    use lab_manager::idrac::Health;
    let color = |health| match health {
        Health::Ok => green(),
        Health::Warning => amber(),
        Health::Critical => red(),
        Health::Unknown => crate::design::MUTED,
    };
    ui.horizontal_wrapped(|ui| {
        badge(
            ui,
            &format!("Reported hardware health: {}", hardware.health.label()),
            color(hardware.health),
        );
        if !hardware.power_state.is_empty() {
            badge(
                ui,
                &format!("Server power: {}", hardware.power_state),
                Color32::from_gray(180),
            );
        }
        if !hardware.model.is_empty() {
            ui.label(&hardware.model);
        }
    });
    for note in &hardware.notes {
        ui.colored_label(amber(), note);
    }
}
fn hardware_details(ui: &mut egui::Ui, hardware: &lab_manager::idrac::Hardware) {
    use lab_manager::idrac::Health;
    let color = |health| match health {
        Health::Ok => green(),
        Health::Warning => amber(),
        Health::Critical => red(),
        Health::Unknown => crate::design::MUTED,
    };
    egui::CollapsingHeader::new("Hardware sensors")
        .default_open(true)
        .show(ui, |ui| {
            let id = ui.make_persistent_id("idrac_all_sensors");
            let mut all = ui
                .ctx()
                .data(|data| data.get_temp::<bool>(id))
                .unwrap_or(false);
            ui.checkbox(&mut all, "Show all sensor categories");
            ui.ctx().data_mut(|data| data.insert_temp(id, all));
            let sensors: Vec<_> = hardware
                .sensors
                .iter()
                .filter(|sensor| {
                    all || matches!(
                        sensor.category.as_str(),
                        "POWER" | "TEMPERATURE" | "FAN" | "CURRENT"
                    ) || matches!(sensor.health, Health::Warning | Health::Critical)
                })
                .collect();
            let width = ui.available_width();
            let columns = if width >= 600.0 {
                3
            } else if width >= 380.0 {
                2
            } else {
                1
            };
            for chunk in sensors.chunks(columns) {
                ui.columns(columns, |columns| {
                    for (sensor, ui) in chunk.iter().zip(columns) {
                        egui::Frame::new()
                            .fill(Color32::from_gray(24))
                            .corner_radius(9.0)
                            .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(44)))
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.set_min_width((ui.available_width() - 2.0).max(0.0));
                                ui.add(egui::Label::new(&sensor.name).truncate())
                                    .on_hover_text(&sensor.name);
                                ui.colored_label(
                                    color(sensor.health),
                                    format!("{} · {}", sensor.health.label(), sensor.reading),
                                );
                                ui.small(&sensor.category);
                            });
                    }
                });
                ui.add_space(6.0);
            }
        });
    egui::CollapsingHeader::new(format!("Storage health ({})", hardware.storage.len()))
        .default_open(true)
        .show(ui, |ui| {
            if hardware.storage.is_empty() {
                ui.small(
                    "No controller or disk inventory was returned. See any collection notes above.",
                );
            }
            for disk in &hardware.storage {
                ui.horizontal_wrapped(|ui| {
                    badge(ui, disk.health.label(), color(disk.health));
                    ui.label(format!("{} · {}", disk.kind, disk.name));
                    if !disk.state.is_empty() {
                        ui.small(&disk.state);
                    }
                });
            }
        });
    ui.add_space(12.0);
}
fn red() -> Color32 {
    Color32::from_rgb(230, 80, 90)
}
fn amber() -> Color32 {
    Color32::from_rgb(235, 163, 70)
}
fn service(ui: &mut egui::Ui, label: &str, port: u16, online: bool) {
    badge(
        ui,
        &format!(
            "{label} {port}: {}",
            if online { "Open" } else { "Unavailable" }
        ),
        if online { green() } else { red() },
    );
}
fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    let galley = ui.painter().layout(
        text.into(),
        egui::FontId::proportional(11.0),
        color,
        (ui.max_rect().width() - 20.0).max(60.0),
    );
    let (rect, _) =
        ui.allocate_exact_size(galley.size() + Vec2::new(20.0, 12.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 9.0, Color32::from_gray(23));
    ui.painter().rect_stroke(
        rect,
        9.0,
        egui::Stroke::new(1.0_f32, color.gamma_multiply(0.35)),
        egui::StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.min + Vec2::new(10.0, 6.0), galley, color);
}
fn status_line(ui: &mut egui::Ui, sample: &Sample, count: usize) {
    let status = if sample.online() {
        "Device is online"
    } else {
        "Device is offline"
    };
    let checked = if count == 0 {
        "—".into()
    } else {
        lab_manager::notifications::age(sample.timestamp)
    };
    let text = format!("{status} - Last checked {checked} · {count} stored checks");
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .color(if sample.online() { green() } else { red() }),
    );
}
fn monitoring_action(ui: &mut egui::Ui, kind: u8, enabled: bool) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::click());
        let label = if kind == 0 {
            "Refresh monitoring now"
        } else {
            "SSH Shell"
        };
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
        let color = if response.hovered() {
            green()
        } else {
            Color32::from_gray(180)
        };
        if response.hovered() {
            ui.painter().rect_filled(rect, 9.0, Color32::from_gray(38));
        }
        lab_manager::svg_icons::paint(
            ui.painter(),
            rect.shrink(7.0),
            if kind == 0 { "refresh-cw" } else { "terminal" },
            Some(color),
        );
        response
            .on_hover_text(label)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
}
#[derive(Clone, Copy)]
enum Metric {
    Cpu,
    Memory,
    Disk,
    Latency,
    Rx,
    Tx,
    Temperature,
    Power,
    Fan,
}
impl Metric {
    fn label(self) -> &'static str {
        match self {
            Self::Temperature => "Peak temperature",
            Self::Power => "Power consumption",
            Self::Fan => "Minimum fan speed",
            Self::Cpu => "CPU",
            Self::Memory => "Memory",
            Self::Disk => "Root / system disk",
            Self::Latency => "Connection latency",
            Self::Rx => "Network received",
            Self::Tx => "Network sent",
        }
    }
    fn value(self, s: &Sample, previous: Option<&Sample>) -> Option<f64> {
        match self {
            Self::Temperature => s.hardware.as_ref()?.peak_temperature,
            Self::Power => s.hardware.as_ref()?.power_watts,
            Self::Fan => s.hardware.as_ref()?.minimum_fan_rpm,
            Self::Cpu => s.cpu,
            Self::Memory => s.memory,
            Self::Disk => s.disk,
            Self::Latency => s.latency_ms,
            Self::Rx | Self::Tx => {
                let p = previous?;
                if s.timestamp <= p.timestamp || s.uptime.zip(p.uptime).is_some_and(|(a, b)| a < b)
                {
                    return None;
                }
                let (a, b) = if matches!(self, Self::Rx) {
                    (s.received?, p.received?)
                } else {
                    (s.sent?, p.sent?)
                };
                (a >= b).then_some((a - b) / (s.timestamp - p.timestamp) as f64 / 1024.0)
            }
        }
    }
    fn unit(self) -> &'static str {
        match self {
            Self::Temperature => "C",
            Self::Power => "W",
            Self::Fan => "RPM",
            Self::Latency => "ms",
            Self::Rx | Self::Tx => "KiB/s",
            _ => "%",
        }
    }
}
fn metric_card(ui: &mut egui::Ui, metric: Metric, samples: &[Sample]) {
    let card=egui::Frame::new().fill(Color32::from_gray(24)).stroke(egui::Stroke::new(1.0_f32,Color32::from_gray(44)))
        .corner_radius(12.0).inner_margin(14.0).show(ui,|ui| {
            ui.set_min_width((ui.available_width()-2.0).max(0.0));
            ui.label(metric.label());
            let values:Vec<_>=samples.iter().enumerate().skip(samples.len().saturating_sub(120)).map(|(i,s)|metric.value(s,i.checked_sub(1).and_then(|i|samples.get(i)))).collect();
            let value=values.last().copied().flatten();
            ui.label(RichText::new(value.map_or("—".into(),|v|format!("{v:.1} {}",metric.unit()))).size(26.0).color(green()));
            let (rect,response)=ui.allocate_exact_size(Vec2::new(ui.available_width(),72.0),egui::Sense::hover());
            ui.painter().line_segment([rect.left_bottom(),rect.right_bottom()],egui::Stroke::new(1.0_f32,Color32::from_gray(50)));
            let values=&values[values.len().saturating_sub(120)..];
            let max=if matches!(metric,Metric::Cpu|Metric::Memory|Metric::Disk){100.0}else{values.iter().flatten().copied().fold(1.0_f64,f64::max)};
            let mut previous=None;
            for (i,value) in values.iter().enumerate() {
                if let Some(v)=value {
                    let point=egui::pos2(rect.left()+rect.width()*i as f32/(values.len().saturating_sub(1).max(1)) as f32,rect.bottom()-rect.height()*(*v/max).clamp(0.0,1.0) as f32);
                    if let Some(p)=previous {ui.painter().line_segment([p,point],egui::Stroke::new(1.5_f32,green()));}
                    ui.painter().circle_filled(point,2.0,green());previous=Some(point);
                } else {previous=None;}
            }
            response.on_hover_text("Latest 120 recorded checks. Gaps indicate unavailable metrics; network rates need two successful samples.");
            ui.small(if value.is_some(){"Recent history"}else{"No current metric available"});
        });
    #[cfg(test)]
    ui.ctx().data_mut(|d| {
        d.insert_temp(
            egui::Id::new(("monitoring_chart", metric.label())),
            card.response.rect,
        )
    });
    #[cfg(not(test))]
    let _ = card;
}

fn monitoring_gear(ui: &mut egui::Ui, enabled: bool) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, "Monitoring settings")
        });
        if response.hovered() {
            ui.painter().rect_filled(rect, 10.0, Color32::from_gray(42));
        }
        let color = if enabled {
            Color32::from_gray(190)
        } else {
            Color32::from_gray(80)
        };
        lab_manager::svg_icons::paint(ui.painter(), rect.shrink(7.0), "settings", Some(color));
        #[cfg(test)]
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("monitoring_settings_gear"), rect));
        response
            .on_hover_text("Monitoring settings for selected device")
            .on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
}

fn monitoring_header(
    ui: &mut egui::Ui,
    monitoring: &mut Monitoring,
    servers: &[Server],
) -> Option<Server> {
    let available = ui.available_width();
    let dock_width = 244.0;
    let stacked = available < dock_width + 112.0;
    let inline = available >= 660.0;
    let height = if inline {
        56.0
    } else if stacked {
        168.0
    } else {
        112.0
    };
    let (row, _) = ui.allocate_exact_size(Vec2::new(available, height), egui::Sense::hover());
    let title = egui::Rect::from_min_size(row.min, Vec2::new(170.0, 56.0));
    ui.painter().text(
        title.left_center(),
        egui::Align2::LEFT_CENTER,
        "Monitoring",
        egui::FontId::proportional(30.0),
        ui.visuals().text_color(),
    );
    let controls_y = if inline { row.top() } else { row.top() + 56.0 };
    let selector_width = if stacked {
        available
    } else if inline {
        (available - 170.0 - dock_width - 24.0).clamp(180.0, 320.0)
    } else {
        (available - dock_width - 12.0).max(80.0)
    };
    let dock_rect = egui::Rect::from_min_size(
        egui::pos2(
            row.right() - dock_width,
            controls_y + if stacked { 56.0 } else { 0.0 },
        ),
        Vec2::new(dock_width, 56.0),
    );
    let selector_rect = egui::Rect::from_min_size(
        egui::pos2(
            if stacked {
                row.left()
            } else {
                dock_rect.left() - 12.0 - selector_width
            },
            controls_y,
        ),
        Vec2::new(selector_width, 56.0),
    );
    let mut selector = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("monitoring_heading_selector")
            .max_rect(selector_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    selector.add_enabled_ui(!monitoring.paused, |ui| {
        crate::monitoring_selector::show_sized(ui, monitoring, servers, selector_width);
    });
    ui.painter()
        .rect_filled(dock_rect, 28.0, Color32::from_gray(16));
    ui.painter().rect_stroke(
        dock_rect,
        28.0,
        egui::Stroke::new(1.0_f32, Color32::from_gray(48)),
        egui::StrokeKind::Inside,
    );
    let mut dock = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("monitoring_header_dock")
            .max_rect(dock_rect.shrink2(Vec2::new(16.0, 0.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let mut shell = None;
    dock.add_enabled_ui(!monitoring.paused, |ui| {
        if monitoring_action(ui, 0, !servers.is_empty() && !monitoring.busy()).clicked() {
            monitoring.refresh();
        }
        if monitoring_action(ui, 1, !servers.is_empty()).clicked() {
            shell = servers
                .iter()
                .find(|s| Some(key(s)) == monitoring.selected)
                .cloned();
        }
        if monitoring_gear(ui, !servers.is_empty()).clicked() {
            monitoring.settings_open = true;
        }
    });
    monitoring_switch(&mut dock, monitoring);
    ui.add_space(8.0);
    shell
}

fn monitoring_switch(ui: &mut egui::Ui, monitoring: &mut Monitoring) {
    let (track, _) = ui.allocate_exact_size(Vec2::new(64.0, 28.0), egui::Sense::hover());
    let mut response = ui.interact(
        track,
        ui.make_persistent_id("monitoring_run_switch"),
        egui::Sense::click(),
    );
    if response.clicked() {
        monitoring.set_paused(!monitoring.paused);
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            true,
            !monitoring.paused,
            "Monitoring enabled",
        )
    });
    let progress = ui.ctx().animate_bool_with_time(
        ui.make_persistent_id("monitoring_run_animation"),
        !monitoring.paused,
        0.4,
    );
    let color = egui::lerp(
        egui::Rgba::from(Color32::from_rgb(185, 51, 61))
            ..=egui::Rgba::from(Color32::from_rgb(34, 145, 85)),
        progress,
    );
    ui.painter().rect_filled(track, 14.0, Color32::from(color));
    if response.has_focus() {
        ui.painter().rect_stroke(
            track.expand(3.0),
            17.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Outside,
        );
    }
    ui.painter().text(
        track.left_center() + Vec2::new(16.0, 0.0),
        egui::Align2::CENTER_CENTER,
        "ON",
        egui::FontId::proportional(10.0),
        Color32::from_white_alpha((progress * 255.0) as u8),
    );
    ui.painter().text(
        track.right_center() - Vec2::new(18.0, 0.0),
        egui::Align2::CENTER_CENTER,
        "OFF",
        egui::FontId::proportional(10.0),
        Color32::from_white_alpha(((1.0 - progress) * 255.0) as u8),
    );
    ui.painter().circle_filled(
        track.left_center() + Vec2::new(14.0 + 36.0 * progress, 0.0),
        11.0,
        Color32::from_gray(245),
    );
    #[cfg(test)]
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new("monitoring_run_switch"), track));
    response
        .on_hover_text(if monitoring.paused {
            "Turn monitoring on"
        } else {
            "Turn monitoring off"
        })
        .on_hover_cursor(egui::CursorIcon::PointingHand);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idrac_dashboard_uses_hardware_charts_and_keeps_narrow_layout_bounded() {
        let server = Server::new("192.0.2.1".parse().unwrap());
        for width in [360.0, 900.0, 1800.0] {
            let ctx = egui::Context::default();
            let mut monitoring = Monitoring::default();
            let history = monitoring.database.devices.entry(key(&server)).or_default();
            history.config.platform = lab_manager::monitoring::Platform::Idrac;
            history.config.host_metrics = true;
            history.samples.push(Sample { timestamp: 1, ssh: true,
                hardware: Some(lab_manager::idrac::Hardware {
                    health: lab_manager::idrac::Health::Ok, model: "PowerEdge R7515".into(), power_state: "OFF".into(),
                    peak_temperature: Some(28.0), power_watts: Some(14.0),
                    sensors: lab_manager::idrac::parse_sensors("Sensor Type : TEMPERATURE\nInlet Temp Ok 28C\nSensor Type : POWER\nPS1 Status Present AC 7Watts\n"),
                    ..Default::default()
                }), ..Default::default() });
            for _ in 0..2 {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(width, 1200.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            show(ui, &mut monitoring, std::slice::from_ref(&server));
                            assert!(
                                ui.min_rect().right() <= width + 1.0,
                                "iDRAC layout overflow"
                            );
                        });
                    },
                );
            }
            assert!(
                ctx.data(|d| d.get_temp::<egui::Rect>(egui::Id::new((
                    "monitoring_chart",
                    "Peak temperature"
                ))))
                .is_some()
            );
            assert!(
                ctx.data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("monitoring_chart", "CPU"))))
                    .is_none()
            );
        }
    }
    #[test]
    fn header_switch_pauses_and_can_resume_while_body_is_disabled() {
        let ctx = egui::Context::default();
        let mut monitoring = Monitoring::default();
        let server = Server::new("192.0.2.10".parse().unwrap());
        let run = |m: &mut Monitoring, events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1000.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        show(ui, m, std::slice::from_ref(&server));
                    });
                },
            );
        };
        for _ in 0..2 {
            run(&mut monitoring, vec![]);
        }
        let toggle = ctx
            .data(|d| {
                d.get_temp::<egui::Rect>(egui::Id::new("monitoring_run_switch"))
                    .unwrap()
            })
            .center();
        for paused in [true, false] {
            for pressed in [true, false] {
                run(
                    &mut monitoring,
                    vec![
                        egui::Event::PointerMoved(toggle),
                        egui::Event::PointerButton {
                            pos: toggle,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ],
                );
            }
            assert_eq!(monitoring.paused, paused);
            if paused {
                let gear = ctx
                    .data(|d| {
                        d.get_temp::<egui::Rect>(egui::Id::new("monitoring_settings_gear"))
                            .unwrap()
                    })
                    .center();
                for pressed in [true, false] {
                    run(
                        &mut monitoring,
                        vec![
                            egui::Event::PointerMoved(gear),
                            egui::Event::PointerButton {
                                pos: gear,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: Default::default(),
                            },
                        ],
                    );
                }
                assert!(!monitoring.settings_open);
            }
        }
    }
    #[test]
    fn network_rates_use_elapsed_time_and_reject_reboots() {
        let before = Sample {
            timestamp: 10,
            received: Some(1024.0),
            uptime: Some(100.0),
            ..Default::default()
        };
        let mut after = Sample {
            timestamp: 12,
            received: Some(5120.0),
            uptime: Some(102.0),
            ..Default::default()
        };
        assert_eq!(Metric::Rx.value(&after, Some(&before)), Some(2.0));
        after.uptime = Some(1.0);
        assert_eq!(Metric::Rx.value(&after, Some(&before)), None);
        assert_eq!(Metric::Rx.value(&after, None), None);
    }
    #[test]
    fn dashboard_renders_actual_sample_at_narrow_and_wide_sizes() {
        let server = Server::new("192.0.2.1".parse().unwrap());
        for width in [360.0, 900.0, 1800.0] {
            let ctx = egui::Context::default();
            let mut monitoring = Monitoring::default();
            monitoring
                .database
                .devices
                .entry(key(&server))
                .or_default()
                .samples
                .push(Sample {
                    timestamp: lab_manager::notifications::now(),
                    web: true,
                    ssh: true,
                    cpu: Some(22.0),
                    memory: Some(30.0),
                    ..Default::default()
                });
            monitoring
                .database
                .devices
                .get_mut(&key(&server))
                .unwrap()
                .config
                .host_metrics = true;
            for _ in 0..2 {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(width, 1000.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            assert!(
                                show(ui, &mut monitoring, std::slice::from_ref(&server)).is_none()
                            );
                            for metric in [
                                "CPU",
                                "Memory",
                                "Root / system disk",
                                "Connection latency",
                                "Network received",
                                "Network sent",
                            ] {
                                let card = ctx.data(|d| {
                                    d.get_temp::<egui::Rect>(egui::Id::new((
                                        "monitoring_chart",
                                        metric,
                                    )))
                                    .unwrap()
                                });
                                assert!(card.width() <= 361.0, "Chart grows too wide: {:?}", card);
                            }
                            for label in ["Online", "Offline", "Needs attention"] {
                                let card = ctx.data(|d| {
                                    d.get_temp::<egui::Rect>(egui::Id::new((
                                        "monitoring_summary",
                                        label,
                                    )))
                                    .unwrap()
                                });
                                assert!(
                                    card.width() <= 221.0,
                                    "Summary card grows too wide: {:?}",
                                    card
                                );
                            }
                            assert!(
                                ui.min_rect().right() <= width + 1.0,
                                "Monitoring overflow at {width}: {:?}",
                                ui.min_rect()
                            );
                        });
                    },
                );
            }
        }
    }
}
