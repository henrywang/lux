#!/bin/bash
# On cloud: firefox is in the Fedora dnf repo, so intent matcher routes to
# install_package (dnf). On bootc: lux routes to flatpak (flathub).
set -euo pipefail

$VM_SSH 'sudo lux -c "install firefox"'

if [ "${VM_TARGET:-cloud}" = "bootc" ]; then
    # lux's install_flatpak uses --user, so with `sudo lux` the install lands
    # in root's user scope. Query as root so we see that scope.
    $VM_SSH 'sudo flatpak list --app --columns=application | grep -qi firefox'
else
    $VM_SSH 'rpm -q firefox'
fi
