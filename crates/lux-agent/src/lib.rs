//! Core agent loop for lux.
//!
//! The agent takes user input, sends it to the LLM with tool definitions,
//! executes any tool calls, feeds results back, and repeats until the LLM
//! responds with text (no more tool calls).

mod intent;

use anyhow::Result;
use lux_llm::{LlmBackend, LlmResponse, Message, Role, ToolCall};
use lux_tools::ToolRegistry;
use tracing::{debug, info};

/// Maximum tool call rounds before stopping (prevents infinite loops).
const MAX_ROUNDS: usize = 10;

/// The lux agent.
pub struct Agent<B: LlmBackend> {
    backend: B,
    tools: ToolRegistry,
    history: Vec<Message>,
}

impl<B: LlmBackend> Agent<B> {
    pub fn new(backend: B, tools: ToolRegistry) -> Self {
        let system_msg = Message {
            role: Role::System,
            content: Some(
                "You are lux, an AI system agent for Linux desktop. \
                 You help users manage their system by calling tools. \
                 Always call a tool when the user asks you to do something — \
                 never respond with only text when a tool action is appropriate.\n\n\
                 Rules:\n\
                 - Desktop GUI apps (firefox, gimp, vlc, steam, etc.) → install_flatpak\n\
                 - CLI tools and system packages (vim, git, gcc, htop) → install_package\n\
                 - Printer issues → check cups service\n\
                 - Network/wifi/internet issues → network_diagnose\n\
                 - Log queries → read_logs\n\
                 - bootc rollback when user wants to undo an update\n\
                 - bootc status to show current image info"
                    .into(),
            ),
            tool_calls: None,
        };

        Self {
            backend,
            tools,
            history: vec![system_msg],
        }
    }

    /// Process a user message and return the agent's final text response.
    pub async fn process(&mut self, user_input: &str) -> Result<String> {
        // Fast path: rule-based intent matching (instant, no LLM call)
        if let Some(tc) = intent::match_intent(user_input) {
            info!("Intent matched: {} args={}", tc.name, tc.arguments);
            return Ok(self.execute_tool(&tc).await);
        }

        // Slow path: LLM
        // Add user message
        self.history.push(Message {
            role: Role::User,
            content: Some(user_input.into()),
            tool_calls: None,
        });

        let tool_defs = self.tools.definitions();

        for round in 0..MAX_ROUNDS {
            debug!("Agent round {round}");

            let response: LlmResponse = self.backend.chat(&self.history, &tool_defs).await?;

            if response.tool_calls.is_empty() {
                // No tool calls — this is the final text response
                let text = response.content.unwrap_or_default();
                self.history.push(Message {
                    role: Role::Assistant,
                    content: Some(text.clone()),
                    tool_calls: None,
                });
                return Ok(text);
            }

            // Add assistant message with tool calls
            self.history.push(Message {
                role: Role::Assistant,
                content: response.content.clone(),
                tool_calls: Some(response.tool_calls.clone()),
            });

            // Execute each tool call and return output directly.
            // The 1.7B model is trained for tool selection, not summarization,
            // so we skip the second LLM call entirely.
            let mut tool_outputs = Vec::new();
            for tc in &response.tool_calls {
                let result = self.execute_tool(tc).await;
                tool_outputs.push(result.clone());
                self.history.push(Message {
                    role: Role::Tool,
                    content: Some(result),
                    tool_calls: None,
                });
            }

            let output = tool_outputs.join("\n");
            if !output.trim().is_empty() {
                return Ok(output);
            }
        }

        Ok("I've reached the maximum number of steps. Please try a more specific request.".into())
    }

    /// Execute a single tool call and return the result as a string.
    async fn execute_tool(&self, tc: &ToolCall) -> String {
        info!("Executing tool: {} args={}", tc.name, tc.arguments);

        match self.tools.execute(&tc.name, &tc.arguments).await {
            Ok(output) => {
                // Truncate very long output
                if output.len() > 4000 {
                    format!("{}...\n[output truncated]", &output[..4000])
                } else {
                    output
                }
            }
            Err(e) => format!("Error: {e}"),
        }
    }

    /// Clear conversation history (keeps system prompt).
    pub fn clear_history(&mut self) {
        self.history.truncate(1);
    }
}
