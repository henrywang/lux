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

check: lint test

bench:
    python bench/run_bench.py

install: build
    mkdir -p ~/.local/bin
    ln -sf $(pwd)/target/release/lux ~/.local/bin/lux
    @echo "Installed to ~/.local/bin/lux"

clean:
    cargo clean
