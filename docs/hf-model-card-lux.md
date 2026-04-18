---
license: apache-2.0
base_model: Qwen/Qwen3-1.7B
tags:
  - linux
  - sysadmin
  - agent
  - tool-use
  - gguf
language:
  - en
pipeline_tag: text-generation
---

# lux

Fine-tuned [Qwen3-1.7B](https://huggingface.co/Qwen/Qwen3-1.7B) for Linux system administration tool calls. Powers the slow path of the [lux CLI](https://github.com/henrywang/lux) — when the rule-based fast path can't classify a request, this model parses it into a structured tool invocation.

This repo ships the **raw GGUF weights + Modelfile**. If you want the full CLI experience (with luxd, ollama, pre-pulled weights, and a REPL), use [`henrywangxf/lux-portable`](https://huggingface.co/henrywangxf/lux-portable) instead.

## Usage

### With ollama (recommended)

```bash
ollama pull hf.co/henrywangxf/lux
ollama run hf.co/henrywangxf/lux "restart nginx"
```

Ollama reads the bundled `Modelfile` automatically — SYSTEM prompt, template, and sampling params come with the pull.

### With llama.cpp

```bash
wget https://huggingface.co/henrywangxf/lux/resolve/main/qwen3-1.7b.Q4_K_M.gguf
./llama-cli -m qwen3-1.7b.Q4_K_M.gguf -p "restart nginx"
```

You'll need to supply your own prompt template — see `Modelfile` in this repo for the one lux expects.

## What the model outputs

Trained to emit tool calls for lux's registry. Typical interaction:

```
user:    my disk is almost full
model:   {"tool": "check_disk_usage", "args": {}}
```

```
user:    install firefox
model:   {"tool": "install_flatpak", "args": {"app": "org.mozilla.firefox"}}
```

The full tool schema lives in [lux-tools](https://github.com/henrywang/lux/tree/main/crates/lux-tools). If you're using this model outside of lux, you'll need to parse these tool calls and dispatch them yourself.

## Training

- **Base:** [Qwen/Qwen3-1.7B](https://huggingface.co/Qwen/Qwen3-1.7B) (Apache 2.0)
- **Method:** LoRA fine-tuning on a curated set of Linux sysadmin tool-use traces
- **Quantization:** Q4_K_M (GGUF)
- **Recipe:** see [finetune/](https://github.com/henrywang/lux/tree/main/finetune) in the main repo

## Intended use

- Parsing natural-language sysadmin requests into structured tool calls
- Running locally on a laptop (≥8 GB RAM is comfortable for Q4_K_M)
- Integrating with your own agent loop over lux's tool schema — or using lux directly

## Limitations

- **English only.**
- **Tuned for Fedora / RHEL-family systems** (dnf, systemd, firewalld). Other distros work for most tools but package-manager queries degrade.
- **Not a general-purpose chatbot** — asking unrelated questions produces garbage tool calls.
- **Small model, small context.** Multi-turn conversations longer than a handful of exchanges will drift; lux avoids this by keeping history trimmed.
- **Destructive actions are not gated by the model** — that's the job of the lux CLI, which asks for confirmation before running anything destructive. If you call the model directly, implement your own confirmation step.

## License

Apache-2.0, inherited from Qwen3-1.7B. Fine-tuning derived weights are redistributed under the same terms.

## Links

- Source: https://github.com/henrywang/lux
- Portable bundle: https://huggingface.co/henrywangxf/lux-portable
- Base model: https://huggingface.co/Qwen/Qwen3-1.7B
- Issues / feedback: https://github.com/henrywang/lux/issues
