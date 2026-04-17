//! Ollama backend for LLM inference.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{LlmBackend, LlmConfig, LlmResponse, Message, ToolCall, ToolDef};

pub struct OllamaBackend {
    config: LlmConfig,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    tools: Vec<OllamaTool<'a>>,
    stream: bool,
    think: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    num_ctx: u32,
}

#[derive(Serialize)]
struct OllamaTool<'a> {
    r#type: &'static str,
    function: OllamaFunction<'a>,
}

#[derive(Serialize)]
struct OllamaFunction<'a> {
    name: &'a str,
    description: &'a str,
    parameters: &'a serde_json::Value,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: Option<ResponseMessage>,
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
    arguments: serde_json::Value,
}

impl OllamaBackend {
    pub fn new(config: LlmConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("failed to create HTTP client");
        Self { config, client }
    }
}

impl LlmBackend for OllamaBackend {
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse> {
        let ollama_tools: Vec<OllamaTool> = tools
            .iter()
            .map(|t| OllamaTool {
                r#type: "function",
                function: OllamaFunction {
                    name: &t.name,
                    description: &t.description,
                    parameters: &t.parameters,
                },
            })
            .collect();

        let request = ChatRequest {
            model: &self.config.model,
            messages,
            tools: ollama_tools,
            stream: false,
            think: self.config.thinking,
            options: OllamaOptions { num_ctx: 4096 },
        };

        let url = format!("{}/api/chat", self.config.base_url);
        debug!("POST {url} model={}", self.config.model);

        let resp = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("failed to connect to ollama")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("ollama returned {status}: {body}");
        }

        let chat_resp: ChatResponse = resp
            .json()
            .await
            .context("failed to parse ollama response")?;

        let message = chat_resp.message.unwrap_or(ResponseMessage {
            content: None,
            tool_calls: None,
        });

        let tool_calls = message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| ToolCall {
                name: tc.function.name,
                arguments: tc.function.arguments,
            })
            .collect();

        Ok(LlmResponse {
            content: message.content,
            tool_calls,
        })
    }
}
