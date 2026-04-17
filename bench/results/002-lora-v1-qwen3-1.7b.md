# LoRA v1: Fine-tuned Qwen3 1.7B

**Date:** 2026-04-17
**Base model:** unsloth/Qwen3-1.7B
**Method:** QLoRA (4-bit), LoRA r=16, alpha=32, dropout=0.05
**Training:** 161 examples, 5 epochs, lr=2e-4, packing=False
**Hardware:** Google Colab T4 GPU
**Output:** GGUF Q4_K_M

## Summary

| Metric | Baseline | Fine-tuned | Change |
|--------|----------|------------|--------|
| Tool Accuracy | 23/30 (77%) | 28/30 (93%) | +16% |
| Args Accuracy | 22/30 (73%) | 27/30 (90%) | +17% |
| Avg Latency | 2.7s | 0.8s | -1.9s |

## Per-Category

| Category | Tool Correct | Args Correct | Accuracy |
|----------|-------------|--------------|----------|
| bootc | 3/3 | 3/3 | 100% |
| diagnosis | 6/6 | 6/6 | 100% |
| firewall | 2/3 | 2/3 | 67% |
| logs | 3/4 | 3/4 | 75% |
| package_management | 9/9 | 9/9 | 100% |
| service_management | 5/5 | 4/5 | 100% |

## Remaining Failures

| # | Input | Expected | Got | Notes |
|---|-------|----------|-----|-------|
| 23 | "disable the firewall" | manage_service | manage_firewall | Ambiguous |
| 28 | "enable and start bluetooth" | manage_service(enable) | manage_service(start) | Correct tool, wrong action |
| 30 | "sshd fail authentication today?" | read_logs | check_service_status | Persistent |

## What Fixed

- Text-instead-of-tool failures: all 3 fixed
- Wrong diagnostic tool (check_service_status vs read_logs): 2 of 3 fixed
- Wrong service name ("printer" → "cups"): fixed
- diagnosis category: 50% → 100%
- logs category: 50% → 75%

## Key Learnings

- **packing=False is critical** — packing corrupts tool-call format across short examples
- **System prompt must match** between training data and inference
- **Higher LoRA rank (32) caused overfitting** — r=16 with 5 epochs was the sweet spot
- **Higher epochs (8) caused regressions** — args quality degraded with nested null values
- **161 examples was enough** for 93% accuracy on 30 scenarios
