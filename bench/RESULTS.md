# Benchmark Results

Tuning log for lux tool-calling accuracy. Each run is a separate file in `results/`.

## Current Best

**[002 - LoRA v1](results/002-lora-v1-qwen3-1.7b.md)** — 93% tool / 90% args / 0.8s latency

## Run History

| # | Model | Tool Acc | Args Acc | Latency | Notes |
|---|-------|----------|----------|---------|-------|
| [001](results/001-baseline-qwen3-1.7b.md) | Qwen3 1.7B (baseline) | 77% | 73% | 2.7s | No fine-tuning |
| [002](results/002-lora-v1-qwen3-1.7b.md) | Qwen3 1.7B + LoRA v1 | 93% | 90% | 0.8s | 161 examples, r=16, 5 epochs |
