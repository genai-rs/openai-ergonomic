#!/bin/bash
# Setup git hooks for the openai-ergonomic project

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "Setting up git hooks for openai-ergonomic..."

cd "$PROJECT_ROOT"

# Configure git to use our hooks directory
git config core.hooksPath .githooks

echo "Git hooks configured successfully!"
echo ""
echo "The following hooks are now active:"
echo "  - pre-commit: Runs cargo fmt, clippy, build, and tests"
echo ""
echo "To bypass hooks temporarily, use: git commit --no-verify"