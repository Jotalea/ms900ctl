//! Sinowealth SH68F90-family HID protocol as used by the Havit MS900
//! (USB 258a:0029, PMW3327 sensor).
//!
//! Framing (confirmed by disassembling the Windows `OemDrv.exe`):
//!
//! * Feature report `0x05`, 6 bytes: command channel.
//!   `[05, cmd, a, b, c, d]`.
//! * Feature report `0x04`, 520 bytes: data channel.  The firmware only
//!   transfers as many bytes as the selected block has (config ≈ 131-167,
//!   buttons 88, macro 515); the rest is padding.
//!
//! Commands (byte 1 of report 5):
//!
//! | cmd  | meaning                                   |
//! |------|-------------------------------------------|
//! | 0x01 | firmware version (4 bytes come back)      |
//! | 0x02 | get / set active profile (1..3)           |
//! | 0x11 | select config block of profile 1 (0x21/0x31 for 2/3) |
//! | 0x12 | select button block of profile 1 (0x22/0x32)         |
//! | 0x30 | macro block (write only)                  |
//!
//! Reading a block: write report 5 `[05, 0x11, 0,0,0,0]`, wait ~20 ms,
//! then GET report 4.  Writing: SET report 4 with byte 1 = block command and
//! byte 3 = `block_size - 8`.
//!
//! The config block layout is the one documented by libratbag's
//! `driver-sinowealth.c`; the offsets below were cross-checked against the
//! Windows binary (sensor id at +9, DPI count/active at +11, disabled mask
//! at +12, lift-off distance at +129).

use crate::model::*;
use anyhow::{anyhow, bail, Result};

pub const VID: u16 = 0x258a;
pub const PID: u16 = 0x0029;

pub const REPORT_CMD: u8 = 0x05;
pub const REPORT_DATA: u8 = 0x04;
pub const CMD_LEN: usize = 6;
pub const DATA_LEN: usize = 520;

pub const CMD_FIRMWARE: u8 = 0x01;
pub const CMD_PROFILE: u8 = 0x02;
pub const CMD_CONFIG: u8 = 0x11;
pub const CMD_BUTTONS: u8 = 0x12;
pub const CMD_MACRO: u8 = 0x30;

pub const BUTTON_BLOCK_LEN: usize = 88;
pub const BUTTON_SLOTS: usize = 20;
pub const MACRO_BLOCK_LEN: usize = 515;
pub const MACRO_MAX_EVENTS: usize = 168;
/// Minimum config block length we accept.
pub const CONFIG_MIN_LEN: usize = 123;

/// Sensor id reported at config[9].
pub const SENSOR_PMW3327: u8 = 0x0e;

/// Delay the original software waits between selecting a block and
/// reading it (`Sleep(20)`), in milliseconds.
pub const SELECT_DELAY_MS: u64 = 20;

/// Config block offsets (report byte index, byte 0 = report id).
pub mod cfg {
    pub const CMD: usize = 1;
    pub const WRITE_LEN: usize = 3;
    pub const SENSOR: usize = 9;
    pub const RATE_FLAGS: usize = 10;
    pub const DPI_COUNT_ACTIVE: usize = 11;
    pub const DPI_DISABLED: usize = 12;
    pub const DPI_VALUES: usize = 13; // 16 bytes
    pub const DPI_COLORS: usize = 29; // 8 * 3
    pub const EFFECT: usize = 53;
    pub const STREAMING_MODE: usize = 54;
    pub const STREAMING_DIR: usize = 55;
    pub const STEADY_MODE: usize = 56;
    pub const STEADY_COLOR: usize = 57;
    pub const BREATHING_MODE: usize = 60;
    pub const BREATHING_COUNT: usize = 61;
    pub const BREATHING_COLORS: usize = 62; // 7 * 3
    pub const TAIL_MODE: usize = 83;
    pub const NEON_MODE: usize = 84;
    pub const COLORFUL_STEADY_MODE: usize = 85;
    pub const COLORFUL_STEADY_COLORS: usize = 86; // 6 * 3
    pub const FLICKER_MODE: usize = 116;
    pub const FLICKER_COLORS: usize = 117; // 2 * 3
    pub const STARS_MODE: usize = 123;
    pub const WAVE_MODE: usize = 124;
    pub const REACTION_MODE: usize = 125;
    pub const REACTION_COLOR: usize = 126;
    pub const LOD: usize = 129;
}

/// Lighting effect ids stored at `cfg::EFFECT`.
pub mod effect {
    pub const NONE: u8 = 0x00;
    pub const COLORFUL_STREAMING: u8 = 0x01;
    pub const STEADY: u8 = 0x02;
    pub const BREATHING: u8 = 0x03;
    pub const TAIL: u8 = 0x04;
    pub const NEON: u8 = 0x05;
    pub const COLORFUL_STEADY: u8 = 0x06;
    pub const FLICKER: u8 = 0x07;
    pub const STARS_TWINKLE: u8 = 0x08;
    pub const WAVE: u8 = 0x09;
    /// "Reaction" (lights up on click).  The Windows software builds its
    /// effect table as 1..9 followed by 2+8 = 0x0a for this entry; libratbag
    /// knows the same id as "single colour breathing" on other mice.
    pub const REACTION: u8 = 0x0a;
}

pub fn effect_to_raw(e: LightEffect) -> u8 {
    match e {
        LightEffect::ColorfulStreaming => effect::COLORFUL_STREAMING,
        LightEffect::Steady => effect::STEADY,
        LightEffect::Breathing => effect::BREATHING,
        LightEffect::Tail => effect::TAIL,
        LightEffect::Neon => effect::NEON,
        LightEffect::ColorfulSteady => effect::COLORFUL_STEADY,
        LightEffect::Flicker => effect::FLICKER,
        LightEffect::StarsTwinkle => effect::STARS_TWINKLE,
        LightEffect::Wave => effect::WAVE,
        LightEffect::Reaction => effect::REACTION,
        LightEffect::None => effect::NONE,
    }
}

pub fn effect_from_raw(raw: u8) -> LightEffect {
    match raw {
        effect::COLORFUL_STREAMING => LightEffect::ColorfulStreaming,
        effect::STEADY => LightEffect::Steady,
        effect::BREATHING => LightEffect::Breathing,
        effect::TAIL => LightEffect::Tail,
        effect::NEON => LightEffect::Neon,
        effect::COLORFUL_STEADY => LightEffect::ColorfulSteady,
        effect::FLICKER => LightEffect::Flicker,
        effect::STARS_TWINKLE => LightEffect::StarsTwinkle,
        effect::WAVE => LightEffect::Wave,
        effect::REACTION => LightEffect::Reaction,
        _ => LightEffect::None,
    }
}

/// Button block entry types (byte 0 of each 4-byte slot).
pub mod btn {
    pub const NONE: u8 = 0x00;
    pub const MOUSE: u8 = 0x11; // [mask]
    pub const WHEEL: u8 = 0x12; // [0x01 up | 0xff down]
    pub const WHEEL_H: u8 = 0x13; // [0x01 right | 0xff left]
    pub const KEY: u8 = 0x21; // [modifiers, usage, usage2]
    pub const MULTIMEDIA: u8 = 0x22; // [mask1, mask2, mask3]
    pub const REPEAT_MOUSE: u8 = 0x31; // [mask, interval, count]
    pub const REPEAT_KEY: u8 = 0x32; // [usage, interval, count]
    pub const REPEAT_MODIFIER: u8 = 0x33; // [modifier mask, interval, count]
    pub const DPI_SWITCH: u8 = 0x41; // [0 loop | 1 up | 2 down]
    pub const DPI_LOCK: u8 = 0x42; // [raw dpi]
    pub const SPECIAL: u8 = 0x50; // [1 disable, 4 polling, 6 mode, 7 effect ...]
    pub const MACRO: u8 = 0x70; // [index(1-based), mode, count]
}

/// Polling rate <-> raw nibble at `cfg::RATE_FLAGS`.
pub fn rate_to_raw(hz: u16) -> u8 {
    match hz {
        125 => 1,
        250 => 2,
        500 => 3,
        _ => 4,
    }
}
pub fn rate_from_raw(raw: u8) -> u16 {
    match raw & 0x0f {
        1 => 125,
        2 => 250,
        3 => 500,
        _ => 1000,
    }
}

/// The raw DPI byte is the 1-based position in [`DPI_VALUES`]
/// (confirmed by capture: 1200 -> 0x0b, 6200 -> 0x3d, 10000 -> 0x44).
pub fn dpi_to_raw(dpi: u16) -> u8 {
    (dpi_index(dpi) + 1) as u8
}
pub fn dpi_from_raw(raw: u8) -> u16 {
    let i = (raw.max(1) as usize - 1).min(DPI_VALUES.len() - 1);
    DPI_VALUES[i]
}

fn mode_byte(speed: u8, brightness: u8) -> u8 {
    (speed.clamp(1, 3) & 0x0f) | ((brightness.clamp(1, 4) & 0x0f) << 4)
}
fn mode_speed(b: u8) -> u8 {
    (b & 0x0f).clamp(1, 3)
}
fn mode_brightness(b: u8) -> u8 {
    ((b >> 4) & 0x0f).clamp(1, 4)
}

/// Brightness shown as a percentage by the "Steady" panel.
pub fn brightness_to_percent(b: u8) -> u8 {
    match b.clamp(1, 4) {
        1 => 25,
        2 => 50,
        3 => 75,
        _ => 100,
    }
}
pub fn brightness_from_percent(p: u8) -> u8 {
    match p {
        0..=37 => 1,
        38..=62 => 2,
        63..=87 => 3,
        _ => 4,
    }
}

/// Build the 6-byte command report.
pub fn cmd_report(cmd: u8, args: &[u8]) -> [u8; CMD_LEN] {
    let mut r = [0u8; CMD_LEN];
    r[0] = REPORT_CMD;
    r[1] = cmd;
    for (i, a) in args.iter().take(4).enumerate() {
        r[2 + i] = *a;
    }
    r
}

/// Command selecting the config / button block of `profile` (0-based).
pub fn config_cmd(profile: u8) -> u8 {
    CMD_CONFIG + 0x10 * profile
}
pub fn buttons_cmd(profile: u8) -> u8 {
    CMD_BUTTONS + 0x10 * profile
}

// ---------------------------------------------------------------------------
// Config block
// ---------------------------------------------------------------------------

/// A raw config block as read from the mouse.  We keep every byte so that
/// fields we do not understand are written back unchanged.
#[derive(Clone, Debug)]
pub struct ConfigBlock {
    pub data: Vec<u8>,
}

impl ConfigBlock {
    pub fn from_report(report: &[u8], len: usize) -> Result<ConfigBlock> {
        if len < CONFIG_MIN_LEN || report.len() < len {
            bail!("config block too short ({len} bytes)");
        }
        Ok(ConfigBlock { data: report[..len].to_vec() })
    }

    pub fn sensor(&self) -> u8 {
        self.data[cfg::SENSOR]
    }
    pub fn xy_independent(&self) -> bool {
        self.data[cfg::RATE_FLAGS] & 0x80 != 0
    }

    /// Copy DPI, lighting and polling-rate settings of `p` into the block.
    pub fn apply_profile(&mut self, p: &Profile) {
        let d = &mut self.data;
        // --- polling rate (low nibble), keep the flag nibble but clear XY-independent
        d[cfg::RATE_FLAGS] = (d[cfg::RATE_FLAGS] & 0x70) | rate_to_raw(p.polling_rate);
        // --- DPI stages
        let mut disabled = 0u8;
        let mut active_among_enabled = 1u8;
        let mut seen_enabled = 0u8;
        for (i, s) in p.dpi.slots.iter().enumerate() {
            if !s.enabled {
                disabled |= 1 << i;
            } else {
                seen_enabled += 1;
                if i == p.dpi.current {
                    active_among_enabled = seen_enabled;
                }
            }
            d[cfg::DPI_VALUES + i] = dpi_to_raw(s.dpi);
            d[cfg::DPI_COLORS + i * 3..cfg::DPI_COLORS + i * 3 + 3].copy_from_slice(&s.color);
        }
        // the firmware has 8 slots; the two the UI does not show stay disabled
        for i in NUM_DPI..8 {
            disabled |= 1 << i;
        }
        d[cfg::DPI_COUNT_ACTIVE] = (NUM_DPI as u8 & 0x0f) | (active_among_enabled << 4);
        d[cfg::DPI_DISABLED] = disabled;
        // --- lighting
        let l = &p.lighting;
        d[cfg::EFFECT] = effect_to_raw(l.effect);
        d[cfg::STREAMING_MODE] = mode_byte(l.streaming_speed, l.streaming_brightness);
        d[cfg::STREAMING_DIR] = l.streaming_direction & 1;
        d[cfg::STEADY_MODE] = (l.steady_brightness.clamp(1, 4)) << 4;
        d[cfg::STEADY_COLOR..cfg::STEADY_COLOR + 3].copy_from_slice(&l.steady_color);
        d[cfg::BREATHING_MODE] = mode_byte(l.breathing_speed, l.breathing_brightness);
        let n = l.breathing_colors.len().clamp(1, 7);
        d[cfg::BREATHING_COUNT] = n as u8;
        for i in 0..7 {
            let c = l.breathing_colors.get(i).copied().unwrap_or([0, 0, 0]);
            d[cfg::BREATHING_COLORS + i * 3..cfg::BREATHING_COLORS + i * 3 + 3].copy_from_slice(&c);
        }
        d[cfg::TAIL_MODE] = mode_byte(l.tail_speed, l.tail_brightness);
        d[cfg::NEON_MODE] = mode_byte(l.neon_speed, l.neon_brightness);
        d[cfg::COLORFUL_STEADY_MODE] = (l.colorful_steady_brightness.clamp(1, 4)) << 4;
        for i in 0..6 {
            d[cfg::COLORFUL_STEADY_COLORS + i * 3..cfg::COLORFUL_STEADY_COLORS + i * 3 + 3]
                .copy_from_slice(&l.colorful_steady_colors[i]);
        }
        d[cfg::FLICKER_MODE] = mode_byte(l.flicker_speed, l.flicker_brightness);
        for i in 0..2 {
            d[cfg::FLICKER_COLORS + i * 3..cfg::FLICKER_COLORS + i * 3 + 3]
                .copy_from_slice(&l.flicker_colors[i]);
        }
        d[cfg::STARS_MODE] = mode_byte(l.stars_speed, l.stars_brightness);
        d[cfg::WAVE_MODE] = mode_byte(l.wave_speed, l.wave_brightness);
        if d.len() > cfg::REACTION_COLOR + 3 {
            d[cfg::REACTION_MODE] = l.reaction_speed.clamp(1, 3);
            d[cfg::REACTION_COLOR..cfg::REACTION_COLOR + 3].copy_from_slice(&l.reaction_color);
        }
    }

    /// Extract DPI, lighting and polling rate into `p`.
    pub fn read_into_profile(&self, p: &mut Profile) {
        let d = &self.data;
        p.polling_rate = rate_from_raw(d[cfg::RATE_FLAGS]);
        let disabled = d[cfg::DPI_DISABLED];
        let active = (d[cfg::DPI_COUNT_ACTIVE] >> 4).max(1);
        let xy = self.xy_independent();
        let mut seen = 0u8;
        p.dpi.current = 0;
        for i in 0..NUM_DPI {
            let s = &mut p.dpi.slots[i];
            s.enabled = disabled & (1 << i) == 0;
            let raw = if xy { d[cfg::DPI_VALUES + i * 2] } else { d[cfg::DPI_VALUES + i] };
            s.dpi = dpi_from_raw(raw);
            s.color.copy_from_slice(&d[cfg::DPI_COLORS + i * 3..cfg::DPI_COLORS + i * 3 + 3]);
            if s.enabled {
                seen += 1;
                if seen == active {
                    p.dpi.current = i;
                }
            }
        }
        let l = &mut p.lighting;
        l.effect = effect_from_raw(d[cfg::EFFECT]);
        l.streaming_speed = mode_speed(d[cfg::STREAMING_MODE]);
        l.streaming_brightness = mode_brightness(d[cfg::STREAMING_MODE]);
        l.streaming_direction = d[cfg::STREAMING_DIR] & 1;
        l.steady_brightness = mode_brightness(d[cfg::STEADY_MODE]);
        l.steady_color.copy_from_slice(&d[cfg::STEADY_COLOR..cfg::STEADY_COLOR + 3]);
        l.breathing_speed = mode_speed(d[cfg::BREATHING_MODE]);
        l.breathing_brightness = mode_brightness(d[cfg::BREATHING_MODE]);
        let n = (d[cfg::BREATHING_COUNT] as usize).clamp(1, 7);
        l.breathing_colors = (0..n)
            .map(|i| {
                let mut c = [0u8; 3];
                c.copy_from_slice(&d[cfg::BREATHING_COLORS + i * 3..cfg::BREATHING_COLORS + i * 3 + 3]);
                c
            })
            .collect();
        l.tail_speed = mode_speed(d[cfg::TAIL_MODE]);
        l.tail_brightness = mode_brightness(d[cfg::TAIL_MODE]);
        l.neon_speed = mode_speed(d[cfg::NEON_MODE]);
        l.neon_brightness = mode_brightness(d[cfg::NEON_MODE]);
        l.colorful_steady_brightness = mode_brightness(d[cfg::COLORFUL_STEADY_MODE]);
        for i in 0..6 {
            l.colorful_steady_colors[i]
                .copy_from_slice(&d[cfg::COLORFUL_STEADY_COLORS + i * 3..cfg::COLORFUL_STEADY_COLORS + i * 3 + 3]);
        }
        l.flicker_speed = mode_speed(d[cfg::FLICKER_MODE]);
        l.flicker_brightness = mode_brightness(d[cfg::FLICKER_MODE]);
        for i in 0..2 {
            l.flicker_colors[i].copy_from_slice(&d[cfg::FLICKER_COLORS + i * 3..cfg::FLICKER_COLORS + i * 3 + 3]);
        }
        l.stars_speed = mode_speed(d[cfg::STARS_MODE]);
        l.stars_brightness = mode_brightness(d[cfg::STARS_MODE]);
        l.wave_speed = mode_speed(d[cfg::WAVE_MODE]);
        l.wave_brightness = mode_brightness(d[cfg::WAVE_MODE]);
        if d.len() > cfg::REACTION_COLOR + 3 {
            l.reaction_speed = mode_speed(d[cfg::REACTION_MODE]);
            l.reaction_color.copy_from_slice(&d[cfg::REACTION_COLOR..cfg::REACTION_COLOR + 3]);
        }
    }

    /// Serialise as a 520-byte SET_FEATURE report for `profile`.
    pub fn to_report(&self, profile: u8) -> Vec<u8> {
        let mut r = vec![0u8; DATA_LEN];
        r[..self.data.len()].copy_from_slice(&self.data);
        r[0] = REPORT_DATA;
        r[cfg::CMD] = config_cmd(profile);
        r[2] = 0;
        r[cfg::WRITE_LEN] = (self.data.len() - 8) as u8;
        r
    }
}

// ---------------------------------------------------------------------------
// Button block
// ---------------------------------------------------------------------------

/// Encode one key function into its 4-byte hardware slot.
/// `macro_index` is the 1-based macro buffer used when the function is a macro.
pub fn encode_key(f: &KeyFunction, macro_index: u8) -> Result<[u8; 4]> {
    use crate::hidkeys::mouse_mask as mm;
    Ok(match f {
        KeyFunction::LeftButton => [btn::MOUSE, mm::LEFT, 0, 0],
        KeyFunction::RightButton => [btn::MOUSE, mm::RIGHT, 0, 0],
        KeyFunction::MiddleButton => [btn::MOUSE, mm::MIDDLE, 0, 0],
        KeyFunction::Forward => [btn::MOUSE, mm::FORWARD, 0, 0],
        KeyFunction::Back => [btn::MOUSE, mm::BACK, 0, 0],
        KeyFunction::ScrollUp => [btn::WHEEL, 0x01, 0, 0],
        KeyFunction::ScrollDown => [btn::WHEEL, 0xff, 0, 0],
        KeyFunction::ThreeClick => [btn::REPEAT_MOUSE, mm::LEFT, 0x32, 3],
        KeyFunction::FireKey { button, interval, count } => {
            let mask = match button {
                FireButton::Left => mm::LEFT,
                FireButton::Right => mm::RIGHT,
                FireButton::Middle => mm::MIDDLE,
                FireButton::Forward => mm::FORWARD,
                FireButton::Back => mm::BACK,
            };
            [btn::REPEAT_MOUSE, mask, (*interval).max(5), *count]
        }
        KeyFunction::KeyCombination { modifiers, key } => {
            if *modifiers == 0 && *key == 0 {
                [btn::SPECIAL, 0x01, 0, 0]
            } else {
                [btn::KEY, *modifiers, *key, 0]
            }
        }
        KeyFunction::Macro { mode, count, .. } => {
            let m = match mode {
                MacroLoop::Count => 1,
                MacroLoop::UntilReleased => 2,
                MacroLoop::UntilAnyKey => 4,
            };
            [btn::MACRO, macro_index, m, (*count).max(1)]
        }
        KeyFunction::Disable => [btn::SPECIAL, 0x01, 0, 0],
        KeyFunction::DpiPlus => [btn::DPI_SWITCH, 0x01, 0, 0],
        KeyFunction::DpiMinus => [btn::DPI_SWITCH, 0x02, 0, 0],
        KeyFunction::DpiLoop => [btn::DPI_SWITCH, 0x00, 0, 0],
        KeyFunction::Multimedia(m) => {
            let (a, b, c) = match m {
                MediaKey::Next => (0x01, 0, 0),
                MediaKey::Previous => (0x02, 0, 0),
                MediaKey::Stop => (0x04, 0, 0),
                MediaKey::PlayPause => (0x08, 0, 0),
                MediaKey::Mute => (0x10, 0, 0),
                MediaKey::VolumeUp => (0x40, 0, 0),
                MediaKey::VolumeDown => (0x80, 0, 0),
                MediaKey::MediaPlayer => (0, 0x01, 0),
                MediaKey::Explorer => (0, 0x02, 0),
                MediaKey::Email => (0, 0x10, 0),
                MediaKey::Calculator => (0, 0x20, 0),
                MediaKey::HomePage => (0, 0, 0x02),
            };
            [btn::MULTIMEDIA, a, b, c]
        }
        KeyFunction::DpiLock(dpi) => [btn::DPI_LOCK, dpi_to_raw(*dpi), 0, 0],
    })
}

/// Decode a hardware slot back into a key function (used when importing
/// the configuration currently stored in the mouse).  `macro_names` maps
/// 1-based macro indices to names.
pub fn decode_key(slot: &[u8], macro_names: &[String]) -> KeyFunction {
    use crate::hidkeys::mouse_mask as mm;
    let (t, a, b, c) = (slot[0], slot[1], slot[2], slot[3]);
    match t {
        btn::MOUSE => match a {
            mm::LEFT => KeyFunction::LeftButton,
            mm::RIGHT => KeyFunction::RightButton,
            mm::MIDDLE => KeyFunction::MiddleButton,
            mm::FORWARD => KeyFunction::Forward,
            mm::BACK => KeyFunction::Back,
            _ => KeyFunction::Disable,
        },
        btn::WHEEL => {
            if a == 0xff {
                KeyFunction::ScrollDown
            } else {
                KeyFunction::ScrollUp
            }
        }
        btn::KEY => KeyFunction::KeyCombination { modifiers: a, key: b },
        btn::MULTIMEDIA => {
            let m = match (a, b, c) {
                (0x01, _, _) => MediaKey::Next,
                (0x02, _, _) => MediaKey::Previous,
                (0x04, _, _) => MediaKey::Stop,
                (0x08, _, _) => MediaKey::PlayPause,
                (0x10, _, _) => MediaKey::Mute,
                (0x40, _, _) => MediaKey::VolumeUp,
                (0x80, _, _) => MediaKey::VolumeDown,
                (_, 0x01, _) => MediaKey::MediaPlayer,
                (_, 0x02, _) => MediaKey::Explorer,
                (_, 0x10, _) => MediaKey::Email,
                (_, 0x20, _) => MediaKey::Calculator,
                _ => MediaKey::HomePage,
            };
            KeyFunction::Multimedia(m)
        }
        btn::REPEAT_MOUSE => {
            if a == mm::LEFT && c == 3 && b == 0x32 {
                KeyFunction::ThreeClick
            } else {
                let button = match a {
                    mm::RIGHT => FireButton::Right,
                    mm::MIDDLE => FireButton::Middle,
                    mm::FORWARD => FireButton::Forward,
                    mm::BACK => FireButton::Back,
                    _ => FireButton::Left,
                };
                KeyFunction::FireKey { button, interval: b, count: c }
            }
        }
        btn::DPI_SWITCH => match a {
            1 => KeyFunction::DpiPlus,
            2 => KeyFunction::DpiMinus,
            _ => KeyFunction::DpiLoop,
        },
        btn::DPI_LOCK => KeyFunction::DpiLock(dpi_from_raw(a)),
        btn::MACRO => {
            let name = macro_names
                .get(a.saturating_sub(1) as usize)
                .cloned()
                .unwrap_or_else(|| format!("Macro {a}"));
            let mode = match b {
                2 => MacroLoop::UntilReleased,
                4 => MacroLoop::UntilAnyKey,
                _ => MacroLoop::Count,
            };
            KeyFunction::Macro { name, mode, count: c.max(1) }
        }
        _ => KeyFunction::Disable,
    }
}

/// Build the 520-byte button block for `profile`.  Returns the report and
/// the list of (macro name, 1-based index) that must be uploaded with
/// [`macro_report`].
/// Macro buffers referenced by a button block: (macro name, 1-based index).
pub type MacroRefs = Vec<(String, u8)>;

pub fn buttons_report(p: &Profile, profile: u8) -> Result<(Vec<u8>, MacroRefs)> {
    if p.keys.len() != NUM_KEYS {
        bail!("profile has {} keys, expected {NUM_KEYS}", p.keys.len());
    }
    let mut r = vec![0u8; DATA_LEN];
    r[0] = REPORT_DATA;
    r[1] = buttons_cmd(profile);
    r[3] = (BUTTON_BLOCK_LEN - 8) as u8;
    let mut macros: MacroRefs = Vec::new();
    // slots without a physical key are written as "Disable" by the original
    for i in NUM_KEYS..BUTTON_SLOTS {
        r[8 + i * 4..8 + i * 4 + 4].copy_from_slice(&[btn::SPECIAL, 0x01, 0, 0]);
    }
    for (i, f) in p.keys.iter().enumerate() {
        let mut idx = 0u8;
        if let KeyFunction::Macro { name, .. } = f {
            if p.macro_by_name(name).is_none() {
                return Err(anyhow!("macro '{name}' does not exist"));
            }
            idx = match macros.iter().find(|(n, _)| n == name) {
                Some((_, i)) => *i,
                None => {
                    let i = macros.len() as u8 + 1;
                    macros.push((name.clone(), i));
                    i
                }
            };
        }
        let slot = encode_key(f, idx)?;
        r[8 + i * 4..8 + i * 4 + 4].copy_from_slice(&slot);
    }
    Ok((r, macros))
}

/// Decode a button block into key functions.
pub fn decode_buttons(report: &[u8], macro_names: &[String]) -> Vec<KeyFunction> {
    (0..NUM_KEYS)
        .map(|i| decode_key(&report[8 + i * 4..8 + i * 4 + 4], macro_names))
        .collect()
}

// ---------------------------------------------------------------------------
// Macro block
// ---------------------------------------------------------------------------

pub mod macro_cmd {
    pub const BUTTON_PRESS: u8 = 0x10;
    pub const BUTTON_RELEASE: u8 = 0x90;
    pub const KEY_PRESS: u8 = 0x50;
    pub const KEY_RELEASE: u8 = 0xd0;
}

/// Build the macro upload report.  As in the firmware, a delay is not an
/// event of its own: it is stored in the delay byte of the *preceding*
/// event (1 = no delay, max 255 ms).  Longer delays are spread over filler
/// repeats of the preceding event, which are no-ops for the mouse.
pub fn macro_report(m: &Macro, index: u8) -> Result<Vec<u8>> {
    let mut events: Vec<[u8; 3]> = Vec::new();
    for ev in &m.events {
        match ev {
            MacroEvent::Delay(ms) => {
                let Some(last) = events.last_mut() else { continue };
                let mut remaining = *ms as u32 + (last[1] as u32 - 1);
                last[1] = remaining.min(255) as u8;
                remaining = remaining.saturating_sub(255);
                while remaining > 0 {
                    let filler = [events[events.len() - 1][0], remaining.min(255) as u8, events[events.len() - 1][2]];
                    events.push(filler);
                    remaining = remaining.saturating_sub(255);
                }
            }
            _ => {
                let (cmd, code) = match ev {
                    MacroEvent::KeyDown(k) => (macro_cmd::KEY_PRESS, *k),
                    MacroEvent::KeyUp(k) => (macro_cmd::KEY_RELEASE, *k),
                    MacroEvent::MouseDown(b) => (macro_cmd::BUTTON_PRESS, *b),
                    MacroEvent::MouseUp(b) => (macro_cmd::BUTTON_RELEASE, *b),
                    MacroEvent::Delay(_) => unreachable!(),
                };
                events.push([cmd, 1, code]);
            }
        }
    }
    if events.len() > MACRO_MAX_EVENTS {
        bail!("macro '{}' has too many events ({} > {MACRO_MAX_EVENTS})", m.name, events.len());
    }
    let mut r = vec![0u8; DATA_LEN];
    r[0] = REPORT_DATA;
    r[1] = CMD_MACRO;
    r[2] = 0x02;
    r[3] = 0;
    r[8] = index;
    r[9] = 0;
    r[10] = events.len() as u8;
    for (i, e) in events.iter().enumerate() {
        r[11 + i * 3..11 + i * 3 + 3].copy_from_slice(e);
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_roundtrip() {
        for d in DPI_VALUES {
            assert_eq!(dpi_from_raw(dpi_to_raw(d)), d);
        }
        // values taken from a capture of the Windows software
        assert_eq!(dpi_to_raw(1200), 0x0b);
        assert_eq!(dpi_to_raw(6200), 0x3d);
        assert_eq!(dpi_to_raw(10000), 0x44);
        assert_eq!(dpi_to_raw(12400), 0x49);
        assert_eq!(DpiConfig::clamp_dpi(9700), 10000);
        assert_eq!(DpiConfig::clamp_dpi(6300), 6200);
    }

    #[test]
    fn key_roundtrip() {
        let p = Profile::factory("x");
        let (rep, _) = buttons_report(&p, 0).unwrap();
        let back = decode_buttons(&rep, &[]);
        assert_eq!(back, p.keys);
    }

    #[test]
    fn macro_delays_attach_to_previous_event() {
        let m = Macro {
            name: "t".into(),
            events: vec![
                MacroEvent::KeyDown(0x04),
                MacroEvent::Delay(120),
                MacroEvent::KeyUp(0x04),
                MacroEvent::Delay(600),
                MacroEvent::MouseDown(1),
                MacroEvent::MouseUp(1),
            ],
        };
        let r = macro_report(&m, 3).unwrap();
        assert_eq!(&r[..4], &[REPORT_DATA, CMD_MACRO, 0x02, 0]);
        assert_eq!(r[8], 3);
        let n = r[10] as usize;
        let ev: Vec<[u8; 3]> = (0..n).map(|i| [r[11 + i * 3], r[12 + i * 3], r[13 + i * 3]]).collect();
        assert_eq!(
            ev,
            vec![
                [0x50, 120, 0x04],
                [0xd0, 255, 0x04],
                [0xd0, 255, 0x04],
                [0xd0, 90, 0x04],
                [0x10, 1, 1],
                [0x90, 1, 1],
            ]
        );
    }

    #[test]
    fn config_roundtrip() {
        let mut data = vec![0u8; 167];
        data[cfg::SENSOR] = SENSOR_PMW3327;
        let mut blk = ConfigBlock { data };
        let mut p = Profile::factory("x");
        p.lighting.effect = LightEffect::Breathing;
        p.dpi.slots[2].enabled = false;
        p.dpi.current = 3;
        p.polling_rate = 500;
        blk.apply_profile(&p);
        let mut q = Profile::factory("y");
        blk.read_into_profile(&mut q);
        assert_eq!(q.dpi, p.dpi);
        assert_eq!(q.lighting, p.lighting);
        assert_eq!(q.polling_rate, 500);
        let rep = blk.to_report(0);
        assert_eq!(rep[0], REPORT_DATA);
        assert_eq!(rep[1], CMD_CONFIG);
        assert_eq!(rep[3], 167 - 8);
    }
}
