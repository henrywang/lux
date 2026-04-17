use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

use crate::{Tool, run_cmd_sudo};

pub struct ManageService;
pub struct CheckServiceStatus;

#[async_trait]
impl Tool for ManageService {
    fn name(&self) -> &str {
        "manage_service"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "manage_service".into(),
            description: "Start, stop, enable, disable, or restart a systemd service".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "service": {
                        "type": "string",
                        "description": "Service name (e.g. bluetooth, cups, sshd)"
                    },
                    "action": {
                        "type": "string",
                        "enum": ["start", "stop", "enable", "disable", "restart"],
                        "description": "Action to perform"
                    }
                },
                "required": ["service", "action"]
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let service = args
            .get("service")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'service' argument"))?;
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'action' argument"))?;

        run_cmd_sudo("systemctl", &[action, service]).await
    }
}

#[async_trait]
impl Tool for CheckServiceStatus {
    fn name(&self) -> &str {
        "check_service_status"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "check_service_status".into(),
            description: "Check the status of a systemd service".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "service": {
                        "type": "string",
                        "description": "Service name to check"
                    }
                },
                "required": ["service"]
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let service = args
            .get("service")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'service' argument"))?;

        // systemctl status returns non-zero for inactive services, so handle that
        let output = tokio::process::Command::new("systemctl")
            .args(["status", service])
            .output()
            .await?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr))
    }
}
