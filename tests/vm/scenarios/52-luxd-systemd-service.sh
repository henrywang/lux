#!/bin/bash
# End-to-end deployment: install the systemd user unit, enable+start it,
# and verify luxd is active and writing its default config.
#
# Works on both targets:
#   cloud — lux + luxd at /usr/local/bin, luxd.service scp'd to /home/fedora.
#   bootc — lux + luxd at /usr/bin, luxd.service baked into /usr/lib/systemd/system.
set -euo pipefail

$VM_SSH bash -s <<'SH'
set -euo pipefail

# Fresh state: kill any stray luxd, drop prior config.
systemctl --user disable --now luxd.service 2>/dev/null || true
pkill -x luxd 2>/dev/null || true
rm -rf ~/.config/lux

# Locate luxd binary — cloud: /usr/local/bin, bootc: /usr/bin.
LUXD_BIN=""
for cand in /usr/local/bin/luxd /usr/bin/luxd; do
    if [ -x "$cand" ]; then LUXD_BIN="$cand"; break; fi
done
: "${LUXD_BIN:?luxd binary not found}"

# The packaged unit uses %h/.local/bin/luxd; symlink to the binary we have.
mkdir -p ~/.local/bin
ln -sf "$LUXD_BIN" ~/.local/bin/luxd

# Locate the unit file — cloud: scp'd to home, bootc: baked into /usr.
LUXD_UNIT=""
for cand in /home/fedora/luxd.service /usr/lib/systemd/system/luxd.service; do
    if [ -f "$cand" ]; then LUXD_UNIT="$cand"; break; fi
done
: "${LUXD_UNIT:?luxd.service not found}"

mkdir -p ~/.config/systemd/user
install -m 644 "$LUXD_UNIT" ~/.config/systemd/user/luxd.service

systemctl --user daemon-reload
systemctl --user enable --now luxd.service

# Give it a moment to start and write its default config.
sleep 3

systemctl --user is-active luxd.service > /dev/null
test -f ~/.config/lux/luxd.toml

# Cleanup.
systemctl --user disable --now luxd.service
SH
