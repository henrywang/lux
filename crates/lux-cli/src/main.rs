//! lux CLI — interactive AI agent for Linux desktop.

use anyhow::Result;
use clap::Parser;
use lux_agent::Agent;
use lux_llm::{LlmConfig, OllamaBackend};
use lux_tools::{SystemMode, ToolRegistry};
use std::io::{self, BufRead, Write};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "lux", about = "AI agent for Linux desktop")]
struct Cli {
    /// Model to use
    #[arg(long, default_value = "qwen3:1.7b")]
    model: String,

    /// Ollama server URL
    #[arg(long, default_value = "http://localhost:11434")]
    ollama_url: String,

    /// Enable thinking mode (slower but more accurate)
    #[arg(long)]
    think: bool,

    /// Force system mode instead of auto-detecting
    #[arg(long, value_parser = ["image", "package"])]
    mode: Option<String>,

    /// Run a single command and exit (non-interactive)
    #[arg(short, long)]
    command: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

    let config = LlmConfig {
        model: cli.model.clone(),
        base_url: cli.ollama_url,
        thinking: cli.think,
    };

    let mode = match cli.mode.as_deref() {
        Some("image") => SystemMode::Image,
        Some("package") => SystemMode::Package,
        _ => SystemMode::detect(),
    };

    let backend = OllamaBackend::new(config);
    let tools = ToolRegistry::new(mode);
    let mut agent = Agent::new(backend, tools);

    eprintln!(
        "lux v{} — AI agent for Linux desktop",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!("System mode: {mode:?}");
    eprintln!("Model: {}", cli.model);
    eprintln!("Type 'quit' to exit, 'clear' to reset conversation.\n");

    // Single command mode
    if let Some(cmd) = cli.command {
        let response = agent.process(&cmd).await?;
        println!("{response}");
        return Ok(());
    }

    // Interactive REPL
    let stdin = io::stdin();
    let mut reader = stdin.lock().lines();

    loop {
        print!("lux> ");
        io::stdout().flush()?;

        let line = match reader.next() {
            Some(Ok(line)) => line,
            _ => break,
        };

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "quit" | "exit" => break,
            "clear" => {
                agent.clear_history();
                eprintln!("Conversation cleared.");
                continue;
            }
            _ => {}
        }

        match agent.process(input).await {
            Ok(response) => println!("{response}"),
            Err(e) => eprintln!("Error: {e}"),
        }

        println!();
    }

    Ok(())
}
