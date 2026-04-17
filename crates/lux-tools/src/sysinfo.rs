//! Host summary printed in the lux banner and via the /sysinfo REPL command.
//!
//! Pure best-effort: every field falls back to "unknown" if the source file
//! or command is missing, so we never panic on exotic systems.

use std::fmt;
use std::fs;
use std::process::Command;

use crate::SystemMode;

#[derive(Debug, Clone)]
pub struct SysInfo {
    pub distro: String,
    pub host_type: String,
    pub cpu: String,
    pub cpu_cores: usize,
    pub mem_total_gb: f64,
    pub mem_avail_gb: f64,
    pub disk_total_gb: f64,
    pub disk_free_gb: f64,
    pub uptime: String,
    pub network: String,
    pub vpn: String,
    pub mode: SystemMode,
}

pub fn collect(mode: SystemMode) -> SysInfo {
    let (network, vpn) = read_network();
    SysInfo {
        distro: read_distro(),
        host_type: detect_host_type(),
        cpu: read_cpu_model(),
        cpu_cores: read_cpu_cores(),
        mem_total_gb: read_mem("MemTotal"),
        mem_avail_gb: read_mem("MemAvailable"),
        disk_total_gb: read_disk_total(),
        disk_free_gb: read_disk_free(),
        uptime: read_uptime(),
        network,
        vpn,
        mode,
    }
}

fn read_distro() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("PRETTY_NAME="))
                .map(|v| v.trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "Linux".into())
}

fn detect_host_type() -> String {
    // Cloud DMI hints beat systemd-detect-virt for naming.
    if let Ok(v) = fs::read_to_string("/sys/class/dmi/id/sys_vendor") {
        let v = v.trim();
        match v {
            "Amazon EC2" => return "AWS EC2".into(),
            "Google" => return "Google Cloud".into(),
            "Microsoft Corporation" => return "Azure".into(),
            _ => {}
        }
    }

    // systemd-detect-virt exits non-zero when no virt is detected, so
    // we look at stdout regardless of status.
    match Command::new("systemd-detect-virt").output() {
        Ok(out) => {
            let virt = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if virt.is_empty() || virt == "none" {
                "bare-metal".into()
            } else {
                virt
            }
        }
        Err(_) => "unknown".into(),
    }
}

fn read_cpu_model() -> String {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|v| v.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".into())
}

fn read_cpu_cores() -> usize {
    fs::read_to_string("/proc/cpuinfo")
        .map(|s| s.lines().filter(|l| l.starts_with("processor")).count())
        .unwrap_or(0)
}

fn read_mem(key: &str) -> f64 {
    fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix(key))
                .and_then(|r| r.trim_start().strip_prefix(':'))
                .and_then(|v| v.split_whitespace().next())
                .and_then(|n| n.parse::<u64>().ok())
        })
        .map(|kb| kb as f64 / 1024.0 / 1024.0)
        .unwrap_or(0.0)
}

/// Parse `df -BK --output=size,avail /` — portable and dep-free.
fn read_df() -> Option<(u64, u64)> {
    let out = Command::new("df")
        .args(["-BK", "--output=size,avail", "/"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().nth(1)?;
    let mut it = line.split_whitespace();
    let size = it.next()?.trim_end_matches('K').parse::<u64>().ok()?;
    let avail = it.next()?.trim_end_matches('K').parse::<u64>().ok()?;
    Some((size, avail))
}

fn read_disk_total() -> f64 {
    read_df()
        .map(|(s, _)| s as f64 / 1024.0 / 1024.0)
        .unwrap_or(0.0)
}

fn read_disk_free() -> f64 {
    read_df()
        .map(|(_, a)| a as f64 / 1024.0 / 1024.0)
        .unwrap_or(0.0)
}

fn read_uptime() -> String {
    let secs = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(str::to_string))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0) as u64;
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

/// Returns (network, vpn). Physical access is always WIFI or ETHERNET
/// (or "offline"); ppp/tun/wg are reported separately as VPN overlays.
///
/// Examples:
///   ("WIFI 192.168.1.42",       "N/A")
///   ("ETHERNET 10.0.0.5",       "IPsec")
///   ("offline",                 "SSL")
fn read_network() -> (String, String) {
    let out = match Command::new("ip").args(["-br", "addr"]).output() {
        Ok(o) if o.status.success() => o,
        _ => return ("offline".into(), "N/A".into()),
    };
    let text = String::from_utf8_lossy(&out.stdout);

    let mut primary: Option<(&'static str, String)> = None;
    let mut vpn: Option<&'static str> = None;

    for line in text.lines() {
        let mut it = line.split_whitespace();
        let Some(dev) = it.next() else { continue };
        let Some(state) = it.next() else { continue };
        let Some(ipcidr) = it.next() else { continue };
        let ip = ipcidr.split('/').next().unwrap_or(ipcidr);
        if ip.starts_with("127.") || ip == "::1" {
            continue;
        }

        // Physical access: require UP so a plugged-in-but-unused NIC
        // doesn't mask the real route.
        if state == "UP" && primary.is_none() {
            if dev.starts_with("wl") {
                primary = Some(("WIFI", ip.into()));
                continue;
            }
            if dev.starts_with("en") || dev.starts_with("eth") {
                primary = Some(("ETHERNET", ip.into()));
                continue;
            }
        }

        // VPN overlays: state is often UNKNOWN for ppp/tun even when active,
        // so don't gate on it here.
        if vpn.is_none() {
            if dev.starts_with("ppp") {
                vpn = Some("IPsec");
            } else if dev.starts_with("tun") || dev.starts_with("tap") {
                vpn = Some("SSL");
            } else if dev.starts_with("wg") {
                vpn = Some("WireGuard");
            }
        }
    }

    let network = primary
        .map(|(kind, ip)| format!("{kind} {ip}"))
        .unwrap_or_else(|| "offline".into());
    let vpn = vpn.unwrap_or("N/A").to_string();
    (network, vpn)
}

impl fmt::Display for SysInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode = match self.mode {
            SystemMode::Image => "image",
            SystemMode::Package => "package",
        };
        writeln!(f, "  Host:    {} ({})", self.distro, self.host_type)?;
        writeln!(f, "  CPU:     {} ({} cores)", self.cpu, self.cpu_cores)?;
        writeln!(
            f,
            "  Memory:  {:.1} / {:.1} GB available",
            self.mem_avail_gb, self.mem_total_gb
        )?;
        writeln!(
            f,
            "  Disk /:  {:.0} GB free of {:.0} GB",
            self.disk_free_gb, self.disk_total_gb
        )?;
        writeln!(f, "  Network: {}", self.network)?;
        writeln!(f, "  VPN:     {}", self.vpn)?;
        write!(f, "  Uptime:  {}    Mode: {}", self.uptime, mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_does_not_panic() {
        let info = collect(SystemMode::Package);
        // We can't assert specific values (varies per host), but formatting
        // must succeed and produce non-empty output.
        let s = format!("{info}");
        assert!(!s.is_empty());
    }

    #[test]
    fn uptime_formats_correctly() {
        // Just ensures read_uptime returns a sane string.
        let u = read_uptime();
        assert!(!u.is_empty());
    }
}
