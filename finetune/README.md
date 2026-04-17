# Qwen3 1.7B LoRA Fine-tuning for lux

## Goal

Push tool-calling accuracy from 77% → 90%+ in non-thinking mode.

## Failure patterns to fix

1. **No tool called** — model responds with text instead of calling a tool (3 cases)
2. **Wrong diagnostic tool** — uses check_service_status when read_logs or network_diagnose is correct (3 cases)
3. **Wrong service name** — "printer" instead of "cups" (1 case)
4. **Wrong action mapping** — manage_firewall vs manage_service for "disable firewall" (1 case)

## Dataset structure

Training data is in `dataset.jsonl` — one JSON object per line, each containing
a conversation with system prompt, user message, and assistant tool call response.

## Fine-tuning

```bash
# Using unsloth (recommended for QLoRA on consumer GPU)
pip install unsloth
python train.py
```
