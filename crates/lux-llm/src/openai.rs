//! OpenAI-compatible HTTP backend for LLM inference.
//!
//! Speaks `/v1/chat/completions`, which is supported by both Ollama and
//! llama-server (with `--jinja` and a tool-capable chat template). The
//! portable bundle ships llama-server; users running the non-portable
//! install keep pointing at their local Ollama server.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::debug;

use crate::{LlmBackend, LlmConfig, LlmResponse, Message, Role, ToolCall, ToolDef};

pub struct OpenAiBackend {
    config: LlmConfig,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Value>,
    tools: Vec<Value>,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ResponseToolCall>>,
}

#[derive(Deserialize)]
struct ResponseToolCall {
    function: ResponseFunction,
}

#[derive(Deserialize)]
struct ResponseFunction {
    name: String,
    /// OpenAI spec: JSON-encoded string. Some servers return a raw object —
    /// we accept both via `Value` and normalize below.
    arguments: Value,
}

impl OpenAiBackend {
    pub fn new(config: LlmConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("failed to create HTTP client");
        Self { config, client }
    }
}

/// Convert lux's internal `Message` shape to OpenAI wire format. Assistant
/// tool calls get synthetic `id`s; following tool-role messages reference
/// those ids positionally, which is what `/v1/chat/completions` expects.
fn to_wire_messages(messages: &[Message]) -> Vec<Value> {
    let mut wire = Vec::with_capacity(messages.len());
    let mut last_ids: Vec<String> = Vec::new();
    let mut tool_idx = 0usize;
    let mut counter = 0u64;

    for m in messages {
        let role = match m.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        };

        match (&m.role, m.tool_calls.as_ref()) {
            (Role::Assistant, Some(calls)) if !calls.is_empty() => {
                let mut ids = Vec::with_capacity(calls.len());
                let wire_calls: Vec<Value> = calls
                    .iter()
                    .map(|tc| {
                        counter += 1;
                        let id = format!("call_{counter}");
                        ids.push(id.clone());
                        json!({
                            "id": id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": serde_json::to_string(&tc.arguments)
                                    .unwrap_or_else(|_| "{}".into()),
                            }
                        })
                    })
                    .collect();
                last_ids = ids;
                tool_idx = 0;
                wire.push(json!({
                    "role": role,
                    "content": m.content.clone().unwrap_or_default(),
                    "tool_calls": wire_calls,
                }));
            }
            (Role::Tool, _) => {
                let id = last_ids
                    .get(tool_idx)
                    .cloned()
                    .unwrap_or_else(|| "call_unknown".into());
                tool_idx += 1;
                wire.push(json!({
                    "role": "tool",
                    "tool_call_id": id,
                    "content": m.content.clone().unwrap_or_default(),
                }));
            }
            _ => {
                wire.push(json!({
                    "role": role,
                    "content": m.content.clone().unwrap_or_default(),
                }));
            }
        }
    }
    wire
}

impl LlmBackend for OpenAiBackend {
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse> {
        let wire_tools: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();

        let request = ChatRequest {
            model: &self.config.model,
            messages: to_wire_messages(messages),
            tools: wire_tools,
            stream: false,
        };

        let url = format!("{}/v1/chat/completions", self.config.base_url);
        debug!("POST {url} model={}", self.config.model);

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("failed to connect to LLM server")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM server returned {status}: {body}");
        }

        let chat_resp: ChatResponse = resp.json().await.context("failed to parse LLM response")?;

        let Some(choice) = chat_resp.choices.into_iter().next() else {
            return Ok(LlmResponse {
                content: None,
                tool_calls: Vec::new(),
            });
        };

        let tool_calls = choice
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| {
                // OpenAI: arguments is a JSON-encoded string. Ollama /v1 sometimes
                // returns a raw object. Accept both.
                let arguments = match tc.function.arguments {
                    Value::String(s) => serde_json::from_str(&s).unwrap_or(Value::String(s)),
                    other => other,
                };
                ToolCall {
                    name: tc.function.name,
                    arguments,
                }
            })
            .collect();

        Ok(LlmResponse {
            content: choice.message.content,
            tool_calls,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_messages_synthesize_ids_and_link_tool_responses() {
        let msgs = vec![
            Message {
                role: Role::User,
                content: Some("install htop".into()),
                tool_calls: None,
            },
            Message {
                role: Role::Assistant,
                content: Some(String::new()),
                tool_calls: Some(vec![ToolCall {
                    name: "install_package".into(),
                    arguments: json!({"name": "htop"}),
                }]),
            },
            Message {
                role: Role::Tool,
                content: Some("installed".into()),
                tool_calls: None,
            },
        ];

        let wire = to_wire_messages(&msgs);
        assert_eq!(wire.len(), 3);

        let assistant = &wire[1];
        let id = assistant["tool_calls"][0]["id"].as_str().unwrap();
        assert!(id.starts_with("call_"));
        // arguments must be a JSON-encoded string, not an object.
        let args = assistant["tool_calls"][0]["function"]["arguments"]
            .as_str()
            .unwrap();
        assert_eq!(args, r#"{"name":"htop"}"#);

        let tool = &wire[2];
        assert_eq!(tool["role"], "tool");
        assert_eq!(tool["tool_call_id"].as_str().unwrap(), id);
    }
}
