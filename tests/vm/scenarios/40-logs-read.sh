#!/bin/bash
# Phrasing must hit the intent-matcher fast path (no ollama in VM).
# "show me recent errors in the system log" → read_logs(priority=err).
set -euo pipefail
$VM_SSH 'sudo lux -c "show me recent errors in the system log"' >/dev/null
