#!/bin/bash
# Thin wrapper: run a command inside the VM via SSH.
# Usage: ssh.sh "command string"
set -euo pipefail

: "${SSH_PORT:=2222}"
: "${SSH_KEY:?SSH_KEY not set}"

exec ssh -o StrictHostKeyChecking=no \
         -o UserKnownHostsFile=/dev/null \
         -o LogLevel=ERROR \
         -i "$SSH_KEY" \
         -p "$SSH_PORT" \
         fedora@127.0.0.1 \
         "$@"
