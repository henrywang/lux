//! luxd — lux daemon.
//!
//! Periodically runs a set of pure-Rust detectors (failed units, AVC
//! denials, disk usage) and surfaces findings to the user via desktop
//! notifications and a JSONL file the lux REPL reads on startup.

mod config;
mod detectors;

use anyhow::Result;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

use crate::config::{Config, Mode};
use crate::detectors::Finding;

/// Hand-picked fix commands that `auto` mode is allowed to run without asking.
/// Anything else falls back to `suggest` behavior.
const AUTO_ALLOWLIST: &[&str] = &["dnf clean all"];

fn findings_path() -> PathBuf {
    let uid = unsafe { libc::geteuid() };
    let dir = PathBuf::from(format!("/run/user/{uid}/lux"));
    let _ = fs::create_dir_all(&dir);
    dir.join("findings.jsonl")
}

fn write_finding(path: &std::path::Path, f: &Finding) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(f)?)?;
    Ok(())
}

fn desktop_notify(f: &Finding) {
    // Best effort — fail silently if notify-send isn't installed.
    let _ = Command::new("notify-send")
        .args(["-a", "lux", "lux detected an issue", &f.summary])
        .status();
}

fn try_auto_fix(f: &Finding) -> bool {
    let Some(fix) = f.suggested_fix.as_deref() else {
        return false;
    };
    if !AUTO_ALLOWLIST.contains(&fix) {
        return false;
    }
    let mut parts = fix.split_whitespace();
    let Some(cmd) = parts.next() else {
        return false;
    };
    let args: Vec<&str> = parts.collect();
    match Command::new(cmd).args(&args).status() {
        Ok(s) if s.success() => {
            tracing::info!("auto-fixed: {fix}");
            true
        }
        _ => false,
    }
}

fn handle(cfg: &Config, findings: Vec<Finding>) {
    if findings.is_empty() {
        return;
    }
    let path = findings_path();
    // Truncate so the REPL always sees the current snapshot, not history.
    let _ = fs::write(&path, "");

    for f in findings {
        tracing::info!("finding: {} - {}", f.category, f.summary);

        let mut handled = false;
        if cfg.mode == Mode::Auto {
            handled = try_auto_fix(&f);
        }

        if !handled {
            if cfg.notify_repl
                && let Err(e) = write_finding(&path, &f)
            {
                tracing::warn!("failed to write finding: {e}");
            }
            if cfg.notify_desktop {
                desktop_notify(&f);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("luxd v{} starting", env!("CARGO_PKG_VERSION"));

    let cfg = config::load_or_create()?;
    tracing::info!("mode={:?} interval={}s", cfg.mode, cfg.interval_secs);

    let mut ticker = tokio::time::interval(Duration::from_secs(cfg.interval_secs));
    loop {
        ticker.tick().await;
        let findings = detectors::run_all(&cfg);
        handle(&cfg, findings);
    }
}
