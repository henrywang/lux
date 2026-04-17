use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

use crate::{Tool, run_cmd_sudo};

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

        match run_cmd_sudo("dnf", &cmd_args).await {
            Ok(out) => Ok(out),
            Err(e) if e.to_string().contains("No match for argument") => {
                try_flatpak_fallback(&packages, e).await
            }
            Err(e) => Err(e),
        }
    }
}

/// If dnf can't find a package, try searching Flathub by name.
/// Returns the original dnf error if no flatpak match is found.
async fn try_flatpak_fallback(packages: &[String], dnf_err: anyhow::Error) -> Result<String> {
    use crate::run_cmd;
    let pkg = packages.first().ok_or(dnf_err)?;
    let search = run_cmd("flatpak", &["search", "--columns=application", pkg])
        .await
        .unwrap_or_default();
    let app_id = search
        .lines()
        .find(|l| {
            let l = l.trim().to_lowercase();
            !l.is_empty() && l.contains(&pkg.to_lowercase())
        })
        .map(str::trim);
    if let Some(app_id) = app_id {
        run_cmd("flatpak", &["install", "-y", "--user", "flathub", app_id]).await
    } else {
        anyhow::bail!(
            "'{pkg}' not found in dnf or flathub. Try a different name, or install manually."
        )
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

        run_cmd_sudo("dnf", &cmd_args).await
    }
}
