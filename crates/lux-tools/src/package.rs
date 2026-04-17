use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

use crate::{Tool, run_cmd};

pub struct InstallPackage;
pub struct RemovePackage;

#[async_trait]
impl Tool for InstallPackage {
    fn name(&self) -> &str {
        "install_package"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "install_package".into(),
            description: "Install system packages via dnf".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "packages": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Package names to install"
                    }
                },
                "required": ["packages"]
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let packages: Vec<String> = serde_json::from_value(
            args.get("packages")
                .ok_or_else(|| anyhow::anyhow!("missing 'packages' argument"))?
                .clone(),
        )?;

        let mut cmd_args = vec!["-y", "install"];
        let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
        cmd_args.extend(pkg_refs.iter());

        run_cmd("dnf", &cmd_args).await
    }
}

#[async_trait]
impl Tool for RemovePackage {
    fn name(&self) -> &str {
        "remove_package"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "remove_package".into(),
            description: "Remove system packages".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "packages": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Package names to remove"
                    }
                },
                "required": ["packages"]
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let packages: Vec<String> = serde_json::from_value(
            args.get("packages")
                .ok_or_else(|| anyhow::anyhow!("missing 'packages' argument"))?
                .clone(),
        )?;

        let mut cmd_args = vec!["-y", "remove"];
        let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
        cmd_args.extend(pkg_refs.iter());

        run_cmd("dnf", &cmd_args).await
    }
}
