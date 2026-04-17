# Baseline: Qwen3 1.7B (no fine-tuning)

**Date:** 2026-04-15
**Model:** qwen3:1.7b (ollama, default quantization)
**Hardware:** Local machine, CPU inference

## Summary

| Mode | Tool Accuracy | Args Accuracy | Avg Latency |
|------|--------------|---------------|-------------|
| Thinking ON | 26/30 (87%) | 26/30 (87%) | 16.6s |
| Thinking OFF | 23/30 (77%) | 22/30 (73%) | 2.7s |

## Per-Category (Thinking OFF)

| Category | Tool Correct | Args Correct | Accuracy |
|----------|-------------|--------------|----------|
| bootc | 3/3 | 3/3 | 100% |
| diagnosis | 3/6 | 2/6 | 50% |
| firewall | 2/3 | 2/3 | 67% |
| logs | 2/4 | 2/4 | 50% |
| package_management | 9/9 | 9/9 | 100% |
| service_management | 4/5 | 4/5 | 80% |

## Failure Patterns (Thinking OFF)

| # | Input | Expected | Got | Pattern |
|---|-------|----------|-----|---------|
| 3 | "bluetooth headphones won't connect" | check_service_status | (no tool called) | Text instead of tool |
| 4 | "enable SSH" | manage_service | (no tool called) | Text instead of tool |
| 7 | "wifi is not working" | network_diagnose | check_service_status | Wrong diagnostic tool |
| 17 | "sshd in the last hour" | read_logs | check_service_status | Should read logs |
| 22 | "printer isn't working" | check_service_status (cups) | check_service_status (printer) | Wrong service name |
| 23 | "disable the firewall" | manage_service | manage_firewall | Ambiguous |
| 25 | "critical errors today" | read_logs | check_service_status | Should read logs |
| 27 | "can't reach the internet" | network_diagnose | (no tool called) | Text instead of tool |

## Other Models Tested

| Model | Size | Tool Accuracy | Avg Latency | Verdict |
|-------|------|--------------|-------------|---------|
| Qwen3 1.7B (thinking) | 1.4GB | 87% | 16.6s | Too slow |
| Qwen3 1.7B (no-think) | 1.4GB | 77% | 2.7s | Best balance |
| Qwen3 0.6B | 522MB | 57% | 1.4s | Too inaccurate |
| Qwen3 4B | 2.5GB | ~100%* | ~40s+ | Too slow on CPU |
| Granite 4.0 tiny | 4.0GB | 67% | 16.9s | Slow + inaccurate |
| Granite 4.0 nano | 366MB | 67% | 3.5s | Less accurate, slower |

## Decision

Ship Qwen3 1.7B (non-thinking) + LoRA fine-tuning to close the gap from 77% to 90%+.
