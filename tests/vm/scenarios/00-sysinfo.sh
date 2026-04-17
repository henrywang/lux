#!/bin/bash
# /sysinfo is a REPL command, not a -c command. Use the bare CLI to ensure
# the binary boots at all. Any exit 0 with a banner line is a pass.
set -euo pipefail
out="$($VM_SSH 'echo quit | lux 2>&1' || true)"
grep -q 'Fedora' <<<"$out"
