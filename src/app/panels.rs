//! Right-hand accordion: DPI Setting, Lighting, Mouse Parameter, Polling Rate.

use super::widgets::{Canvas, POP_BKG, POP_FRAME, RED, TXT, TXT_OV};
use super::{Action, App, ColorTarget, Dialog, Popup};
use crate::model::*;
use egui::{Color32, Id, Pos2, Rect, Stroke, Ui, Vec2};

pub const PANEL_X: f32 = 741.0;
pub const PANEL_W: f32 = 245.0;
pub const HEADER_Y0: f32 = 41.0;
pub const HEADER_PITCH: f32 = 30.0;
pub const BODY_GAP: f32 = 27.0;
pub const BODY_AFTER: f32 = 9.0;
pub const TITLES: [&str; 4] = ["DPI Setting", "Lighting", "Mouse Parameter", "Polling Rate"];
pub const BODY_H: [f32; 4] = [392.0, 320.0, 300.0, 120.0];

/// Window-relative top of the open panel's body (used by the popup code).
pub fn body_origin(open: usize) -> Vec2 {
    Vec2::new(PANEL_X, HEADER_Y0 + HEADER_PITCH * open as f32 + BODY_GAP)
}

pub fn draw(app: &mut App, ui: &mut Ui, origin: Pos2, enabled: bool) {
    let App { skins, store, popup, actions, .. } = app;
    let open = store.open_panel.min(3);
    let profile = &mut store.profiles[store.current];
    let cv = Canvas::new(ui, origin, skins, enabled, Id::new("panels"));

    let mut y = HEADER_Y0;
    for i in 0..4 {
        let is_open = i == open;
        let r = cv.rect(PANEL_X, y, PANEL_W, 25.0);
        let resp = cv.ui.interact(r, cv.id.with(("header", i)), egui::Sense::click());
        let hovered = enabled && resp.hovered();
        cv.img_rect(if is_open || hovered { "header_ov" } else { "header_nr" }, r);
        let color = if is_open { RED } else if hovered { TXT_OV } else { TXT };
        cv.painter().text(r.min + Vec2::new(10.0, 12.5), egui::Align2::LEFT_CENTER, TITLES[i], super::widgets::font(super::widgets::FS), color);
        if enabled && resp.clicked() && !is_open {
            actions.push(Action::SetPanel(i));
        }
        if is_open {
            let by = y + BODY_GAP;
            let body = cv.rect(PANEL_X, by, PANEL_W, BODY_H[i]);
            cv.painter().rect_filled(body, 0.0, POP_BKG);
            cv.painter().rect_stroke(body, 0.0, Stroke::new(1.0, POP_FRAME), egui::StrokeKind::Inside);
            let mut sub = Canvas::new(cv.ui, origin + Vec2::new(PANEL_X, by), skins, enabled, Id::new(("body", i)));
            let mut changed = false;
            match i {
                0 => changed = draw_dpi(&mut sub, profile, actions),
                1 => changed = draw_lighting(&mut sub, profile, popup, actions),
                2 => changed = draw_mouse_params(&mut sub, profile),
                3 => changed = draw_polling(&mut sub, profile, popup, actions),
                _ => {}
            }
            if changed {
                actions.push(Action::Save);
            }
            y = by + BODY_H[i] + BODY_AFTER;
        } else {
            y += HEADER_PITCH;
        }
    }
}

fn draw_dpi(cv: &mut Canvas, p: &mut Profile, actions: &mut Vec<Action>) -> bool {
    let mut changed = false;
    cv.text(29.0, 24.0, "ON", TXT);
    cv.text(58.0, 24.0, "DPI", TXT);
    cv.text(91.0, 24.0, "Color", TXT);
    for i in 0..NUM_DPI {
        let yy = 52.0 * i as f32;
        let s = p.dpi.slots[i];
        if cv.checkbox(&format!("on{i}"), 28.0, 63.0 + yy, s.enabled, "") {
            if s.enabled && p.dpi.enabled_count() <= 1 {
                actions.push(Action::OpenDialog(Dialog::Message {
                    text: "You can not disable all DPI.".into(),
                    yes_no: false,
                    on_yes: None,
                }));
            } else {
                p.dpi.slots[i].enabled = !s.enabled;
                if !p.dpi.slots[i].enabled && p.dpi.current == i {
                    p.dpi.current = p.dpi.slots.iter().position(|s| s.enabled).unwrap_or(0);
                }
                changed = true;
            }
        }
        let selected = p.dpi.current == i;
        let r = cv.rect(40.0, 42.0 + yy, 42.0, 42.0);
        let resp = cv.ui.interact(r, cv.id.with(("grid", i)), egui::Sense::click());
        cv.img_rect(if selected { "dpigrid_dn" } else { "dpigrid_nr" }, r);
        let c1 = if selected { TXT_OV } else { TXT };
        let c2 = if selected { RED } else { TXT };
        cv.painter().text(r.center() - Vec2::new(0.0, 9.0), egui::Align2::CENTER_CENTER, format!("DPI {}", i + 1), super::widgets::font(11.5), c1);
        cv.painter().text(r.center() + Vec2::new(0.0, 10.0), egui::Align2::CENTER_CENTER, s.dpi.to_string(), super::widgets::font(11.5), c2);
        if cv.enabled && resp.clicked() && s.enabled && !selected {
            p.dpi.current = i;
            changed = true;
        }
        let (_, dbl) = cv.swatch(&format!("sw{i}"), 88.0, 55.0 + yy, 16.0, s.color, false);
        if dbl {
            actions.push(Action::OpenDialog(super::colorpicker::new_dialog(ColorTarget::Dpi(i), s.color)));
        }
    }
    cv.dashed_vline(139.0, 40.0, 347.0, Color32::from_rgb(110, 110, 110));
    // vertical slider
    let track = cv.rect(195.0, 40.0, 4.0, 307.0);
    cv.img_rect("dpibar", track);
    cv.painter().text(cv.p(197.0, 25.0), egui::Align2::CENTER_CENTER, "200", super::widgets::font(super::widgets::FS), TXT);
    cv.painter().text(cv.p(197.0, 362.0), egui::Align2::CENTER_CENTER, "12400", super::widgets::font(super::widgets::FS), TXT);
    let cur = p.dpi.current.min(NUM_DPI - 1);
    let dpi = p.dpi.slots[cur].dpi;
    // the slider is linear in table position, like the original
    let last = (DPI_VALUES.len() - 1) as f32;
    let t = dpi_index(dpi) as f32 / last;
    let ky = track.min.y + t * track.height();
    let knob = Rect::from_center_size(Pos2::new(track.center().x, ky), Vec2::splat(30.0));
    let hit = Rect::from_min_max(Pos2::new(track.min.x - 15.0, track.min.y - 15.0), Pos2::new(track.max.x + 15.0, track.max.y + 15.0));
    let resp = cv.ui.interact(hit, cv.id.with("vslider"), egui::Sense::click_and_drag());
    if cv.enabled && (resp.dragged() || resp.is_pointer_button_down_on()) {
        if let Some(pos) = cv.ui.ctx().pointer_latest_pos() {
            let t = ((pos.y - track.min.y) / track.height()).clamp(0.0, 1.0);
            let v = DPI_VALUES[(t * last).round() as usize];
            if v != dpi {
                p.dpi.slots[cur].dpi = v;
                changed = true;
            }
        }
    }
    cv.img_rect("slider", knob);
    cv.text(20.0, 379.0, &format!("Current DPI: {}", p.dpi.slots[cur].dpi), TXT);
    changed
}

fn color_row(cv: &mut Canvas, salt: &str, y: f32, colors: &[[u8; 3]], target: impl Fn(usize) -> ColorTarget, actions: &mut Vec<Action>) {
    cv.text(21.0, 105.0, "Color  -  Double click to pick a color", TXT);
    for (i, c) in colors.iter().enumerate() {
        let (_, dbl) = cv.swatch(&format!("{salt}{i}"), 21.0 + 27.0 * i as f32, y, 18.0, *c, false);
        if dbl {
            actions.push(Action::OpenDialog(super::colorpicker::new_dialog(target(i), *c)));
        }
    }
}

fn speed_slider(cv: &mut Canvas, salt: &str, y: f32, v: &mut u8, label: &str) -> bool {
    cv.text(21.0, y, label, TXT);
    let mut val = *v as i32;
    let text = val.to_string();
    let ch = cv.slider(salt, 21.0, y + 16.0, 176.0, &mut val, 1, 3, &text);
    if ch {
        *v = val as u8;
    }
    ch
}

fn draw_lighting(cv: &mut Canvas, p: &mut Profile, popup: &Option<Popup>, actions: &mut Vec<Action>) -> bool {
    let mut changed = false;
    cv.text(20.0, 35.0, "Effect", TXT);
    let open = matches!(popup, Some(Popup::EffectList));
    let (clicked, _) = cv.combo("effect", "combo2_nr", "combo2_ov", 20.0, 47.0, p.lighting.effect.name(), open);
    if clicked {
        actions.push(if open { Action::ClosePopup } else { Action::OpenPopup(Popup::EffectList) });
    }
    let l = &mut p.lighting;
    match l.effect {
        LightEffect::ColorfulStreaming => {
            changed |= speed_slider(cv, "spd", 199.0, &mut l.streaming_speed, "Speed");
            cv.text(21.0, 252.0, "Direction", TXT);
            let up = l.streaming_direction == 0;
            if cv.button("dirup", if up { "up_dn" } else { "up_nr" }, if up { "up_dn" } else { "up_nr" }, 20.0, 276.0, "") && !up {
                l.streaming_direction = 0;
                changed = true;
            }
            if cv.button("dirdown", if !up { "down_dn" } else { "down_nr" }, if !up { "down_dn" } else { "down_nr" }, 68.0, 276.0, "") && up {
                l.streaming_direction = 1;
                changed = true;
            }
        }
        LightEffect::Steady => {
            color_row(cv, "steady", 129.0, &[l.steady_color], |_| ColorTarget::Steady, actions);
            cv.text(21.0, 199.0, "Brightness", TXT);
            let mut val = crate::protocol::brightness_to_percent(l.steady_brightness) as i32;
            let text = format!("{val} %");
            if cv.slider("bri", 21.0, 215.0, 176.0, &mut val, 25, 100, &text) {
                l.steady_brightness = crate::protocol::brightness_from_percent(val as u8);
                changed = true;
            }
        }
        LightEffect::Breathing => {
            while l.breathing_colors.len() < 7 {
                l.breathing_colors.push([255, 255, 255]);
            }
            let colors = l.breathing_colors.clone();
            color_row(cv, "br", 129.0, &colors, ColorTarget::Breathing, actions);
            changed |= speed_slider(cv, "spd", 199.0, &mut l.breathing_speed, "Speed");
        }
        LightEffect::Tail => changed |= speed_slider(cv, "spd", 199.0, &mut l.tail_speed, "Speed"),
        LightEffect::Neon => changed |= speed_slider(cv, "spd", 199.0, &mut l.neon_speed, "Speed"),
        LightEffect::ColorfulSteady => {
            let colors = l.colorful_steady_colors;
            color_row(cv, "cs", 129.0, &colors, ColorTarget::ColorfulSteady, actions);
        }
        LightEffect::Flicker => {
            let colors = l.flicker_colors;
            color_row(cv, "fl", 129.0, &colors, ColorTarget::Flicker, actions);
        }
        LightEffect::StarsTwinkle => changed |= speed_slider(cv, "spd", 199.0, &mut l.stars_speed, "Speed"),
        LightEffect::Wave => changed |= speed_slider(cv, "spd", 199.0, &mut l.wave_speed, "Speed"),
        LightEffect::Reaction => changed |= speed_slider(cv, "spd", 199.0, &mut l.reaction_speed, "Speed"),
        LightEffect::None => {}
    }
    changed
}

fn draw_mouse_params(cv: &mut Canvas, p: &mut Profile) -> bool {
    let mut changed = false;
    let m = &mut p.mouse_params;
    cv.text(21.0, 31.0, "Mouse Sensitivity", TXT);
    let mut v = m.sensitivity as i32;
    let t = v.to_string();
    if cv.slider("sens", 21.0, 50.0, 176.0, &mut v, 1, 20, &t) {
        m.sensitivity = v as u8;
        changed = true;
    }
    if cv.checkbox("prec", 21.0, 80.0, m.enhance_precision, "Improve pointer precision") {
        m.enhance_precision = !m.enhance_precision;
        changed = true;
    }
    cv.text(21.0, 130.0, "Scrolling Speed", TXT);
    let mut v = m.scroll_lines as i32;
    let t = v.to_string();
    if cv.slider("scroll", 21.0, 149.0, 176.0, &mut v, 1, 100, &t) {
        m.scroll_lines = v as u8;
        changed = true;
    }
    if cv.checkbox("page", 21.0, 180.0, m.scroll_one_page, "Scroll One Page") {
        m.scroll_one_page = !m.scroll_one_page;
        changed = true;
    }
    cv.text(21.0, 229.0, "Double-Click Speed", TXT);
    let mut v = m.double_click_ms as i32;
    let t = v.to_string();
    if cv.slider("dclick", 21.0, 248.0, 176.0, &mut v, 200, 900, &t) {
        m.double_click_ms = v as u16;
        changed = true;
    }
    changed
}

fn draw_polling(cv: &mut Canvas, p: &mut Profile, popup: &Option<Popup>, actions: &mut Vec<Action>) -> bool {
    cv.text(30.0, 36.0, "USB Polling Rate（Hz）", TXT);
    let open = matches!(popup, Some(Popup::PollingList));
    let (clicked, _) = cv.combo("poll", "combo_nr", "combo_ov", 31.0, 59.0, &format!("{} Hz", p.polling_rate), open);
    if clicked {
        actions.push(if open { Action::ClosePopup } else { Action::OpenPopup(Popup::PollingList) });
    }
    false
}
