//! hidraw transport for the MS900 (see `protocol.rs` for the byte layout).

use crate::model::*;
use crate::protocol::{self as p, ConfigBlock};
use anyhow::{anyhow, bail, Context, Result};
use hidapi::{HidApi, HidDevice};
use std::thread::sleep;
use std::time::Duration;

/// Open MS900 handle.
pub struct Mouse {
    dev: HidDevice,
    pub path: String,
    pub firmware: String,
    pub serial: Option<String>,
}

/// Enumerate candidate HID interfaces.  The vendor collections (feature
/// reports 4/5) live on the second interface (keyboard-class, `mi_01`), but
/// hidraw exposes one node per interface, so we probe each node for the
/// 520-byte feature report.
pub fn find_devices(api: &HidApi) -> Vec<String> {
    let mut paths: Vec<(i32, String)> = api
        .device_list()
        .filter(|d| d.vendor_id() == p::VID && d.product_id() == p::PID)
        .map(|d| (d.interface_number(), d.path().to_string_lossy().into_owned()))
        .collect();
    // interface 1 first, it is the one carrying the vendor reports
    paths.sort_by_key(|(iface, _)| if *iface == 1 { 0 } else { 1 });
    paths.into_iter().map(|(_, p)| p).collect()
}

impl Mouse {
    /// Open the first MS900 that answers to the firmware-version command.
    pub fn open() -> Result<Mouse> {
        let api = HidApi::new().context("initialising hidapi")?;
        let paths = find_devices(&api);
        if paths.is_empty() {
            bail!("no MS900 (258a:0029) found");
        }
        let mut last_err = anyhow!("no usable interface");
        for path in paths {
            let cpath = std::ffi::CString::new(path.clone())?;
            match api.open_path(&cpath) {
                Ok(dev) => {
                    let serial = dev.get_serial_number_string().ok().flatten();
                    let mut m = Mouse { dev, path: path.clone(), firmware: String::new(), serial };
                    match m.read_firmware_version() {
                        Ok(fw) => {
                            m.firmware = fw;
                            return Ok(m);
                        }
                        Err(e) => last_err = e,
                    }
                }
                Err(e) => {
                    last_err = anyhow!("{path}: {e} (missing udev rule? see README)");
                }
            }
        }
        Err(last_err)
    }

    fn send_cmd(&self, cmd: u8, args: &[u8]) -> Result<()> {
        let r = p::cmd_report(cmd, args);
        self.dev.send_feature_report(&r).with_context(|| format!("sending command {cmd:#04x}"))?;
        Ok(())
    }

    fn read_cmd(&self) -> Result<[u8; p::CMD_LEN]> {
        let mut r = [0u8; p::CMD_LEN];
        r[0] = p::REPORT_CMD;
        self.dev.get_feature_report(&mut r).context("reading command report")?;
        Ok(r)
    }

    /// Select a data block with `cmd` and read it.  Returns the report
    /// (byte 0 = report id) and the number of bytes the device sent.
    fn read_block(&self, cmd: u8) -> Result<(Vec<u8>, usize)> {
        self.send_cmd(cmd, &[])?;
        sleep(Duration::from_millis(p::SELECT_DELAY_MS));
        let mut r = vec![0u8; p::DATA_LEN];
        r[0] = p::REPORT_DATA;
        let mut n = 0;
        let mut err = None;
        for _ in 0..3 {
            match self.dev.get_feature_report(&mut r) {
                Ok(len) => {
                    n = len;
                    err = None;
                    break;
                }
                Err(e) => {
                    err = Some(e);
                    sleep(Duration::from_millis(200));
                }
            }
        }
        if let Some(e) = err {
            return Err(anyhow!("reading data block {cmd:#04x}: {e}"));
        }
        if r[1] != cmd {
            bail!("device answered for block {:#04x} instead of {cmd:#04x}", r[1]);
        }
        Ok((r, n))
    }

    fn write_block(&self, report: &[u8]) -> Result<()> {
        let mut err = None;
        for _ in 0..3 {
            match self.dev.send_feature_report(report) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    err = Some(e);
                    sleep(Duration::from_millis(200));
                }
            }
        }
        Err(anyhow!("writing data block {:#04x}: {}", report[1], err.unwrap()))
    }

    /// Command 0x01: 4 firmware bytes, shown as "V1.03"-style text when printable.
    pub fn read_firmware_version(&self) -> Result<String> {
        self.send_cmd(p::CMD_FIRMWARE, &[])?;
        sleep(Duration::from_millis(p::SELECT_DELAY_MS));
        let r = self.read_cmd()?;
        if r[1] != p::CMD_FIRMWARE {
            bail!("unexpected answer to firmware query: {:02x?}", r);
        }
        let b = &r[2..6];
        if b.iter().all(|c| c.is_ascii_graphic()) {
            Ok(String::from_utf8_lossy(b).into_owned())
        } else {
            Ok(format!("{:02x}{:02x}{:02x}{:02x}", b[0], b[1], b[2], b[3]))
        }
    }

    /// Command 0x02: active on-board profile, 0-based.
    pub fn active_profile(&self) -> Result<u8> {
        self.send_cmd(p::CMD_PROFILE, &[])?;
        sleep(Duration::from_millis(p::SELECT_DELAY_MS));
        let r = self.read_cmd()?;
        match r[2] {
            1..=3 => Ok(r[2] - 1),
            _ => Ok(0),
        }
    }

    pub fn set_active_profile(&self, idx: u8) -> Result<()> {
        self.send_cmd(p::CMD_PROFILE, &[idx + 1])
    }

    pub fn read_config(&self, profile: u8) -> Result<ConfigBlock> {
        let (r, n) = self.read_block(p::config_cmd(profile))?;
        // Some kernels report the full buffer size; fall back to the
        // canonical "long" size in that case.
        let len = if n >= p::DATA_LEN { 167 } else { n };
        ConfigBlock::from_report(&r, len)
    }

    pub fn read_buttons(&self, profile: u8) -> Result<Vec<u8>> {
        let (r, _) = self.read_block(p::buttons_cmd(profile))?;
        Ok(r)
    }

    /// Read the whole on-board state into a profile named `name`.
    pub fn read_profile(&self, profile: u8, name: &str) -> Result<Profile> {
        let mut out = Profile::factory(name);
        let cfg = self.read_config(profile)?;
        if cfg.sensor() != p::SENSOR_PMW3327 {
            log::warn!("unexpected sensor id {:#04x}", cfg.sensor());
        }
        cfg.read_into_profile(&mut out);
        let btn = self.read_buttons(profile)?;
        let names: Vec<String> = (1..=p::BUTTON_SLOTS).map(|i| format!("Macro {i}")).collect();
        out.keys = p::decode_buttons(&btn, &names);
        // Macro bodies cannot be read back from the mouse; keep the binding
        // valid with an empty placeholder the user can fill in the editor.
        for k in &out.keys {
            if let KeyFunction::Macro { name, .. } = k {
                if out.macro_by_name(name).is_none() {
                    out.macros.push(Macro { name: name.clone(), events: Vec::new() });
                }
            }
        }
        Ok(out)
    }

    /// Write DPI / lighting / polling rate, all key bindings and the macros
    /// they reference.  Mirrors the "Apply" button of the original software.
    pub fn apply_profile(&self, profile: u8, prof: &Profile) -> Result<()> {
        if !prof.has_left_click() {
            bail!("At least one button must be configured as \"click\"!");
        }
        if prof.dpi.enabled_count() == 0 {
            bail!("You can not disable all DPI.");
        }
        // Same order as the Windows software: macros, buttons, then the
        // config block (read, patch, write back).
        let (buttons, macros) = p::buttons_report(prof, profile)?;
        for (name, idx) in &macros {
            let m = prof.macro_by_name(name).ok_or_else(|| anyhow!("macro '{name}' missing"))?;
            let rep = p::macro_report(m, *idx)?;
            self.write_block(&rep)?;
            sleep(Duration::from_millis(50));
        }
        self.write_block(&buttons)?;
        sleep(Duration::from_millis(50));

        let mut cfg = self.read_config(profile)?;
        cfg.apply_profile(prof);
        self.write_block(&cfg.to_report(profile))?;
        sleep(Duration::from_millis(50));
        Ok(())
    }
}
