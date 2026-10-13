//! "Mouse Parameter" panel backend.
//!
//! On Windows these five values are the system mouse settings
//! (`SystemParametersInfo`).  On Linux there is no single API, so we apply
//! them best-effort to whatever desktop we can detect:
//!
//! * GNOME / Cinnamon / MATE: `gsettings` (speed, accel profile, double-click).
//! * KDE Plasma 5/6: `kwriteconfig{5,6}` + KWin reconfigure.
//! * X11 fallback: `xinput set-prop` on the mouse's libinput properties.
//!
//! Every step is optional; failures are reported but never fatal.

use crate::model::MouseParams;
use std::process::Command;

const DEVICE_NAME: &str = "SINOWEALTH Wired Gaming Mouse";

fn have(cmd: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {cmd} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(cmd).args(args).output().map_err(|e| format!("{cmd}: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(format!("{cmd} {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// Windows pointer speed 1..=20 -> libinput speed -1.0..=1.0.
fn libinput_speed(sensitivity: u8) -> f32 {
    ((sensitivity.clamp(1, 20) as f32) - 10.0) / 10.0
}

/// Apply `p` to the desktop.  Returns a list of human readable notes
/// (what was applied, what was skipped).
pub fn apply(p: &MouseParams) -> Vec<String> {
    let mut notes = Vec::new();
    let speed = libinput_speed(p.sensitivity);
    let profile = if p.enhance_precision { "adaptive" } else { "flat" };
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();

    if have("gsettings") && (desktop.contains("gnome") || desktop.contains("cinnamon") || desktop.contains("unity") || desktop.contains("budgie")) {
        let schema = if desktop.contains("cinnamon") { "org.cinnamon.desktop.peripherals.mouse" } else { "org.gnome.desktop.peripherals.mouse" };
        for (k, v) in [
            ("speed", format!("{speed:.2}")),
            ("accel-profile", format!("'{profile}'")),
            ("double-click", p.double_click_ms.to_string()),
        ] {
            match run("gsettings", &["set", schema, k, &v]) {
                Ok(_) => notes.push(format!("gsettings {k} = {v}")),
                Err(e) => notes.push(format!("skipped: {e}")),
            }
        }
    } else if desktop.contains("kde") || desktop.contains("plasma") {
        let kw = if have("kwriteconfig6") { "kwriteconfig6" } else { "kwriteconfig5" };
        if have(kw) {
            let accel = format!("{speed:.2}");
            let prof = if p.enhance_precision { "1" } else { "2" };
            let dev_group = [
                "--file", "kcminputrc", "--group", "Libinput", "--group", "9610", "--group", "41",
                "--group", DEVICE_NAME,
            ];
            let mut args: Vec<&str> = dev_group.to_vec();
            args.extend(["--key", "PointerAcceleration", &accel]);
            let _ = run(kw, &args).map(|_| notes.push(format!("kcminputrc PointerAcceleration = {accel}")));
            let mut args: Vec<&str> = dev_group.to_vec();
            args.extend(["--key", "PointerAccelerationProfile", prof]);
            let _ = run(kw, &args).map(|_| notes.push(format!("kcminputrc PointerAccelerationProfile = {prof}")));
            let dc = p.double_click_ms.to_string();
            let _ = run(kw, &["--file", "kdeglobals", "--group", "KDE", "--key", "DoubleClickInterval", &dc])
                .map(|_| notes.push(format!("kdeglobals DoubleClickInterval = {dc}")));
            let wheel = p.scroll_lines.to_string();
            let _ = run(kw, &["--file", "kdeglobals", "--group", "KDE", "--key", "WheelScrollLines", &wheel])
                .map(|_| notes.push(format!("kdeglobals WheelScrollLines = {wheel}")));
            let qdbus = if have("qdbus6") { "qdbus6" } else { "qdbus" };
            let _ = run(qdbus, &["org.kde.KWin", "/KWin", "reconfigure"]);
            let _ = run(qdbus, &["org.kde.KWin", "/KWin", "org.kde.KWin.reconfigure"]);
        } else {
            notes.push("skipped: kwriteconfig not found".into());
        }
    }

    // X11: talk to the device directly (works on any X11 desktop).
    if std::env::var("DISPLAY").is_ok() && have("xinput") {
        if let Ok(list) = run("xinput", &["list", "--id-only"]) {
            for id in list.lines() {
                let id = id.trim();
                let name = run("xinput", &["list", "--name-only", id]).unwrap_or_default();
                if !name.contains(DEVICE_NAME) || name.contains("Keyboard") || name.contains("Consumer") {
                    continue;
                }
                let sp = format!("{speed:.2}");
                let _ = run("xinput", &["set-prop", id, "libinput Accel Speed", &sp])
                    .map(|_| notes.push(format!("xinput #{id} Accel Speed = {sp}")));
                let prof = if p.enhance_precision { ["1", "0"] } else { ["0", "1"] };
                let _ = run("xinput", &["set-prop", id, "libinput Accel Profile Enabled", prof[0], prof[1]]);
            }
        }
    }

    if notes.is_empty() {
        notes.push("Mouse Parameter: no supported desktop backend found (gsettings / kwriteconfig / xinput)".into());
    }
    notes
}
