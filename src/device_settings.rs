use eframe::egui::{self, Color32, RichText, Vec2};
use lab_manager::{
    model::Server,
    settings_form::{port_input, row},
};

pub fn show(ui: &mut egui::Ui, server: &mut Server, ip: &mut String) -> (bool, bool) {
    lab_manager::settings_form::configure(ui);
    row(ui, "Device", 0, |ui| {
        ui.add(
            egui::TextEdit::singleline(&mut server.name)
                .hint_text(
                    eframe::egui::RichText::new("Device name")
                        .color(eframe::egui::Color32::from_gray(100)),
                )
                .margin(Vec2::new(10.0, 9.0))
                .desired_width(f32::INFINITY),
        );
    });
    row(ui, "IP address", 3, |ui| {
        ui.add(
            egui::TextEdit::singleline(ip)
                .hint_text(
                    eframe::egui::RichText::new("IPv4 or IPv6 address")
                        .color(eframe::egui::Color32::from_gray(100)),
                )
                .margin(Vec2::new(10.0, 9.0))
                .desired_width(f32::INFINITY),
        );
    });
    let valid_ip = ip.trim().parse::<std::net::IpAddr>().is_ok();
    if !valid_ip {
        ui.colored_label(
            Color32::from_rgb(230, 100, 100),
            "Enter a valid IPv4 or IPv6 address.",
        );
    }
    row(ui, "Web port", 3, |ui| {
        port_input(ui, &format!("web_port-{}", server.ip), &mut server.web_port);
    });
    row(ui, "Protocol", 1, |ui| {
        ui.checkbox(&mut server.https, "Use HTTPS");
    });
    row(ui, "Icon", 1, |ui| {
        lab_manager::icon_picker::icon_picker_selected(
            ui,
            &mut server.icon,
            &mut server.custom_icon,
        );
    });
    lab_manager::settings_form::os_selector(ui, &mut server.os, server.icon);
    ui.separator();
    egui::CollapsingHeader::new("SSH Setting")
        .id_salt("device_ssh_settings")
        .default_open(false)
        .show(ui, |ui| {
            row(ui, "SSH user", 2, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut server.username)
                        .hint_text(
                            eframe::egui::RichText::new("Username")
                                .color(eframe::egui::Color32::from_gray(100)),
                        )
                        .margin(Vec2::new(10.0, 9.0))
                        .desired_width(f32::INFINITY),
                );
            });
            row(ui, "SSH port", 3, |ui| {
                port_input(ui, &format!("ssh_port-{}", server.ip), &mut server.ssh_port);
            });
            ui.label(
                RichText::new("SSH uses your OpenSSH keys, agent, or password prompt.")
                    .size(11.0)
                    .color(crate::design::MUTED),
            );
        });
    ui.push_id(server.ip, |ui| {
        lab_manager::settings_form::organization(
            ui,
            &mut server.group,
            &mut server.tags,
            &mut server.notes,
            &mut server.links,
            &mut server.reminder_at,
            &mut server.reminder_repeat_seconds,
        );
    });
    ui.add_space(4.0);
    let mut save = false;
    let mut cancel = false;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        save = ui
            .add_enabled_ui(valid_ip, |ui| {
                ui.add_sized(
                    [90.0, 36.0],
                    egui::Button::new("Done").fill(Color32::from_rgb(36, 96, 63)),
                )
                .on_hover_text("Save settings")
                .clicked()
            })
            .inner;
        cancel = ui
            .add_sized([90.0, 36.0], egui::Button::new("× Cancel").frame(false))
            .clicked();
    });
    (save, cancel)
}
