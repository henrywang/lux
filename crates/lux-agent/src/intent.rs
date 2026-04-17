//! Rule-based intent matcher for common lux queries.
//!
//! Tries to match user input directly to a tool call without the LLM.
//! Returns None for ambiguous input so the LLM can handle it.
//! Conservative by design — a missed match is better than a wrong match.

use lux_llm::ToolCall;
use lux_tools::SystemMode;
use serde_json::{Value, json};

/// Known GUI apps.
/// (user-typed name, flatpak app ID, optional Fedora dnf package name)
/// If a dnf package is set and we're in Package mode, use dnf. Otherwise flatpak.
const GUI_APPS: &[(&str, &str, Option<&str>)] = &[
    ("firefox", "org.mozilla.firefox", Some("firefox")),
    (
        "thunderbird",
        "org.mozilla.Thunderbird",
        Some("thunderbird"),
    ),
    ("gimp", "org.gimp.GIMP", Some("gimp")),
    ("inkscape", "org.inkscape.Inkscape", Some("inkscape")),
    ("vlc", "org.videolan.VLC", Some("vlc")),
    ("audacity", "org.audacityteam.Audacity", Some("audacity")),
    ("kdenlive", "org.kde.kdenlive", Some("kdenlive")),
    ("blender", "org.blender.Blender", Some("blender")),
    (
        "libreoffice",
        "org.libreoffice.LibreOffice",
        Some("libreoffice"),
    ),
    ("calibre", "com.calibre_ebook.calibre", Some("calibre")),
    ("chromium", "org.chromium.Chromium", Some("chromium")),
    ("obs studio", "com.obsproject.Studio", Some("obs-studio")),
    ("obs", "com.obsproject.Studio", Some("obs-studio")),
    ("telegram", "org.telegram.desktop", Some("telegram-desktop")),
    // Flatpak-only (not in Fedora main repos)
    ("handbrake", "fr.handbrake.ghb", None),
    ("steam", "com.valvesoftware.Steam", None),
    ("discord", "com.discordapp.Discord", None),
    ("slack", "com.slack.Slack", None),
    ("zoom", "us.zoom.Zoom", None),
    ("spotify", "com.spotify.Client", None),
    ("signal", "org.signal.Signal", None),
    ("vs code", "com.visualstudio.code", None),
    ("vscode", "com.visualstudio.code", None),
    ("visual studio code", "com.visualstudio.code", None),
    ("chrome", "com.google.Chrome", None),
    ("zed", "dev.zed.Zed", None),
    ("sublime text", "com.sublimetext.three", None),
    ("sublime", "com.sublimetext.three", None),
    ("bitwarden", "com.bitwarden.desktop", None),
    ("obsidian", "md.obsidian.Obsidian", None),
    ("postman", "com.getpostman.Postman", None),
];

/// Known service name aliases (user term → systemd unit).
const SERVICE_ALIASES: &[(&str, &str)] = &[
    ("bluetooth", "bluetooth"),
    ("sshd", "sshd"),
    ("ssh", "sshd"),
    ("nginx", "nginx"),
    ("apache", "httpd"),
    ("httpd", "httpd"),
    ("docker", "docker"),
    ("podman", "podman"),
    ("cups", "cups"),
    ("firewalld", "firewalld"),
    ("networkmanager", "NetworkManager"),
    ("pipewire", "pipewire"),
    ("pulseaudio", "pulseaudio"),
    ("postgresql", "postgresql"),
    ("postgres", "postgresql"),
    ("mysql", "mysqld"),
    ("mariadb", "mariadb"),
    ("redis", "redis"),
    ("crond", "crond"),
    ("cron", "crond"),
];

fn tool_call(name: &str, args: Value) -> ToolCall {
    ToolCall {
        name: name.to_string(),
        arguments: args,
    }
}

/// Try to match user input to a tool call without the LLM.
pub fn match_intent(input: &str, mode: SystemMode) -> Option<ToolCall> {
    let s = input.to_lowercase();
    let s = s.trim();

    try_run_command(s)
        .or_else(|| try_bootc(s))
        .or_else(|| try_firewall(s))
        .or_else(|| try_network(s))
        .or_else(|| try_disk(s))
        .or_else(|| try_logs(s))
        .or_else(|| try_update(s))
        .or_else(|| try_install(s, mode))
        .or_else(|| try_remove(s))
        .or_else(|| try_service_action(s))
        .or_else(|| try_service_status(s))
}

/// High-confidence system info queries → run_command.
fn try_run_command(s: &str) -> Option<ToolCall> {
    let cmd = if s.contains("ip address")
        || s.contains("ip addr")
        || s.contains("my ip")
        || (s.contains("ip") && s.contains("machine"))
        || (s.contains("ip") && s.contains("this"))
    {
        "ip addr show"
    } else if s.contains("selinux") {
        if s.contains("status") {
            "sestatus"
        } else {
            "getenforce"
        }
    } else if s.contains("uname") || s.contains("kernel version") || s.contains("kernel am i") {
        "uname -r"
    } else if s.contains("uptime") || s.contains("how long") && s.contains("running") {
        "uptime"
    } else if s.contains("hostname") {
        "hostname"
    } else if s.contains("memory") && s.contains("free") || s.contains("ram") && s.contains("free")
    {
        "free -h"
    } else if (s.contains("is") || s.contains("have") || s.contains("do i have"))
        && s.contains("installed")
    {
        // "is vim installed?" / "do I have git installed?" → rpm -q <pkg>
        let pkg = extract_package_for_query(s)?;
        return Some(tool_call(
            "run_command",
            json!({"command": format!("rpm -q {pkg}")}),
        ));
    } else {
        return None;
    };

    Some(tool_call("run_command", json!({"command": cmd})))
}

fn extract_package_for_query(s: &str) -> Option<&str> {
    // "is vim installed" / "do i have git installed" / "is python3 installed"
    // Try to grab the word before "installed"
    let words: Vec<&str> = s.split_whitespace().collect();
    let installed_pos = words
        .iter()
        .position(|&w| w.trim_matches(|c: char| !c.is_alphanumeric()) == "installed")?;
    // Walk back to find the package name (skip "is", "have", "i", "do")
    let skip = ["is", "are", "have", "i", "do", "a", "the", "it"];
    words[..installed_pos]
        .iter()
        .rev()
        .copied()
        .find(|word| !skip.contains(word))
}

fn try_bootc(s: &str) -> Option<ToolCall> {
    if s.contains("rollback")
        || s.contains("roll back")
        || s.contains("go back")
        || (s.contains("revert")
            && (s.contains("update") || s.contains("image") || s.contains("version")))
        || (s.contains("undo") && s.contains("update"))
        || (s.contains("broke") && s.contains("update"))
        || (s.contains("last update") && (s.contains("back") || s.contains("broke")))
    {
        return Some(tool_call("bootc_rollback", json!({})));
    }

    if s.contains("bootc status")
        || (s.contains("what image") || s.contains("which image") || s.contains("current image"))
    {
        return Some(tool_call("bootc_status", json!({})));
    }

    if s.contains("switch to") || s.contains("switch image") {
        // Look for "fedora NN" or bare version number after "to"
        if let Some(ver) = extract_fedora_version(s) {
            return Some(tool_call(
                "bootc_switch",
                json!({"image": format!("quay.io/fedora/fedora-bootc:{ver}")}),
            ));
        }
    }

    None
}

fn extract_fedora_version(s: &str) -> Option<String> {
    let words: Vec<&str> = s.split_whitespace().collect();
    for i in 0..words.len() {
        if words[i] == "fedora"
            && let Some(ver) = words.get(i + 1)
        {
            let ver = ver.trim_matches(|c: char| !c.is_ascii_digit());
            if !ver.is_empty() {
                return Some(ver.to_string());
            }
        }
    }
    None
}

fn try_update(s: &str) -> Option<ToolCall> {
    let is_update = (s.contains("update") || s.contains("upgrade"))
        && (s.contains("all")
            || s.contains("packages")
            || s.contains("system")
            || s.contains("available")
            || s.contains("check"));
    if !is_update {
        return None;
    }
    let check_only = s.contains("check") || s.contains("available") || s.contains("list");
    let args = if check_only {
        json!({"check_only": true})
    } else {
        json!({})
    };
    Some(tool_call("update_system", args))
}

fn try_firewall(s: &str) -> Option<ToolCall> {
    // "disable / turn off / stop the firewall" → manage_service(firewalld, disable/stop)
    if s.contains("firewall") {
        if s.contains("disable") || s.contains("turn off") {
            return Some(tool_call(
                "manage_service",
                json!({"service": "firewalld", "action": "disable"}),
            ));
        }
        if s.contains("stop") {
            return Some(tool_call(
                "manage_service",
                json!({"service": "firewalld", "action": "stop"}),
            ));
        }
        if s.contains("enable") || s.contains("turn on") || s.contains("start") {
            return Some(tool_call(
                "manage_service",
                json!({"service": "firewalld", "action": "enable"}),
            ));
        }
    }

    // Port / service / IP firewall rules
    let ip = extract_ip(s);
    let has_firewall_intent = s.contains("firewall")
        || s.contains("port")
        || ip.is_some() && (s.contains("block") || s.contains("allow") || s.contains("ban"))
        || (s.contains("allow") && (s.contains("through") || s.contains("traffic")))
        || (s.contains("block") && s.contains("port"))
        || (s.contains("open") && s.contains("port"));

    if !has_firewall_intent {
        return None;
    }

    // "unblock" / "unban" mean drop the existing rule, not add an allow rule.
    let action = if s.contains("unblock") || s.contains("unban") {
        "remove"
    } else if s.contains("allow") || s.contains("open") {
        "allow"
    } else if s.contains("block") || s.contains("deny") || s.contains("ban") || s.contains("close")
    {
        "block"
    } else {
        return None;
    };

    let mut args = serde_json::Map::new();
    args.insert("action".into(), json!(action));

    if let Some(src) = ip {
        args.insert("source".into(), json!(src));
    }

    if let Some(port) = extract_port(s) {
        args.insert("port".into(), json!(port));
    }

    for svc in ["http", "https", "ssh", "dns", "smtp", "ftp", "nfs"] {
        if s.contains(svc) {
            args.insert("service".into(), json!(svc));
            break;
        }
    }

    Some(tool_call("manage_firewall", Value::Object(args)))
}

fn extract_ip(s: &str) -> Option<String> {
    s.split_whitespace().find_map(|w| {
        let w = w.trim_matches(|c: char| !c.is_ascii_digit() && c != '.' && c != '/');
        let ip_part = w.split('/').next()?;
        let octets: Vec<&str> = ip_part.split('.').collect();
        if octets.len() == 4 && octets.iter().all(|o| o.parse::<u8>().is_ok()) {
            Some(w.to_string())
        } else {
            None
        }
    })
}

fn extract_port(s: &str) -> Option<String> {
    s.split_whitespace()
        .find(|w| w.chars().all(|c| c.is_ascii_digit()) && w.len() <= 5)
        .map(|p| format!("{p}/tcp"))
}

fn try_network(s: &str) -> Option<ToolCall> {
    let is_network = s.contains("wifi")
        || s.contains("wireless")
        || s.contains("ethernet")
        || (s.contains("internet")
            && (s.contains("not")
                || s.contains("down")
                || s.contains("can't")
                || s.contains("no ")))
        || (s.contains("network")
            && (s.contains("not") || s.contains("down") || s.contains("connect")))
        || s.contains("can't reach")
        || s.contains("no internet")
        || s.contains("dns") && s.contains("not")
        || s.contains("vpn") && s.contains("drop");

    if !is_network {
        return None;
    }

    let interface = if s.contains("wifi") || s.contains("wireless") {
        json!({"interface": "wifi"})
    } else if s.contains("ethernet") {
        json!({"interface": "ethernet"})
    } else {
        json!({})
    };

    Some(tool_call("network_diagnose", interface))
}

fn try_disk(s: &str) -> Option<ToolCall> {
    let is_disk = s.contains("disk")
        && (s.contains("space")
            || s.contains("usage")
            || s.contains("full")
            || s.contains("using"))
        || s.contains("storage") && s.contains("space")
        || s.contains("how much space")
        || s.contains("out of space")
        || s.contains("disk is full")
        || s.contains("disk is almost full");

    if !is_disk {
        return None;
    }

    let path = ["/home", "/var", "/tmp", "/boot", "/usr"]
        .iter()
        .find(|&&p| s.contains(p))
        .copied();

    let args = if let Some(p) = path {
        json!({"path": p})
    } else {
        json!({})
    };

    Some(tool_call("check_disk_usage", args))
}

fn try_logs(s: &str) -> Option<ToolCall> {
    let is_logs = s.contains("log")
        && (s.contains("show") || s.contains("read") || s.contains("check") || s.contains("what"))
        || s.contains("journal")
        || (s.contains("error") || s.contains("critical") || s.contains("warning"))
            && (s.contains("system") || s.contains("today") || s.contains("hour"))
        || s.contains("what happened") && s.contains("with")
        || s.contains("authentication failure")
        || s.contains("failed login")
        || s.contains("how many times") && s.contains("fail");

    if !is_logs {
        return None;
    }

    let mut args = serde_json::Map::new();

    // Priority
    if s.contains("critical") || s.contains("crit") {
        args.insert("priority".into(), json!("crit"));
    } else if s.contains("error") || s.contains("failed") || s.contains("failure") {
        args.insert("priority".into(), json!("err"));
    } else if s.contains("warning") {
        args.insert("priority".into(), json!("warning"));
    }

    // Time
    if s.contains("today") {
        args.insert("since".into(), json!("today"));
    } else if s.contains("last hour") || s.contains("past hour") || s.contains("in the last hour") {
        args.insert("since".into(), json!("1 hour ago"));
    } else if s.contains("yesterday") {
        args.insert("since".into(), json!("yesterday"));
    } else if let Some(mins) = extract_minutes(s) {
        args.insert("since".into(), json!(format!("{mins} minutes ago")));
    }

    // Unit — match known services
    for (keyword, unit) in SERVICE_ALIASES {
        if s.contains(keyword) {
            args.insert("unit".into(), json!(unit));
            break;
        }
    }

    Some(tool_call("read_logs", Value::Object(args)))
}

fn extract_minutes(s: &str) -> Option<u32> {
    let words: Vec<&str> = s.split_whitespace().collect();
    for i in 0..words.len() {
        if (words[i].ends_with("minutes") || words.get(i + 1) == Some(&"minutes"))
            && let Ok(n) = words[i].parse::<u32>()
        {
            return Some(n);
        }
    }
    None
}

fn try_install(s: &str, mode: SystemMode) -> Option<ToolCall> {
    // "uninstall" and "installed" are not install commands
    if s.contains("uninstall") || s.contains("installed") {
        return None;
    }
    if !s.contains("install") && !s.starts_with("get ") {
        return None;
    }

    // Known GUI apps: prefer dnf on package-mode Fedora when available,
    // otherwise use flatpak.
    for (name, app_id, dnf_pkg) in GUI_APPS {
        if s.contains(name) {
            return match (mode, dnf_pkg) {
                (SystemMode::Package, Some(pkg)) => {
                    Some(tool_call("install_package", json!({"packages": [pkg]})))
                }
                _ => Some(tool_call("install_flatpak", json!({"app_id": app_id}))),
            };
        }
    }

    // Extract package names — conservative: only match simple "install X [and Y]" patterns
    let rest = s.split("install").nth(1)?.trim().to_string();
    let packages = extract_package_names(&rest);
    if packages.is_empty() {
        return None;
    }

    Some(tool_call("install_package", json!({"packages": packages})))
}

fn extract_package_names(s: &str) -> Vec<String> {
    let stop = [
        "the", "a", "an", "for", "me", "my", "please", "i", "want", "need", "and", "also", "both",
        "some", "using", "with", "on", "in", "to",
    ];
    s.split([',', ' '])
        .flat_map(|w| w.split("and"))
        .map(|w| {
            w.trim()
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                .to_string()
        })
        .filter(|w| !w.is_empty() && !stop.contains(&w.as_str()) && w.len() > 1)
        .collect()
}

fn try_remove(s: &str) -> Option<ToolCall> {
    let prefix = if s.contains("uninstall") {
        "uninstall"
    } else if s.contains("remove") {
        "remove"
    } else {
        return None;
    };

    let rest = s.split(prefix).nth(1)?.trim().to_string();
    let packages = extract_package_names(&rest);
    if packages.is_empty() {
        return None;
    }

    Some(tool_call("remove_package", json!({"packages": packages})))
}

fn try_service_action(s: &str) -> Option<ToolCall> {
    let action = if s.contains("restart") {
        "restart"
    } else if s.contains("enable") {
        "enable"
    } else if s.contains("disable") {
        "disable"
    } else if s.contains("stop") && !s.contains("stopped") {
        "stop"
    } else if s.contains("start") && !s.contains("restart") {
        "start"
    } else {
        return None;
    };

    // Require a known service name to avoid false positives
    let service = resolve_service(s)?;
    Some(tool_call(
        "manage_service",
        json!({"service": service, "action": action}),
    ))
}

fn try_service_status(s: &str) -> Option<ToolCall> {
    let is_status = (s.contains("is ")
        && (s.contains("running") || s.contains("active") || s.contains("working")))
        || (s.contains("check") && s.contains("service"))
        || s.contains("status of")
        || (s.contains("my") && s.contains("working"));

    if !is_status {
        return None;
    }

    // "my printer isn't working" → cups
    if (s.contains("printer") || s.contains("printing")) && s.contains("working") {
        return Some(tool_call(
            "check_service_status",
            json!({"service": "cups"}),
        ));
    }

    let service = resolve_service(s)?;
    Some(tool_call(
        "check_service_status",
        json!({"service": service}),
    ))
}

fn resolve_service(s: &str) -> Option<&'static str> {
    // Check printer/bluetooth device issues first
    if s.contains("printer") || s.contains("printing") {
        return Some("cups");
    }
    if s.contains("bluetooth")
        || s.contains("headphone")
        || s.contains("speaker") && !s.contains("firewall")
    {
        return Some("bluetooth");
    }
    for (keyword, service) in SERVICE_ALIASES {
        if s.contains(keyword) {
            return Some(service);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_tool(input: &str, expected_tool: &str) {
        let result = match_intent(input, SystemMode::Package)
            .unwrap_or_else(|| panic!("no match for: {input:?}"));
        assert_eq!(result.name, expected_tool, "wrong tool for: {input:?}");
    }

    fn assert_tool_args(input: &str, expected_tool: &str, check: impl FnOnce(&Value)) {
        let result = match_intent(input, SystemMode::Package)
            .unwrap_or_else(|| panic!("no match for: {input:?}"));
        assert_eq!(result.name, expected_tool, "wrong tool for: {input:?}");
        check(&result.arguments);
    }

    fn assert_tool_args_mode(
        input: &str,
        mode: SystemMode,
        expected_tool: &str,
        check: impl FnOnce(&Value),
    ) {
        let result = match_intent(input, mode).unwrap_or_else(|| panic!("no match for: {input:?}"));
        assert_eq!(result.name, expected_tool, "wrong tool for: {input:?}");
        check(&result.arguments);
    }

    fn assert_no_match(input: &str) {
        assert!(
            match_intent(input, SystemMode::Package).is_none(),
            "unexpected match for: {input:?}"
        );
    }

    // ---- install_flatpak vs install_package (GUI apps) ----

    #[test]
    fn install_gui_apps_package_mode() {
        // Apps in Fedora repos should use dnf on package-mode systems.
        assert_tool_args("install firefox", "install_package", |args| {
            assert_eq!(args["packages"][0], "firefox");
        });
        assert_tool_args("install VLC media player", "install_package", |args| {
            assert_eq!(args["packages"][0], "vlc");
        });
        // Apps not in Fedora repos must use flatpak.
        assert_tool_args("install steam", "install_flatpak", |args| {
            assert_eq!(args["app_id"], "com.valvesoftware.Steam");
        });
        assert_tool_args("install vscode", "install_flatpak", |args| {
            assert_eq!(args["app_id"], "com.visualstudio.code");
        });
    }

    #[test]
    fn install_gui_apps_image_mode() {
        // Image-mode (bootc) systems always use flatpak.
        assert_tool_args_mode(
            "install firefox",
            SystemMode::Image,
            "install_flatpak",
            |args| {
                assert_eq!(args["app_id"], "org.mozilla.firefox");
            },
        );
        assert_tool_args_mode(
            "install gimp",
            SystemMode::Image,
            "install_flatpak",
            |args| {
                assert_eq!(args["app_id"], "org.gimp.GIMP");
            },
        );
    }

    // ---- install_package ----

    #[test]
    fn install_cli_tools() {
        assert_tool_args("install vim", "install_package", |args| {
            let pkgs = args["packages"].as_array().unwrap();
            assert!(pkgs.iter().any(|p| p == "vim"));
        });
        assert_tool_args("install htop and tmux", "install_package", |args| {
            let pkgs = args["packages"].as_array().unwrap();
            assert!(pkgs.iter().any(|p| p == "htop"));
            assert!(pkgs.iter().any(|p| p == "tmux"));
        });
    }

    // ---- remove_package ----

    #[test]
    fn remove_packages() {
        assert_tool_args("remove libreoffice", "remove_package", |args| {
            let pkgs = args["packages"].as_array().unwrap();
            assert!(pkgs.iter().any(|p| p == "libreoffice"));
        });
        assert_tool("uninstall firefox from system packages", "remove_package");
    }

    // ---- manage_service ----

    #[test]
    fn service_actions() {
        assert_tool_args("restart nginx", "manage_service", |args| {
            assert_eq!(args["service"], "nginx");
            assert_eq!(args["action"], "restart");
        });
        assert_tool_args("stop the docker service", "manage_service", |args| {
            assert_eq!(args["service"], "docker");
            assert_eq!(args["action"], "stop");
        });
        assert_tool_args(
            "enable SSH so I can connect remotely",
            "manage_service",
            |args| {
                assert_eq!(args["service"], "sshd");
                assert_eq!(args["action"], "enable");
            },
        );
        assert_tool_args(
            "enable and start the bluetooth service",
            "manage_service",
            |args| {
                assert_eq!(args["service"], "bluetooth");
                assert_eq!(args["action"], "enable");
            },
        );
    }

    // ---- check_service_status ----

    #[test]
    fn service_status() {
        assert_tool_args("is sshd running?", "check_service_status", |args| {
            assert_eq!(args["service"], "sshd");
        });
        assert_tool_args(
            "check if the cups printing service is running",
            "check_service_status",
            |args| {
                assert_eq!(args["service"], "cups");
            },
        );
        assert_tool_args("my printer isn't working", "check_service_status", |args| {
            assert_eq!(args["service"], "cups");
        });
    }

    // ---- read_logs ----

    #[test]
    fn log_queries() {
        assert_tool_args(
            "show me recent errors in the system log",
            "read_logs",
            |args| {
                assert_eq!(args["priority"], "err");
            },
        );
        assert_tool_args(
            "what critical errors happened today?",
            "read_logs",
            |args| {
                assert_eq!(args["priority"], "crit");
                assert_eq!(args["since"], "today");
            },
        );
        assert_tool_args(
            "show me what happened with sshd in the last hour",
            "read_logs",
            |args| {
                assert_eq!(args["unit"], "sshd");
                assert_eq!(args["since"], "1 hour ago");
            },
        );
        assert_tool_args(
            "how many times did sshd fail authentication today?",
            "read_logs",
            |args| {
                assert_eq!(args["unit"], "sshd");
                assert_eq!(args["since"], "today");
            },
        );
    }

    // ---- network_diagnose ----

    #[test]
    fn network_issues() {
        assert_tool_args("wifi is not working", "network_diagnose", |args| {
            assert_eq!(args["interface"], "wifi");
        });
        assert_tool("I can't reach the internet", "network_diagnose");
        assert_tool_args("ethernet is not working", "network_diagnose", |args| {
            assert_eq!(args["interface"], "ethernet");
        });
    }

    // ---- check_disk_usage ----

    #[test]
    fn disk_queries() {
        assert_tool(
            "my disk is almost full, what's going on?",
            "check_disk_usage",
        );
        assert_tool_args(
            "check how much space /home is using",
            "check_disk_usage",
            |args| {
                assert_eq!(args["path"], "/home");
            },
        );
    }

    // ---- manage_firewall ----

    #[test]
    fn firewall_rules() {
        assert_tool_args(
            "open port 8080 in the firewall",
            "manage_firewall",
            |args| {
                assert_eq!(args["action"], "allow");
                assert_eq!(args["port"], "8080/tcp");
            },
        );
        assert_tool_args(
            "allow HTTP and HTTPS through the firewall",
            "manage_firewall",
            |args| {
                assert_eq!(args["action"], "allow");
                assert_eq!(args["service"], "http");
            },
        );
        assert_tool_args("block IP 192.168.1.100", "manage_firewall", |args| {
            assert_eq!(args["action"], "block");
            assert_eq!(args["source"], "192.168.1.100");
        });
        // "unblock" must remove the rule, not re-add it as block or allow.
        assert_tool_args("unblock IP 192.168.1.100", "manage_firewall", |args| {
            assert_eq!(args["action"], "remove");
            assert_eq!(args["source"], "192.168.1.100");
        });
    }

    #[test]
    fn system_update() {
        assert_tool_args("update all packages", "update_system", |_| {});
        assert_tool_args("upgrade the system", "update_system", |_| {});
        assert_tool_args("check for available updates", "update_system", |args| {
            assert_eq!(args["check_only"], true);
        });
    }

    #[test]
    fn disable_firewall_is_service_action() {
        assert_tool_args("disable the firewall", "manage_service", |args| {
            assert_eq!(args["service"], "firewalld");
            assert_eq!(args["action"], "disable");
        });
    }

    // ---- bootc ----

    #[test]
    fn bootc_operations() {
        assert_tool("the last update broke my system, go back", "bootc_rollback");
        assert_tool("roll back to the previous version", "bootc_rollback");
        assert_tool("what image am I running?", "bootc_status");
        assert_tool_args("switch to the fedora 41 image", "bootc_switch", |args| {
            assert!(args["image"].as_str().unwrap().contains("41"));
        });
    }

    // ---- run_command ----

    #[test]
    fn system_info_queries() {
        assert_tool_args("what's my ip address", "run_command", |args| {
            assert!(args["command"].as_str().unwrap().contains("ip addr"));
        });
        assert_tool_args("what's the selinux mode?", "run_command", |args| {
            assert!(args["command"].as_str().unwrap().contains("getenforce"));
        });
        assert_tool_args("do I have vim installed?", "run_command", |args| {
            assert!(args["command"].as_str().unwrap().contains("rpm -q vim"));
        });
    }

    // ---- fallthrough to LLM ----

    #[test]
    fn ambiguous_input_falls_through() {
        assert_no_match("help me with my computer");
        assert_no_match("what should I do?");
        assert_no_match("hello");
    }
}
