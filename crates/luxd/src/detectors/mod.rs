//! Issue detectors run by the luxd background loop.

mod avc;
mod disk;
mod units;

use serde::Serialize;

use crate::config::Config;

/// A single problem lux has detected on the system.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// Short category tag: "unit-failed", "avc", "disk-full".
    pub category: String,
    /// Human summary (one line).
    pub summary: String,
    /// A shell command the user (or auto mode) can run to fix it.
    /// None means "no safe automatic fix; investigate manually".
    pub suggested_fix: Option<String>,
}

/// Run every enabled detector once and return all findings.
pub fn run_all(cfg: &Config) -> Vec<Finding> {
    let mut out = Vec::new();
    if cfg.watch_journal {
        out.extend(units::detect());
    }
    if cfg.watch_avc {
        out.extend(avc::detect());
    }
    if cfg.watch_disk {
        out.extend(disk::detect());
    }
    out
}
