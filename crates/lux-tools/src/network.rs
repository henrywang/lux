use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

use crate::{run_cmd, Tool};

pub struct NetworkDiagnose;

#[async_trait]
impl Tool for NetworkDiagnose {
    fn name(&self) -> &str {
        "network_diagnose"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "network_diagnose".into(),
            description: "Diagnose network connectivity issues".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "interface": {
                        "type": "string",
                        "description": "Network interface to diagnose (e.g. wifi, ethernet)"
                    }
                }
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let mut results = Vec::new();

        // General connectivity check
        results.push("=== Network Interfaces ===".into());
        results.push(run_cmd("nmcli", &["device", "status"]).await.unwrap_or_else(|e| e.to_string()));

        // Check specific interface if requested
        if let Some(iface) = args.get("interface").and_then(|v| v.as_str()) {
            results.push(format!("\n=== {iface} Details ==="));
            match iface {
                "wifi" | "wireless" => {
                    results.push(run_cmd("nmcli", &["radio", "wifi"]).await.unwrap_or_else(|e| e.to_string()));
                    results.push(run_cmd("nmcli", &["device", "wifi", "list"]).await.unwrap_or_else(|e| e.to_string()));
                }
                _ => {
                    results.push(run_cmd("nmcli", &["device", "show", iface]).await.unwrap_or_else(|e| e.to_string()));
                }
            }
        }

        // DNS check
        results.push("\n=== DNS Resolution ===".into());
        results.push(run_cmd("resolvectl", &["status"]).await.unwrap_or_else(|_| {
            "resolvectl not available".into()
        }));

        // Connectivity check
        results.push("\n=== Connectivity ===".into());
        results.push(run_cmd("ping", &["-c", "2", "-W", "3", "1.1.1.1"]).await.unwrap_or_else(|e| e.to_string()));

        Ok(results.join("\n"))
    }
}
