#!/bin/bash
# Check MSRV (Minimum Supported Rust Version) for openai-ergonomic
# This script verifies that the crate builds and tests pass with the minimum supported Rust version

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# MSRV version for openai-ergonomic (should match Cargo.toml and CI)
MSRV=$(sed -n 's/^rust-version = "\([^"]*\)"/\1/p' "$PROJECT_ROOT/Cargo.toml")

echo "Checking MSRV compatibility with Rust $MSRV..."
echo ""

# Check if the required Rust version is installed
if ! rustup toolchain list | grep -q "$MSRV"; then
    echo "Installing Rust $MSRV..."
    rustup toolchain install "$MSRV" --component rustfmt clippy
fi

echo "Using Rust toolchain $MSRV..."
cd "$PROJECT_ROOT"

echo ""
echo "Building with Rust $MSRV..."
cargo +"$MSRV" build --verbose --all-features

echo ""
echo "Running tests with Rust $MSRV..."
cargo +"$MSRV" test --verbose --all-features

echo ""
echo "Running tests with no default features..."
cargo +"$MSRV" test --verbose --no-default-features

echo ""
echo "Building documentation..."
cargo +"$MSRV" doc --no-deps --all-features

echo ""
echo "Building examples..."
cargo +"$MSRV" build --examples --all-features

echo ""
echo "MSRV check passed! openai-ergonomic is compatible with Rust $MSRV"
