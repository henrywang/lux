#!/bin/bash
set -euo pipefail
$VM_SSH 'sudo lux -c "block IP 10.0.0.99"'
$VM_SSH 'sudo firewall-cmd --list-rich-rules | grep -q "10.0.0.99"'
