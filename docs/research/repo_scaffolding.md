# Repository Scaffolding Reference

This document captures the ***REMOVED***, governance, and tooling patterns observed in the existing Rust repositories that will inform the `openai-ergonomic` setup.

## langfuse-ergonomic
- Dual licensing via `LICENSE-MIT` and `LICENSE-APACHE`.
- Contributor docs: `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md`, ***REMOVED*** agent guidance.
- Tooling: `rustfmt.toml`, `deny.toml`, `release-plz.toml`, `renovate.json5`.
- GitHub configuration recorded in `.github/settings.yml` (topics, branch protections, required status checks, label set).
- Workflows (`.github/workflows/`):
  - `ci.yml` – multi-OS test matrix (stable/beta/MSRV), build, fmt, clippy, doc build, examples.
  - `dependencies.yml` – `cargo-udeps` unused dependency scan.
  - `release-plz.yml` – automates crate releases (requires `CARGO_REGISTRY_TOKEN`).
  - `security.yml` – cargo audit, cargo deny, secret scanning (trufflehog + gitleaks), CodeQL.
- Additional directories: `.github/codeql` for custom CodeQL config.

## openai-client-base
- Shares dual licensing and contributor docs, adds `PIPELINE.md` and richer `docs/` folder.
- `.github/CODEOWNERS` assigns @timvw as owner for key paths (workflows, scripts, docs).
- Workflows mirror langfuse-ergonomic with additions:
  - `generate-client.yml` – scheduled/manual regeneration of the Stainless-derived client using GitHub App auth and `scripts/generate.sh`.
  - Uses `sccache` in CI to speed builds; coverage not included.
- `renovate.json5`, `release-plz.toml`, `deny.toml`, `rustfmt.toml` present with similar structure.

## langgraph-rs
- Multi-crate workspace with docs/examples/TODO but lighter ***REMOVED***.
- Single `ci.yml` covering build/test/clippy/fmt across OS matrix and separate coverage/doc jobs (tarpaulin + docs build).
- No LICENSE files yet; README emphasises project scope.
- Automation agent docs already exist for workflow guidance.

## openai-experiment (openai-builders)
- Rich README and module organization but **no** GitHub workflows or licensing.
- Serves as the primary source for ergonomic API patterns and example coverage rather than scaffolding templates.

## Takeaways for openai-ergonomic
- Adopt langfuse/openai-client scaffolding wholesale: dual licenses, contributor/security docs, CHANGELOG, release-plz + renovate + deny/rustfmt.
- Reproduce `.github/settings.yml` with branch protections and label sets, plus CODEOWNERS for governance.
- Bring over CI/security/dependency/release workflows adjusting crate names and secrets.
- Decide post-MVP whether to add automated generation jobs (similar to `generate-client.yml`).
- Document these expectations in TODO/plan so contributors know the target ***REMOVED*** surface.
