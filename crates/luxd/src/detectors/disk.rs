//! Disk-usage detector: flags filesystems above 90% full.

use std::process::Command;

use super::Finding;

const THRESHOLD_PCT: u32 = 90;

pub fn detect() -> Vec<Finding> {
    let out = match Command::new("df")
        .args([
            "--output=source,pcent,target",
            "-x",
            "tmpfs",
            "-x",
            "devtmpfs",
        ])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let source = it.next()?;
            let pcent = it.next()?.trim_end_matches('%').parse::<u32>().ok()?;
            let mount = it.next()?;
            if pcent >= THRESHOLD_PCT {
                Some(Finding {
                    category: "disk-full".into(),
                    summary: format!("{source} mounted at {mount} is {pcent}% full"),
                    suggested_fix: if mount == "/" {
                        Some("dnf clean all".into())
                    } else {
                        None
                    },
                })
            } else {
                None
            }
        })
        .collect()
}
