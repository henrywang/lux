#!/bin/bash
# Firefox is in the Fedora dnf repo, so intent matcher routes to install_package.
set -euo pipefail
$VM_SSH 'sudo lux -c "install firefox"'
$VM_SSH 'rpm -q firefox'
