# lux

```
  ██╗     ██╗   ██╗██╗  ██╗         ╭─ System ───────────────────────────────────╮
  ██║     ██║   ██║╚██╗██╔╝         │ Host:    Fedora Linux 43 (bare-metal)      │
  ██║     ██║   ██║ ╚███╔╝          │ CPU:     AMD Ryzen 5 5600H (12 cores)      │
  ██║     ██║   ██║ ██╔██╗          │ Memory:  21.7 / 30.7 GB available          │
  ███████╗╚██████╔╝██╔╝██╗          │ Disk /:  903 GB free of 951 GB             │
  ╚══════╝ ╚═════╝ ╚═╝  ╚═╝         │ Network: WIFI 192.168.50.62                │
                                    │ VPN:     IPsec                             │
  light for your Linux desktop      │ Uptime:  11h 50m                           │
  lux v0.1.0                        │ Mode:    package                           │
  Model:   hf.co/henrywangxf/lux    │ Issues:  none                              │
  Ollama:  http://localhost:11434   ╰────────────────────────────────────────────╯
```

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

## Why lux?

Linux is powerful but unfriendly. Finding the right command, flag, or
config file is a Google-and-pray loop — and half the answers are for the
wrong distro. lux is named after the Latin word for *light*: it
illuminates your system. Ask a question in plain English; lux picks the
right tool, runs it, and shows you what happened.

Built for people who want their Linux machine to just work.

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

## Background monitoring (luxd)

`luxd` is an optional companion daemon that watches your system and
surfaces problems to the REPL. It runs a handful of pure-Rust detectors
on a timer:

- Failed systemd units
- Recent SELinux AVC denials
- Disk-usage thresholds

Findings are written as JSONL to `/run/user/<uid>/lux/findings.jsonl`
and optionally pushed as desktop notifications. The REPL's `/findings`
command reads that file, so opening `lux` shows you whatever went wrong
while you were away. No LLM, no network, no privileged actions — it
only observes and reports; fixes still go through `lux`.

Foreground:

```bash
./target/release/luxd
```

Autostart at login via systemd (user unit, runs as you):

```bash
install -m 644 systemd/luxd.service ~/.config/systemd/user/luxd.service
systemctl --user daemon-reload
systemctl --user enable --now luxd.service
```

First run writes `~/.config/lux/luxd.toml`; edit to tune `mode`,
`interval_secs`, and which detectors are active.

## Requirements

- **Fedora 41+ / RHEL 9+ / CentOS Stream 9+** — lux is built around the
  Fedora family (dnf, firewalld, bootc). Ubuntu/Debian/Arch can run the
  binary, but tools that wrap dnf or firewall-cmd will report the distro
  as unsupported until cross-distro backends land.
- Rust 1.85+
- [ollama](https://ollama.com/) (for the LLM fallback path)

## Install

**Quick install** (prebuilt binary):

```bash
curl -fsSL https://raw.githubusercontent.com/henrywang/lux/main/install.sh | bash
```

**From source:**

```bash
git clone https://github.com/henrywang/lux.git
cd lux
./setup.sh
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
  luxd/          Background monitor (failed units, AVC, disk)
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
