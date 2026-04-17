//! SELinux AVC denials from the journal.
//!
//! We scan the last hour so a periodic poll catches recent denials without
//! flooding the user with ancient noise.

use std::process::Command;

use super::Finding;

pub fn detect() -> Vec<Finding> {
    let out = match Command::new("journalctl")
        .args([
            "--since",
            "1 hour ago",
            "--no-pager",
            "-q",
            "-g",
            "AVC.*denied",
        ])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let text = String::from_utf8_lossy(&out.stdout);
    let count = text.lines().filter(|l| l.contains("avc:")).count();
    if count == 0 {
        return Vec::new();
    }
    vec![Finding {
        category: "avc".into(),
        summary: format!("{count} SELinux AVC denial(s) in the last hour"),
        suggested_fix: Some("ausearch -m AVC -ts recent | audit2allow -a".into()),
    }]
}
