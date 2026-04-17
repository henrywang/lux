#!/bin/bash
# End-to-end deployment: install the systemd user unit, enable+start it,
# and verify luxd is active and writing its default config.
set -euo pipefail

$VM_SSH bash -s <<'SH'
set -euo pipefail

# Fresh state: kill any stray luxd, drop prior config.
systemctl --user disable --now luxd.service 2>/dev/null || true
pkill -x luxd 2>/dev/null || true
rm -rf ~/.config/lux

# The packaged unit uses %h/.local/bin/luxd; symlink to the binary we installed.
mkdir -p ~/.local/bin
ln -sf /usr/local/bin/luxd ~/.local/bin/luxd

# Install the real unit file shipped in the repo.
mkdir -p ~/.config/systemd/user
install -m 644 /home/fedora/luxd.service ~/.config/systemd/user/luxd.service

systemctl --user daemon-reload
systemctl --user enable --now luxd.service

# Give it a moment to start and write its default config.
sleep 3

systemctl --user is-active luxd.service > /dev/null
test -f ~/.config/lux/luxd.toml

# Cleanup.
systemctl --user disable --now luxd.service
SH
