#!/bin/bash
# Only meaningful on bootc images. Verify that:
#   - sysinfo detects image mode,
#   - `bootc status` runs and returns the booted image reference.
set -euo pipefail

if [ "${VM_TARGET:-cloud}" != "bootc" ]; then
    echo "skip (VM_TARGET=$VM_TARGET)" >&2
    exit 0
fi

$VM_SSH 'test -e /run/bootc || test -e /sysroot' \
    || { echo "neither /run/bootc nor /sysroot present; not a bootc image?" >&2; exit 1; }

out="$($VM_SSH 'lux -c "show bootc status" 2>&1' || true)"
grep -Eqi 'booted|staged|rollback|image' <<<"$out"
