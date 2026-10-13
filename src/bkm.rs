//! Import of `.bkm` backups written by the original Windows software
//! ("Export profile").  The file is a raw dump of the program's profile
//! structure:
//!
//! * `0x214`  UTF-16LE profile name (zero padded).
//! * `0x5fbc` key table: 21 entries of 0x58 bytes (14 used).  Each entry:
//!   `+0x00 u32 function id`, `+0x04 RECT` (on-screen position),
//!   `+0x14 u32 key number`, `+0x18 u32 macro id`, `+0x1c u8 modifiers`,
//!   `+0x1d u8 Windows virtual key`, `+0x20 u8 DPI-lock index`,
//!   `+0x21 u8 fire interval`, `+0x22 u8 fire target (0xf0 + button)`.
//! * `0x83d4` mouse parameters + DPI stages:
//!   `scroll lines, double-click ms, sensitivity, precision, one-page` (u32 each)
//!   then 6 x `{dpi x, dpi y, rgb, flags}` (u32 each).
//! * `0x8493` lighting block, first u32 = selected effect index.
//!
//! Function ids: `0x01xxxxxx` mouse functions, `0x02xxxxxx` keyboard,
//! `0x05xxxxxx` macro, `0x08xxxxxx` special (DPI, fire key, ...).
//!
//! Macros inside the file are encrypted with a proprietary block cipher
//! and are not imported; a placeholder macro with the right name is
//! created instead so the key binding survives.

use crate::model::*;
use anyhow::{bail, Result};

const NAME_OFF: usize = 0x214;
const KEYS_OFF: usize = 0x5fbc;
const KEY_SIZE: usize = 0x58;
const PARAMS_OFF: usize = 0x83d4;
const LED_OFF: usize = 0x8493;

fn u32_at(d: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
}

/// Windows virtual-key code -> HID keyboard usage.
pub fn vk_to_usage(vk: u8) -> u8 {
    match vk {
        0x08 => 0x2a, // backspace
        0x09 => 0x2b, // tab
        0x0d => 0x28, // enter
        0x13 => 0x48, // pause
        0x14 => 0x39, // caps lock
        0x1b => 0x29, // esc
        0x20 => 0x2c, // space
        0x21 => 0x4b, // page up
        0x22 => 0x4e, // page down
        0x23 => 0x4d, // end
        0x24 => 0x4a, // home
        0x25 => 0x50, // left
        0x26 => 0x52, // up
        0x27 => 0x4f, // right
        0x28 => 0x51, // down
        0x2c => 0x46, // print screen
        0x2d => 0x49, // insert
        0x2e => 0x4c, // delete
        0x30 => 0x27, // 0
        0x31..=0x39 => 0x1e + (vk - 0x31),
        0x41..=0x5a => 0x04 + (vk - 0x41),
        0x5b => 0xe3,
        0x5c => 0xe7,
        0x5d => 0x65,
        0x60 => 0x62,
        0x61..=0x69 => 0x59 + (vk - 0x61),
        0x6a => 0x55,
        0x6b => 0x57,
        0x6d => 0x56,
        0x6e => 0x63,
        0x6f => 0x54,
        0x70..=0x7b => 0x3a + (vk - 0x70),
        0x7c..=0x87 => 0x68 + (vk - 0x7c),
        0x90 => 0x53,
        0x91 => 0x47,
        0xa0 => 0xe1,
        0xa1 => 0xe5,
        0xa2 => 0xe0,
        0xa3 => 0xe4,
        0xa4 => 0xe2,
        0xa5 => 0xe6,
        0xba => 0x33,
        0xbb => 0x2e,
        0xbc => 0x36,
        0xbd => 0x2d,
        0xbe => 0x37,
        0xbf => 0x38,
        0xc0 => 0x35,
        0xdb => 0x2f,
        0xdc => 0x31,
        0xdd => 0x30,
        0xde => 0x34,
        _ => 0,
    }
}

fn decode_entry(e: &[u8]) -> KeyFunction {
    let fid = u32_at(e, 0);
    let kind = (fid >> 24) as u8;
    let code = (fid & 0xff) as u8;
    let modifiers = e[0x1c];
    let vk = e[0x1d];
    match kind {
        0x01 => match code {
            0x11 => KeyFunction::LeftButton,
            0x12 => KeyFunction::MiddleButton,
            0x13 => KeyFunction::RightButton,
            0x14 => KeyFunction::Forward,
            0x15 => KeyFunction::Back,
            0x17 => KeyFunction::ThreeClick,
            0x18 => KeyFunction::ScrollUp,
            0x19 => KeyFunction::ScrollDown,
            _ => KeyFunction::Disable,
        },
        0x02 => KeyFunction::KeyCombination { modifiers, key: vk_to_usage(vk) },
        0x05 => KeyFunction::Macro {
            name: format!("Macro {:06x}", u32_at(e, 0x18) & 0xffffff),
            mode: MacroLoop::Count,
            count: 1,
        },
        0x08 => match code {
            0xa2 => KeyFunction::Disable,
            0xa4 => {
                let interval = e[0x21].max(5);
                let button = match e[0x22] {
                    0xf1 => FireButton::Right,
                    0xf2 => FireButton::Middle,
                    0xf3 => FireButton::Forward,
                    0xf4 => FireButton::Back,
                    _ => FireButton::Left,
                };
                KeyFunction::FireKey { button, interval, count: e[0x20] }
            }
            0xa6 => KeyFunction::DpiLock(200 + 100 * e[0x20] as u16),
            0xa8 => KeyFunction::DpiPlus,
            0xa9 => KeyFunction::DpiMinus,
            0xaa => KeyFunction::DpiLoop,
            _ => KeyFunction::Disable,
        },
        0x03 | 0x04 => {
            let m = match code {
                0x21 => MediaKey::MediaPlayer,
                0x22 => MediaKey::PlayPause,
                0x23 => MediaKey::Stop,
                0x24 => MediaKey::Previous,
                0x25 => MediaKey::Next,
                0x26 => MediaKey::VolumeUp,
                0x27 => MediaKey::VolumeDown,
                0x28 => MediaKey::Mute,
                0x30 => MediaKey::Email,
                0x31 => MediaKey::Calculator,
                0x32 => MediaKey::Explorer,
                0x34 => MediaKey::HomePage,
                _ => MediaKey::PlayPause,
            };
            KeyFunction::Multimedia(m)
        }
        _ => KeyFunction::Disable,
    }
}

pub fn parse(d: &[u8]) -> Result<Profile> {
    if d.len() < LED_OFF + 0x40 {
        bail!("not a MS900 .bkm backup (file too small)");
    }
    // name
    let mut name_u16 = Vec::new();
    let mut off = NAME_OFF;
    while off + 1 < d.len() {
        let c = u16::from_le_bytes([d[off], d[off + 1]]);
        if c == 0 {
            break;
        }
        name_u16.push(c);
        off += 2;
    }
    let name = String::from_utf16_lossy(&name_u16);
    let mut p = Profile::factory(if name.is_empty() { "Imported" } else { &name });

    // keys
    for i in 0..NUM_KEYS {
        let e = &d[KEYS_OFF + i * KEY_SIZE..KEYS_OFF + (i + 1) * KEY_SIZE];
        p.keys[i] = decode_entry(e);
    }
    // placeholder macros so bindings stay valid
    let names: Vec<String> = p
        .keys
        .iter()
        .filter_map(|k| if let KeyFunction::Macro { name, .. } = k { Some(name.clone()) } else { None })
        .collect();
    for n in names {
        if p.macro_by_name(&n).is_none() {
            p.macros.push(Macro { name: n, events: Vec::new() });
        }
    }

    // mouse parameters
    let mp = &mut p.mouse_params;
    mp.scroll_lines = u32_at(d, PARAMS_OFF).clamp(1, 100) as u8;
    mp.double_click_ms = u32_at(d, PARAMS_OFF + 4).clamp(200, 900) as u16;
    mp.sensitivity = u32_at(d, PARAMS_OFF + 8).clamp(1, 20) as u8;
    mp.enhance_precision = u32_at(d, PARAMS_OFF + 12) != 0;
    mp.scroll_one_page = u32_at(d, PARAMS_OFF + 16) != 0;

    // DPI stages
    for i in 0..NUM_DPI {
        let b = PARAMS_OFF + 20 + i * 16;
        let x = u32_at(d, b);
        let rgb = u32_at(d, b + 8);
        let flags = u32_at(d, b + 12);
        let s = &mut p.dpi.slots[i];
        if (DPI_MIN as u32..=DPI_MAX as u32).contains(&x) {
            s.dpi = DpiConfig::clamp_dpi(x as u16);
        }
        s.color = [(rgb & 0xff) as u8, ((rgb >> 8) & 0xff) as u8, ((rgb >> 16) & 0xff) as u8];
        s.enabled = flags & 1 == 0;
    }

    // lighting: selected effect index
    let eff = u32_at(d, LED_OFF) as usize;
    if eff < LightEffect::ALL.len() {
        p.lighting.effect = LightEffect::ALL[eff];
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_sample_backup() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../jotalea.bkm");
        let Ok(bytes) = std::fs::read(&path) else { return };
        let p = parse(&bytes).unwrap();
        assert_eq!(p.name, "jotalea");
        assert_eq!(p.keys[0], KeyFunction::LeftButton);
        assert_eq!(p.keys[3], KeyFunction::KeyCombination { modifiers: 0x08, key: 0 });
        assert_eq!(p.keys[4], KeyFunction::KeyCombination { modifiers: 0, key: 0x28 });
        assert!(matches!(p.keys[5], KeyFunction::FireKey { button: FireButton::Left, interval: 50, .. }));
        assert_eq!(p.keys[7], KeyFunction::KeyCombination { modifiers: 0, key: 0x29 });
        assert!(matches!(p.keys[11], KeyFunction::Macro { .. }));
        assert!(matches!(p.keys[13], KeyFunction::FireKey { button: FireButton::Right, .. }));
        assert_eq!(p.dpi.slots[0].dpi, 1200);
        assert_eq!(p.dpi.slots[5].dpi, 10000);
        assert_eq!(p.dpi.slots[0].color, [255, 0, 0]);
        assert_eq!(p.mouse_params.double_click_ms, 550);
        assert_eq!(p.mouse_params.sensitivity, 10);
        assert_eq!(p.lighting.effect, LightEffect::ColorfulSteady);
    }
}
