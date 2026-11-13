#!/usr/bin/env bash
# Local equivalents of this repo's own heavy CI jobs, run by
# genai-rs/.github scripts/local-ci.sh after the shared checks.
set -euo pipefail

# ci.yml `build` matrix: release build for the host target only (CI builds
# every target on main).
cargo +stable build --release --all-features
