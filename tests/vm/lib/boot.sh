#!/bin/bash
# Boot a Fedora VM from the cached base image + a freshly-rendered seed ISO.
# Expects env: WORKDIR, BASE_IMAGE, SEED_ISO, SSH_PORT, SERIAL_LOG.
set -euo pipefail

: "${WORKDIR:?WORKDIR not set}"
: "${BASE_IMAGE:?BASE_IMAGE not set}"
: "${SEED_ISO:?SEED_ISO not set}"
: "${SSH_PORT:=2222}"
: "${SERIAL_LOG:=$WORKDIR/serial.log}"
: "${VM_RAM_MB:=2048}"
: "${BASE_FMT:=qcow2}"

OVERLAY="$WORKDIR/overlay.qcow2"
qemu-img create -q -f qcow2 -F "$BASE_FMT" -b "$BASE_IMAGE" "$OVERLAY" >/dev/null

KVM_ARGS=()
if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
    KVM_ARGS=(-enable-kvm -cpu host)
else
    echo "warn: /dev/kvm not accessible, falling back to TCG (slow)" >&2
    KVM_ARGS=(-cpu max)
fi

qemu-system-x86_64 \
    "${KVM_ARGS[@]}" \
    -smp 2 -m "$VM_RAM_MB" \
    -display none -serial "file:$SERIAL_LOG" -monitor none \
    -nic "user,hostfwd=tcp::${SSH_PORT}-:22" \
    -drive "file=$OVERLAY,if=virtio,format=qcow2" \
    -drive "file=$SEED_ISO,if=virtio,format=raw,readonly=on" \
    -pidfile "$WORKDIR/qemu.pid" \
    -daemonize

echo "qemu pid: $(cat "$WORKDIR/qemu.pid")"
