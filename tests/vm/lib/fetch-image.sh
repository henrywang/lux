#!/bin/bash
# Provide a bootable disk under fixtures/. Two targets:
#   VM_TARGET=cloud (default) — downloads Fedora Cloud base qcow2.
#   VM_TARGET=bootc           — builds fedora-bootc:43 + cloud-init and
#                               installs it to a raw disk via
#                               `bootc install to-disk` (runs inside a
#                               privileged podman container of the image
#                               itself — no extra host tools needed).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
FIXTURES="$HERE/fixtures"
mkdir -p "$FIXTURES"

: "${VM_TARGET:=cloud}"

case "$VM_TARGET" in
    cloud)
        : "${FEDORA_CLOUD_URL:=https://download.fedoraproject.org/pub/fedora/linux/releases/43/Cloud/x86_64/images/Fedora-Cloud-Base-Generic-43-1.6.x86_64.qcow2}"
        DEST="$FIXTURES/fedora.qcow2"
        if [ -f "$DEST" ]; then
            echo "image already cached: $DEST"
            exit 0
        fi
        echo "downloading $FEDORA_CLOUD_URL"
        curl -fL --retry 3 -o "$DEST.part" "$FEDORA_CLOUD_URL"
        mv "$DEST.part" "$DEST"
        echo "cached: $DEST ($(du -h "$DEST" | cut -f1))"
        ;;
    bootc)
        DEST="$FIXTURES/fedora-bootc.raw"
        TAG="localhost/fedora-bootc-ci:43"
        : "${BOOTC_DISK_SIZE:=10G}"

        LUX_BIN="$ROOT/target/release/lux"
        LUXD_BIN="$ROOT/target/release/luxd"
        LUXD_SERVICE="$ROOT/systemd/luxd.service"
        CONTAINERFILE="$HERE/bootc/Containerfile"
        for f in "$LUX_BIN" "$LUXD_BIN" "$LUXD_SERVICE" "$CONTAINERFILE"; do
            if [ ! -e "$f" ]; then
                echo "missing input for bootc image: $f" >&2
                echo "build first: cargo build --release --bin lux --bin luxd" >&2
                exit 2
            fi
        done

        # Invalidate the cached raw disk if any baked-in input is newer.
        if [ -f "$DEST" ]; then
            if find "$LUX_BIN" "$LUXD_BIN" "$LUXD_SERVICE" "$CONTAINERFILE" \
                -newer "$DEST" -print -quit 2>/dev/null | grep -q .; then
                echo "inputs changed, rebuilding $DEST"
                rm -f "$DEST"
            else
                echo "image already built: $DEST"
                exit 0
            fi
        fi

        # Assemble build context with baked-in binaries.
        BUILD_CTX="$(mktemp -d -t lux-bootc-ctx-XXXXXX)"
        trap 'rm -rf "$BUILD_CTX"' EXIT
        cp "$LUX_BIN" "$BUILD_CTX/lux"
        cp "$LUXD_BIN" "$BUILD_CTX/luxd"
        cp "$LUXD_SERVICE" "$BUILD_CTX/luxd.service"
        cp "$CONTAINERFILE" "$BUILD_CTX/Containerfile"

        echo "building $TAG"
        sudo podman build -t "$TAG" "$BUILD_CTX"
        echo "installing to $DEST (size=$BOOTC_DISK_SIZE) — sudo required for bootc install"
        truncate -s "$BOOTC_DISK_SIZE" "$DEST.part"
        # bootc install to-disk requires the root user namespace, so the
        # podman run must be privileged *and* system-scope (not rootless).
        sudo podman run --rm --privileged --pid=host \
            --security-opt label=type:unconfined_t \
            -v /dev:/dev \
            -v /var/lib/containers:/var/lib/containers \
            -v "$FIXTURES":/output \
            "$TAG" \
            bootc install to-disk \
                --filesystem btrfs \
                --via-loopback \
                --generic-image \
                --skip-fetch-check \
                "/output/$(basename "$DEST.part")"
        sudo chown "$(id -u):$(id -g)" "$DEST.part"
        mv "$DEST.part" "$DEST"
        echo "cached: $DEST ($(du -h --apparent-size "$DEST" | cut -f1) apparent, $(du -h "$DEST" | cut -f1) actual)"
        ;;
    *)
        echo "unknown VM_TARGET: $VM_TARGET (expected 'cloud' or 'bootc')" >&2
        exit 2
        ;;
esac
