---
license: apache-2.0
base_model: Qwen/Qwen3-1.7B
tags:
  - linux
  - sysadmin
  - agent
  - desktop
  - gguf
language:
  - en
pipeline_tag: text-generation
---

# lux-portable

Portable, single-download bundle of [lux](https://github.com/henrywang/lux) — an AI agent for the Linux desktop that manages your system through natural language.

This repo ships the **standalone tarball**: lux CLI + luxd daemon + llama-server (llama.cpp) + pre-downloaded fine-tuned weights. Extract and run — no system install, no network required at runtime.

## Download

```bash
wget https://huggingface.co/henrywangxf/lux-portable/resolve/main/lux-portable-linux-x86_64-latest.tar.gz
tar xzf lux-portable-linux-x86_64-latest.tar.gz
cd lux-portable-linux-x86_64/
./lux
```

## What's inside

```
lux             # interactive CLI / REPL
luxd            # background monitoring daemon (optional)
llama-server    # local inference server (llama.cpp)
libggml*.so     # shared libs loaded by llama-server ($ORIGIN rpath)
libllama*.so
libmtmd*.so
models/         # pre-downloaded Q4_K_M GGUF weights
systemd/        # reference unit file for running luxd via systemd --user
```

On startup, `lux` detects the sibling `llama-server` binary and `models/*.gguf` and auto-spawns a local inference server on an ephemeral port (with the model's chat template enabled via `--jinja`). The server terminates when lux exits. Nothing is written outside this directory.

## Example session

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

## Model

- **Base:** [Qwen/Qwen3-1.7B](https://huggingface.co/Qwen/Qwen3-1.7B) (Apache 2.0)
- **Fine-tuning:** LoRA on a curated set of Linux sysadmin tool-use traces
- **Format:** GGUF Q4_K_M (served via llama-server)
- **Intended use:** parsing ambiguous natural-language requests into structured tool calls for lux's tool registry (install packages, manage services, read logs, manage firewall, etc.)

Most common queries hit lux's **rule-based fast path** and never reach the model. The model handles the long tail — ambiguous, novel, or multi-step requests.

## Limitations

- English only.
- Tuned for Fedora / RHEL-family systems (dnf, systemd, firewalld). Other distros work but with reduced accuracy on package-manager queries.
- Not a general-purpose chatbot — trained exclusively for Linux system administration tasks.
- Running destructive commands (`rm`, `dd`, etc.) still requires your confirmation in the CLI.

## License

- Model weights: Apache 2.0 (inherited from Qwen3).
- `lux` / `luxd`: Apache-2.0.
- `llama-server` + `libggml*`/`libllama*`/`libmtmd*`: MIT (bundled unmodified from [llama.cpp](https://github.com/ggml-org/llama.cpp)).

## Links

- Source: https://github.com/henrywang/lux
- Issues / feedback: https://github.com/henrywang/lux/issues
- Base model: https://huggingface.co/Qwen/Qwen3-1.7B
