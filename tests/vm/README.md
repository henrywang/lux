# VM integration tests

End-to-end tests that boot a throw-away Fedora Cloud VM, install lux inside
it, and run real scenarios against live `dnf`, `systemctl`, and
`firewall-cmd`. Mirrors what a user would do on their own machine, catching
bugs unit tests and the bench harness can't see.

## Run locally

```bash
sudo dnf install -y qemu cloud-utils openssh-clients curl   # Fedora
cargo build --release --bin lux
just vm-test
```

First run downloads ~500 MB Fedora Cloud qcow2 into `fixtures/` (cached).
Subsequent runs use a copy-on-write overlay, so the base image stays pristine.

Runtime is ~2-3 minutes on a laptop with KVM.

## Run in CI

`.github/workflows/vm-tests.yml` runs the same `run.sh` on `ubuntu-latest`,
which exposes `/dev/kvm` for nested virt.

## Add a scenario

Drop a `scenarios/NN-name.sh` file. It runs on the **host**, not inside the
VM. One env var is exported for you:

- `VM_SSH` — a command prefix that runs its argument inside the VM (via
  `lib/ssh.sh`).

Convention:
- Exit 0 = pass, non-zero = fail.
- `set -euo pipefail`.
- Idempotent where possible; scenarios share a single VM boot.

```bash
#!/bin/bash
set -euo pipefail
$VM_SSH 'sudo lux -c "install htop"'
$VM_SSH 'rpm -q htop'
```

Lower numbers run first. Keep numbering sparse (`00`, `10`, `20`) so new
scenarios slot in without renumbering.

## Notes

- The VM runs intent-matcher scenarios only — no ollama inside the VM. That
  keeps CI under 20 minutes and deterministic. If you need to exercise the
  slow path, use bench/ on the host.
- SELinux is left in **enforcing** mode inside the VM (tests should reflect
  user systems).
- `install.sh` inside the VM honors `LUX_SKIP_OLLAMA=1` to skip the model
  download.
