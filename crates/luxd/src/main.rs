//! luxd — lux daemon (systemd service).
//!
//! Runs the AI agent as a background service, listening for requests
//! via a Unix socket. The model is loaded on demand and unloaded after
//! an idle timeout.
//!
//! TODO: Implement socket listener, idle timeout, privilege management.

use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("luxd v{} starting", env!("CARGO_PKG_VERSION"));

    // TODO: Implement daemon mode
    // - Listen on Unix socket (/run/lux/lux.sock)
    // - Accept requests from lux CLI
    // - Manage model lifecycle (load/unload on idle)
    // - Privilege escalation via polkit
    // - Background health monitoring (pure Rust, no model)

    tracing::warn!("daemon mode not yet implemented — use `lux` CLI directly");

    Ok(())
}
