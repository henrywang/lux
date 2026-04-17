#!/bin/bash
# Boot an ephemeral Fedora VM, install lux inside it, run every scenario,
# and report PASS/FAIL. Exit non-zero if any scenario fails.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"

: "${SSH_PORT:=2222}"

# --- preflight ---
for cmd in qemu-system-x86_64 qemu-img cloud-localds ssh scp curl; do
    if ! command -v "$cmd" >/dev/null; then
        echo "missing required command: $cmd" >&2
        echo "on fedora: dnf install -y qemu cloud-utils openssh-clients curl" >&2
        echo "on ubuntu: apt-get install -y qemu-system-x86 cloud-image-utils openssh-client curl" >&2
        exit 2
    fi
done

if [ ! -x "$ROOT/target/release/lux" ]; then
    echo "build the release binary first: cargo build --release --bin lux" >&2
    exit 2
fi

# --- workspace ---
WORKDIR="$(mktemp -d -t lux-vm-XXXXXX)"
export WORKDIR
SERIAL_LOG="/tmp/lux-vm-$(basename "$WORKDIR").log"
export SERIAL_LOG

cleanup() {
    local rc=$?
    if [ -f "$WORKDIR/qemu.pid" ]; then
        kill "$(cat "$WORKDIR/qemu.pid")" 2>/dev/null || true
    fi
    rm -rf "$WORKDIR"
    exit $rc
}
trap cleanup EXIT INT TERM

# --- image ---
bash "$HERE/lib/fetch-image.sh"
BASE_IMAGE="$HERE/fixtures/fedora.qcow2"
export BASE_IMAGE

# --- ssh key + cloud-init seed ---
SSH_KEY="$WORKDIR/id_ed25519"
export SSH_KEY
ssh-keygen -t ed25519 -N "" -f "$SSH_KEY" -q
PUBKEY="$(cat "${SSH_KEY}.pub")"
sed "s|__SSH_PUBKEY__|$PUBKEY|" "$HERE/cloud-init/user-data.tmpl" > "$WORKDIR/user-data"
SEED_ISO="$WORKDIR/seed.iso"
export SEED_ISO
cloud-localds "$SEED_ISO" "$WORKDIR/user-data" "$HERE/cloud-init/meta-data"

# --- boot ---
bash "$HERE/lib/boot.sh"
if ! TIMEOUT=300 bash "$HERE/lib/wait-ssh.sh"; then
    echo "--- serial log tail ---" >&2
    tail -n 200 "$SERIAL_LOG" >&2 || true
    exit 1
fi

SCP() {
    scp -q -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
        -i "$SSH_KEY" -P "$SSH_PORT" "$@"
}
VM_SSH="bash $HERE/lib/ssh.sh"
export VM_SSH

# --- push lux + luxd ---
# We use the locally-built binaries directly; install.sh is exercised on the
# host side via `just lint` (shellcheck). Real release-install flow is
# covered once release tags exist.
SCP "$ROOT/target/release/lux" fedora@127.0.0.1:/home/fedora/lux
SCP "$ROOT/target/release/luxd" fedora@127.0.0.1:/home/fedora/luxd
SCP "$ROOT/systemd/luxd.service" fedora@127.0.0.1:/home/fedora/luxd.service
$VM_SSH 'sudo install -m 755 /home/fedora/lux /usr/local/bin/lux'
$VM_SSH 'sudo install -m 755 /home/fedora/luxd /usr/local/bin/luxd'

# --- scenarios ---
pass=0
fail=0
failed_names=()
for s in "$HERE"/scenarios/*.sh; do
    name="$(basename "$s")"
    printf "  %-35s " "$name"
    if bash "$s"; then
        echo "PASS"
        pass=$((pass + 1))
    else
        echo "FAIL"
        fail=$((fail + 1))
        failed_names+=("$name")
    fi
done

if [ "$fail" -gt 0 ]; then
    echo
    echo "--- serial log tail ---"
    tail -n 100 "$SERIAL_LOG" || true
fi

total=$((pass + fail))
echo
echo "======================================"
echo "  scenarios: $total total"
echo "  passed:    $pass"
echo "  failed:    $fail"
if [ "$fail" -gt 0 ]; then
    echo "  failing:   ${failed_names[*]}"
fi
echo "======================================"

[ "$fail" -eq 0 ]
