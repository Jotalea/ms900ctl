//! Main window: background, key list, mouse picture, profile area,
//! title-bar buttons and the right-hand accordion (see `panels.rs`).

use super::widgets::{Canvas, TXT, TXT_DN};
use super::{Action, App, Popup};
use crate::model::*;
use egui::{Id, Pos2, Rect, Sense, Ui, Vec2};

pub const KEY_X_BADGE: f32 = 40.0;
pub const KEY_X_BTN: f32 = 73.0;
pub const KEY_Y0: f32 = 50.0;
pub const KEY_PITCH: f32 = 31.0;

pub fn key_button_rect(origin: Pos2, i: usize) -> Rect {
    Rect::from_min_size(origin + Vec2::new(KEY_X_BTN, KEY_Y0 + KEY_PITCH * i as f32), Vec2::new(179.0, 25.0))
}

pub fn draw(app: &mut App, ui: &mut Ui, origin: Pos2, enabled: bool) {
    let id = Id::new("main");
    // background
    {
        let painter = ui.painter().clone();
        app.skins.draw(&painter, "main", origin);
        app.skins.draw(&painter, "mouse", origin + Vec2::new(290.0, 80.0));
    }
    // window drag strip
    {
        let strip = Rect::from_min_size(origin, Vec2::new(950.0, 40.0));
        let resp = ui.interact(strip, id.with("drag"), Sense::drag());
        if resp.drag_started() || (resp.dragged() && resp.drag_delta() != Vec2::ZERO) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
    }

    let App { skins, store, popup, actions, device, status, .. } = app;
    let profile = &store.profiles[store.current];
    let mut cv = Canvas::new(ui, origin, skins, enabled, id);

    // --- key list
    for i in 0..NUM_KEYS {
        let y = KEY_Y0 + KEY_PITCH * i as f32;
        cv.img(&format!("number{}", i + 1), KEY_X_BADGE, y);
        let active = matches!(popup, Some(Popup::KeyMenu { key, .. }) if *key == i);
        let label = profile.keys[i].label();
        if cv.button_active(&format!("key{i}"), "button2_nr", "button2_ov", KEY_X_BTN, y, &label, active) {
            if active {
                actions.push(Action::ClosePopup);
            } else {
                actions.push(Action::OpenPopup(Popup::KeyMenu { key: i, sub: None }));
            }
        }
    }

    // --- macro editor / profile area
    if cv.button("macroeditor", "button3_nr", "button3_ov", 112.0, 502.0, "Macro Editor") {
        actions.push(Action::OpenDialog(super::Dialog::MacroEditor(super::macro_editor::MacroEditor::default())));
    }
    cv.text(40.0, 534.0, "Profile", TXT);
    let open = matches!(popup, Some(Popup::ProfileList));
    let (clicked, _) = cv.combo("profilecombo", "combo_nr", "combo_ov", 41.0, 549.0, &profile.name, open);
    if clicked {
        actions.push(if open { Action::ClosePopup } else { Action::OpenPopup(Popup::ProfileList) });
    }
    if cv.button("add", "add_nr", "add_ov", 41.0, 586.0, "") {
        actions.push(Action::AddProfile);
    }
    if cv.button("del", "del_nr", "del_ov", 110.0, 586.0, "") {
        actions.push(Action::DeleteProfile);
    }
    let menu_open = matches!(popup, Some(Popup::ProfileMenu));
    if cv.button_active("dot", "dot_nr", "dot_ov", 180.0, 586.0, "", menu_open) {
        actions.push(if menu_open { Action::ClosePopup } else { Action::OpenPopup(Popup::ProfileMenu) });
    }
    cv.text(40.0, 632.0, &format!("Driver version: {}", env!("CARGO_PKG_VERSION")), TXT);

    // --- device status
    let dev = device.lock().unwrap().clone();
    if !dev.connected {
        cv.text(300.0, 632.0, "Device is disconnected.", egui::Color32::from_rgb(120, 120, 120));
    } else if !dev.firmware.is_empty() {
        cv.text(300.0, 632.0, &format!("Firmware: {}", dev.firmware), egui::Color32::from_rgb(120, 120, 120));
    }
    if !status.is_empty() {
        cv.text(560.0, 632.0, status, egui::Color32::from_rgb(120, 120, 120));
    }

    // --- title bar buttons
    if cv.button("min", "min_nr", "min_ov", 958.0, 11.0, "") {
        actions.push(Action::Minimize);
    }
    if cv.button("exit", "exit_nr", "exit_ov", 980.0, 11.0, "") {
        actions.push(Action::Close);
    }

    // --- restore / apply
    if cv.button("restore", "btn_nr", "btn_ov", 794.0, 601.0, "Restore") {
        actions.push(Action::Restore);
    }
    if cv.button("apply", "btn_nr", "btn_ov", 894.0, 601.0, "Apply") {
        actions.push(Action::Apply);
    }
    let _ = TXT_DN;
    let _ = cv;

    // --- accordion
    super::panels::draw(app, ui, origin, enabled);
}
