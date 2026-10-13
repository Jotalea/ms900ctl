//! Colour picker dialog: 8x6 palette, preview, R/G/B fields, hue bar, OK.

use super::widgets::{Canvas, TXT};
use super::{Action, ColorTarget, Dialog};
use egui::{Align2, Color32, Context, Pos2, Rect, Sense, Vec2};

pub fn new_dialog(target: ColorTarget, rgb: [u8; 3]) -> Dialog {
    Dialog::Color { target, rgb, r: rgb[0].to_string(), g: rgb[1].to_string(), b: rgb[2].to_string() }
}

fn hsv(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [(r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8]
}

/// Palette like the original: first column greys, then 7 hues in 6 shades.
fn palette(col: usize, row: usize) -> [u8; 3] {
    if col == 0 {
        let v = [1.0, 0.8, 0.6, 0.4, 0.2, 0.0][row];
        return hsv(0.0, 0.0, v);
    }
    let hue = [0.0, 30.0, 60.0, 120.0, 180.0, 240.0, 300.0][col - 1];
    match row {
        0 => hsv(hue, 0.3, 1.0),
        1 => hsv(hue, 0.6, 1.0),
        2 => hsv(hue, 1.0, 1.0),
        3 => hsv(hue, 1.0, 0.75),
        4 => hsv(hue, 1.0, 0.5),
        _ => hsv(hue, 1.0, 0.3),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw(cv: &mut Canvas, ctx: &Context, top: bool, target: ColorTarget, rgb: &mut [u8; 3], rs: &mut String, gs: &mut String, bs: &mut String, actions: &mut Vec<Action>) {
    if cv.button("x", "close", "close", 364.0, 4.0, "") {
        actions.push(Action::CloseDialog);
    }
    // palette
    for row in 0..6 {
        for col in 0..8 {
            let c = palette(col, row);
            let (clicked, _) = cv.swatch(&format!("pal{row}_{col}"), 30.0 + 30.0 * col as f32, 24.0 + 28.0 * row as f32, 20.0, c, c == *rgb);
            if clicked {
                *rgb = c;
                sync_fields(rgb, rs, gs, bs);
            }
        }
    }
    // preview
    let pr = cv.rect(292.0, 25.0, 70.0, 70.0);
    cv.painter().rect_filled(pr, 0.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
    cv.painter().rect_stroke(pr, 0.0, egui::Stroke::new(1.0, Color32::from_rgb(90, 90, 90)), egui::StrokeKind::Inside);
    // R G B fields
    let mut any = false;
    for (i, (label, s)) in [("R", &mut *rs), ("G", &mut *gs), ("B", &mut *bs)].into_iter().enumerate() {
        let y = 110.0 + 28.0 * i as f32;
        cv.text(288.0, y + 3.0, label, TXT);
        let before = s.clone();
        cv.number_field(&format!("ch{i}"), 302.0, y, 60.0, s, 0, 255);
        if *s != before {
            any = true;
        }
    }
    if any {
        let parse = |s: &String| s.parse::<i32>().unwrap_or(0).clamp(0, 255) as u8;
        *rgb = [parse(rs), parse(gs), parse(bs)];
    }
    // hue spectrum bar
    let sr = cv.rect(30.0, 192.0, 230.0, 50.0);
    let n = 46;
    for i in 0..n {
        let x0 = sr.min.x + sr.width() * i as f32 / n as f32;
        let x1 = sr.min.x + sr.width() * (i + 1) as f32 / n as f32;
        let hue = 360.0 * (i as f32 + 0.5) / n as f32;
        let c = hsv(hue, 1.0, 1.0);
        cv.painter().rect_filled(Rect::from_min_max(Pos2::new(x0, sr.min.y), Pos2::new(x1 + 0.5, sr.max.y)), 0.0, Color32::from_rgb(c[0], c[1], c[2]));
    }
    let resp = cv.ui.interact(sr, cv.id.with("spectrum"), Sense::click_and_drag());
    if top && (resp.clicked() || resp.dragged() || resp.is_pointer_button_down_on()) {
        if let Some(p) = ctx.pointer_latest_pos() {
            let t = ((p.x - sr.min.x) / sr.width()).clamp(0.0, 0.999);
            let v = 1.0 - ((p.y - sr.min.y) / sr.height()).clamp(0.0, 1.0) * 0.5;
            *rgb = hsv(360.0 * t, 1.0, v);
            sync_fields(rgb, rs, gs, bs);
        }
    }
    if cv.button("ok", "btn_nr", "btn_ov", 275.0, 222.0, "OK") {
        actions.push(Action::SetColor(target, *rgb));
        actions.push(Action::CloseDialog);
    }
    let _ = Align2::CENTER_CENTER;
    let _ = Vec2::ZERO;
}

fn sync_fields(rgb: &[u8; 3], rs: &mut String, gs: &mut String, bs: &mut String) {
    *rs = rgb[0].to_string();
    *gs = rgb[1].to_string();
    *bs = rgb[2].to_string();
}
