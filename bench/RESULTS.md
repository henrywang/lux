# Qwen3 1.7B Benchmark Results

**Date:** 2026-04-15
**Model:** qwen3:1.7b (ollama, default quantization)
**Hardware:** Local machine, CPU inference
**Scenarios:** 30 tool-calling tasks across 7 categories
**Tools:** 13 system administration tools

## Summary

| Mode | Tool Accuracy | Args Accuracy | Avg Latency |
|------|--------------|---------------|-------------|
| Thinking ON | 26/30 (87%) | 26/30 (87%) | 16.6s |
| Thinking OFF | 23/30 (77%) | 22/30 (73%) | 2.7s |

## Per-Category Breakdown

### Thinking ON

| Category | Tool Correct | Args Correct | Accuracy |
|----------|-------------|--------------|----------|
| bootc | 3/3 | 3/3 | 100% |
| diagnosis | 6/6 | 6/6 | 100% |
| firewall | 2/3 | 2/3 | 67% |
| logs | 4/4 | 4/4 | 100% |
| package_management | 7/9 | 7/9 | 78% |
| service_management | 4/5 | 4/5 | 80% |

### Thinking OFF

| Category | Tool Correct | Args Correct | Accuracy |
|----------|-------------|--------------|----------|
| bootc | 3/3 | 3/3 | 100% |
| diagnosis | 3/6 | 2/6 | 50% |
| firewall | 2/3 | 2/3 | 67% |
| logs | 2/4 | 2/4 | 50% |
| package_management | 9/9 | 9/9 | 100% |
| service_management | 4/5 | 4/5 | 80% |

## Failure Analysis

### Thinking ON (4 failures)

| # | Input | Expected | Got | Pattern |
|---|-------|----------|-----|---------|
| 10 | "remove libreoffice" | remove_package | check_service_status | Check-first behavior |
| 21 | "install htop and tmux" | install_package | install_flatpak | Flatpak vs package confusion |
| 23 | "disable the firewall" | manage_service | check_service_status | Check-first behavior |
| 28 | "enable and start bluetooth" | manage_service | check_service_status | Check-first behavior |

**Pattern:** 3/4 failures are the model choosing to check status before acting. This is cautious behavior, not fundamentally wrong reasoning. Fixable with fine-tuning to teach "when user says do X, do X."

### Thinking OFF (7 failures)

| # | Input | Expected | Got | Pattern |
|---|-------|----------|-----|---------|
| 3 | "bluetooth headphones won't connect" | check_service_status | (no tool called) | Responded with text instead |
| 4 | "enable SSH" | manage_service | (no tool called) | Responded with text instead |
| 7 | "wifi is not working" | network_diagnose | check_service_status | Wrong diagnostic tool |
| 17 | "sshd in the last hour" | read_logs | check_service_status | Should read logs, checked service |
| 22 | "printer isn't working" | check_service_status (cups) | check_service_status (printer) | Right tool, wrong service name |
| 23 | "disable the firewall" | manage_service | manage_firewall | Reasonable alternative |
| 25 | "critical errors today" | read_logs | check_service_status | Should read logs, checked service |
| 27 | "can't reach the internet" | network_diagnose | (no tool called) | Responded with text instead |

**Patterns:**
- 3 cases: model responded with text instead of calling a tool (diagnosis scenarios)
- 3 cases: model defaulted to check_service_status when other tools were more appropriate
- 1 case: wrong service name ("printer" instead of "cups")
- 1 case: arguably correct (manage_firewall for "disable the firewall")

## Key Observations

1. **Thinking mode is a quality/speed tradeoff.** Thinking gives 87% accuracy at 16.6s. Non-thinking gives 77% at 2.7s. Neither is ideal out of the box.

2. **Package management is solved.** 100% accuracy in non-thinking mode for install/remove and flatpak vs system package decisions. The system prompt rules are sufficient.

3. **bootc operations are solid.** 100% in both modes. rollback, status, and switch all work correctly.

4. **Diagnosis is the weak spot.** The model struggles to pick the right diagnostic tool (network_diagnose vs check_service_status vs read_logs) especially without thinking. This is the primary target for LoRA fine-tuning.

5. **Non-thinking mode sometimes doesn't call tools at all.** For open-ended problems ("bluetooth won't connect", "can't reach internet"), the model wants to give advice text instead of taking action. Fine-tuning should fix this.

6. **16.6s latency with thinking is too slow** for an interactive agent. Users expect sub-3s responses.

## Recommendation

**Ship non-thinking mode + LoRA fine-tuning.**

- Non-thinking mode gives acceptable latency (2.7s)
- LoRA adapter can close the accuracy gap from 77% to 90%+
- Fine-tuning targets: (1) always call a tool, don't respond with text, (2) correct diagnostic tool selection, (3) correct service name mapping
- Estimated fine-tuning data needed: ~200-500 examples covering the failure patterns

**Alternative:** Benchmark Qwen3 4B to see if it hits 90%+ without fine-tuning. If so, compare the RAM/latency tradeoff vs fine-tuned 1.7B.

## Qwen3 0.6B Results

**Too inaccurate.** 57% tool accuracy, 43% args accuracy at 1.4s avg latency.

| Category | Tool Correct | Args Correct | Accuracy |
|----------|-------------|--------------|----------|
| bootc | 1/3 | 1/3 | 33% |
| diagnosis | 3/6 | 3/6 | 50% |
| firewall | 2/3 | 2/3 | 67% |
| logs | 3/4 | 0/4 | 75% |
| package_management | 3/9 | 3/9 | 33% |
| service_management | 5/5 | 4/5 | 100% |

The model is fast (1.4s avg) but fails to call tools at all in 13/30 cases — it responds with text instead. Args accuracy is also poor: logs always get wrong priority ("info" instead of "err"/"crit"), and service args are incomplete. Closing a 33% accuracy gap with LoRA is too much to ask. Ruled out.

## Qwen3 4B Results

**Too slow for CPU inference.** Tested on same hardware:

- "hi" (no tools): 37s
- "install firefox" (with tools): 64s
- Scenario 4 timed out at 300s

The 4B model is ~20x slower than 1.7B non-thinking. This rules it out as a default model for CPU-only machines. It could be an opt-in option for users with a GPU, but not the default.

## Granite 4.0 Tiny Results

**Slower and less accurate than Qwen3 1.7B.** 67% tool accuracy, 53% args accuracy at 16.9s avg latency. Model size is 4GB.

| Category | Tool Correct | Args Correct | Accuracy |
|----------|-------------|--------------|----------|
| bootc | 3/3 | 3/3 | 100% |
| diagnosis | 3/6 | 3/6 | 50% |
| firewall | 0/3 | 0/3 | 0% |
| logs | 4/4 | 2/4 | 100% |
| package_management | 8/9 | 7/9 | 89% |
| service_management | 2/5 | 1/5 | 40% |

Despite being specifically designed for tool use, Granite 4.0 tiny underperforms Qwen3 1.7B on every metric: slower (16.9s vs 2.7s), less accurate (67% vs 77%), larger (4GB vs 1.4GB). Completely fails on firewall tools (0%). Ruled out.

## Granite 4.0 Nano (350M) Results

**Too inaccurate and surprisingly slow for its size.** 67% tool accuracy, 60% args accuracy at 3.5s avg latency. Model size is only 366MB but slower than Qwen3 1.7B (1.4GB).

| Category | Tool Correct | Args Correct | Accuracy |
|----------|-------------|--------------|----------|
| bootc | 2/3 | 2/3 | 67% |
| diagnosis | 4/6 | 4/6 | 67% |
| firewall | 2/3 | 2/3 | 67% |
| logs | 2/4 | 1/4 | 50% |
| package_management | 5/9 | 5/9 | 56% |
| service_management | 5/5 | 4/5 | 100% |

Key weakness: cannot distinguish flatpak vs system package (firefox, GIMP, steam, VLC all routed to install_package). Also confuses read_logs with check_service_status. Ruled out.

## All Models Comparison

| Model | Size | Tool Accuracy | Args Accuracy | Avg Latency | Verdict |
|-------|------|--------------|---------------|-------------|---------|
| Qwen3 1.7B (thinking) | 1.4GB | 87% | 87% | 16.6s | Too slow |
| **Qwen3 1.7B (no-think)** | **1.4GB** | **77%** | **73%** | **2.7s** | **Winner** |
| Qwen3 0.6B (no-think) | 522MB | 57% | 43% | 1.4s | Too inaccurate |
| Qwen3 4B (no-think) | 2.5GB | ~100%* | ~100%* | ~40s+ | Too slow |
| Granite 4.0 tiny | 4.0GB | 67% | 53% | 16.9s | Slow + inaccurate |
| Granite 4.0 nano | 366MB | 67% | 60% | 3.5s | Less accurate, slower |

*4B only completed 3/30 scenarios before timeout, all correct.

## Conclusion

**Qwen3 1.7B (non-thinking) + LoRA fine-tuning is the path forward.**

- 1.7B non-thinking gives acceptable latency (2.7s avg)
- 77% accuracy out of the box, with clear failure patterns that are fixable via fine-tuning
- 0.6B is too dumb (57%), closing a 33% gap via LoRA is unrealistic
- 4B is too slow on CPU (~40s per query), not viable as default
- Fine-tuning targets: always use tools (not text), correct diagnostic tool routing, service name mapping
- Estimated: ~200-500 training examples to reach 90%+

## Next Steps

- [ ] Benchmark Granite 3.1 2B for comparison
- [ ] Build LoRA fine-tuning dataset from failure patterns
- [ ] Fine-tune Qwen3 1.7B and re-benchmark
- [ ] Measure idle/active RAM usage precisely
- [ ] Test on lower-end hardware (8GB RAM laptop)
