default:
    @just --list

build:
    cargo build --release

run *ARGS:
    cargo run --release -- {{ARGS}}

test:
    cargo test

fmt:
    cargo fmt --all

lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    shellcheck install.sh setup.sh .claude/commit-checks.sh
    shellcheck tests/vm/run.sh tests/vm/lib/*.sh tests/vm/scenarios/*.sh

check: lint test

bench:
    python bench/run_bench.py

# Boot an ephemeral Fedora VM and run end-to-end scenarios.
# Requires: qemu, cloud-utils, ssh. Downloads ~500 MB on first run.
vm-test:
    cargo build --release --bin lux --bin luxd
    bash tests/vm/run.sh

install: build
    mkdir -p ~/.local/bin
    ln -sf $(pwd)/target/release/lux ~/.local/bin/lux
    ln -sf $(pwd)/target/release/luxd ~/.local/bin/luxd
    mkdir -p ~/.config/systemd/user
    cp systemd/luxd.service ~/.config/systemd/user/luxd.service
    systemctl --user daemon-reload
    @echo "Installed lux + luxd to ~/.local/bin"
    @echo "Enable the daemon with: systemctl --user enable --now luxd"

clean:
    cargo clean
