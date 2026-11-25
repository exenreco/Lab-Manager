use super::*;

pub(super) fn fixture() -> LabManager {
    let servers = (1..=2)
        .map(|n| {
            serde_json::from_value(serde_json::json!({
                "name": format!("Test {n}"), "ip": format!("192.168.1.{n}"),
                "web_port": 80, "ssh_port": 22, "https": false, "username": "root"
            }))
            .unwrap()
        })
        .collect();
    LabManager {
        servers,
        store: None,
        history: Default::default(),
        page: Page::Devices,
        server_form: Default::default(),
        right_sidebar: Default::default(),
        account_removal: Default::default(),
        search: String::new(),
        message: String::new(),
        writable: true,
        editor: None,
        editor_ip: String::new(),
        remote: None,
        terminal: Default::default(),
        local_terminal: Default::default(),
        command: String::new(),
        remove: None,
        navigation_open: false,
        navigation_hover: false,
        view: ViewMode::Grid,
        table: Default::default(),
        pending_reorder: None,
        monitor: Default::default(),
        monitoring: Default::default(),
        auth: auth_screen::AuthScreen::new(
            std::env::temp_dir().join(format!("lab-ui-{}-missing/account.enc", std::process::id())),
        ),
        data_root: Default::default(),
        active_user: None,
        notifications: Default::default(),
        discovery: Default::default(),
        backup: Default::default(),
        updates: Default::default(),
    }
}

#[test]
fn settings_shows_updates_before_backup_and_restore() {
    let mut app = fixture();
    app.page = Page::Storage;
    app.data_root = std::env::temp_dir().join("lab-settings-layout-fixture");
    app.store = Some(ServerStore {
        path: app.data_root.join("users/test/servers.json"),
    });
    let ctx = egui::Context::default();
    let mut labels = Vec::new();
    fn collect(shape: &egui::Shape, labels: &mut Vec<(String, f32)>) {
        match shape {
            egui::Shape::Text(text) => labels.push((text.galley.job.text.clone(), text.pos.y)),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, labels);
                }
            }
            _ => (),
        }
    }
    for _ in 0..2 {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1100.0, 1400.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.content(ui));
            },
        );
        labels.clear();
        for shape in output.shapes {
            collect(&shape.shape, &mut labels);
        }
    }
    let y = |label: &str| labels.iter().find(|(text, _)| text == label).unwrap().1;
    assert!(y("Updates") < y("Backup & restore"));
    assert!(labels.iter().any(|(text, _)| text == "Automatic updates"));
}

#[test]
fn command_sidebar_uses_current_device_os_without_stale_presets() {
    let mut app = fixture();
    let mut server = app.servers[0].clone();
    server.username = "root".into();
    server.icon = lab_manager::model::ServerIcon::Ubuntu;
    app.open_command_panel(server.clone());
    assert_eq!(app.command, "apt update && apt upgrade");
    server.os = Some(lab_manager::model::DeviceOs::Idrac);
    app.open_command_panel(server.clone());
    assert_eq!(app.command, "racadm getsysinfo");
    server.os = Some(lab_manager::model::DeviceOs::Other);
    app.open_command_panel(server);
    assert!(app.command.is_empty());
}

fn frame(app: &mut LabManager, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1000.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.dashboard(ui));
        },
    )
}

fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}

#[test]
fn grid_and_table_drag_sequence_queues_reorder_without_opening_settings() {
    for view in [ViewMode::Grid, ViewMode::List] {
        let ctx = egui::Context::default();
        design::configure(&ctx);
        let mut app = fixture();
        app.view = view;
        let _ = frame(&mut app, &ctx, vec![]);
        let rect = |ip| {
            ctx.data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new(("device_rect", ip)))
                    .unwrap()
            })
        };
        let to = rect(app.servers[1].ip);
        let start = ctx
            .data(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new((
                    "device_drag_handle",
                    app.servers[0].ip,
                )))
                .unwrap()
            })
            .center();
        let end = egui::pos2(
            if view == ViewMode::Grid {
                to.right() - 15.0
            } else {
                to.left() + 80.0
            },
            to.bottom() - 8.0,
        );
        let hover = frame(&mut app, &ctx, vec![egui::Event::PointerMoved(start)]);
        assert_eq!(hover.platform_output.cursor_icon, egui::CursorIcon::Grab);
        let _ = frame(&mut app, &ctx, vec![button(start, true)]);
        let moving = frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        assert!(
            egui::DragAndDrop::has_payload_of_type::<std::net::IpAddr>(&ctx),
            "drag did not start for grid={}",
            view == ViewMode::Grid
        );
        assert_eq!(
            moving.platform_output.cursor_icon,
            egui::CursorIcon::Grabbing
        );
        let _ = frame(&mut app, &ctx, vec![button(end, false)]);
        assert_eq!(
            app.pending_reorder,
            Some((app.servers[0].ip, app.servers[1].ip, true))
        );
        assert!(app.editor.is_none(), "drag must not open settings");
        let original = app.servers.clone();
        let path = std::env::temp_dir().join(format!(
            "lab-reorder-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        app.store = Some(ServerStore { path: path.clone() });
        app.apply_reorder();
        assert_eq!(app.servers, vec![original[1].clone(), original[0].clone()]);
        assert_eq!(app.store.as_ref().unwrap().load().unwrap(), app.servers);
        app.restore_inventory(false);
        assert_eq!(app.servers, original);
        assert_eq!(app.store.as_ref().unwrap().load().unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn device_backgrounds_do_not_offer_or_start_dragging() {
    for view in [ViewMode::Grid, ViewMode::List] {
        let ctx = egui::Context::default();
        design::configure(&ctx);
        let mut app = fixture();
        app.view = view;
        let _ = frame(&mut app, &ctx, vec![]);
        let rect = ctx.data(|data| {
            data.get_temp::<egui::Rect>(egui::Id::new(("device_rect", app.servers[0].ip)))
                .unwrap()
        });
        let start = egui::pos2(rect.left() + 5.0, rect.bottom() - 4.0);
        let hover = frame(&mut app, &ctx, vec![egui::Event::PointerMoved(start)]);
        assert_eq!(hover.platform_output.cursor_icon, egui::CursorIcon::Default);
        let _ = frame(&mut app, &ctx, vec![button(start, true)]);
        let end = start + Vec2::new(50.0, 35.0);
        let _ = frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        assert!(!egui::DragAndDrop::has_any_payload(&ctx));
        let _ = frame(&mut app, &ctx, vec![button(end, false)]);
        assert!(app.pending_reorder.is_none());
    }
}

#[test]
fn collection_filters_share_a_row_with_metadata_at_the_right_edge() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let _ = frame(&mut app, &ctx, vec![]);
    let left = ctx
        .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("device_status_filters_rect")))
        .unwrap();
    let right = ctx
        .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("device_metadata_filters_rect")))
        .unwrap();
    assert_eq!(left.center().y, right.center().y);
    assert!(left.right() < right.left());
    assert!(right.right() > 950.0);
    for label in ["All devices", "Online", "Offline"] {
        let button = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("status_filter", label))))
            .unwrap();
        assert!(
            (button.center().y - left.center().y).abs() < 1.0,
            "{label} wrapped outside the filter row"
        );
    }
}

#[test]
fn both_views_show_the_same_filters_and_pagination() {
    fn text(shape: &egui::Shape, result: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(text) => result.push(text.galley.job.text.clone()),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    text(shape, result);
                }
            }
            _ => {}
        }
    }
    for (page, view) in [
        (Page::Devices, ViewMode::Grid),
        (Page::Devices, ViewMode::List),
    ] {
        let ctx = egui::Context::default();
        design::configure(&ctx);
        let mut app = fixture();
        app.page = page;
        app.view = view;
        let output = frame(&mut app, &ctx, vec![]);
        let mut labels = Vec::new();
        for shape in output.shapes {
            text(&shape.shape, &mut labels);
        }
        for expected in [
            "All devices",
            "Online",
            "Offline",
            "Rows per page",
            "1–2 of 2",
        ] {
            assert!(
                labels.iter().any(|text| text == expected),
                "missing {expected} in grid={}",
                view == ViewMode::Grid
            );
        }
    }
}

#[test]
fn navigation_animates_and_delays_hover_exit() {
    let mut app = fixture();
    let ctx = egui::Context::default();
    let mut run = |time: f64, x: f32| {
        let mut width = 0.0;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1000.0, 900.0),
                )),
                time: Some(time),
                events: vec![egui::Event::PointerMoved(egui::pos2(x, 200.0))],
                ..Default::default()
            },
            |ctx| {
                app.navigation(ctx);
                width = ctx.available_rect().left();
            },
        );
        (width, app.navigation_hover)
    };
    assert!((run(0.0, 500.0).0 - 60.0).abs() < 1.0);
    run(0.01, 25.0);
    let opening = run(0.10, 25.0).0;
    assert!(opening > 60.0 && opening < 230.0);
    assert!((run(0.4, 25.0).0 - 230.0).abs() < 1.0);
    assert!(run(0.41, 500.0).1);
    assert!(run(0.46, 500.0).1);
    assert!(!run(0.60, 500.0).1);
    let closing = run(0.62, 500.0).0;
    assert!(closing > 60.0 && closing < 230.0);
    assert!((run(1.0, 500.0).0 - 60.0).abs() < 1.0);
}

#[test]
fn monitoring_gear_opens_shared_sidebar_and_follows_selected_device() {
    let mut app = fixture();
    app.page = Page::Monitoring;
    let ctx = egui::Context::default();
    let run = |app: &mut LabManager, events| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1200.0, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                app.right_panel(ctx);
                egui::CentralPanel::default().show(ctx, |ui| app.content(ui));
            },
        );
    };
    for _ in 0..2 {
        run(&mut app, vec![]);
    }
    let gear = ctx
        .data(|d| {
            d.get_temp::<egui::Rect>(egui::Id::new("monitoring_settings_gear"))
                .unwrap()
        })
        .center();
    for pressed in [true, false] {
        run(
            &mut app,
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
    run(&mut app, vec![]);
    assert!(app.monitoring.settings_open && app.right_sidebar.open);
    let first = app.right_sidebar.content_id.clone();
    app.monitoring.selected = Some(lab_manager::monitoring::key(&app.servers[1]));
    run(&mut app, vec![]);
    assert_ne!(first, app.right_sidebar.content_id);
    assert_eq!(app.monitoring.password_target, app.monitoring.selected);
}

#[test]
fn changing_pages_closes_sidebar_content_but_same_page_keeps_it() {
    let mut app = fixture();
    app.open_settings_panel(0, app.servers[0].clone());
    app.change_page(Page::Devices);
    assert!(app.right_sidebar.open);
    app.change_page(Page::Monitoring);
    assert!(!app.right_sidebar.open);
    assert!(app.editor.is_none());
    app.open_scan_panel();
    app.change_page(Page::Local);
    assert!(!app.discovery.open);
    assert!(!app.right_sidebar.open);
    app.monitoring.settings_open = true;
    app.right_sidebar.open = true;
    app.change_page(Page::Storage);
    assert!(!app.monitoring.settings_open);
    assert!(!app.right_sidebar.open);
}
