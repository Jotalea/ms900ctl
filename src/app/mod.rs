//! egui front-end replicating the original "MS900 Gaming Mouse" window.

pub mod colorpicker;
pub mod dialogs;
pub mod macro_editor;
pub mod main_window;
pub mod menus;
pub mod panels;
pub mod skins;
pub mod widgets;

use crate::model::*;
use crate::store::Store;
use egui::{Context, Pos2, Rect};
use skins::Skins;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const WIN_W: f32 = 1030.0;
pub const WIN_H: f32 = 662.0;

/// Floating lists that close when clicking elsewhere.
#[derive(Clone, Debug, PartialEq)]
pub enum Popup {
    KeyMenu { key: usize, sub: Option<usize> },
    ProfileList,
    ProfileMenu,
    EffectList,
    PollingList,
    /// Fire-key dialog's button combo.
    FireButtonList,
    /// Macro editor's "Insert event" combo.
    InsertEventList,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Confirm {
    Restore,
    DeleteProfile,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InputAction {
    NewProfile,
    RenameProfile,
    NewMacro,
    MacroDelay,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorTarget {
    Dpi(usize),
    Steady,
    Breathing(usize),
    ColorfulSteady(usize),
    Flicker(usize),
}

#[derive(Clone, Debug)]
pub enum Dialog {
    FireKey { key: usize, button: usize, interval: String, count: String },
    KeyCombo { key: usize, modifiers: u8, usage: u8 },
    MacroPick { key: usize, selected: Option<usize>, mode: MacroLoop, count: String },
    Message { text: String, yes_no: bool, on_yes: Option<Confirm> },
    Input { title: String, text: String, action: InputAction },
    Color { target: ColorTarget, rgb: [u8; 3], r: String, g: String, b: String },
    MacroEditor(macro_editor::MacroEditor),
    Wait,
}

/// Deferred mutations collected while the UI borrows the skins.
#[derive(Debug)]
pub enum Action {
    OpenPopup(Popup),
    ClosePopup,
    OpenDialog(Dialog),
    CloseDialog,
    SetKey(usize, KeyFunction),
    SelectProfile(usize),
    AddProfile,
    DeleteProfile,
    Restore,
    Apply,
    Minimize,
    Close,
    SetPanel(usize),
    ExportProfile,
    ImportProfile,
    ReadFromMouse,
    SetColor(ColorTarget, [u8; 3]),
    Confirmed(Confirm),
    InputDone(InputAction, String),
    Save,
}

/// Serialises HID access between the poller and the apply/read workers.
pub static DEVICE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Default, Clone)]
pub struct DeviceState {
    pub connected: bool,
    pub firmware: String,
}

pub struct App {
    pub skins: Skins,
    pub store: Store,
    pub popup: Option<Popup>,
    /// Rects (popup + its anchor) drawn last frame, for click-outside detection.
    pub popup_rects: Vec<Rect>,
    pub dialogs: Vec<Dialog>,
    pub device: Arc<Mutex<DeviceState>>,
    pub apply_rx: Option<Receiver<Result<Vec<String>, String>>>,
    pub read_rx: Option<Receiver<Result<Profile, String>>>,
    pub status: String,
    pub actions: Vec<Action>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        App::from_ctx(&cc.egui_ctx)
    }

    pub fn from_ctx(ctx: &Context) -> App {
        let mut visuals = egui::Visuals::dark();
        visuals.window_shadow = egui::Shadow::NONE;
        visuals.popup_shadow = egui::Shadow::NONE;
        visuals.selection.bg_fill = widgets::LIST_ITEM;
        ctx.set_visuals(visuals);
        let skins = Skins::load(ctx);
        let store = Store::load();
        let device = Arc::new(Mutex::new(DeviceState::default()));
        spawn_device_poller(device.clone(), ctx.clone());
        App {
            skins,
            store,
            popup: None,
            popup_rects: Vec::new(),
            dialogs: Vec::new(),
            device,
            apply_rx: None,
            read_rx: None,
            status: String::new(),
            actions: Vec::new(),
        }
    }

    pub fn profile(&self) -> &Profile {
        self.store.current()
    }
    pub fn profile_mut(&mut self) -> &mut Profile {
        self.store.current_mut()
    }

    pub fn message(&mut self, text: impl Into<String>) {
        self.dialogs.push(Dialog::Message { text: text.into(), yes_no: false, on_yes: None });
    }

    pub fn save(&mut self) {
        if let Err(e) = self.store.save() {
            log::error!("saving profiles: {e}");
            self.status = format!("Could not save profiles: {e}");
        }
    }

    fn poll_workers(&mut self, ctx: &Context) {
        if let Some(rx) = &self.apply_rx {
            match rx.try_recv() {
                Ok(res) => {
                    self.apply_rx = None;
                    self.dialogs.retain(|d| !matches!(d, Dialog::Wait));
                    match res {
                        Ok(notes) => {
                            self.status = "Applied.".into();
                            for n in notes {
                                log::info!("{n}");
                            }
                        }
                        Err(e) => self.message(e),
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
                Err(_) => {
                    self.apply_rx = None;
                    self.dialogs.retain(|d| !matches!(d, Dialog::Wait));
                }
            }
        }
        if let Some(rx) = &self.read_rx {
            match rx.try_recv() {
                Ok(res) => {
                    self.read_rx = None;
                    self.dialogs.retain(|d| !matches!(d, Dialog::Wait));
                    match res {
                        Ok(p) => {
                            let name = self.store.unique_name("From mouse");
                            let mut p = p;
                            p.name = name;
                            self.store.profiles.push(p);
                            self.store.current = self.store.profiles.len() - 1;
                            self.save();
                        }
                        Err(e) => self.message(e),
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
                Err(_) => {
                    self.read_rx = None;
                    self.dialogs.retain(|d| !matches!(d, Dialog::Wait));
                }
            }
        }
    }

    fn close_popup_on_outside_click(&mut self, ctx: &Context) {
        if self.popup.is_none() {
            return;
        }
        let pressed = ctx.input(|i| i.pointer.any_pressed());
        if !pressed {
            return;
        }
        let pos = ctx.input(|i| i.pointer.interact_pos());
        let inside = pos.map(|p| self.popup_rects.iter().any(|r| r.contains(p))).unwrap_or(false);
        if !inside {
            self.popup = None;
        }
    }

    fn start_apply(&mut self) {
        let prof = self.profile().clone();
        if !prof.has_left_click() {
            self.message("At least one button must be configured as \"click\"!");
            return;
        }
        if prof.dpi.enabled_count() == 0 {
            self.message("You can not disable all DPI.");
            return;
        }
        let apply_params = self.store.apply_mouse_params;
        let (tx, rx): (Sender<Result<Vec<String>, String>>, _) = std::sync::mpsc::channel();
        self.apply_rx = Some(rx);
        self.dialogs.push(Dialog::Wait);
        std::thread::spawn(move || {
            let res = (|| -> Result<Vec<String>, String> {
                let _guard = DEVICE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
                let m = crate::device::Mouse::open().map_err(|e| format!("Device is disconnected.\n{e}"))?;
                m.apply_profile(0, &prof).map_err(|e| e.to_string())?;
                let mut notes = Vec::new();
                if apply_params {
                    notes = crate::sysmouse::apply(&prof.mouse_params);
                }
                Ok(notes)
            })();
            let _ = tx.send(res);
        });
    }

    fn start_read_from_mouse(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.read_rx = Some(rx);
        self.dialogs.push(Dialog::Wait);
        std::thread::spawn(move || {
            let _guard = DEVICE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let res = crate::device::Mouse::open()
                .and_then(|m| m.read_profile(0, "From mouse"))
                .map_err(|e| format!("Device is disconnected.\n{e}"));
            let _ = tx.send(res);
        });
    }

    fn apply_actions(&mut self, ctx: &Context) {
        let actions = std::mem::take(&mut self.actions);
        for a in actions {
            match a {
                Action::OpenPopup(p) => self.popup = Some(p),
                Action::ClosePopup => self.popup = None,
                Action::OpenDialog(d) => {
                    self.popup = None;
                    self.dialogs.push(d)
                }
                Action::CloseDialog => {
                    self.dialogs.pop();
                    self.popup = None;
                }
                Action::SetKey(i, f) => {
                    self.profile_mut().keys[i] = f;
                    self.popup = None;
                    self.save();
                }
                Action::SelectProfile(i) => {
                    if i < self.store.profiles.len() {
                        self.store.current = i;
                    }
                    self.popup = None;
                    self.save();
                }
                Action::AddProfile => {
                    let title = "Please input profile name".to_string();
                    self.dialogs.push(Dialog::Input { title, text: String::new(), action: InputAction::NewProfile });
                }
                Action::DeleteProfile => {
                    if self.store.profiles.len() <= 1 {
                        self.message("Keep at least one profile.");
                    } else {
                        self.dialogs.push(Dialog::Message {
                            text: "Are you sure you want to delete？".into(),
                            yes_no: true,
                            on_yes: Some(Confirm::DeleteProfile),
                        });
                    }
                }
                Action::Restore => self.dialogs.push(Dialog::Message {
                    text: "Are you sure you want to restore？".into(),
                    yes_no: true,
                    on_yes: Some(Confirm::Restore),
                }),
                Action::Apply => self.start_apply(),
                Action::Minimize => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                Action::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Action::SetPanel(i) => {
                    self.store.open_panel = i;
                    self.popup = None;
                    self.save();
                }
                Action::ExportProfile => {
                    self.popup = None;
                    let name = format!("{}.json", self.profile().name);
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Export profile")
                        .add_filter("Profile", &["json"])
                        .set_file_name(name)
                        .save_file()
                    {
                        if let Err(e) = crate::store::export_profile(self.profile(), &path) {
                            self.message(format!("Export failed: {e}"));
                        }
                    }
                }
                Action::ImportProfile => {
                    self.popup = None;
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Import profile")
                        .add_filter("Profile", &["json", "bkm"])
                        .pick_file()
                    {
                        match crate::store::import_profile(&path) {
                            Ok(mut p) => {
                                if p.keys.len() != NUM_KEYS {
                                    p.keys.resize(NUM_KEYS, KeyFunction::Disable);
                                }
                                p.name = self.store.unique_name(&p.name);
                                self.store.profiles.push(p);
                                self.store.current = self.store.profiles.len() - 1;
                                self.save();
                            }
                            Err(e) => self.message(format!("Import failed: {e}")),
                        }
                    }
                }
                Action::ReadFromMouse => {
                    self.popup = None;
                    self.start_read_from_mouse();
                }
                Action::SetColor(target, rgb) => {
                    let l = &mut self.profile_mut().lighting;
                    match target {
                        ColorTarget::Dpi(i) => self.profile_mut().dpi.slots[i].color = rgb,
                        ColorTarget::Steady => l.steady_color = rgb,
                        ColorTarget::Breathing(i) => {
                            if i < l.breathing_colors.len() {
                                l.breathing_colors[i] = rgb;
                            }
                        }
                        ColorTarget::ColorfulSteady(i) => l.colorful_steady_colors[i] = rgb,
                        ColorTarget::Flicker(i) => l.flicker_colors[i] = rgb,
                    }
                    self.save();
                }
                Action::Confirmed(c) => match c {
                    Confirm::Restore => {
                        let name = self.profile().name.clone();
                        *self.profile_mut() = Profile::factory(&name);
                        self.save();
                    }
                    Confirm::DeleteProfile => {
                        if self.store.profiles.len() > 1 {
                            let i = self.store.current;
                            self.store.profiles.remove(i);
                            self.store.current = i.min(self.store.profiles.len() - 1);
                            self.save();
                        }
                    }
                },
                Action::InputDone(action, text) => {
                    let text = text.trim().to_string();
                    match action {
                        InputAction::NewProfile => {
                            if !text.is_empty() {
                                let name = self.store.unique_name(&text);
                                let p = Profile::factory(&name);
                                self.store.profiles.push(p);
                                self.store.current = self.store.profiles.len() - 1;
                                self.save();
                            }
                        }
                        InputAction::RenameProfile => {
                            if !text.is_empty() {
                                let name = self.store.unique_name(&text);
                                self.profile_mut().name = name;
                                self.save();
                            }
                        }
                        InputAction::NewMacro => {
                            if !text.is_empty() {
                                if self.profile().macro_by_name(&text).is_some() {
                                    self.message("This directory has a file with the same name.");
                                } else {
                                    self.profile_mut().macros.push(Macro { name: text, events: Vec::new() });
                                    let n = self.profile().macros.len() - 1;
                                    for d in self.dialogs.iter_mut() {
                                        if let Dialog::MacroEditor(me) = d {
                                            me.selected = Some(n);
                                            me.selected_event = None;
                                        }
                                    }
                                    self.save();
                                }
                            }
                        }
                        InputAction::MacroDelay => {
                            if let Ok(ms) = text.parse::<u32>() {
                                let ms = ms.clamp(1, 65535) as u16;
                                let mut sel = None;
                                for d in self.dialogs.iter_mut() {
                                    if let Dialog::MacroEditor(me) = d {
                                        sel = Some((me.selected, me.selected_event));
                                    }
                                }
                                if let Some((Some(mi), ev)) = sel {
                                    let p = self.profile_mut();
                                    if let Some(m) = p.macros.get_mut(mi) {
                                        let at = ev.map(|e| e + 1).unwrap_or(m.events.len()).min(m.events.len());
                                        m.events.insert(at, MacroEvent::Delay(ms));
                                    }
                                    self.save();
                                }
                            }
                        }
                    }
                }
                Action::Save => self.save(),
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame(root);
    }
}

impl App {
    /// One frame of the whole UI (also used by the headless test).
    pub fn frame(&mut self, root: &mut egui::Ui) {
        let ctx = root.ctx().clone();
        let ctx = &ctx;
        self.poll_workers(ctx);
        self.close_popup_on_outside_click(ctx);
        let enabled = self.dialogs.is_empty();

        egui::CentralPanel::default().frame(egui::Frame::NONE).show(root, |ui| {
            let origin = ui.max_rect().min;
            main_window::draw(self, ui, origin, enabled);
        });
        let mut rects = Vec::new();
        menus::draw_popup(self, ctx, &mut rects);
        dialogs::draw_all(self, ctx, &mut rects);
        self.popup_rects = rects;
        self.apply_actions(ctx);
    }
}

fn spawn_device_poller(state: Arc<Mutex<DeviceState>>, ctx: Context) {
    std::thread::spawn(move || loop {
        let res = {
            let _guard = DEVICE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            crate::device::Mouse::open()
        };
        let new = match res {
            Ok(m) => DeviceState { connected: true, firmware: m.firmware.clone() },
            Err(_) => DeviceState::default(),
        };
        let changed = {
            let mut s = state.lock().unwrap();
            let changed = s.connected != new.connected || s.firmware != new.firmware;
            *s = new;
            changed
        };
        if changed {
            ctx.request_repaint();
        }
        std::thread::sleep(Duration::from_secs(2));
    });
}

/// Centre a dialog of `size` inside the window whose top-left is `origin`.
pub fn centered(origin: Pos2, size: egui::Vec2) -> Pos2 {
    Pos2::new(origin.x + (WIN_W - size.x) / 2.0, origin.y + (WIN_H - size.y) / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_frame(app: &mut App, ctx: &Context, input: egui::RawInput) {
        let mut out = ctx.run_ui(input, |ui| app.frame(ui));
        out.textures_delta.clear();
    }

    fn raw() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(WIN_W, WIN_H))),
            ..Default::default()
        }
    }

    #[test]
    fn headless_frames_do_not_panic() {
        let ctx = Context::default();
        let mut app = App::from_ctx(&ctx);
        run_frame(&mut app, &ctx, raw());
        for panel in 0..4 {
            app.store.open_panel = panel;
            run_frame(&mut app, &ctx, raw());
        }
        for e in LightEffect::ALL {
            app.store.open_panel = 1;
            app.profile_mut().lighting.effect = e;
            run_frame(&mut app, &ctx, raw());
        }
        for p in [
            Popup::KeyMenu { key: 3, sub: Some(12) },
            Popup::KeyMenu { key: 13, sub: Some(14) },
            Popup::ProfileList,
            Popup::ProfileMenu,
            Popup::EffectList,
            Popup::PollingList,
        ] {
            app.popup = Some(p);
            run_frame(&mut app, &ctx, raw());
        }
        app.popup = None;
        app.profile_mut().macros.push(Macro { name: "m".into(), events: vec![MacroEvent::KeyDown(4), MacroEvent::Delay(10), MacroEvent::KeyUp(4)] });
        let me = macro_editor::MacroEditor { selected: Some(0), selected_event: Some(1), ..Default::default() };
        for d in [
            Dialog::Message { text: "hi".into(), yes_no: true, on_yes: Some(Confirm::Restore) },
            Dialog::Wait,
            Dialog::Input { title: "t".into(), text: "x".into(), action: InputAction::NewProfile },
            Dialog::FireKey { key: 0, button: 1, interval: "50".into(), count: "0".into() },
            Dialog::KeyCombo { key: 0, modifiers: 1, usage: 4 },
            Dialog::MacroPick { key: 0, selected: Some(0), mode: MacroLoop::Count, count: "1".into() },
            colorpicker::new_dialog(ColorTarget::Dpi(0), [1, 2, 3]),
            Dialog::MacroEditor(me),
        ] {
            app.dialogs = vec![d];
            run_frame(&mut app, &ctx, raw());
            // click somewhere + a key press while the dialog is up
            let mut inp = raw();
            inp.events.push(egui::Event::PointerMoved(Pos2::new(500.0, 300.0)));
            inp.events.push(egui::Event::PointerButton { pos: Pos2::new(500.0, 300.0), button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
            inp.events.push(egui::Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() });
            run_frame(&mut app, &ctx, inp);
            let mut inp = raw();
            inp.events.push(egui::Event::PointerButton { pos: Pos2::new(500.0, 300.0), button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
            run_frame(&mut app, &ctx, inp);
        }
        app.dialogs.clear();
        // click every key button and walk the menu
        for i in 0..NUM_KEYS {
            let r = main_window::key_button_rect(Pos2::ZERO, i);
            let mut inp = raw();
            inp.events.push(egui::Event::PointerMoved(r.center()));
            inp.events.push(egui::Event::PointerButton { pos: r.center(), button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
            run_frame(&mut app, &ctx, inp);
            let mut inp = raw();
            inp.events.push(egui::Event::PointerButton { pos: r.center(), button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
            run_frame(&mut app, &ctx, inp);
            run_frame(&mut app, &ctx, raw());
            assert!(matches!(app.popup, Some(Popup::KeyMenu { key, .. }) if key == i), "key {i} menu did not open");
            app.popup = None;
            // the popup's layer is still registered for one frame
            run_frame(&mut app, &ctx, raw());
        }
        // apply all pending actions one more time
        run_frame(&mut app, &ctx, raw());
    }
}
