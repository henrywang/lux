use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

use crate::{run_cmd, Tool};

pub struct BootcSwitch;
pub struct BootcRollback;
pub struct BootcStatus;

#[async_trait]
impl Tool for BootcSwitch {
    fn name(&self) -> &str {
        "bootc_switch"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "bootc_switch".into(),
            description: "Switch to a different bootc image (image mode only)".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "image": {
                        "type": "string",
                        "description": "OCI image reference to switch to"
                    }
                },
                "required": ["image"]
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let image = args
            .get("image")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'image' argument"))?;

        run_cmd("bootc", &["switch", image]).await
    }
}

#[async_trait]
impl Tool for BootcRollback {
    fn name(&self) -> &str {
        "bootc_rollback"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "bootc_rollback".into(),
            description: "Rollback to the previous bootc image".into(),
            parameters: serde_json::json!({
                "type": "object"
            }),
        }
    }

    async fn execute(&self, _args: &Value) -> Result<String> {
        run_cmd("bootc", &["rollback"]).await
    }
}

#[async_trait]
impl Tool for BootcStatus {
    fn name(&self) -> &str {
        "bootc_status"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "bootc_status".into(),
            description: "Show current bootc image status and available updates".into(),
            parameters: serde_json::json!({
                "type": "object"
            }),
        }
    }

    async fn execute(&self, _args: &Value) -> Result<String> {
        run_cmd("bootc", &["status"]).await
    }
}
