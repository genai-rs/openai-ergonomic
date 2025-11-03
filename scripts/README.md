# Development Scripts

This directory contains utility scripts to help with development, testing, and release processes for the `openai-ergonomic` crate.

## Scripts Overview

### `check-msrv.sh`
**Purpose**: Verify Minimum Supported Rust Version (MSRV) compatibility

This script:
- Installs the specified MSRV (currently Rust 1.83) if not already available
- Switches to the MSRV toolchain temporarily
- Builds the project with all features
- Runs the full test suite
- Tests with no default features
- Builds documentation and examples
- Verifies that the crate works with the minimum supported Rust version

**Usage**:
```bash
./scripts/check-msrv.sh
```

**When to use**: Before releases, when updating dependencies, or when making changes that might affect MSRV compatibility.

### `install-hooks.sh`
**Purpose**: Set up Git hooks for automated code quality checks

This script:
- Configures Git to use the project's custom hooks from `.githooks/`
- Enables pre-commit hooks that run formatting, linting, build, and tests

**Usage**:
```bash
./scripts/install-hooks.sh
```

**When to use**: When setting up a new development environment or after cloning the repository.

**Note**: You can bypass hooks temporarily with `git commit --no-verify` if needed.

### `pre-release.sh`
**Purpose**: Comprehensive pre-release validation

This script performs extensive checks before a release:
- Verifies you're on the `main` branch
- Ensures working directory is clean
- Checks that local branch is up-to-date with remote
- Runs code formatting checks
- Executes Clippy linting
- Builds the project with all features
- Runs the complete test suite (with and without default features)
- Builds documentation
- Builds all examples
- Scans for TODO/FIXME comments that might need attention

**Usage**:
```bash
./scripts/pre-release.sh
```

**When to use**: Before creating a release PR or publishing a new version to ensure everything is ready.

## Script Patterns

All scripts follow these conventions:
- Use `set -euo pipefail` for robust error handling
- Provide clear progress indicators with emoji and descriptive messages
- Exit with non-zero status codes on failures
- Include helpful next-steps information
- Are adapted from patterns established in the `langfuse-ergonomic` project

## Integration with CI/CD

These scripts complement the automated CI/CD pipeline:
- The checks performed locally mirror those in GitHub Actions
- MSRV testing aligns with the CI matrix testing strategy
- Pre-release validation ensures releases are stable

## Contributing

When adding new scripts:
1. Follow the existing naming convention (`action-target.sh`)
2. Make scripts executable (`chmod +x`)
3. Use consistent error handling and user feedback patterns
4. Update this README to document the new script's purpose and usage
