use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

use crate::{Tool, run_cmd};

pub struct InstallFlatpak;

#[async_trait]
impl Tool for InstallFlatpak {
    fn name(&self) -> &str {
        "install_flatpak"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "install_flatpak".into(),
            description: "Install a desktop application via Flatpak".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "app_id": {
                        "type": "string",
                        "description": "Flatpak application ID (e.g. org.mozilla.firefox)"
                    }
                },
                "required": ["app_id"]
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let app_id = args
            .get("app_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'app_id' argument"))?;

        run_cmd("flatpak", &["install", "-y", "flathub", app_id]).await
    }
}
