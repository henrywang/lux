#!/usr/bin/env python3
"""Benchmark Qwen3 1.7B tool-calling accuracy for lux agent.

Usage:
    ollama pull qwen3:1.7b
    python3 bench/run_bench.py

Measures:
    - Tool selection accuracy (did it pick the right tool?)
    - Argument correctness (did it pass the right args?)
    - Response latency
    - Whether the model stays in non-thinking mode (fast path)
"""

import json
import subprocess
import sys
import time
from pathlib import Path

BENCH_DIR = Path(__file__).parent
MODEL = sys.argv[1] if len(sys.argv) > 1 else "qwen3:1.7b"
SYSTEM_PROMPT = """\
You are lux, an AI system agent for Linux desktop. You help users manage their system.

You have access to tools. When the user asks you to do something, call the appropriate tool.

Rules:
- Desktop GUI apps (firefox, gimp, vlc, steam, etc.) should be installed via install_flatpak
- CLI tools and system packages (vim, git, gcc, htop, etc.) should use install_package
- Always check service status before trying to fix service-related issues
- Use read_logs to investigate system problems
- Use network_diagnose for connectivity issues
- bootc_rollback when user wants to undo a system update
- bootc_status to show current image info
"""


def load_json(path: str):
    return json.loads((BENCH_DIR / path).read_text())


def call_ollama(user_input: str, tools: list) -> dict:
    """Call ollama API with tool definitions."""
    payload = {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": user_input},
        ],
        "tools": tools,
        "stream": False,
        # Disable thinking for fast tool-calling mode
        "options": {
            "num_ctx": 4096,
        },
        "think": False,
    }

    start = time.time()
    try:
        result = subprocess.run(
            ["curl", "-s", "-X", "POST", "-H", "Content-Type: application/json",
             "-d", json.dumps(payload), "http://localhost:11434/api/chat"],
            input=None,
            capture_output=True,
            text=True,
            timeout=600,
        )
    except subprocess.TimeoutExpired:
        return {"error": "timeout", "latency": time.time() - start}
    elapsed = time.time() - start

    if result.returncode != 0:
        return {"error": result.stderr, "latency": elapsed}

    try:
        resp = json.loads(result.stdout)
    except json.JSONDecodeError:
        return {"error": f"Invalid JSON: {result.stdout[:200]}", "latency": elapsed}

    return {**resp, "latency": elapsed}


def check_tool_match(response: dict, scenario: dict) -> dict:
    """Check if the model picked the right tool with correct args."""
    result = {
        "id": scenario["id"],
        "input": scenario["input"],
        "expected_tool": scenario["expected_tool"],
        "tool_correct": False,
        "args_correct": False,
        "actual_tool": None,
        "actual_args": None,
        "latency": response.get("latency", 0),
    }

    if "error" in response:
        result["error"] = response["error"]
        return result

    # Extract tool calls from response
    message = response.get("message", {})
    tool_calls = message.get("tool_calls", [])

    if not tool_calls:
        result["error"] = "No tool called"
        result["response_text"] = message.get("content", "")[:200]
        return result

    # Check the first tool call
    first_call = tool_calls[0]
    func = first_call.get("function", {})
    actual_tool = func.get("name", "")
    actual_args = func.get("arguments", {})

    result["actual_tool"] = actual_tool
    result["actual_args"] = actual_args

    # Tool name match
    if actual_tool == scenario["expected_tool"]:
        result["tool_correct"] = True

    # Argument match (partial - check expected args are present)
    expected_args = scenario.get("expected_args_contain")
    if expected_args is None:
        # No specific args expected, just tool match is enough
        result["args_correct"] = result["tool_correct"]
    elif result["tool_correct"]:
        args_ok = True
        for key, expected_val in expected_args.items():
            actual_val = actual_args.get(key)
            if actual_val is None:
                args_ok = False
                break
            # Flexible matching: check if expected value is contained
            if isinstance(expected_val, list):
                if isinstance(actual_val, list):
                    # Check all expected items appear in actual (case-insensitive)
                    for item in expected_val:
                        if not any(
                            item.lower() in str(a).lower() for a in actual_val
                        ):
                            args_ok = False
                            break
                else:
                    args_ok = False
            elif isinstance(expected_val, str):
                if expected_val.lower() not in str(actual_val).lower():
                    args_ok = False
            else:
                if actual_val != expected_val:
                    args_ok = False
        result["args_correct"] = args_ok

    return result


def print_result(r: dict):
    tool_icon = "\u2705" if r["tool_correct"] else "\u274c"
    args_icon = "\u2705" if r["args_correct"] else "\u274c"
    print(f"  [{r['id']:2d}] {tool_icon} tool {args_icon} args  {r['latency']:.1f}s  \"{r['input']}\"")
    if not r["tool_correct"]:
        print(f"       expected: {r['expected_tool']}, got: {r['actual_tool']}")
    if r["tool_correct"] and not r["args_correct"]:
        print(f"       args: {r['actual_args']}")
    if "error" in r:
        print(f"       error: {r['error'][:100]}")


def main():
    tools = load_json("tools.json")
    scenarios = load_json("scenarios.json")

    print(f"Benchmarking model: {MODEL}")
    print(f"Scenarios: {len(scenarios)}")
    print(f"Tools: {len(tools)}")
    print("-" * 70)

    results = []
    for scenario in scenarios:
        resp = call_ollama(scenario["input"], tools)
        result = check_tool_match(resp, scenario)
        results.append(result)
        print_result(result)

    # Summary
    print("\n" + "=" * 70)
    tool_correct = sum(1 for r in results if r["tool_correct"])
    args_correct = sum(1 for r in results if r["args_correct"])
    total = len(results)
    avg_latency = sum(r["latency"] for r in results) / total

    print(f"Tool selection:  {tool_correct}/{total} ({100*tool_correct/total:.0f}%)")
    print(f"Args correct:    {args_correct}/{total} ({100*args_correct/total:.0f}%)")
    print(f"Avg latency:     {avg_latency:.1f}s")
    print()

    # Per-category breakdown
    categories = {}
    for r in results:
        cat = next(
            (s["category"] for s in scenarios if s["id"] == r["id"]), "unknown"
        )
        if cat not in categories:
            categories[cat] = {"total": 0, "tool_ok": 0, "args_ok": 0}
        categories[cat]["total"] += 1
        if r["tool_correct"]:
            categories[cat]["tool_ok"] += 1
        if r["args_correct"]:
            categories[cat]["args_ok"] += 1

    print("Per category:")
    for cat, stats in sorted(categories.items()):
        pct = 100 * stats["tool_ok"] / stats["total"]
        print(f"  {cat:20s}  {stats['tool_ok']}/{stats['total']} tool  {stats['args_ok']}/{stats['total']} args  ({pct:.0f}%)")

    # Verdict
    print()
    if tool_correct / total >= 0.9:
        print(f"PASS - {MODEL} is viable for lux agent")
    elif tool_correct / total >= 0.75:
        print(f"MARGINAL - {MODEL} needs LoRA fine-tuning")
    else:
        print(f"FAIL - {MODEL} is not suitable, try a larger model")


if __name__ == "__main__":
    main()
