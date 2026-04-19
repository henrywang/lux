//! System tools for the lux agent.
//!
//! Each tool wraps a system operation (bootc, dnf, systemctl, etc.)
//! and can be invoked by the LLM via function calling.

mod bootc;
mod disk;
mod firewall;
mod flatpak;
mod logs;
mod network;
mod package;
mod recipes;
mod service;
mod shell;
pub mod sysinfo;
mod update;

use anyhow::Result;
use async_trait::async_trait;
use lux_llm::ToolDef;
use serde_json::Value;

pub use bootc::*;
pub use disk::*;
pub use firewall::*;
pub use flatpak::*;
pub use logs::*;
pub use network::*;
pub use package::*;
pub use recipes::*;
pub use service::*;
pub use shell::*;
pub use update::*;

/// A tool that can be executed by the agent.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (must match what the LLM calls).
    fn name(&self) -> &str;

    /// Tool definition for the LLM.
    fn definition(&self) -> ToolDef;

    /// Execute the tool with the given arguments.
    async fn execute(&self, args: &Value) -> Result<String>;
}

/// Registry of all available tools.
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a registry with all default tools.
    pub fn new(mode: SystemMode) -> Self {
        let mut tools: Vec<Box<dyn Tool>> = vec![
            Box::new(package::InstallPackage),
            Box::new(package::RemovePackage),
            Box::new(update::UpdateSystem),
            Box::new(flatpak::InstallFlatpak),
            Box::new(service::ManageService),
            Box::new(service::CheckServiceStatus),
            Box::new(logs::ReadLogs),
            Box::new(disk::CheckDiskUsage),
            Box::new(firewall::ManageFirewall),
            Box::new(network::NetworkDiagnose),
            Box::new(shell::RunCommand),
            Box::new(recipes::ListRecipes),
            Box::new(recipes::ApplyRecipe),
        ];

        if mode == SystemMode::Image {
            tools.push(Box::new(bootc::BootcSwitch));
            tools.push(Box::new(bootc::BootcRollback));
            tools.push(Box::new(bootc::BootcStatus));
        }

        Self { tools }
    }

    /// Get tool definitions for the LLM.
    pub fn definitions(&self) -> Vec<ToolDef> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    /// Find and execute a tool by name.
    pub async fn execute(&self, name: &str, args: &Value) -> Result<String> {
        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == name)
            .ok_or_else(|| anyhow::anyhow!("unknown tool: {name}"))?;

        tool.execute(args).await
    }
}

/// System mode: image-based (bootc) or package-based (dnf).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemMode {
    Image,
    Package,
}

impl SystemMode {
    /// Auto-detect the system mode.
    pub fn detect() -> Self {
        if std::path::Path::new("/run/bootc").exists() || std::path::Path::new("/sysroot").exists()
        {
            SystemMode::Image
        } else {
            SystemMode::Package
        }
    }
}

/// Helper to run a shell command and return stdout.
pub(crate) async fn run_cmd(cmd: &str, args: &[&str]) -> Result<String> {
    let output = tokio::process::Command::new(cmd)
        .args(args)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        anyhow::bail!("command `{cmd}` failed: {stderr}");
    }

    if stdout.is_empty() {
        Ok(stderr)
    } else {
        Ok(stdout)
    }
}

/// Run a privileged command via pkexec (polkit). Prompts the user with a
/// desktop password dialog. If already running as root, runs directly.
pub(crate) async fn run_cmd_sudo(cmd: &str, args: &[&str]) -> Result<String> {
    if unsafe { libc::geteuid() } == 0 {
        return run_cmd(cmd, args).await;
    }
    let mut full_args = vec![cmd];
    full_args.extend_from_slice(args);
    run_cmd("pkexec", &full_args).await
}

/// Tools that wrap Fedora-specific binaries (dnf, firewall-cmd, bootc) call
/// this before exec so users on Ubuntu/Debian/Arch get a clear message
/// instead of an opaque ENOENT from the child process.
pub(crate) fn require_binary(cmd: &str) -> Result<()> {
    if binary_on_path(cmd) {
        return Ok(());
    }
    anyhow::bail!(
        "`{cmd}` is not installed. This tool targets the Fedora family \
         (Fedora/RHEL/CentOS); other distros aren't supported yet."
    )
}

fn binary_on_path(cmd: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(cmd).is_file())
}
