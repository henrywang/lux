//! LLM inference backend for lux.
//!
//! Provides a trait-based abstraction over LLM backends (ollama, llama.cpp)
//! and manages model lifecycle (load on demand, unload after idle).

mod ollama;

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub use ollama::OllamaBackend;

/// A tool definition passed to the LLM for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A tool call returned by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Response from the LLM.
#[derive(Debug)]
pub struct LlmResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

/// Configuration for the LLM backend.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub model: String,
    pub base_url: String,
    pub thinking: bool,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: "qwen3:1.7b".to_string(),
            base_url: "http://localhost:11434".to_string(),
            thinking: false,
        }
    }
}

/// Trait for LLM backends.
#[trait_variant::make(Send)]
pub trait LlmBackend {
    /// Send a conversation with tool definitions and get a response.
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse>;
}
