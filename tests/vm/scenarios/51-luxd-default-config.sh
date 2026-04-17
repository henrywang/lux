#!/bin/bash
# Fresh luxd writes a default config file on first run.
set -euo pipefail

$VM_SSH bash -s <<'SH'
set -euo pipefail

pkill -x luxd 2>/dev/null || true
rm -rf ~/.config/lux

# Run luxd briefly; it should create the default config and start polling.
timeout 2 luxd || true

test -f ~/.config/lux/luxd.toml
grep -q '^mode = "suggest"' ~/.config/lux/luxd.toml
SH
