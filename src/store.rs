//! On-disk profile storage: `$XDG_CONFIG_HOME/ms900ctl/profiles.json`.

use crate::model::Profile;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Store {
    pub profiles: Vec<Profile>,
    pub current: usize,
    /// Which collapsible panel is open (0 DPI, 1 Lighting, 2 Mouse Parameter, 3 Polling Rate).
    #[serde(default)]
    pub open_panel: usize,
    /// Apply the "Mouse Parameter" panel to the desktop (xinput/gsettings) on Apply.
    #[serde(default = "default_true")]
    pub apply_mouse_params: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Store {
    fn default() -> Self {
        Store {
            profiles: vec![Profile::factory("Profile 1")],
            current: 0,
            open_panel: 0,
            apply_mouse_params: true,
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("ms900ctl")
}

pub fn store_path() -> PathBuf {
    config_dir().join("profiles.json")
}

impl Store {
    pub fn load() -> Store {
        match std::fs::read(store_path()) {
            Ok(bytes) => match serde_json::from_slice::<Store>(&bytes) {
                Ok(mut s) => {
                    if s.profiles.is_empty() {
                        s.profiles.push(Profile::factory("Profile 1"));
                    }
                    s.current = s.current.min(s.profiles.len() - 1);
                    s
                }
                Err(e) => {
                    log::error!("profiles.json is corrupt ({e}); starting with defaults");
                    Store::default()
                }
            },
            Err(_) => Store::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let dir = config_dir();
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let tmp = dir.join("profiles.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, store_path())?;
        Ok(())
    }

    pub fn current(&self) -> &Profile {
        &self.profiles[self.current]
    }
    pub fn current_mut(&mut self) -> &mut Profile {
        &mut self.profiles[self.current]
    }

    /// Unique name for a new profile ("Profile 2", ...).
    pub fn unique_name(&self, base: &str) -> String {
        if !self.profiles.iter().any(|p| p.name == base) {
            return base.to_string();
        }
        for i in 2.. {
            let n = format!("{base} {i}");
            if !self.profiles.iter().any(|p| p.name == n) {
                return n;
            }
        }
        unreachable!()
    }
}

/// Export a single profile as JSON (the "Export profile" menu entry).
pub fn export_profile(p: &Profile, path: &std::path::Path) -> Result<()> {
    std::fs::write(path, serde_json::to_vec_pretty(p)?)?;
    Ok(())
}

/// Import a profile from JSON, or from an original `.bkm` backup.
pub fn import_profile(path: &std::path::Path) -> Result<Profile> {
    let bytes = std::fs::read(path)?;
    if path.extension().map(|e| e.eq_ignore_ascii_case("bkm")).unwrap_or(false) {
        return crate::bkm::parse(&bytes);
    }
    let p: Profile = serde_json::from_slice(&bytes).context("parsing profile JSON")?;
    Ok(p)
}
