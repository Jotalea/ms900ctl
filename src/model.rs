//! Data model shared by the GUI, the on-disk profile store and the device
//! protocol.  Everything here is plain data; the HID encoding lives in
//! `protocol.rs`.

use serde::{Deserialize, Serialize};

/// Number of programmable keys shown by the original software
/// (1-8 on the main body, S1-S6 on the side grid).
pub const NUM_KEYS: usize = 14;
/// Number of DPI stages exposed by the UI (the firmware has 8 slots).
pub const NUM_DPI: usize = 6;
/// DPI values the firmware accepts, in hardware order (the raw byte sent to
/// the mouse is `index + 1`).  This is the PMW3327 table built into the
/// Windows software: 100 DPI steps up to 6200, coarser above.
pub const DPI_VALUES: [u16; 73] = [
    200, 300, 400, 500, 600, 700, 800, 900, 1000, 1100, 1200, 1300, 1400, 1500, 1600, 1700, 1800, 1900,
    2000, 2100, 2200, 2300, 2400, 2500, 2600, 2700, 2800, 2900, 3000, 3100, 3200, 3300, 3400, 3500, 3600,
    3700, 3800, 3900, 4000, 4100, 4200, 4300, 4400, 4500, 4600, 4700, 4800, 4900, 5000, 5100, 5200, 5300,
    5400, 5500, 5600, 5700, 5800, 5900, 6000, 6100, 6200, 6500, 7000, 7500, 8000, 8500, 9000, 10000,
    10500, 11000, 11500, 12000, 12400,
];
pub const DPI_MIN: u16 = DPI_VALUES[0];
pub const DPI_MAX: u16 = DPI_VALUES[DPI_VALUES.len() - 1];
/// Step of the fine part of the table (used for the DPI lock menu).
pub const DPI_STEP: u16 = 100;

/// Index of the table entry nearest to `dpi`.
pub fn dpi_index(dpi: u16) -> usize {
    let mut best = 0;
    for (i, v) in DPI_VALUES.iter().enumerate() {
        if (*v as i32 - dpi as i32).abs() < (DPI_VALUES[best] as i32 - dpi as i32).abs() {
            best = i;
        }
    }
    best
}

/// Labels drawn next to each key button, in UI order.
pub const KEY_LABELS: [&str; NUM_KEYS] = [
    "1", "2", "3", "4", "5", "6", "7", "8", "S1", "S2", "S3", "S4", "S5", "S6",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FireButton {
    Left,
    Right,
    Middle,
    Forward,
    Back,
}

impl FireButton {
    pub const ALL: [FireButton; 5] = [
        FireButton::Left,
        FireButton::Right,
        FireButton::Middle,
        FireButton::Forward,
        FireButton::Back,
    ];
    pub fn name(self) -> &'static str {
        match self {
            FireButton::Left => "Left Button",
            FireButton::Right => "Right Button",
            FireButton::Middle => "Middle Button",
            FireButton::Forward => "Forward",
            FireButton::Back => "Back",
        }
    }
}

/// Multimedia / consumer-control keys offered by the "Multimedia" submenu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaKey {
    MediaPlayer,
    PlayPause,
    Next,
    Previous,
    Stop,
    Mute,
    VolumeUp,
    VolumeDown,
    Email,
    Calculator,
    Explorer,
    HomePage,
}

impl MediaKey {
    /// Menu order of the original software.
    pub const ALL: [MediaKey; 12] = [
        MediaKey::MediaPlayer,
        MediaKey::PlayPause,
        MediaKey::Next,
        MediaKey::Previous,
        MediaKey::Stop,
        MediaKey::Mute,
        MediaKey::VolumeUp,
        MediaKey::VolumeDown,
        MediaKey::Email,
        MediaKey::Calculator,
        MediaKey::Explorer,
        MediaKey::HomePage,
    ];
    pub fn name(self) -> &'static str {
        match self {
            MediaKey::MediaPlayer => "Media Player",
            MediaKey::PlayPause => "Play/Pause",
            MediaKey::Next => "Next",
            MediaKey::Previous => "Previous",
            MediaKey::Stop => "Stop",
            MediaKey::Mute => "Mute",
            MediaKey::VolumeUp => "Volume up",
            MediaKey::VolumeDown => "Volume down",
            MediaKey::Email => "Email",
            MediaKey::Calculator => "Calculator",
            MediaKey::Explorer => "Explorer",
            MediaKey::HomePage => "Home page",
        }
    }
}

/// How a macro bound to a key repeats.  Order matches the original dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MacroLoop {
    /// "Specified cycle times"
    #[default]
    Count,
    /// "Cycle until the key released"
    UntilReleased,
    /// "Cycle until any key pressed"
    UntilAnyKey,
}

impl MacroLoop {
    pub const ALL: [MacroLoop; 3] = [MacroLoop::Count, MacroLoop::UntilReleased, MacroLoop::UntilAnyKey];
    pub fn name(self) -> &'static str {
        match self {
            MacroLoop::Count => "Specified cycle times",
            MacroLoop::UntilReleased => "Cycle until the key released",
            MacroLoop::UntilAnyKey => "Cycle until any key pressed",
        }
    }
}

/// Keyboard modifier bit mask (HID boot-protocol order, left-hand keys).
pub mod modifier {
    pub const CTRL: u8 = 0x01;
    pub const SHIFT: u8 = 0x02;
    pub const ALT: u8 = 0x04;
    pub const WIN: u8 = 0x08;
}

/// The function assigned to a key.  One variant per entry of the original
/// right-click menu (plus the submenu entries).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KeyFunction {
    LeftButton,
    RightButton,
    MiddleButton,
    Forward,
    Back,
    ScrollUp,
    ScrollDown,
    ThreeClick,
    /// "Fire key": repeat a mouse button `count` times (0 = while held)
    /// every `interval` ms.
    FireKey { button: FireButton, interval: u8, count: u8 },
    /// "Key Combination": modifiers (see [`modifier`]) + HID keyboard usage
    /// (0 = modifiers only).
    KeyCombination { modifiers: u8, key: u8 },
    /// Run the macro called `name` from the profile's macro list.
    Macro { name: String, mode: MacroLoop, count: u8 },
    Disable,
    DpiPlus,
    DpiMinus,
    DpiLoop,
    Multimedia(MediaKey),
    /// Hold to temporarily switch to the given DPI.
    DpiLock(u16),
}

impl KeyFunction {
    /// Text shown on the key button in the main window.
    pub fn label(&self) -> String {
        match self {
            KeyFunction::LeftButton => "Left Button".into(),
            KeyFunction::RightButton => "Right Button".into(),
            KeyFunction::MiddleButton => "Middle Button".into(),
            KeyFunction::Forward => "Forward".into(),
            KeyFunction::Back => "Back".into(),
            KeyFunction::ScrollUp => "Scroll Up".into(),
            KeyFunction::ScrollDown => "Scroll Down".into(),
            KeyFunction::ThreeClick => "Three Click".into(),
            KeyFunction::FireKey { button, .. } => format!("Fire key - {}", button.name()),
            KeyFunction::KeyCombination { modifiers, key } => key_combination_label(*modifiers, *key),
            KeyFunction::Macro { name, .. } => format!("Macro - {name}"),
            KeyFunction::Disable => "Disable".into(),
            KeyFunction::DpiPlus => "DPI +".into(),
            KeyFunction::DpiMinus => "DPI -".into(),
            KeyFunction::DpiLoop => "DPI Loop".into(),
            KeyFunction::Multimedia(m) => m.name().into(),
            KeyFunction::DpiLock(dpi) => format!("DPI Lock - {dpi}"),
        }
    }
}

/// Human readable name of a key combination, e.g. "Ctrl + Shift + A" or "Win".
pub fn key_combination_label(modifiers: u8, key: u8) -> String {
    let mut parts: Vec<String> = Vec::new();
    if modifiers & modifier::CTRL != 0 {
        parts.push("Ctrl".into());
    }
    if modifiers & modifier::SHIFT != 0 {
        parts.push("Shift".into());
    }
    if modifiers & modifier::ALT != 0 {
        parts.push("Alt".into());
    }
    if modifiers & modifier::WIN != 0 {
        parts.push("Win".into());
    }
    if key != 0 {
        parts.push(crate::hidkeys::usage_name(key).to_string());
    }
    if parts.is_empty() {
        "Disable".into()
    } else {
        parts.join(" + ")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DpiSlot {
    pub enabled: bool,
    pub dpi: u16,
    pub color: [u8; 3],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DpiConfig {
    pub slots: [DpiSlot; NUM_DPI],
    /// Index (0-based) of the active stage.
    pub current: usize,
}

impl Default for DpiConfig {
    fn default() -> Self {
        // DPI= / DC= defaults from the original Cfg.ini [SENSOR_1] section.
        let dpis = [1200u16, 2400, 3600, 4800, 6200, 10000];
        let colors = [
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [255, 255, 0],
            [0, 255, 255],
            [255, 0, 255],
        ];
        let mut slots = [DpiSlot { enabled: true, dpi: 0, color: [0; 3] }; NUM_DPI];
        for i in 0..NUM_DPI {
            slots[i].dpi = dpis[i];
            slots[i].color = colors[i];
        }
        DpiConfig { slots, current: 0 }
    }
}

impl DpiConfig {
    pub fn enabled_count(&self) -> usize {
        self.slots.iter().filter(|s| s.enabled).count()
    }
    /// Snap to the nearest value the firmware supports.
    pub fn clamp_dpi(dpi: u16) -> u16 {
        DPI_VALUES[dpi_index(dpi)]
    }
}

/// Lighting effects, in the order of the original "Effect" combo box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LightEffect {
    ColorfulStreaming,
    Steady,
    Breathing,
    Tail,
    Neon,
    #[default]
    ColorfulSteady,
    Flicker,
    StarsTwinkle,
    Wave,
    Reaction,
    None,
}

impl LightEffect {
    pub const ALL: [LightEffect; 11] = [
        LightEffect::ColorfulStreaming,
        LightEffect::Steady,
        LightEffect::Breathing,
        LightEffect::Tail,
        LightEffect::Neon,
        LightEffect::ColorfulSteady,
        LightEffect::Flicker,
        LightEffect::StarsTwinkle,
        LightEffect::Wave,
        LightEffect::Reaction,
        LightEffect::None,
    ];
    pub fn name(self) -> &'static str {
        match self {
            LightEffect::ColorfulStreaming => "Colorful Streaming",
            LightEffect::Steady => "Steady",
            LightEffect::Breathing => "Breathing",
            LightEffect::Tail => "Tail",
            LightEffect::Neon => "Neon",
            LightEffect::ColorfulSteady => "Colorful Steady",
            LightEffect::Flicker => "Flicker",
            LightEffect::StarsTwinkle => "Stars twinkle",
            LightEffect::Wave => "Wave",
            LightEffect::Reaction => "Reaction",
            LightEffect::None => "None",
        }
    }
}

/// Lighting settings.  Every effect keeps its own parameters so switching
/// back and forth in the combo box does not lose them (the original stores
/// them the same way, one block per effect, inside the device config).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lighting {
    pub effect: LightEffect,
    /// 1 (slow) .. 3 (fast) for animated effects.
    pub streaming_speed: u8,
    pub streaming_brightness: u8,
    pub streaming_direction: u8,
    /// "Steady" single colour + brightness.
    pub steady_color: [u8; 3],
    pub steady_brightness: u8,
    /// "Breathing": up to seven colours cycling.
    pub breathing_speed: u8,
    pub breathing_brightness: u8,
    pub breathing_colors: Vec<[u8; 3]>,
    pub tail_speed: u8,
    pub tail_brightness: u8,
    pub neon_speed: u8,
    pub neon_brightness: u8,
    /// "Colorful Steady": one colour per LED zone (6 swatches in the UI).
    pub colorful_steady_colors: [[u8; 3]; 6],
    pub colorful_steady_brightness: u8,
    pub flicker_speed: u8,
    pub flicker_brightness: u8,
    pub flicker_colors: [[u8; 3]; 2],
    pub stars_speed: u8,
    pub stars_brightness: u8,
    pub wave_speed: u8,
    pub wave_brightness: u8,
    pub reaction_speed: u8,
    pub reaction_brightness: u8,
    pub reaction_color: [u8; 3],
}

impl Default for Lighting {
    fn default() -> Self {
        Lighting {
            effect: LightEffect::ColorfulSteady,
            streaming_speed: 2,
            streaming_brightness: 3,
            streaming_direction: 0,
            steady_color: [255, 0, 0],
            steady_brightness: 3,
            breathing_speed: 2,
            breathing_brightness: 3,
            breathing_colors: vec![
                [255, 0, 0],
                [0, 255, 0],
                [0, 0, 255],
                [255, 255, 0],
                [0, 255, 255],
                [255, 0, 255],
                [255, 255, 255],
            ],
            tail_speed: 2,
            tail_brightness: 3,
            neon_speed: 2,
            neon_brightness: 3,
            colorful_steady_colors: [
                [255, 0, 0],
                [0, 255, 0],
                [0, 0, 255],
                [255, 255, 0],
                [0, 255, 255],
                [255, 0, 255],
            ],
            colorful_steady_brightness: 3,
            flicker_speed: 2,
            flicker_brightness: 3,
            flicker_colors: [[255, 0, 0], [0, 0, 255]],
            stars_speed: 2,
            stars_brightness: 3,
            wave_speed: 2,
            wave_brightness: 3,
            reaction_speed: 2,
            reaction_brightness: 3,
            reaction_color: [255, 0, 0],
        }
    }
}

/// USB polling rates offered by the "Polling Rate" panel.
pub const POLLING_RATES: [u16; 4] = [125, 250, 500, 1000];

/// Windows "Mouse Properties" style settings shown in the
/// "Mouse Parameter" panel.  They are desktop settings, not mouse firmware
/// settings; see `sysmouse.rs` for how they are applied on Linux.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MouseParams {
    /// 1..=20 (Windows pointer speed slider).
    pub sensitivity: u8,
    pub enhance_precision: bool,
    /// Lines per wheel notch, 1..=100.
    pub scroll_lines: u8,
    pub scroll_one_page: bool,
    /// 200..=900 ms.
    pub double_click_ms: u16,
}

impl Default for MouseParams {
    fn default() -> Self {
        MouseParams {
            sensitivity: 10,
            enhance_precision: true,
            scroll_lines: 3,
            scroll_one_page: false,
            double_click_ms: 500,
        }
    }
}

/// One recorded macro step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MacroEvent {
    /// HID keyboard usage.
    KeyDown(u8),
    KeyUp(u8),
    /// Mouse button mask: 1 left, 2 right, 4 middle, 8 back, 16 forward.
    MouseDown(u8),
    MouseUp(u8),
    /// Delay in milliseconds (1..=65535).
    Delay(u16),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Macro {
    pub name: String,
    pub events: Vec<MacroEvent>,
}

/// A complete, user-named configuration ("Profile" combo box).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub keys: Vec<KeyFunction>,
    pub dpi: DpiConfig,
    pub lighting: Lighting,
    pub polling_rate: u16,
    pub mouse_params: MouseParams,
    pub macros: Vec<Macro>,
}

impl Profile {
    /// Factory defaults, matching `K1_1`..`K14_1` of the original `Cfg.ini`.
    pub fn factory(name: &str) -> Profile {
        Profile {
            name: name.to_string(),
            keys: vec![
                KeyFunction::LeftButton,
                KeyFunction::RightButton,
                KeyFunction::MiddleButton,
                KeyFunction::DpiPlus,
                KeyFunction::DpiMinus,
                KeyFunction::ThreeClick,
                KeyFunction::Forward,
                KeyFunction::Back,
                KeyFunction::KeyCombination { modifiers: 0, key: 0x1e }, // 1
                KeyFunction::KeyCombination { modifiers: 0, key: 0x1f }, // 2
                KeyFunction::KeyCombination { modifiers: 0, key: 0x20 }, // 3
                KeyFunction::KeyCombination { modifiers: 0, key: 0x21 }, // 4
                KeyFunction::KeyCombination { modifiers: 0, key: 0x22 }, // 5
                KeyFunction::DpiLock(300),
            ],
            dpi: DpiConfig::default(),
            lighting: Lighting::default(),
            polling_rate: 1000,
            mouse_params: MouseParams::default(),
            macros: Vec::new(),
        }
    }

    pub fn key(&self, idx: usize) -> &KeyFunction {
        &self.keys[idx]
    }

    /// True if at least one key is still a plain left click (the original
    /// refuses to apply otherwise: "At least one button must be configured as click!").
    pub fn has_left_click(&self) -> bool {
        self.keys.contains(&KeyFunction::LeftButton)
    }

    pub fn macro_by_name(&self, name: &str) -> Option<&Macro> {
        self.macros.iter().find(|m| m.name == name)
    }
}

impl Default for Profile {
    fn default() -> Self {
        Profile::factory("Profile 1")
    }
}
