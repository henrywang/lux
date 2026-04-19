//! Recipe tools: `apply_recipe` and `list_recipes`.
//!
//! A recipe bundles packages + shell/write_file steps into one named setup
//! (e.g. `zsh-popular`, `ai-dev-cpu`). `apply_recipe` prints the planned
//! actions, asks for y/N confirmation on a TTY, then executes.
//!
//! Non-interactive execution (`-c` mode, scripted) must pass
//! `assume_yes: true` in the tool arguments — blind execution in a non-TTY
//! context would be a trust violation.

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use lux_knowledge::{Distro, Recipe, RecipeRegistry, Step};
use lux_llm::ToolDef;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::{Tool, run_cmd, run_cmd_sudo};

pub struct ApplyRecipe;
pub struct ListRecipes;

fn registry() -> &'static RecipeRegistry {
    static R: OnceLock<RecipeRegistry> = OnceLock::new();
    R.get_or_init(|| RecipeRegistry::new().expect("bundled recipes must parse at startup"))
}

#[async_trait]
impl Tool for ListRecipes {
    fn name(&self) -> &str {
        "list_recipes"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "list_recipes".into(),
            description:
                "List all available recipes (opinionated multi-step setups like zsh-popular, \
                 ai-dev-cpu). Use this when the user asks what's available or wants a setup \
                 but the specific recipe is ambiguous."
                    .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
        }
    }

    async fn execute(&self, _args: &Value) -> Result<String> {
        let mut out = String::from("Available recipes:\n");
        for r in registry().list() {
            out.push_str(&format!("  • {} — {}\n", r.name, r.summary));
        }
        Ok(out)
    }
}

#[async_trait]
impl Tool for ApplyRecipe {
    fn name(&self) -> &str {
        "apply_recipe"
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "apply_recipe".into(),
            description: "Apply a named recipe (multi-step opinionated setup). Shows the planned \
                 actions and asks for confirmation before executing. Known recipes: \
                 zsh-popular, ghostty-default, ai-dev-cpu, ai-dev-cuda, editor-vscodium, \
                 editor-zed."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Recipe name (e.g. 'zsh-popular')"
                    },
                    "assume_yes": {
                        "type": "boolean",
                        "description": "Skip the confirmation prompt. Required when stdin is not a TTY.",
                        "default": false
                    }
                },
                "required": ["name"]
            }),
        }
    }

    async fn execute(&self, args: &Value) -> Result<String> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'name' argument"))?;
        let assume_yes = args
            .get("assume_yes")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let recipe = registry()
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("unknown recipe '{name}'. Try list_recipes."))?;
        let distro = detect_distro()?;

        let plan = format_plan(recipe, distro);
        eprintln!("{plan}");

        if !assume_yes {
            if !stdin_is_tty() {
                bail!("refusing to run recipe '{name}' non-interactively without assume_yes: true");
            }
            if !prompt_yes_no("Apply this recipe?").await? {
                return Ok("Cancelled.".into());
            }
        }

        run_recipe(recipe, distro).await
    }
}

fn format_plan(recipe: &Recipe, distro: Distro) -> String {
    let mut out = String::new();
    out.push_str(&format!("\nRecipe: {}\n", recipe.name));
    out.push_str(&format!("  {}\n", recipe.summary));

    let native = recipe.packages.native_for(distro);
    let flatpaks = &recipe.packages.flatpak;
    if !native.is_empty() {
        out.push_str(&format!(
            "  packages ({}): {}\n",
            distro_cmd(distro),
            native.join(", ")
        ));
    }
    if !flatpaks.is_empty() {
        out.push_str(&format!("  flatpak: {}\n", flatpaks.join(", ")));
    }
    if !recipe.steps.is_empty() {
        out.push_str("  steps:\n");
        for (i, step) in recipe.steps.iter().enumerate() {
            let mark = if let Step::Shell { optional: true, .. } = step {
                " (optional)"
            } else {
                ""
            };
            out.push_str(&format!("    {}. {}{}\n", i + 1, step.describe(), mark));
        }
    }
    out
}

async fn run_recipe(recipe: &Recipe, distro: Distro) -> Result<String> {
    let mut log = String::new();

    let native = recipe.packages.native_for(distro);
    if !native.is_empty() {
        log.push_str(&format!(
            "[packages/{}] installing {}\n",
            distro_cmd(distro),
            native.join(" ")
        ));
        let refs: Vec<&str> = native.iter().map(String::as_str).collect();
        install_native(distro, &refs)
            .await
            .with_context(|| format!("installing native packages ({})", distro_cmd(distro)))?;
    }

    for app_id in &recipe.packages.flatpak {
        log.push_str(&format!("[flatpak] installing {app_id}\n"));
        run_cmd("flatpak", &["install", "-y", "--user", "flathub", app_id])
            .await
            .with_context(|| format!("installing flatpak {app_id}"))?;
    }

    for step in &recipe.steps {
        log.push_str(&format!("[step] {}\n", step.describe()));
        match run_step(step).await {
            Ok(_) => {}
            Err(e) => {
                if let Step::Shell { optional: true, .. } = step {
                    log.push_str(&format!("  (optional step failed, continuing: {e})\n"));
                } else {
                    return Err(e).with_context(|| format!("step: {}", step.describe()));
                }
            }
        }
    }

    log.push_str(&format!("\n✓ Recipe '{}' applied.\n", recipe.name));
    Ok(log)
}

async fn run_step(step: &Step) -> Result<()> {
    match step {
        Step::Shell { shell, .. } => {
            let out = tokio::process::Command::new("sh")
                .args(["-c", shell])
                .output()
                .await?;
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                bail!("shell step failed: {stderr}");
            }
            Ok(())
        }
        Step::Write { write_file, .. } => write_user_file(&write_file.path, &write_file.content),
    }
}

fn write_user_file(path: &str, content: &str) -> Result<()> {
    let expanded = expand_tilde(path)?;
    if let Some(parent) = expanded.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating parent dir {}", parent.display()))?;
    }
    std::fs::write(&expanded, content)
        .with_context(|| format!("writing {}", expanded.display()))?;
    Ok(())
}

fn expand_tilde(path: &str) -> Result<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME not set"))?;
        Ok(PathBuf::from(home).join(rest))
    } else if path == "~" {
        let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME not set"))?;
        Ok(PathBuf::from(home))
    } else {
        Ok(PathBuf::from(path))
    }
}

async fn install_native(distro: Distro, packages: &[&str]) -> Result<String> {
    match distro {
        Distro::Dnf => {
            let mut args = vec!["-y", "install"];
            args.extend_from_slice(packages);
            run_cmd_sudo("dnf", &args).await
        }
        Distro::Apt => {
            // update once so fresh installs don't fail on stale cache
            let _ = run_cmd_sudo("apt-get", &["update"]).await;
            let mut args = vec!["-y", "install"];
            args.extend_from_slice(packages);
            run_cmd_sudo("apt-get", &args).await
        }
        Distro::Pacman => {
            let mut args = vec!["-S", "--noconfirm", "--needed"];
            args.extend_from_slice(packages);
            run_cmd_sudo("pacman", &args).await
        }
    }
}

fn distro_cmd(distro: Distro) -> &'static str {
    match distro {
        Distro::Dnf => "dnf",
        Distro::Apt => "apt",
        Distro::Pacman => "pacman",
    }
}

/// Detect the package manager family from `/etc/os-release`. Falls back to
/// PATH-probing if os-release is unavailable or inconclusive.
pub fn detect_distro() -> Result<Distro> {
    if let Ok(text) = std::fs::read_to_string("/etc/os-release") {
        let ids = parse_os_release_ids(&text);
        for id in &ids {
            match id.as_str() {
                "fedora" | "rhel" | "centos" | "rocky" | "almalinux" | "ol" => {
                    return Ok(Distro::Dnf);
                }
                "debian" | "ubuntu" | "linuxmint" | "pop" | "elementary" => {
                    return Ok(Distro::Apt);
                }
                "arch" | "manjaro" | "endeavouros" | "cachyos" => return Ok(Distro::Pacman),
                _ => {}
            }
        }
    }
    // Fallback: whichever package manager is on PATH
    for (cmd, distro) in [
        ("dnf", Distro::Dnf),
        ("apt-get", Distro::Apt),
        ("pacman", Distro::Pacman),
    ] {
        if binary_on_path(cmd) {
            return Ok(distro);
        }
    }
    bail!("could not detect package manager family (no dnf/apt/pacman on PATH)")
}

/// Pull `ID` and all entries in `ID_LIKE` from os-release.
fn parse_os_release_ids(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let (key, val) = match line.split_once('=') {
            Some(kv) => kv,
            None => continue,
        };
        let val = val.trim().trim_matches('"');
        match key.trim() {
            "ID" => out.push(val.to_string()),
            "ID_LIKE" => out.extend(val.split_whitespace().map(str::to_string)),
            _ => {}
        }
    }
    out
}

fn binary_on_path(cmd: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(cmd).is_file())
}

fn stdin_is_tty() -> bool {
    // fd 0 = stdin
    unsafe { libc::isatty(0) != 0 }
}

async fn prompt_yes_no(question: &str) -> Result<bool> {
    eprint!("{question} [y/N] ");
    // flush the prompt; eprint! doesn't auto-flush before async read
    use std::io::Write as _;
    let _ = std::io::stderr().flush();

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let answer = line.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_os_release_fedora() {
        let text = r#"NAME=Fedora
ID=fedora
ID_LIKE=""
PRETTY_NAME="Fedora Linux 43"
"#;
        let ids = parse_os_release_ids(text);
        assert_eq!(ids, vec!["fedora"]);
    }

    #[test]
    fn parse_os_release_ubuntu() {
        let text = r#"NAME="Ubuntu"
ID=ubuntu
ID_LIKE=debian
"#;
        let ids = parse_os_release_ids(text);
        assert!(ids.contains(&"ubuntu".to_string()));
        assert!(ids.contains(&"debian".to_string()));
    }

    #[test]
    fn parse_os_release_manjaro() {
        let text = r#"NAME="Manjaro Linux"
ID=manjaro
ID_LIKE=arch
"#;
        let ids = parse_os_release_ids(text);
        assert!(ids.contains(&"manjaro".to_string()));
    }

    #[test]
    fn expand_tilde_resolves_home() {
        // SAFETY: tests run single-threaded via #[test] here; we only read
        // HOME inside expand_tilde, no other thread observes this env.
        unsafe {
            std::env::set_var("HOME", "/home/test");
        }
        let p = expand_tilde("~/.zshrc").unwrap();
        assert_eq!(p, PathBuf::from("/home/test/.zshrc"));
        let p = expand_tilde("/etc/issue").unwrap();
        assert_eq!(p, PathBuf::from("/etc/issue"));
    }

    #[test]
    fn format_plan_includes_name_and_steps() {
        let reg = RecipeRegistry::new().unwrap();
        let r = reg.get("zsh-popular").unwrap();
        let out = format_plan(r, Distro::Dnf);
        assert!(out.contains("zsh-popular"));
        assert!(out.contains("dnf"));
        assert!(out.contains("oh-my-zsh"));
    }
}
