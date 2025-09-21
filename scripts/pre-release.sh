#!/bin/bash
# Pre-release checks for openai-ergonomic
# This script runs comprehensive checks before releasing a new version

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🚀 Running pre-release checks for openai-ergonomic..."
echo ""

cd "$PROJECT_ROOT"

# Check if we're on the main branch
CURRENT_BRANCH=$(git branch --show-current)
if [[ "$CURRENT_BRANCH" != "main" ]]; then
    echo "❌ Not on main branch (currently on: $CURRENT_BRANCH)"
    echo "   Please switch to main branch before releasing"
    exit 1
fi

# Check if working directory is clean
if ! git diff-index --quiet HEAD --; then
    echo "❌ Working directory is not clean"
    echo "   Please commit or stash your changes before releasing"
    exit 1
fi

# Check if we're up to date with remote
echo "🔄 Fetching latest changes..."
git fetch origin

if [[ $(git rev-list HEAD...origin/main --count) -gt 0 ]]; then
    echo "❌ Local branch is not up to date with origin/main"
    echo "   Please pull the latest changes: git pull origin main"
    exit 1
fi

echo "✅ Git status checks passed"
echo ""

# Format check
echo "🎨 Checking code formatting..."
if ! cargo fmt --all -- --check; then
    echo "❌ Code formatting check failed"
    echo "   Please run: cargo fmt --all"
    exit 1
fi
echo "✅ Code formatting is correct"
echo ""

# Clippy check
echo "📎 Running clippy..."
if ! cargo clippy --all-targets --all-features -- -D warnings; then
    echo "❌ Clippy check failed"
    echo "   Please fix clippy warnings"
    exit 1
fi
echo "✅ Clippy checks passed"
echo ""

# Build check
echo "🏗️  Building project..."
if ! cargo build --all-features; then
    echo "❌ Build failed"
    exit 1
fi
echo "✅ Build successful"
echo ""

# Test suite
echo "🧪 Running test suite..."
if ! cargo test --all-features; then
    echo "❌ Tests failed"
    exit 1
fi
echo "✅ All tests passed"
echo ""

# Test with no default features
echo "🧪 Running tests with no default features..."
if ! cargo test --no-default-features; then
    echo "❌ Tests with no default features failed"
    exit 1
fi
echo "✅ Tests with no default features passed"
echo ""

# Documentation build
echo "📚 Building documentation..."
if ! cargo doc --no-deps --all-features; then
    echo "❌ Documentation build failed"
    exit 1
fi
echo "✅ Documentation built successfully"
echo ""

# Examples build
echo "💼 Building examples..."
if ! cargo build --examples --all-features; then
    echo "❌ Examples build failed"
    exit 1
fi
echo "✅ Examples built successfully"
echo ""

# Check for TODO comments that might need addressing
echo "🔍 Checking for TODO comments..."
TODO_COUNT=$(grep -r "TODO\|FIXME\|XXX" src/ examples/ --exclude-dir=target || true | wc -l)
if [[ $TODO_COUNT -gt 0 ]]; then
    echo "⚠️  Found $TODO_COUNT TODO/FIXME/XXX comments:"
    grep -r "TODO\|FIXME\|XXX" src/ examples/ --exclude-dir=target || true
    echo ""
    echo "   Consider addressing these before release"
    echo "   Continue anyway? (y/N)"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        echo "❌ Release cancelled"
        exit 1
    fi
fi

echo ""
echo "🎉 All pre-release checks passed!"
echo ""
echo "Ready to release! Next steps:"
echo "  1. Run: release-plz release-pr"
echo "  2. Review and merge the release PR"
echo "  3. Run: release-plz release"
echo ""
echo "Or use the automated workflow if configured in GitHub Actions."