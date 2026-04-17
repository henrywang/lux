#!/bin/bash
set -euo pipefail
$VM_SSH 'sudo lux -c "restart sshd"'
$VM_SSH 'systemctl is-active sshd'
