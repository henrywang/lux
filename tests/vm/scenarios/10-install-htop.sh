#!/bin/bash
set -euo pipefail
$VM_SSH 'sudo lux -c "install htop"'
$VM_SSH 'rpm -q htop'
