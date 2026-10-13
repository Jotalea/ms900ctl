#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ms900ctl::app::{App, WIN_H, WIN_W};

fn load_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/appico.ico");
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Ico).ok()?;
    let rgba = img.to_rgba8();
    Some(egui::IconData { width: rgba.width(), height: rgba.height(), rgba: rgba.into_raw() })
}

/// Small command line interface used for testing without the GUI:
///   ms900ctl --dump               print firmware, raw blocks and the decoded on-board profile
///   ms900ctl --read-json FILE     save the on-board configuration as a profile JSON
///   ms900ctl --apply-json FILE    apply a profile JSON (or .bkm) to the mouse
fn cli(args: &[String]) -> anyhow::Result<bool> {
    use ms900ctl::device::Mouse;
    match args.first().map(String::as_str) {
        Some("--dump") => {
            let m = Mouse::open()?;
            println!("device: {}  firmware: {}", m.path, m.firmware);
            println!("active profile: {}", m.active_profile()? + 1);
            let cfg = m.read_config(0)?;
            println!("config ({} bytes):", cfg.data.len());
            for (i, chunk) in cfg.data.chunks(16).enumerate() {
                println!("  {:3}: {}", i * 16, chunk.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "));
            }
            let btn = m.read_buttons(0)?;
            println!("buttons:");
            for i in 0..ms900ctl::protocol::BUTTON_SLOTS {
                let s = &btn[8 + i * 4..12 + i * 4];
                println!("  slot {i:2}: {:02x} {:02x} {:02x} {:02x}", s[0], s[1], s[2], s[3]);
            }
            let p = m.read_profile(0, "on-board")?;
            println!("{}", serde_json::to_string_pretty(&p)?);
            Ok(true)
        }
        Some("--read-json") => {
            let path = args.get(1).ok_or_else(|| anyhow::anyhow!("missing file"))?;
            let m = Mouse::open()?;
            let p = m.read_profile(0, "on-board")?;
            std::fs::write(path, serde_json::to_vec_pretty(&p)?)?;
            println!("wrote {path}");
            Ok(true)
        }
        Some("--apply-json") => {
            let path = args.get(1).ok_or_else(|| anyhow::anyhow!("missing file"))?;
            let p = ms900ctl::store::import_profile(std::path::Path::new(path))?;
            let m = Mouse::open()?;
            m.apply_profile(0, &p)?;
            println!("applied '{}' to {}", p.name, m.path);
            Ok(true)
        }
        Some("--help" | "-h") => {
            println!("usage: ms900ctl [--dump | --read-json FILE | --apply-json FILE]");
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn main() -> eframe::Result {
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli(&args) {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
    }
    // The original program is not DPI aware: its window is exactly 1030x662
    // physical pixels.  Keep the same so the skins stay pixel-perfect.
    if std::env::var_os("WINIT_X11_SCALE_FACTOR").is_none() {
        std::env::set_var("WINIT_X11_SCALE_FACTOR", "1");
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("MS900 Gaming Mouse")
        .with_inner_size([WIN_W, WIN_H])
        .with_min_inner_size([WIN_W, WIN_H])
        .with_max_inner_size([WIN_W, WIN_H])
        .with_decorations(false)
        .with_resizable(false)
        .with_window_type(egui::X11WindowType::Dialog)
        .with_app_id("ms900ctl");
    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("MS900 Gaming Mouse", options, Box::new(|cc| {
        // Wayland compositors may still scale us; undo it so one logical
        // point is one physical pixel and request the native size again.
        let ppp = cc.egui_ctx.pixels_per_point();
        if (ppp - 1.0).abs() > 0.01 {
            cc.egui_ctx.set_zoom_factor(1.0 / ppp);
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(WIN_W, WIN_H)));
        }
        Ok(Box::new(App::new(cc)))
    }))
}
