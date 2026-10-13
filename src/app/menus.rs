//! Floating menus and combo lists (one open at a time, see `Popup`).

use super::main_window::key_button_rect;
use super::panels;
use super::widgets::popup_list;
use super::{Action, App, Dialog, Popup};
use crate::model::*;
use egui::{Context, Id, Pos2, Rect, Vec2};

/// Entries of the key function menu, in the original order.
const KEY_MENU: [(&str, bool); 15] = [
    ("Left Button", false),
    ("Right Button", false),
    ("Middle Button", false),
    ("Forward", false),
    ("Back", false),
    ("Scroll Up", false),
    ("Scroll Down", false),
    ("Three Click", false),
    ("Fire key", false),
    ("Key Combination", false),
    ("Macro", false),
    ("Disable", false),
    ("DPI", true),
    ("Multimedia", true),
    ("DPI Lock", true),
];

fn window_origin(ctx: &Context) -> Pos2 {
    ctx.content_rect().min
}

pub fn draw_popup(app: &mut App, ctx: &Context, rects: &mut Vec<Rect>) {
    let Some(popup) = app.popup.clone() else { return };
    let origin = window_origin(ctx);
    let open_panel = app.store.open_panel.min(3);
    let body = origin + panels::body_origin(open_panel);
    match popup {
        Popup::KeyMenu { key, sub } => {
            let kr = key_button_rect(origin, key);
            rects.push(kr);
            let items: Vec<(String, bool)> = KEY_MENU.iter().map(|(s, b)| (s.to_string(), *b)).collect();
            let (clicked, hovered, r) = popup_list(ctx, &app.skins, Id::new("keymenu"), kr.left_bottom(), kr.width(), &items, true);
            rects.push(r);
            // submenu management: hovering a submenu item opens it, hovering a leaf closes it
            let mut new_sub = sub;
            if let Some(h) = hovered {
                new_sub = if KEY_MENU[h].1 { Some(h) } else { None };
            }
            if let Some(c) = clicked {
                let f = match c {
                    0 => Some(KeyFunction::LeftButton),
                    1 => Some(KeyFunction::RightButton),
                    2 => Some(KeyFunction::MiddleButton),
                    3 => Some(KeyFunction::Forward),
                    4 => Some(KeyFunction::Back),
                    5 => Some(KeyFunction::ScrollUp),
                    6 => Some(KeyFunction::ScrollDown),
                    7 => Some(KeyFunction::ThreeClick),
                    11 => Some(KeyFunction::Disable),
                    _ => None,
                };
                if let Some(f) = f {
                    app.actions.push(Action::SetKey(key, f));
                } else {
                    match c {
                        8 => {
                            let (button, interval, count) = match &app.profile().keys[key] {
                                KeyFunction::FireKey { button, interval, count } => {
                                    (FireButton::ALL.iter().position(|b| b == button).unwrap_or(0), *interval, *count)
                                }
                                _ => (0, 50, 0),
                            };
                            app.actions.push(Action::OpenDialog(Dialog::FireKey {
                                key,
                                button,
                                interval: interval.to_string(),
                                count: count.to_string(),
                            }));
                        }
                        9 => {
                            let (modifiers, usage) = match &app.profile().keys[key] {
                                KeyFunction::KeyCombination { modifiers, key } => (*modifiers, *key),
                                _ => (0, 0),
                            };
                            app.actions.push(Action::OpenDialog(Dialog::KeyCombo { key, modifiers, usage }));
                        }
                        10 => {
                            if app.profile().macros.is_empty() {
                                app.actions.push(Action::OpenDialog(Dialog::Message {
                                    text: "There are currently no available macros.".into(),
                                    yes_no: false,
                                    on_yes: None,
                                }));
                            } else {
                                let (selected, mode, count) = match &app.profile().keys[key] {
                                    KeyFunction::Macro { name, mode, count } => {
                                        (app.profile().macros.iter().position(|m| &m.name == name), *mode, *count)
                                    }
                                    _ => (None, MacroLoop::Count, 1),
                                };
                                app.actions.push(Action::OpenDialog(Dialog::MacroPick {
                                    key,
                                    selected: selected.or(Some(0)),
                                    mode,
                                    count: count.max(1).to_string(),
                                }));
                            }
                        }
                        _ => {}
                    }
                }
            }
            if let Some(s) = new_sub {
                let row_y = r.min.y + 1.0 + 22.0 * s as f32;
                let pos = Pos2::new(r.max.x, row_y);
                let sub_items: Vec<(String, bool)> = match s {
                    12 => vec![("DPI +".into(), false), ("DPI -".into(), false), ("DPI Loop".into(), false)],
                    13 => MediaKey::ALL.iter().map(|m| (m.name().to_string(), false)).collect(),
                    _ => (0..11).map(|i| ((200 + 100 * i).to_string(), false)).collect(),
                };
                let (sc, sh, sr) = popup_list(ctx, &app.skins, Id::new(("submenu", s)), pos, 179.0, &sub_items, true);
                rects.push(sr);
                if sh.is_some() {
                    new_sub = Some(s);
                }
                if let Some(c) = sc {
                    let f = match s {
                        12 => [KeyFunction::DpiPlus, KeyFunction::DpiMinus, KeyFunction::DpiLoop][c].clone(),
                        13 => KeyFunction::Multimedia(MediaKey::ALL[c]),
                        _ => KeyFunction::DpiLock(200 + 100 * c as u16),
                    };
                    app.actions.push(Action::SetKey(key, f));
                }
                // keep the submenu open while the pointer travels to it
                if let Some(p) = ctx.pointer_latest_pos() {
                    let bridge = Rect::from_min_max(Pos2::new(r.max.x, row_y), Pos2::new(sr.max.x, sr.max.y.max(row_y + 22.0)));
                    if sr.contains(p) || bridge.contains(p) || hovered == Some(s) {
                        new_sub = Some(s);
                    } else if hovered.is_none() && !r.contains(p) {
                        // pointer left both lists: keep the submenu until something else is hovered
                        new_sub = Some(s);
                    }
                }
            }
            if new_sub != sub {
                app.popup = Some(Popup::KeyMenu { key, sub: new_sub });
            }
        }
        Popup::ProfileList => {
            let anchor = Rect::from_min_size(origin + Vec2::new(41.0, 549.0), Vec2::new(189.0, 25.0));
            rects.push(anchor);
            let items: Vec<(String, bool)> = app.store.profiles.iter().map(|p| (p.name.clone(), false)).collect();
            let (clicked, _, r) = popup_list(ctx, &app.skins, Id::new("profilelist"), anchor.left_bottom(), anchor.width(), &items, false);
            rects.push(r);
            if let Some(c) = clicked {
                app.actions.push(Action::SelectProfile(c));
            }
        }
        Popup::ProfileMenu => {
            let anchor = Rect::from_min_size(origin + Vec2::new(180.0, 586.0), Vec2::new(46.0, 20.0));
            rects.push(anchor);
            let items = vec![
                ("Export profile".to_string(), false),
                ("Import profile".to_string(), false),
                ("Rename".to_string(), false),
                ("Read from mouse".to_string(), false),
            ];
            let (clicked, _, r) = popup_list(ctx, &app.skins, Id::new("profilemenu"), anchor.left_bottom(), 160.0, &items, true);
            rects.push(r);
            match clicked {
                Some(0) => app.actions.push(Action::ExportProfile),
                Some(1) => app.actions.push(Action::ImportProfile),
                Some(2) => {
                    let name = app.profile().name.clone();
                    app.actions.push(Action::OpenDialog(Dialog::Input {
                        title: "Please input profile name".into(),
                        text: name,
                        action: super::InputAction::RenameProfile,
                    }));
                }
                Some(3) => app.actions.push(Action::ReadFromMouse),
                _ => {}
            }
        }
        Popup::EffectList => {
            let anchor = Rect::from_min_size(body + Vec2::new(20.0, 47.0), Vec2::new(149.0, 21.0));
            rects.push(anchor);
            let items: Vec<(String, bool)> = LightEffect::ALL.iter().map(|e| (e.name().to_string(), false)).collect();
            let (clicked, _, r) = popup_list(ctx, &app.skins, Id::new("effectlist"), anchor.left_bottom(), anchor.width(), &items, false);
            rects.push(r);
            if let Some(c) = clicked {
                app.profile_mut().lighting.effect = LightEffect::ALL[c];
                app.actions.push(Action::ClosePopup);
                app.actions.push(Action::Save);
            }
        }
        Popup::PollingList => {
            let anchor = Rect::from_min_size(body + Vec2::new(31.0, 59.0), Vec2::new(189.0, 25.0));
            rects.push(anchor);
            let items: Vec<(String, bool)> = POLLING_RATES.iter().map(|r| (format!("{r} Hz"), false)).collect();
            let (clicked, _, r) = popup_list(ctx, &app.skins, Id::new("pollinglist"), anchor.left_bottom(), anchor.width(), &items, false);
            rects.push(r);
            if let Some(c) = clicked {
                app.profile_mut().polling_rate = POLLING_RATES[c];
                app.actions.push(Action::ClosePopup);
                app.actions.push(Action::Save);
            }
        }
        Popup::FireButtonList | Popup::InsertEventList => {
            // drawn by the owning dialog (dialogs.rs / macro_editor.rs)
        }
    }
}
