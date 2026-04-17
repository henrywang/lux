#!/bin/bash
# Download the Fedora Cloud base qcow2 once and cache it under fixtures/.
# Override URL via FEDORA_CLOUD_URL if a newer point release is available.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES="$HERE/fixtures"
mkdir -p "$FIXTURES"

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
