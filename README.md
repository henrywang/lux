# lux

AI agent for Linux desktop. Manages your system through natural language.

```
lux> install firefox
Installing org.mozilla.firefox from Flathub...

lux> restart nginx
Restarting nginx.service...

lux> show me recent errors in the system log
Apr 17 01:22:15 fedora sshd[1234]: error: Authentication failure...

lux> my disk is almost full
Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1       100G   89G   11G  89% /
```

## How it works

lux has a two-layer architecture:

1. **Fast path** -- rule-based intent matcher handles common queries instantly (install, restart, logs, etc.)
2. **Slow path** -- a fine-tuned Qwen3 1.7B model handles ambiguous queries via ollama

Both paths route to the same set of system tools that actually execute operations.

## Tools

| Tool | Description |
|------|-------------|
| `install_flatpak` | Install GUI apps from Flathub (firefox, vlc, gimp, etc.) |
| `install_package` | Install CLI tools via dnf (vim, git, gcc, etc.) |
| `remove_package` | Remove system packages |
| `manage_service` | Start, stop, enable, disable, restart systemd services |
| `check_service_status` | Check if a service is running |
| `read_logs` | Read journalctl logs with filters |
| `check_disk_usage` | Check disk space |
| `manage_firewall` | Add/remove firewall rules |
| `network_diagnose` | Diagnose wifi, ethernet, connectivity |
| `bootc_switch` | Switch bootc image |
| `bootc_rollback` | Rollback to previous image |
| `bootc_status` | Show current image status |
| `run_command` | Run arbitrary shell commands (fallback) |

## Requirements

- Fedora 41+ (or any Linux with systemd, dnf, flatpak)
- Rust 1.85+
- [ollama](https://ollama.com/) (for the LLM fallback path)

## Install

```bash
# Install ollama
curl -fsSL https://ollama.com/install.sh | sh

# Clone and build
git clone https://github.com/lux-linux/lux.git
cd lux
cargo build --release

# Download the fine-tuned model (optional, for LLM fallback)
# See finetune/README.md for training your own
ollama pull qwen3:1.7b
```

## Usage

```bash
# Interactive mode
./target/release/lux

# Single command
./target/release/lux -c "install firefox"

# Use a custom model
./target/release/lux --model lux-qwen3

# Enable debug logging
RUST_LOG=debug ./target/release/lux
```

## Development

Common tasks are available via [`just`](https://github.com/casey/just):

```bash
just              # list recipes
just build        # cargo build --release
just test         # cargo test
just lint         # fmt check + clippy
just check        # lint + test (what CI runs)
just run -c "install firefox"
just install      # symlink to ~/.local/bin
```

## Project structure

```
crates/
  lux-cli/       CLI binary (interactive REPL + single-command mode)
  lux-agent/     Core agent loop + intent matcher
  lux-llm/       LLM backend (ollama HTTP client)
  lux-tools/     System tool implementations
  lux-knowledge/ Knowledge base (planned)
  luxd/          Daemon mode (planned)
bench/           Benchmark harness + scenarios
finetune/        LoRA fine-tuning scripts + dataset
```

## Fine-tuning

lux includes a LoRA fine-tuning pipeline to improve tool-calling accuracy:

```bash
pip install unsloth datasets trl
python finetune/train.py
```

See [finetune/README.md](finetune/README.md) for details.

## License

Apache-2.0
