#!/bin/bash
# Pre-commit checks run by Claude Code's /commit skill.
# All checks must pass before changes are committed.
set -e

just check
