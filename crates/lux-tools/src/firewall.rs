use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

use crate::{Tool, run_cmd_sudo};

pub struct ManageFirewall;

#[async_trait]
impl Tool for ManageFirewall {
    fn name(&self) -> &str {
        "manage_firewall"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "manage_firewall".into(),
            description: "Manage firewalld rules: allow/block ports, services, or source IPs"
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["allow", "block", "remove"],
                        "description": "allow: open port/service. block: reject traffic (source IP, port, or service). remove: remove an existing rule."
                    },
                    "service": {
                        "type": "string",
                        "description": "Service name (e.g. ssh, http, https)"
                    },
                    "port": {
                        "type": "string",
                        "description": "Port with protocol (e.g. '8080/tcp')"
                    },
                    "source": {
                        "type": "string",
                        "description": "Source IP or CIDR to target (e.g. '192.168.1.100' or '10.0.0.0/8')"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'action' argument"))?;
        let service = args.get("service").and_then(|v| v.as_str());
        let port = args.get("port").and_then(|v| v.as_str());
        let source = args.get("source").and_then(|v| v.as_str());

        let flag = build_flag(action, service, port, source)?;
        run_cmd_sudo("firewall-cmd", &[&flag, "--permanent"]).await?;
        run_cmd_sudo("firewall-cmd", &["--reload"]).await
    }
}

fn build_flag(
    action: &str,
    service: Option<&str>,
    port: Option<&str>,
    source: Option<&str>,
) -> Result<String> {
    match (action, source, service, port) {
        // Rich rule: source IP with optional port/service
        ("allow" | "block", Some(src), svc, prt) => {
            let verb = if action == "allow" {
                "accept"
            } else {
                "reject"
            };
            let rule = rich_rule(src, svc, prt, verb)?;
            Ok(format!("--add-rich-rule={rule}"))
        }
        // Simple allow: open port or service
        ("allow", None, Some(s), _) => Ok(format!("--add-service={s}")),
        ("allow", None, None, Some(p)) => Ok(format!("--add-port={p}")),
        // Remove existing rule by source IP (mirrors the rule `block` added).
        ("remove", Some(src), svc, prt) => {
            let rule = rich_rule(src, svc, prt, "reject")?;
            Ok(format!("--remove-rich-rule={rule}"))
        }
        ("remove", None, Some(s), _) => Ok(format!("--remove-service={s}")),
        ("remove", None, None, Some(p)) => Ok(format!("--remove-port={p}")),
        // Block without source: use rich rule
        ("block", None, Some(s), _) => {
            Ok(format!("--add-rich-rule=rule service name=\"{s}\" reject"))
        }
        ("block", None, None, Some(p)) => {
            let (num, proto) = parse_port(p)?;
            Ok(format!(
                "--add-rich-rule=rule port port=\"{num}\" protocol=\"{proto}\" reject"
            ))
        }
        _ => anyhow::bail!("must specify 'service', 'port', or 'source'"),
    }
}

fn parse_port(p: &str) -> Result<(&str, &str)> {
    p.split_once('/')
        .ok_or_else(|| anyhow::anyhow!("port must be in 'NUMBER/PROTOCOL' format, got '{p}'"))
}

fn rich_rule(src: &str, svc: Option<&str>, prt: Option<&str>, verb: &str) -> Result<String> {
    let filter = match (svc, prt) {
        (Some(s), _) => format!(" service name=\"{s}\""),
        (_, Some(p)) => {
            let (num, proto) = parse_port(p)?;
            format!(" port port=\"{num}\" protocol=\"{proto}\"")
        }
        _ => String::new(),
    };
    Ok(format!(
        "rule family=\"ipv4\" source address=\"{src}\"{filter} {verb}"
    ))
}
