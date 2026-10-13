//! HID keyboard usage table (USB HID Usage Tables, page 0x07) and helpers
//! to map egui key presses to usages.

/// Human readable name of a keyboard usage, used on the key buttons.
pub fn usage_name(usage: u8) -> &'static str {
    match usage {
        0x04 => "A", 0x05 => "B", 0x06 => "C", 0x07 => "D", 0x08 => "E", 0x09 => "F",
        0x0a => "G", 0x0b => "H", 0x0c => "I", 0x0d => "J", 0x0e => "K", 0x0f => "L",
        0x10 => "M", 0x11 => "N", 0x12 => "O", 0x13 => "P", 0x14 => "Q", 0x15 => "R",
        0x16 => "S", 0x17 => "T", 0x18 => "U", 0x19 => "V", 0x1a => "W", 0x1b => "X",
        0x1c => "Y", 0x1d => "Z",
        0x1e => "1", 0x1f => "2", 0x20 => "3", 0x21 => "4", 0x22 => "5", 0x23 => "6",
        0x24 => "7", 0x25 => "8", 0x26 => "9", 0x27 => "0",
        0x28 => "Enter", 0x29 => "Esc", 0x2a => "Backspace", 0x2b => "Tab", 0x2c => "Space",
        0x2d => "-", 0x2e => "=", 0x2f => "[", 0x30 => "]", 0x31 => "\\", 0x32 => "#",
        0x33 => ";", 0x34 => "'", 0x35 => "`", 0x36 => ",", 0x37 => ".", 0x38 => "/",
        0x39 => "Caps Lock",
        0x3a => "F1", 0x3b => "F2", 0x3c => "F3", 0x3d => "F4", 0x3e => "F5", 0x3f => "F6",
        0x40 => "F7", 0x41 => "F8", 0x42 => "F9", 0x43 => "F10", 0x44 => "F11", 0x45 => "F12",
        0x46 => "Print Screen", 0x47 => "Scroll Lock", 0x48 => "Pause", 0x49 => "Insert",
        0x4a => "Home", 0x4b => "Page Up", 0x4c => "Delete", 0x4d => "End", 0x4e => "Page Down",
        0x4f => "Right", 0x50 => "Left", 0x51 => "Down", 0x52 => "Up",
        0x53 => "Num Lock", 0x54 => "Num /", 0x55 => "Num *", 0x56 => "Num -", 0x57 => "Num +",
        0x58 => "Num Enter", 0x59 => "Num 1", 0x5a => "Num 2", 0x5b => "Num 3", 0x5c => "Num 4",
        0x5d => "Num 5", 0x5e => "Num 6", 0x5f => "Num 7", 0x60 => "Num 8", 0x61 => "Num 9",
        0x62 => "Num 0", 0x63 => "Num .", 0x64 => "\\", 0x65 => "Menu",
        0x68 => "F13", 0x69 => "F14", 0x6a => "F15", 0x6b => "F16", 0x6c => "F17", 0x6d => "F18",
        0x6e => "F19", 0x6f => "F20", 0x70 => "F21", 0x71 => "F22", 0x72 => "F23", 0x73 => "F24",
        0xe0 => "Ctrl", 0xe1 => "Shift", 0xe2 => "Alt", 0xe3 => "Win",
        0xe4 => "Right Ctrl", 0xe5 => "Right Shift", 0xe6 => "Right Alt", 0xe7 => "Right Win",
        0 => "",
        _ => "Key",
    }
}

/// Map an egui key to a HID usage.
pub fn usage_from_egui(key: egui::Key) -> Option<u8> {
    use egui::Key as K;
    Some(match key {
        K::A => 0x04, K::B => 0x05, K::C => 0x06, K::D => 0x07, K::E => 0x08, K::F => 0x09,
        K::G => 0x0a, K::H => 0x0b, K::I => 0x0c, K::J => 0x0d, K::K => 0x0e, K::L => 0x0f,
        K::M => 0x10, K::N => 0x11, K::O => 0x12, K::P => 0x13, K::Q => 0x14, K::R => 0x15,
        K::S => 0x16, K::T => 0x17, K::U => 0x18, K::V => 0x19, K::W => 0x1a, K::X => 0x1b,
        K::Y => 0x1c, K::Z => 0x1d,
        K::Num1 => 0x1e, K::Num2 => 0x1f, K::Num3 => 0x20, K::Num4 => 0x21, K::Num5 => 0x22,
        K::Num6 => 0x23, K::Num7 => 0x24, K::Num8 => 0x25, K::Num9 => 0x26, K::Num0 => 0x27,
        K::Enter => 0x28, K::Escape => 0x29, K::Backspace => 0x2a, K::Tab => 0x2b, K::Space => 0x2c,
        K::Minus => 0x2d, K::Equals => 0x2e, K::OpenBracket => 0x2f, K::CloseBracket => 0x30,
        K::Backslash => 0x31, K::Semicolon => 0x33, K::Quote => 0x34, K::Backtick => 0x35,
        K::Comma => 0x36, K::Period => 0x37, K::Slash => 0x38,
        K::F1 => 0x3a, K::F2 => 0x3b, K::F3 => 0x3c, K::F4 => 0x3d, K::F5 => 0x3e, K::F6 => 0x3f,
        K::F7 => 0x40, K::F8 => 0x41, K::F9 => 0x42, K::F10 => 0x43, K::F11 => 0x44, K::F12 => 0x45,
        K::F13 => 0x68, K::F14 => 0x69, K::F15 => 0x6a, K::F16 => 0x6b, K::F17 => 0x6c,
        K::F18 => 0x6d, K::F19 => 0x6e, K::F20 => 0x6f, K::F21 => 0x70, K::F22 => 0x71,
        K::F23 => 0x72, K::F24 => 0x73,
        K::Insert => 0x49, K::Home => 0x4a, K::PageUp => 0x4b, K::Delete => 0x4c, K::End => 0x4d,
        K::PageDown => 0x4e, K::ArrowRight => 0x4f, K::ArrowLeft => 0x50, K::ArrowDown => 0x51,
        K::ArrowUp => 0x52,
        _ => return None,
    })
}

/// Mouse button masks used by fire keys and macro events
/// (same bit order the firmware uses).
pub mod mouse_mask {
    pub const LEFT: u8 = 0x01;
    pub const RIGHT: u8 = 0x02;
    pub const MIDDLE: u8 = 0x04;
    pub const BACK: u8 = 0x08;
    pub const FORWARD: u8 = 0x10;
}

pub fn mouse_mask_name(mask: u8) -> &'static str {
    match mask {
        mouse_mask::LEFT => "Left Button",
        mouse_mask::RIGHT => "Right Button",
        mouse_mask::MIDDLE => "Middle Button",
        mouse_mask::BACK => "Back",
        mouse_mask::FORWARD => "Forward",
        _ => "Button",
    }
}
