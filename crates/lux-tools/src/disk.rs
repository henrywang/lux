use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

use crate::{run_cmd, Tool};

pub struct CheckDiskUsage;

#[async_trait]
impl Tool for CheckDiskUsage {
    fn name(&self) -> &str {
        "check_disk_usage"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "check_disk_usage".into(),
            description: "Check disk space usage".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to check (defaults to /)"
                    }
                }
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("/");

        run_cmd("df", &["-h", path]).await
    }
}
