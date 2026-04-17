use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

use crate::Tool;

pub struct RunCommand;

#[async_trait]
impl Tool for RunCommand {
    fn name(&self) -> &str {
        "run_command"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "run_command".into(),
            description: "Run a shell command. Use as last resort when no specific tool fits."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'command' argument"))?;

        let output = tokio::process::Command::new("sh")
            .args(["-c", command])
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(format!("{stdout}{stderr}"))
    }
}
