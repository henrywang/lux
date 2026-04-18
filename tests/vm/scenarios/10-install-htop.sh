#!/bin/bash
set -euo pipefail
if [ "${VM_TARGET:-cloud}" = "bootc" ]; then
    echo "skip (VM_TARGET=bootc: dnf-install requires mutable /usr)" >&2
    exit 0
fi
$VM_SSH 'sudo lux -c "install htop"'
$VM_SSH 'rpm -q htop'
