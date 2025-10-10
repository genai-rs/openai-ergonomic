#!/bin/bash
# Check MSRV (Minimum Supported Rust Version) for openai-ergonomic
# This script verifies that the crate builds and tests pass with the minimum supported Rust version

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# MSRV version for openai-ergonomic (should match Cargo.toml and CI)
MSRV="1.82"

echo "Checking MSRV compatibility with Rust $MSRV..."
echo ""

# Check if the required Rust version is installed
if ! rustup toolchain list | grep -q "$MSRV"; then
    echo "Installing Rust $MSRV..."
    rustup toolchain install "$MSRV" --component rustfmt clippy
fi

echo "Setting Rust toolchain to $MSRV..."
cd "$PROJECT_ROOT"
rustup override set "$MSRV"

echo ""
echo "Building with Rust $MSRV..."
cargo build --verbose --all-features

echo ""
echo "Running tests with Rust $MSRV..."
cargo test --verbose --all-features

echo ""
echo "Running tests with no default features..."
cargo test --verbose --no-default-features

echo ""
echo "Building documentation..."
cargo doc --no-deps --all-features

echo ""
echo "Building examples..."
cargo build --examples --all-features

echo ""
echo "MSRV check passed! openai-ergonomic is compatible with Rust $MSRV"

# Reset to default toolchain
rustup override unset