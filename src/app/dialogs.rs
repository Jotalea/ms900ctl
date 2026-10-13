//! Modal dialogs (message boxes, fire key, key combination, macro pick, ...).

use super::widgets::{popup_list, Canvas, TXT, TXT_OV};
use super::{centered, Action, App, Dialog, InputAction, Popup};
use crate::model::*;
use egui::{Align2, Context, Id, Pos2, Rect, Vec2};

pub fn draw_all(app: &mut App, ctx: &Context, rects: &mut Vec<Rect>) {
    let n = app.dialogs.len();
    for idx in 0..n {
        let mut d = std::mem::replace(&mut app.dialogs[idx], Dialog::Wait);
        let top = idx + 1 == n;
        draw_dialog(app, ctx, &mut d, idx, top, rects);
        app.dialogs[idx] = d;
    }
}

/// Frame skin + size for each dialog kind.
fn frame_of(d: &Dialog) -> (&'static str, Vec2) {
    match d {
        Dialog::MacroEditor(_) => ("macro", Vec2::new(767.0, 418.0)),
        Dialog::MacroPick { .. } => ("pickfun", Vec2::new(540.0, 340.0)),
        Dialog::Color { .. } => ("", Vec2::new(384.0, 256.0)),
        _ => ("msg", Vec2::new(530.0, 200.0)),
    }
}

fn draw_dialog(app: &mut App, ctx: &Context, d: &mut Dialog, idx: usize, top: bool, rects: &mut Vec<Rect>) {
    let origin = ctx.content_rect().min;
    let (frame, size) = frame_of(d);
    let pos = centered(origin, size);
    let area_rect = Rect::from_min_size(pos, size);
    rects.push(area_rect);
    egui::Area::new(Id::new(("dialog", idx)))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .interactable(true)
        .show(ctx, |ui| {
            ui.allocate_space(size);
            let App { skins, store, actions, popup, .. } = app;
            let profile = &mut store.profiles[store.current];
            let mut cv = Canvas::new(ui, pos, skins, top, Id::new(("dlg", idx)));
            if frame.is_empty() {
                cv.painter().rect_filled(area_rect, 0.0, egui::Color32::from_rgb(18, 18, 18));
                cv.painter().rect_stroke(area_rect, 0.0, egui::Stroke::new(1.0, super::widgets::POP_FRAME), egui::StrokeKind::Inside);
            } else {
                cv.img_rect(frame, area_rect);
            }
            match d {
                Dialog::Message { text, yes_no, on_yes } => {
                    let lines: Vec<&str> = text.lines().collect();
                    for (i, l) in lines.iter().enumerate() {
                        cv.painter().text(cv.p(265.0, 80.0 + 18.0 * i as f32), Align2::CENTER_CENTER, l, super::widgets::font(super::widgets::FS), TXT_OV);
                    }
                    if *yes_no {
                        if cv.button("ok", "btn_nr", "btn_ov", 285.0, 160.0, "OK") {
                            if let Some(c) = on_yes.clone() {
                                actions.push(Action::Confirmed(c));
                            }
                            actions.push(Action::CloseDialog);
                        }
                        if cv.button("cancel", "btn_nr", "btn_ov", 386.0, 160.0, "Cancel") {
                            actions.push(Action::CloseDialog);
                        }
                    } else if cv.button("ok", "btn_nr", "btn_ov", 215.0, 160.0, "OK") {
                        actions.push(Action::CloseDialog);
                    }
                }
                Dialog::Wait => {
                    cv.painter().text(cv.p(265.0, 100.0), Align2::CENTER_CENTER, "Please wait...", super::widgets::font(super::widgets::FS), TXT_OV);
                }
                Dialog::Input { title, text, action } => {
                    if cv.button("x", "exit_nr", "exit_ov", 486.0, 7.0, "") {
                        actions.push(Action::CloseDialog);
                    }
                    cv.text(63.0, 72.0, title, TXT);
                    let resp = cv.text_field("input", 63.0, 94.0, 400.0, 22.0, text);
                    if top {
                        resp.request_focus();
                    }
                    let enter = top && ctx.input(|i| i.key_pressed(egui::Key::Enter));
                    if cv.button("ok", "btn_nr", "btn_ov", 285.0, 160.0, "OK") || enter {
                        actions.push(Action::InputDone(action.clone(), text.clone()));
                        actions.push(Action::CloseDialog);
                    }
                    if cv.button("cancel", "btn_nr", "btn_ov", 386.0, 160.0, "Cancel") {
                        actions.push(Action::CloseDialog);
                    }
                }
                Dialog::FireKey { key, button, interval, count } => {
                    cv.text(12.0, 10.0, "Fire key", TXT);
                    if cv.button("x", "exit_nr", "exit_ov", 486.0, 7.0, "") {
                        actions.push(Action::CloseDialog);
                    }
                    cv.text(44.0, 64.0, "Fire key", TXT);
                    cv.text(289.0, 64.0, "Intervals ( 5-255 )", TXT);
                    cv.text(409.0, 64.0, "Loop ( 1-255 )", TXT);
                    let open = matches!(popup, Some(Popup::FireButtonList));
                    let (clicked, cr) = cv.combo("firebtn", "combo_nr", "combo_ov", 44.0, 82.0, FireButton::ALL[*button].name(), open);
                    if clicked {
                        actions.push(if open { Action::ClosePopup } else { Action::OpenPopup(Popup::FireButtonList) });
                    }
                    cv.number_field("interval", 289.0, 84.0, 68.0, interval, 5, 255);
                    cv.number_field("count", 409.0, 84.0, 68.0, count, 0, 255);
                    cv.text(44.0, 120.0, "Note：The smaller intervals value is, the faster speed of keystroke is.", TXT);
                    if cv.button("ok", "btn_nr", "btn_ov", 402.0, 160.0, "OK") {
                        let iv = interval.parse::<i32>().unwrap_or(50).clamp(5, 255) as u8;
                        let ct = count.parse::<i32>().unwrap_or(0).clamp(0, 255) as u8;
                        actions.push(Action::SetKey(*key, KeyFunction::FireKey { button: FireButton::ALL[*button], interval: iv, count: ct }));
                        actions.push(Action::CloseDialog);
                    }
                    if open && top {
                        rects.push(cr);
                        let items: Vec<(String, bool)> = FireButton::ALL.iter().map(|b| (b.name().to_string(), false)).collect();
                        let (c, _, r) = popup_list(ctx, skins, Id::new("firebtnlist"), cr.left_bottom(), cr.width(), &items, false);
                        rects.push(r);
                        if let Some(c) = c {
                            *button = c;
                            actions.push(Action::ClosePopup);
                        }
                    }
                }
                Dialog::KeyCombo { key, modifiers, usage } => {
                    if cv.button("x", "exit_nr", "exit_ov", 486.0, 7.0, "") {
                        actions.push(Action::CloseDialog);
                    }
                    cv.text(63.0, 72.0, "Please enter keystroke", TXT);
                    // capture the next key press
                    if top {
                        let events = ctx.input(|i| i.events.clone());
                        for ev in events {
                            if let egui::Event::Key { key: k, pressed: true, .. } = ev {
                                if let Some(u) = crate::hidkeys::usage_from_egui(k) {
                                    *usage = u;
                                }
                            }
                        }
                    }
                    let r = cv.rect(63.0, 94.0, 400.0, 22.0);
                    cv.painter().rect_filled(r, 0.0, super::widgets::EDIT_BKG);
                    cv.painter().rect_stroke(r, 0.0, egui::Stroke::new(1.0, super::widgets::POP_FRAME), egui::StrokeKind::Inside);
                    let label = crate::hidkeys::usage_name(*usage);
                    cv.painter().text(r.min + Vec2::new(6.0, 11.0), Align2::LEFT_CENTER, label, super::widgets::font(super::widgets::FS), TXT_OV);
                    let mods = [("Shift", modifier::SHIFT, 63.0), ("Ctrl", modifier::CTRL, 128.0), ("Alt", modifier::ALT, 193.0), ("Win", modifier::WIN, 258.0)];
                    for (name, bit, x) in mods {
                        if cv.checkbox(name, x, 134.0, *modifiers & bit != 0, name) {
                            *modifiers ^= bit;
                        }
                    }
                    if cv.button("ok", "btn_nr", "btn_ov", 285.0, 160.0, "OK") {
                        if *modifiers == 0 && *usage == 0 {
                            actions.push(Action::OpenDialog(Dialog::Message { text: "Please input new key".into(), yes_no: false, on_yes: None }));
                        } else {
                            actions.push(Action::SetKey(*key, KeyFunction::KeyCombination { modifiers: *modifiers, key: *usage }));
                            actions.push(Action::CloseDialog);
                        }
                    }
                    if cv.button("cancel", "btn_nr", "btn_ov", 386.0, 160.0, "Cancel") {
                        actions.push(Action::CloseDialog);
                    }
                }
                Dialog::MacroPick { key, selected, mode, count } => {
                    if cv.button("x", "exit_nr", "exit_ov", 516.0, 8.0, "") {
                        actions.push(Action::CloseDialog);
                    }
                    cv.text(20.0, 22.0, "Macro list", TXT);
                    let names: Vec<String> = profile.macros.iter().map(|m| m.name.clone()).collect();
                    if names.is_empty() {
                        cv.text(20.0, 60.0, "There are currently no available macros.", TXT);
                    }
                    let lr = cv.rect(20.0, 40.0, 240.0, 230.0);
                    if let Some(c) = cv.list_box("macros", lr, &names, *selected) {
                        *selected = Some(c);
                    }
                    for (i, m) in MacroLoop::ALL.iter().enumerate() {
                        if cv.radio(&format!("mode{i}"), 290.0, 60.0 + 30.0 * i as f32, *mode == *m, m.name()) {
                            *mode = *m;
                        }
                    }
                    cv.number_field("count", 310.0, 150.0, 78.0, count, 1, 255);
                    cv.text(395.0, 153.0, "1 ~ 255", TXT);
                    if cv.button("ok", "btn_nr", "btn_ov", 320.0, 290.0, "OK") {
                        if let Some(s) = *selected {
                            if let Some(m) = profile.macros.get(s) {
                                let ct = count.parse::<i32>().unwrap_or(1).clamp(1, 255) as u8;
                                actions.push(Action::SetKey(*key, KeyFunction::Macro { name: m.name.clone(), mode: *mode, count: ct }));
                                actions.push(Action::CloseDialog);
                            }
                        } else {
                            actions.push(Action::OpenDialog(Dialog::Message { text: "Please select a macro first.".into(), yes_no: false, on_yes: None }));
                        }
                    }
                    if cv.button("cancel", "btn_nr", "btn_ov", 420.0, 290.0, "Cancel") {
                        actions.push(Action::CloseDialog);
                    }
                }
                Dialog::Color { target, rgb, r, g, b } => {
                    super::colorpicker::draw(&mut cv, ctx, top, *target, rgb, r, g, b, actions);
                }
                Dialog::MacroEditor(me) => {
                    super::macro_editor::draw(&mut cv, ctx, top, me, profile, popup, actions, rects);
                }
            }
            let _ = InputAction::NewProfile;
            let _ = Pos2::ZERO;
        });
}
