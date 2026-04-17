use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

use crate::{Tool, run_cmd_sudo};

pub struct UpdateSystem;

#[async_trait]
impl Tool for UpdateSystem {
    fn name(&self) -> &str {
        "update_system"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "update_system".into(),
            description: "Update all system packages via dnf upgrade".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "check_only": {
                        "type": "boolean",
                        "description": "Only list available updates, don't install"
                    }
                }
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let check_only = args
            .get("check_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if check_only {
            run_cmd_sudo("dnf", &["check-upgrade"]).await
        } else {
            run_cmd_sudo("dnf", &["-y", "upgrade"]).await
        }
    }
}
