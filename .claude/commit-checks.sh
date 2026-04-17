#!/bin/bash
# Pre-commit checks run by Claude Code's /commit skill.
# All checks must pass before changes are committed.
set -e

echo "==> cargo fmt --check"
cargo fmt --all -- --check

echo "==> cargo clippy"
cargo clippy --all-targets -- -D warnings

echo "==> cargo test"
cargo test
