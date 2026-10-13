//! Skin bitmaps of the original software, embedded into the binary and
//! uploaded as egui textures on first use.

use egui::{Color32, ColorImage, Context, Pos2, Rect, TextureHandle, TextureOptions, Vec2};
use std::collections::HashMap;

macro_rules! skin_list {
    ($($name:literal => $path:literal),* $(,)?) => {
        const RAW: &[(&str, &[u8])] = &[
            $(($name, include_bytes!(concat!("../../assets/skins/", $path))),)*
        ];
    };
}

skin_list! {
    "main" => "main_nr.png",
    "mouse" => "mouse/mouse_nr.png",
    "button2_nr" => "button2_nr.png",
    "button2_ov" => "button2_ov.png",
    "button3_nr" => "button3_nr.png",
    "button3_ov" => "button3_ov.png",
    "button_nr" => "button_nr.png",
    "button_ov" => "button_ov.png",
    "btn_nr" => "btn_nr.png",
    "btn_ov" => "btn_ov.png",
    "combo_nr" => "combo_nr.png",
    "combo_ov" => "combo_ov.png",
    "combo2_nr" => "combo2_nr.png",
    "combo2_ov" => "combo2_ov.png",
    "add_nr" => "add_nr.png",
    "add_ov" => "add_ov.png",
    "del_nr" => "del_nr.png",
    "del_ov" => "del_ov.png",
    "dot_nr" => "dot_nr.png",
    "dot_ov" => "dot_ov.png",
    "header_nr" => "header_nr.png",
    "header_ov" => "header_ov.png",
    "check_nr" => "check_nr.png",
    "check_ov" => "check_ov.png",
    "check_dn" => "check_dn.png",
    "radio_nr" => "radio_nr.png",
    "radio_ov" => "radio_ov.png",
    "radio_dn" => "radio_dn.png",
    "bar" => "bar.png",
    "slider2" => "slider2.png",
    "up_nr" => "up_nr.png",
    "up_dn" => "up_dn.png",
    "down_nr" => "down_nr.png",
    "down_dn" => "down_dn.png",
    "min_nr" => "min_nr.png",
    "min_ov" => "min_ov.png",
    "exit_nr" => "exit_nr.png",
    "exit_ov" => "exit_ov.png",
    "close" => "close.png",
    "msg" => "msg_nr.png",
    "msg2" => "msg2_nr.png",
    "pickfun" => "pickfundlg.png",
    "macro" => "macro_nr.png",
    "recbtn_nr" => "recbtn_nr.png",
    "recbtn_dn" => "recbtn_dn.png",
    "menu_nr" => "menu_nr.bmp",
    "menu_ov" => "menu_ov.bmp",
    "dpigrid_nr" => "mouse/dpigrid_nr.png",
    "dpigrid_dn" => "mouse/dpigrid_dn.png",
    "dpibar" => "mouse/dpi-bar.png",
    "slider" => "mouse/slider.png",
    "number1" => "mouse/number1.png",
    "number2" => "mouse/number2.png",
    "number3" => "mouse/number3.png",
    "number4" => "mouse/number4.png",
    "number5" => "mouse/number5.png",
    "number6" => "mouse/number6.png",
    "number7" => "mouse/number7.png",
    "number8" => "mouse/number8.png",
    "number9" => "mouse/number9.png",
    "number10" => "mouse/number10.png",
    "number11" => "mouse/number11.png",
    "number12" => "mouse/number12.png",
    "number13" => "mouse/number13.png",
    "number14" => "mouse/number14.png",
    "flag" => "flag.png",
    "fire" => "fire.png",
    "UP_N" => "UP_N.png",
    "DN_N" => "DN_N.png",
}

pub struct Skins {
    tex: HashMap<&'static str, TextureHandle>,
}

impl Skins {
    pub fn load(ctx: &Context) -> Skins {
        let mut tex = HashMap::new();
        for (name, bytes) in RAW {
            match image::load_from_memory(bytes) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let ci = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                    let h = ctx.load_texture(*name, ci, TextureOptions::LINEAR);
                    tex.insert(*name, h);
                }
                Err(e) => log::error!("skin {name}: {e}"),
            }
        }
        Skins { tex }
    }

    pub fn get(&self, name: &str) -> Option<&TextureHandle> {
        self.tex.get(name)
    }

    pub fn size(&self, name: &str) -> Vec2 {
        self.tex.get(name).map(|t| t.size_vec2()).unwrap_or(Vec2::ZERO)
    }

    /// Draw a skin at its natural size with the top-left corner at `pos`.
    pub fn draw(&self, painter: &egui::Painter, name: &str, pos: Pos2) -> Rect {
        let size = self.size(name);
        let rect = Rect::from_min_size(pos, size);
        self.draw_rect(painter, name, rect);
        rect
    }

    pub fn draw_rect(&self, painter: &egui::Painter, name: &str, rect: Rect) {
        if let Some(t) = self.tex.get(name) {
            painter.image(t.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        } else {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(60, 60, 60));
        }
    }
}
