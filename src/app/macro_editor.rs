//! Macro Editor dialog (macro list, key list, recording, insert event).

use super::widgets::{popup_list, Canvas, TXT};
use super::{Action, Dialog, InputAction, Popup};
use crate::hidkeys::{self, mouse_mask};
use crate::model::*;
use egui::{Align2, Context, Id, Rect, Vec2};
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct MacroEditor {
    pub selected: Option<usize>,
    pub selected_event: Option<usize>,
    pub recording: bool,
    pub auto_delay: bool,
    pub mode: MacroLoop,
    pub count: String,
    pub last_event: Option<Instant>,
    /// Scroll offset (rows) of the key list.
    pub scroll: usize,
    /// "Modify" mode: the next key press replaces the selected event's key.
    pub modifying: bool,
    /// "Insert event > Keystroke": waiting for the key to insert.
    pub pending_keystroke: bool,
}

impl Default for MacroEditor {
    fn default() -> Self {
        MacroEditor {
            selected: None,
            selected_event: None,
            recording: false,
            auto_delay: true,
            mode: MacroLoop::Count,
            count: "1".into(),
            last_event: None,
            scroll: 0,
            modifying: false,
            pending_keystroke: false,
        }
    }
}

fn event_label(e: &MacroEvent) -> String {
    match e {
        MacroEvent::KeyDown(k) => format!("↓ {}", hidkeys::usage_name(*k)),
        MacroEvent::KeyUp(k) => format!("↑ {}", hidkeys::usage_name(*k)),
        MacroEvent::MouseDown(b) => format!("↓ {}", hidkeys::mouse_mask_name(*b)),
        MacroEvent::MouseUp(b) => format!("↑ {}", hidkeys::mouse_mask_name(*b)),
        MacroEvent::Delay(ms) => format!("Delay {ms} ms"),
    }
}

const INSERT_ITEMS: [&str; 5] = ["Keystroke", "Delay", "Left Button", "Right Button", "Middle Button"];

#[allow(clippy::too_many_arguments)]
pub fn draw(cv: &mut Canvas, ctx: &Context, top: bool, me: &mut MacroEditor, profile: &mut Profile, popup: &Option<Popup>, actions: &mut Vec<Action>, rects: &mut Vec<Rect>) {
    let mut changed = false;
    if cv.button("x", "exit_nr", "exit_ov", 727.0, 10.0, "") {
        me.recording = false;
        actions.push(Action::CloseDialog);
    }
    cv.text(38.0, 27.0, "Macro list", TXT);
    cv.text(248.0, 27.0, "Key list", TXT);
    if me.selected.map(|s| s >= profile.macros.len()).unwrap_or(false) {
        me.selected = None;
    }

    // --- macro list
    let names: Vec<String> = profile.macros.iter().map(|m| m.name.clone()).collect();
    let lr = cv.rect(37.0, 47.0, 190.0, 310.0);
    if let Some(c) = cv.list_box("macros", lr, &names, me.selected) {
        if me.selected != Some(c) {
            me.selected = Some(c);
            me.selected_event = None;
            me.scroll = 0;
            me.recording = false;
        }
    }
    if cv.button("newmacro", "button3_nr", "button3_ov", 36.0, 368.0, "New macro") {
        if me.recording {
            actions.push(Action::OpenDialog(Dialog::Message { text: "Please stop recording macro.".into(), yes_no: false, on_yes: None }));
        } else {
            actions.push(Action::OpenDialog(Dialog::Input {
                title: "Please enter a macro name：".into(),
                text: String::new(),
                action: InputAction::NewMacro,
            }));
        }
    }
    if cv.button("delmacro", "button3_nr", "button3_ov", 134.0, 368.0, "Delete") {
        if let Some(s) = me.selected {
            let name = profile.macros[s].name.clone();
            let used = profile.keys.iter().any(|k| matches!(k, KeyFunction::Macro { name: n, .. } if *n == name));
            if used {
                actions.push(Action::OpenDialog(Dialog::Message {
                    text: "This item is being used and cannot be deleted.".into(),
                    yes_no: false,
                    on_yes: None,
                }));
            } else {
                profile.macros.remove(s);
                me.selected = None;
                me.selected_event = None;
                me.recording = false;
                changed = true;
            }
        } else {
            actions.push(Action::OpenDialog(Dialog::Message { text: "Please select a macro first.".into(), yes_no: false, on_yes: None }));
        }
    }

    // --- key list
    let kr = cv.rect(247.0, 47.0, 190.0, 310.0);
    let rows = 15usize;
    let events: Vec<String> = me
        .selected
        .and_then(|s| profile.macros.get(s))
        .map(|m| m.events.iter().map(event_label).collect())
        .unwrap_or_default();
    if me.selected_event.map(|e| e >= events.len()).unwrap_or(false) {
        me.selected_event = None;
    }
    // keep the selection visible
    if let Some(e) = me.selected_event {
        if e < me.scroll {
            me.scroll = e;
        } else if e >= me.scroll + rows {
            me.scroll = e + 1 - rows;
        }
    } else if me.recording && events.len() > rows {
        me.scroll = events.len() - rows;
    }
    me.scroll = me.scroll.min(events.len().saturating_sub(rows));
    let visible: Vec<String> = events.iter().skip(me.scroll).take(rows).cloned().collect();
    let sel_vis = me.selected_event.and_then(|e| e.checked_sub(me.scroll)).filter(|e| *e < rows);
    if let Some(c) = cv.list_box("events", kr, &visible, sel_vis) {
        me.selected_event = Some(c + me.scroll);
        me.modifying = false;
    }
    // wheel scrolling over the key list
    if top && cv.ui.rect_contains_pointer(kr) {
        let dy = ctx.input(|i| i.smooth_scroll_delta.y);
        if dy > 0.0 {
            me.scroll = me.scroll.saturating_sub(1);
        } else if dy < 0.0 {
            me.scroll = (me.scroll + 1).min(events.len().saturating_sub(rows));
        }
    }
    if cv.button("modify", "button3_nr", "button3_ov", 247.0, 368.0, "Modify") {
        if me.selected_event.is_some() {
            me.modifying = !me.modifying;
        } else {
            actions.push(Action::OpenDialog(Dialog::Message { text: "Please select a macro first.".into(), yes_no: false, on_yes: None }));
        }
    }
    if me.modifying {
        cv.painter().text(cv.p(342.0, 400.0), Align2::CENTER_CENTER, "Please input new key", super::widgets::font(11.0), TXT);
    }
    if cv.button("delevent", "button3_nr", "button3_ov", 343.0, 368.0, "Delete") {
        if let (Some(s), Some(e)) = (me.selected, me.selected_event) {
            if let Some(m) = profile.macros.get_mut(s) {
                if e < m.events.len() {
                    m.events.remove(e);
                    changed = true;
                }
            }
            me.selected_event = if e > 0 { Some(e - 1) } else { None };
            me.modifying = false;
        }
    }

    // --- record button
    let rec_label = if me.recording { "Stop record" } else { "Start record" };
    let rec_skin = if me.recording { "recbtn_dn" } else { "recbtn_nr" };
    if cv.button("record", rec_skin, rec_skin, 486.0, 47.0, "") {
        if me.selected.is_none() {
            actions.push(Action::OpenDialog(Dialog::Message { text: "Please select a macro first.".into(), yes_no: false, on_yes: None }));
        } else {
            me.recording = !me.recording;
            me.last_event = None;
            me.modifying = false;
        }
    }
    {
        let r = cv.rect(486.0, 47.0, 128.0, 30.0);
        let dot = r.min + Vec2::new(18.0, 15.0);
        let col = if me.recording { egui::Color32::from_rgb(255, 0, 0) } else { egui::Color32::from_rgb(70, 70, 70) };
        cv.painter().circle_filled(dot, 5.0, col);
        cv.painter().text(r.min + Vec2::new(36.0, 15.0), Align2::LEFT_CENTER, rec_label, super::widgets::font(super::widgets::FS), super::widgets::TXT_OV);
    }
    if cv.checkbox("autodelay", 486.0, 103.0, me.auto_delay, "Automatic insert delay") {
        me.auto_delay = !me.auto_delay;
    }
    let radios = [(MacroLoop::UntilReleased, 130.0), (MacroLoop::UntilAnyKey, 158.0), (MacroLoop::Count, 186.0)];
    for (i, (m, y)) in radios.iter().enumerate() {
        if cv.radio(&format!("loop{i}"), 486.0, *y, me.mode == *m, m.name()) {
            me.mode = *m;
        }
    }
    cv.number_field("count", 511.0, 206.0, 78.0, &mut me.count, 1, 255);
    cv.text(600.0, 209.0, "1 ~ 255", TXT);
    // apply the loop settings to every key bound to the selected macro
    if let Some(s) = me.selected {
        if let Some(m) = profile.macros.get(s) {
            let name = m.name.clone();
            let ct = me.count.parse::<i32>().unwrap_or(1).clamp(1, 255) as u8;
            for k in profile.keys.iter_mut() {
                if let KeyFunction::Macro { name: n, mode, count } = k {
                    if *n == name && (*mode != me.mode || *count != ct) {
                        *mode = me.mode;
                        *count = ct;
                        changed = true;
                    }
                }
            }
        }
    }

    // --- insert event
    cv.text(486.0, 250.0, "Insert event", TXT);
    let open = matches!(popup, Some(Popup::InsertEventList));
    let (clicked, cr) = cv.combo("insert", "combo2_nr", "combo2_ov", 486.0, 270.0, "", open);
    if clicked {
        if me.selected.is_none() {
            actions.push(Action::OpenDialog(Dialog::Message { text: "Please select a macro first.".into(), yes_no: false, on_yes: None }));
        } else {
            actions.push(if open { Action::ClosePopup } else { Action::OpenPopup(Popup::InsertEventList) });
        }
    }
    let mut insert: Option<Vec<MacroEvent>> = None;
    let mut keystroke_wait = false;
    if open && top {
        rects.push(cr);
        let items: Vec<(String, bool)> = INSERT_ITEMS.iter().map(|s| (s.to_string(), false)).collect();
        let (c, _, r) = popup_list(ctx, cv.skins, Id::new("insertlist"), cr.left_bottom(), cr.width(), &items, false);
        rects.push(r);
        if let Some(c) = c {
            actions.push(Action::ClosePopup);
            match c {
                0 => keystroke_wait = true,
                1 => actions.push(Action::OpenDialog(Dialog::Input {
                    title: "Please enter a delay time(ms)：".into(),
                    text: "100".into(),
                    action: InputAction::MacroDelay,
                })),
                2 => insert = Some(vec![MacroEvent::MouseDown(mouse_mask::LEFT), MacroEvent::MouseUp(mouse_mask::LEFT)]),
                3 => insert = Some(vec![MacroEvent::MouseDown(mouse_mask::RIGHT), MacroEvent::MouseUp(mouse_mask::RIGHT)]),
                4 => insert = Some(vec![MacroEvent::MouseDown(mouse_mask::MIDDLE), MacroEvent::MouseUp(mouse_mask::MIDDLE)]),
                _ => {}
            }
        }
    }
    if keystroke_wait {
        me.modifying = false;
        me.pending_keystroke = true;
    }

    // --- keyboard capture: recording, modify, insert keystroke
    if top && (me.recording || me.modifying || me.pending_keystroke) {
        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            if let egui::Event::Key { key, pressed, repeat, .. } = ev {
                if repeat {
                    continue;
                }
                let Some(u) = hidkeys::usage_from_egui(key) else { continue };
                if let Some(s) = me.selected {
                    if let Some(m) = profile.macros.get_mut(s) {
                        if me.recording {
                            let now = Instant::now();
                            if me.auto_delay {
                                if let Some(last) = me.last_event {
                                    let ms = now.duration_since(last).as_millis().clamp(1, 65535) as u16;
                                    m.events.push(MacroEvent::Delay(ms));
                                }
                            }
                            me.last_event = Some(now);
                            m.events.push(if pressed { MacroEvent::KeyDown(u) } else { MacroEvent::KeyUp(u) });
                            changed = true;
                        } else if pressed && me.modifying {
                            if let Some(e) = me.selected_event {
                                if let Some(ev) = m.events.get_mut(e) {
                                    match ev {
                                        MacroEvent::KeyDown(k) | MacroEvent::KeyUp(k) => *k = u,
                                        MacroEvent::MouseDown(_) => *ev = MacroEvent::KeyDown(u),
                                        MacroEvent::MouseUp(_) => *ev = MacroEvent::KeyUp(u),
                                        MacroEvent::Delay(_) => {}
                                    }
                                    changed = true;
                                }
                            }
                            me.modifying = false;
                        } else if pressed && me.pending_keystroke {
                            insert = Some(vec![MacroEvent::KeyDown(u), MacroEvent::KeyUp(u)]);
                            me.pending_keystroke = false;
                        }
                    }
                }
            }
        }
    }
    if me.pending_keystroke {
        cv.painter().text(cv.p(560.0, 300.0), Align2::CENTER_CENTER, "Please enter keystroke", super::widgets::font(11.0), TXT);
    }
    if let (Some(evs), Some(s)) = (insert, me.selected) {
        if let Some(m) = profile.macros.get_mut(s) {
            let at = me.selected_event.map(|e| e + 1).unwrap_or(m.events.len()).min(m.events.len());
            for (i, e) in evs.into_iter().enumerate() {
                m.events.insert(at + i, e);
            }
            changed = true;
        }
    }
    if changed {
        actions.push(Action::Save);
    }
}
