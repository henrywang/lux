//! Failed systemd units.

use std::process::Command;

use super::Finding;

pub fn detect() -> Vec<Finding> {
    let out = match Command::new("systemctl")
        .args(["--failed", "--no-legend", "--plain"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter_map(|line| {
            let unit = line.split_whitespace().next()?;
            if unit.is_empty() {
                return None;
            }
            Some(Finding {
                category: "unit-failed".into(),
                summary: format!("systemd unit {unit} is failed"),
                suggested_fix: Some(format!("systemctl restart {unit}")),
            })
        })
        .collect()
}
