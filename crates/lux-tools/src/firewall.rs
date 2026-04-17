use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

use crate::{run_cmd, Tool};

pub struct ManageFirewall;

#[async_trait]
impl Tool for ManageFirewall {
    fn name(&self) -> &str {
        "manage_firewall"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "manage_firewall".into(),
            description: "Add or remove firewall rules via firewalld".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["allow", "deny"],
                        "description": "Allow or deny traffic"
                    },
                    "service": {
                        "type": "string",
                        "description": "Service name (e.g. ssh, http, https)"
                    },
                    "port": {
                        "type": "string",
                        "description": "Port/protocol (e.g. '8080/tcp')"
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

        let fw_action = match action {
            "allow" => "--add",
            "deny" => "--remove",
            _ => anyhow::bail!("unknown action: {action}"),
        };

        if let Some(service) = args.get("service").and_then(|v| v.as_str()) {
            let flag = format!("{fw_action}-service={service}");
            run_cmd("firewall-cmd", &[&flag, "--permanent"]).await?;
        } else if let Some(port) = args.get("port").and_then(|v| v.as_str()) {
            let flag = format!("{fw_action}-port={port}");
            run_cmd("firewall-cmd", &[&flag, "--permanent"]).await?;
        } else {
            anyhow::bail!("must specify 'service' or 'port'");
        }

        // Reload to apply
        run_cmd("firewall-cmd", &["--reload"]).await
    }
}
