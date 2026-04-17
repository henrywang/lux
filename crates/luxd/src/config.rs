//! luxd configuration: ~/.config/lux/luxd.toml.
//!
//! A default file is written on first run so users can discover and edit it
//! without reading docs first.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Detect only. Write findings and desktop-notify. Never act.
    Monitor,
    /// Detect and surface fixes to the REPL. User runs them. (default)
    #[default]
    Suggest,
    /// Detect and auto-run fixes that match a hand-picked allowlist.
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub mode: Mode,

    #[serde(default = "default_true")]
    pub watch_journal: bool,

    #[serde(default = "default_true")]
    pub watch_avc: bool,

    #[serde(default = "default_true")]
    pub watch_disk: bool,

    #[serde(default = "default_true")]
    pub notify_desktop: bool,

    #[serde(default = "default_true")]
    pub notify_repl: bool,

    /// Poll interval in seconds.
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
}

fn default_true() -> bool {
    true
}

fn default_interval() -> u64 {
    60
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            watch_journal: true,
            watch_avc: true,
            watch_disk: true,
            notify_desktop: true,
            notify_repl: true,
            interval_secs: 60,
        }
    }
}

const DEFAULT_TOML: &str = r#"# luxd configuration
# mode: "monitor" (detect only), "suggest" (tell the REPL), "auto" (run allowlisted fixes)
mode = "suggest"

watch_journal = true
watch_avc = true
watch_disk = true

notify_desktop = true
notify_repl = true

# Seconds between detector runs
interval_secs = 60
"#;

pub fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".config/lux/luxd.toml")
}

pub fn load_or_create() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {parent:?}"))?;
        }
        fs::write(&path, DEFAULT_TOML).with_context(|| format!("write default config {path:?}"))?;
        tracing::info!("wrote default config to {path:?}");
        return Ok(Config::default());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {path:?}"))?;
    let cfg: Config = toml::from_str(&text).with_context(|| format!("parse {path:?}"))?;
    Ok(cfg)
}
