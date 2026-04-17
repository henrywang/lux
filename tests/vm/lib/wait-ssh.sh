#!/bin/bash
# Poll SSH until the VM accepts a connection, or fail after TIMEOUT seconds.
set -euo pipefail

: "${SSH_PORT:=2222}"
: "${SSH_KEY:?SSH_KEY not set}"
: "${TIMEOUT:=180}"

deadline=$(( $(date +%s) + TIMEOUT ))
while [ "$(date +%s)" -lt "$deadline" ]; do
    if ssh -o StrictHostKeyChecking=no \
           -o UserKnownHostsFile=/dev/null \
           -o ConnectTimeout=3 \
           -o BatchMode=yes \
           -i "$SSH_KEY" \
           -p "$SSH_PORT" \
           fedora@127.0.0.1 \
           'test -f /tmp/cloud-init-done' 2>/dev/null; then
        echo "vm ready"
        exit 0
    fi
    sleep 2
done

echo "timed out waiting for SSH after ${TIMEOUT}s" >&2
exit 1
