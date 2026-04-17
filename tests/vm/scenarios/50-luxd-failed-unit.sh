#!/bin/bash
# Start luxd with a short poll interval, deliberately fail a systemd unit,
# and verify luxd detects it and writes a finding to the jsonl log.
set -euo pipefail

$VM_SSH bash -s <<'SH'
set -euo pipefail

# Fast-polling config so the test doesn't wait a full minute.
mkdir -p ~/.config/lux
cat > ~/.config/lux/luxd.toml <<CFG
mode = "monitor"
watch_journal = true
watch_avc = true
watch_disk = true
notify_desktop = false
notify_repl = true
interval_secs = 3
CFG

findings_file="/run/user/$(id -u)/lux/findings.jsonl"

# Fresh state.
pkill -x luxd 2>/dev/null || true
rm -f "$findings_file"

# Start the daemon detached.
nohup luxd > /tmp/luxd.log 2>&1 &
disown

# Create a transient system unit that exits non-zero → ends up in failed state.
sudo systemd-run --unit=lux-test-fail.service /bin/false || true
sleep 2
sudo systemctl is-failed lux-test-fail.service > /dev/null

# Wait for at least one luxd poll (interval=3s) plus some slack.
sleep 7

# Assert: a finding with category "unit-failed" was written.
test -s "$findings_file"
grep -q '"unit-failed"' "$findings_file"
grep -q 'lux-test-fail' "$findings_file"

# Cleanup.
sudo systemctl reset-failed lux-test-fail.service 2>/dev/null || true
pkill -x luxd 2>/dev/null || true
SH
