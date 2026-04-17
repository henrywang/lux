use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

use crate::{run_cmd, Tool};

pub struct ReadLogs;

#[async_trait]
impl Tool for ReadLogs {
    fn name(&self) -> &str {
        "read_logs"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "read_logs".into(),
            description: "Read system logs from journalctl".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit": {
                        "type": "string",
                        "description": "Filter by systemd unit name"
                    },
                    "since": {
                        "type": "string",
                        "description": "Show logs since (e.g. '1 hour ago', 'today')"
                    },
                    "priority": {
                        "type": "string",
                        "enum": ["emerg", "alert", "crit", "err", "warning", "notice", "info", "debug"],
                        "description": "Minimum priority level"
                    },
                    "lines": {
                        "type": "integer",
                        "description": "Number of recent lines to return"
                    }
                }
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let mut cmd_args = vec!["--no-pager".to_string()];

        if let Some(unit) = args.get("unit").and_then(|v| v.as_str()) {
            cmd_args.push("-u".into());
            cmd_args.push(unit.into());
        }
        if let Some(since) = args.get("since").and_then(|v| v.as_str()) {
            cmd_args.push("--since".into());
            cmd_args.push(since.into());
        }
        if let Some(priority) = args.get("priority").and_then(|v| v.as_str()) {
            cmd_args.push("-p".into());
            cmd_args.push(priority.into());
        }
        if let Some(lines) = args.get("lines").and_then(|v| v.as_u64()) {
            cmd_args.push("-n".into());
            cmd_args.push(lines.to_string());
        } else if args.get("since").is_none() {
            // Default to last 50 lines if no time filter
            cmd_args.push("-n".into());
            cmd_args.push("50".into());
        }

        let refs: Vec<&str> = cmd_args.iter().map(|s| s.as_str()).collect();
        run_cmd("journalctl", &refs).await
    }
}
