//! lux CLI — interactive AI agent for Linux desktop.

use anyhow::Result;
use clap::Parser;
use lux_agent::Agent;
use lux_llm::{LlmConfig, OllamaBackend};
use lux_tools::{SystemMode, ToolRegistry, sysinfo};
use std::io::{self, BufRead, Write};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "lux", about = "AI agent for Linux desktop")]
struct Cli {
    /// Model to use
    #[arg(long, default_value = "henrywang/lux")]
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

fn findings_path() -> std::path::PathBuf {
    let uid = unsafe { libc::geteuid() };
    std::path::PathBuf::from(format!("/run/user/{uid}/lux/findings.jsonl"))
}

fn print_findings() {
    let path = findings_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("No findings file at {path:?}. Is luxd running?\n");
        return;
    };
    if text.trim().is_empty() {
        eprintln!("No issues detected.\n");
        return;
    }
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let summary = v.get("summary").and_then(|s| s.as_str()).unwrap_or("?");
        let fix = v.get("suggested_fix").and_then(|s| s.as_str());
        match fix {
            Some(f) => eprintln!("  • {summary}\n    fix: {f}"),
            None => eprintln!("  • {summary}"),
        }
    }
    eprintln!();
}

fn count_findings() -> usize {
    std::fs::read_to_string(findings_path())
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

const LOGO_LINES: &[&str] = &[
    "  ██╗     ██╗   ██╗██╗  ██╗",
    "  ██║     ██║   ██║╚██╗██╔╝",
    "  ██║     ██║   ██║ ╚███╔╝ ",
    "  ██║     ██║   ██║ ██╔██╗ ",
    "  ███████╗╚██████╔╝██╔╝██╗",
    "  ╚══════╝ ╚═════╝ ╚═╝  ╚═╝",
];

fn print_banner(mode: SystemMode, model: &str, ollama_url: &str) {
    let info = sysinfo::collect(mode);
    let findings = count_findings();
    let mode_str = match mode {
        SystemMode::Image => "image",
        SystemMode::Package => "package",
    };

    // Left column: logo + version + LLM info
    let mut left: Vec<String> = LOGO_LINES.iter().map(|l| (*l).to_string()).collect();
    left.push(String::new());
    left.push("  light for your Linux desktop".into());
    left.push(format!("  lux v{}", env!("CARGO_PKG_VERSION")));
    left.push(format!("  Model:   {}", shorten(model, 28)));
    left.push(format!("  Ollama:  {}", shorten(ollama_url, 28)));

    // Right column: system info in a box
    let inner_w: usize = 44;
    let title = " System ";
    let issues_line = if findings == 0 {
        "Issues:  none".to_string()
    } else {
        format!("Issues:  {findings} detected — /findings")
    };
    let rows = [
        format!(
            "Host:    {}",
            shorten(
                &format!("{} ({})", info.distro, info.host_type),
                inner_w - 11
            )
        ),
        format!(
            "CPU:     {} ({} cores)",
            shorten(&info.cpu, inner_w - 20),
            info.cpu_cores
        ),
        format!(
            "Memory:  {:.1} / {:.1} GB available",
            info.mem_avail_gb, info.mem_total_gb
        ),
        format!(
            "Disk /:  {:.0} GB free of {:.0} GB",
            info.disk_free_gb, info.disk_total_gb
        ),
        format!("Network: {}", info.network),
        format!("VPN:     {}", info.vpn),
        format!("Uptime:  {}", info.uptime),
        format!("Mode:    {mode_str}"),
        issues_line,
    ];

    let dash_count = inner_w - 1 - title.chars().count();
    let dashes: String = "─".repeat(dash_count);
    let top = format!("╭─{title}{dashes}╮");
    let bottom = format!("╰{}╯", "─".repeat(inner_w));
    let mut right: Vec<String> = Vec::with_capacity(rows.len() + 2);
    right.push(top);
    for r in &rows {
        let content = shorten(r, inner_w - 2);
        let pad = (inner_w - 2).saturating_sub(content.chars().count());
        right.push(format!("│ {content}{} │", " ".repeat(pad)));
    }
    right.push(bottom);

    // Zip columns side by side, padding the left to its max char-width.
    let left_w = left.iter().map(|s| s.chars().count()).max().unwrap_or(0);
    let lines = left.len().max(right.len());
    for i in 0..lines {
        let l = left.get(i).map(String::as_str).unwrap_or("");
        let r = right.get(i).map(String::as_str).unwrap_or("");
        let pad = left_w.saturating_sub(l.chars().count());
        eprintln!("{l}{}   {r}", " ".repeat(pad));
    }
    eprintln!();
    eprintln!("  Type 'quit' to exit, 'clear' to reset, '/sysinfo' or '/findings' for status.");
    eprintln!();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

    let ollama_url = cli.ollama_url.clone();
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
    let mut agent = Agent::new(backend, tools, mode);

    print_banner(mode, &cli.model, &ollama_url);

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
            "/sysinfo" => {
                eprintln!("{}\n", sysinfo::collect(mode));
                continue;
            }
            "/findings" => {
                print_findings();
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
