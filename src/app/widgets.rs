//! Hand-painted widgets styled like the original (skin bitmaps + text).
#![allow(clippy::too_many_arguments)]

use super::skins::Skins;
use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2};

pub const TXT: Color32 = Color32::from_rgb(180, 180, 180);
pub const TXT_OV: Color32 = Color32::from_rgb(255, 255, 255);
pub const TXT_DN: Color32 = Color32::from_rgb(255, 0, 0);
pub const RED: Color32 = Color32::from_rgb(255, 0, 0);
pub const MENU_TXT: Color32 = Color32::from_rgb(150, 150, 150);
pub const LIST_BKG: Color32 = Color32::from_rgb(39, 39, 39);
pub const LIST_ITEM: Color32 = Color32::from_rgb(67, 167, 214);
pub const POP_BKG: Color32 = Color32::from_rgba_premultiplied(6, 6, 7, 240);
pub const POP_FRAME: Color32 = Color32::from_rgb(100, 100, 100);
pub const EDIT_BKG: Color32 = Color32::from_rgb(39, 39, 39);

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}
pub const FS: f32 = 11.5;

/// Drawing context anchored at `origin` (window or dialog top-left).
pub struct Canvas<'a> {
    pub ui: &'a mut Ui,
    pub origin: Pos2,
    pub skins: &'a Skins,
    pub enabled: bool,
    pub id: Id,
}

impl<'a> Canvas<'a> {
    pub fn new(ui: &'a mut Ui, origin: Pos2, skins: &'a Skins, enabled: bool, id: Id) -> Self {
        Canvas { ui, origin, skins, enabled, id }
    }

    pub fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + Vec2::new(x, y)
    }
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.p(x, y), Vec2::new(w, h))
    }
    pub fn painter(&self) -> egui::Painter {
        self.ui.painter().clone()
    }

    pub fn img(&self, name: &str, x: f32, y: f32) -> Rect {
        self.skins.draw(&self.painter(), name, self.p(x, y))
    }
    pub fn img_rect(&self, name: &str, r: Rect) {
        self.skins.draw_rect(&self.painter(), name, r)
    }

    pub fn text(&self, x: f32, y: f32, s: &str, color: Color32) {
        self.painter().text(self.p(x, y), Align2::LEFT_TOP, s, font(FS), color);
    }
    pub fn text_sized(&self, x: f32, y: f32, s: &str, color: Color32, size: f32) {
        self.painter().text(self.p(x, y), Align2::LEFT_TOP, s, font(size), color);
    }
    pub fn text_center(&self, r: Rect, s: &str, color: Color32) {
        self.painter().text(r.center(), Align2::CENTER_CENTER, s, font(FS), color);
    }
    pub fn text_width(&self, s: &str) -> f32 {
        self.painter().layout_no_wrap(s.to_string(), font(FS), TXT).size().x
    }

    fn interact(&mut self, r: Rect, salt: &str, sense: Sense) -> egui::Response {
        let id = self.id.with(salt);
        let resp = self.ui.interact(r, id, sense);
        if !self.enabled {
            // report nothing when a modal dialog is open
            return self.ui.interact(Rect::NOTHING, id.with("disabled"), Sense::hover());
        }
        resp
    }

    /// Image button: `nr` normal, `ov` hovered.  Returns true when clicked.
    pub fn button(&mut self, salt: &str, nr: &str, ov: &str, x: f32, y: f32, label: &str) -> bool {
        let size = self.skins.size(nr);
        let r = self.rect(x, y, size.x, size.y);
        let resp = self.interact(r, salt, Sense::click());
        let hovered = resp.hovered() || resp.is_pointer_button_down_on();
        self.img_rect(if hovered { ov } else { nr }, r);
        let color = if resp.is_pointer_button_down_on() {
            TXT_DN
        } else if hovered {
            TXT_OV
        } else {
            TXT
        };
        if !label.is_empty() {
            self.text_center(r, label, color);
        }
        resp.clicked()
    }

    /// Button variant that stays highlighted while `active`.
    pub fn button_active(&mut self, salt: &str, nr: &str, ov: &str, x: f32, y: f32, label: &str, active: bool) -> bool {
        let size = self.skins.size(nr);
        let r = self.rect(x, y, size.x, size.y);
        let resp = self.interact(r, salt, Sense::click());
        let hovered = resp.hovered() || active;
        self.img_rect(if hovered { ov } else { nr }, r);
        let color = if active || resp.is_pointer_button_down_on() {
            TXT_DN
        } else if hovered {
            TXT_OV
        } else {
            TXT
        };
        self.text_center(r, label, color);
        resp.clicked()
    }

    /// 11x11 checkbox.  Returns true when toggled (caller flips the value).
    pub fn checkbox(&mut self, salt: &str, x: f32, y: f32, checked: bool, label: &str) -> bool {
        let r = self.rect(x, y, 11.0, 11.0);
        let hit = self.rect(x, y, 11.0 + 6.0 + self.text_width(label), 14.0);
        let resp = self.interact(hit, salt, Sense::click());
        let name = if checked {
            "check_dn"
        } else if resp.hovered() {
            "check_ov"
        } else {
            "check_nr"
        };
        self.img_rect(name, r);
        if !label.is_empty() {
            self.painter().text(self.p(x + 17.0, y + 5.5), Align2::LEFT_CENTER, label, font(FS), TXT);
        }
        resp.clicked()
    }

    /// 15x15 radio button.
    pub fn radio(&mut self, salt: &str, x: f32, y: f32, selected: bool, label: &str) -> bool {
        let r = self.rect(x, y, 15.0, 15.0);
        let hit = self.rect(x, y, 15.0 + 6.0 + self.text_width(label), 15.0);
        let resp = self.interact(hit, salt, Sense::click());
        let name = if selected {
            "radio_dn"
        } else if resp.hovered() {
            "radio_ov"
        } else {
            "radio_nr"
        };
        self.img_rect(name, r);
        self.painter().text(self.p(x + 21.0, y + 7.5), Align2::LEFT_CENTER, label, font(FS), TXT);
        resp.clicked()
    }

    /// Horizontal slider: bar.png track (`width` px) with slider2 knob.
    /// Returns true when the value changed.
    pub fn slider(&mut self, salt: &str, x: f32, y: f32, width: f32, value: &mut i32, min: i32, max: i32, text: &str) -> bool {
        let track = self.rect(x, y + 6.0, width, 5.0);
        let hit = self.rect(x - 9.0, y, width + 18.0, 18.0);
        let resp = self.interact(hit, salt, Sense::click_and_drag());
        self.img_rect("bar", track);
        let span = (max - min).max(1) as f32;
        let mut changed = false;
        if resp.dragged() || resp.clicked() || resp.is_pointer_button_down_on() {
            if let Some(pos) = self.ui.ctx().pointer_latest_pos() {
                let t = ((pos.x - track.min.x) / width).clamp(0.0, 1.0);
                let v = min + (t * span).round() as i32;
                if v != *value {
                    *value = v;
                    changed = true;
                }
            }
        }
        let t = ((*value - min) as f32 / span).clamp(0.0, 1.0);
        let kx = track.min.x + t * width - 9.0;
        self.img_rect("slider2", Rect::from_min_size(Pos2::new(kx, track.min.y - 6.5), Vec2::splat(18.0)));
        if !text.is_empty() {
            self.painter().text(Pos2::new(track.max.x + 10.0, track.center().y), Align2::LEFT_CENTER, text, font(FS), TXT);
        }
        changed
    }

    /// Flat colour swatch (double click opens picker).  Returns (clicked, double_clicked).
    pub fn swatch(&mut self, salt: &str, x: f32, y: f32, size: f32, color: [u8; 3], selected: bool) -> (bool, bool) {
        let r = self.rect(x, y, size, size);
        let resp = self.interact(r, salt, Sense::click());
        let c = Color32::from_rgb(color[0], color[1], color[2]);
        self.painter().rect_filled(r, 0.0, c);
        let frame = if selected { Color32::WHITE } else { Color32::from_rgb(90, 90, 90) };
        self.painter().rect_stroke(r, 0.0, Stroke::new(1.0, frame), egui::StrokeKind::Inside);
        (resp.clicked(), resp.double_clicked())
    }

    /// Combo box face.  Returns true when clicked (caller toggles the list).
    pub fn combo(&mut self, salt: &str, nr: &str, ov: &str, x: f32, y: f32, text: &str, open: bool) -> (bool, Rect) {
        let size = self.skins.size(nr);
        let r = self.rect(x, y, size.x, size.y);
        let resp = self.interact(r, salt, Sense::click());
        let hovered = resp.hovered() || open;
        self.img_rect(if hovered { ov } else { nr }, r);
        // leave room for the arrow painted into the skin
        let tr = Rect::from_min_max(r.min, Pos2::new(r.max.x - 14.0, r.max.y));
        self.text_center(tr, text, if hovered { TXT_OV } else { TXT });
        (resp.clicked(), r)
    }

    /// Single-line text field with the original dark look.
    pub fn text_field(&mut self, salt: &str, x: f32, y: f32, w: f32, h: f32, text: &mut String) -> egui::Response {
        let r = self.rect(x, y, w, h);
        self.painter().rect_filled(r, 0.0, EDIT_BKG);
        self.painter().rect_stroke(r, 0.0, Stroke::new(1.0, POP_FRAME), egui::StrokeKind::Inside);
        let inner = r.shrink2(Vec2::new(4.0, 1.0));
        let te = egui::TextEdit::singleline(text)
            .id(self.id.with(salt))
            .frame(egui::Frame::NONE)
            .font(font(FS))
            .text_color(TXT_OV)
            .vertical_align(egui::Align::Center)
            .desired_width(inner.width())
            .interactive(self.enabled);
        self.ui.put(inner, te)
    }

    /// Number field with range clamping on focus loss.
    pub fn number_field(&mut self, salt: &str, x: f32, y: f32, w: f32, text: &mut String, min: i32, max: i32) {
        let resp = self.text_field(salt, x, y, w, 20.0, text);
        text.retain(|c| c.is_ascii_digit());
        if text.len() > 5 {
            text.truncate(5);
        }
        if resp.lost_focus() {
            let v = text.parse::<i32>().unwrap_or(min).clamp(min, max);
            *text = v.to_string();
        }
    }

    /// Simple list box (LstBox colours).  Returns clicked index.
    pub fn list_box(&mut self, salt: &str, r: Rect, items: &[String], selected: Option<usize>) -> Option<usize> {
        self.painter().rect_filled(r, 0.0, LIST_BKG);
        self.painter().rect_stroke(r, 0.0, Stroke::new(1.0, POP_FRAME), egui::StrokeKind::Inside);
        let mut clicked = None;
        let row_h = 20.0;
        for (i, item) in items.iter().enumerate() {
            let rr = Rect::from_min_size(r.min + Vec2::new(1.0, 1.0 + i as f32 * row_h), Vec2::new(r.width() - 2.0, row_h));
            if rr.max.y > r.max.y {
                break;
            }
            let resp = self.interact(rr, &format!("{salt}{i}"), Sense::click());
            if selected == Some(i) {
                self.painter().rect_filled(rr, 0.0, LIST_ITEM);
            }
            let color = if selected == Some(i) { Color32::BLACK } else { TXT };
            self.painter().text(rr.min + Vec2::new(4.0, row_h / 2.0), Align2::LEFT_CENTER, item, font(FS), color);
            if resp.clicked() {
                clicked = Some(i);
            }
        }
        clicked
    }

    pub fn dashed_vline(&self, x: f32, y0: f32, y1: f32, color: Color32) {
        let mut y = y0;
        while y < y1 {
            let e = (y + 3.0).min(y1);
            self.painter().line_segment([self.p(x, y), self.p(x, e)], Stroke::new(1.0, color));
            y += 6.0;
        }
    }
}

/// A floating list (combo dropdown / menu).  Drawn in a Foreground area at
/// absolute screen position.  Returns (clicked index, hovered index, rect).
pub fn popup_list(
    ctx: &egui::Context,
    skins: &Skins,
    id: Id,
    pos: Pos2,
    width: f32,
    items: &[(String, bool)], // (label, has submenu)
    menu_style: bool,
) -> (Option<usize>, Option<usize>, Rect) {
    let row_h = 22.0;
    let h = items.len() as f32 * row_h + 2.0;
    // keep the popup inside the window (the original window cannot grow either)
    let screen = ctx.content_rect();
    let pos = Pos2::new(
        pos.x.min(screen.max.x - width).max(screen.min.x),
        pos.y.min(screen.max.y - h).max(screen.min.y),
    );
    let rect = Rect::from_min_size(pos, Vec2::new(width, h));
    let mut clicked = None;
    let mut hovered = None;
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .interactable(true)
        .show(ctx, |ui| {
            let (_, _) = ui.allocate_space(Vec2::new(width, h));
            let painter = ui.painter().clone();
            painter.rect_filled(rect, 0.0, POP_BKG);
            painter.rect_stroke(rect, 0.0, Stroke::new(1.0, POP_FRAME), egui::StrokeKind::Inside);
            for (i, (label, sub)) in items.iter().enumerate() {
                let rr = Rect::from_min_size(pos + Vec2::new(1.0, 1.0 + i as f32 * row_h), Vec2::new(width - 2.0, row_h));
                let resp = ui.interact(rr, id.with(i), Sense::click());
                let hov = resp.hovered();
                if hov {
                    hovered = Some(i);
                }
                if menu_style {
                    skins.draw_rect(&painter, if hov { "menu_ov" } else { "menu_nr" }, rr);
                } else if hov {
                    painter.rect_filled(rr, 0.0, Color32::from_rgb(60, 0, 0));
                }
                let color = if hov { TXT_OV } else { MENU_TXT };
                painter.text(rr.center(), Align2::CENTER_CENTER, label, font(FS), color);
                if *sub {
                    let c = Pos2::new(rr.max.x - 12.0, rr.center().y);
                    painter.add(egui::Shape::convex_polygon(
                        vec![c + Vec2::new(-4.0, -5.0), c + Vec2::new(4.0, 0.0), c + Vec2::new(-4.0, 5.0)],
                        RED,
                        Stroke::NONE,
                    ));
                }
                if resp.clicked() {
                    clicked = Some(i);
                }
            }
        });
    (clicked, hovered, rect)
}
