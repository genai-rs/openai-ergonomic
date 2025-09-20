# Claude Agent Instructions – openai-ergonomic

> This document is the comprehensive guide for LLM/GenAI agents collaborating on the repository.

## Project Overview
- **Repository**: `/Users/tim.van.wassenhove/src/github/openai-ergonomic`
- **Goal**: Ship an ergonomic Rust wrapper around `openai-client-base`, inspired by `openai-builders` and the `langfuse-ergonomic` crate.
- **Status**: Greenfield setup. Only planning artefacts exist (`PLAN.md`, `TODO.md`).
- **Source Dependencies**:
  - `openai-client-base` (generated API client)
  - `openai-experiment/crates/openai-builders` (reference ergonomics)
  - `langfuse-ergonomic` (scaffolding + workflow inspiration)

## Guiding Principles
1. **Plan First** – Brainstorm in Claude.ai, capture actionable steps in `PLAN.md`/`TODO.md` before touching code.
2. **Small Diffs** – Execute in Claude Code using tight loops: plan → apply → review → update TODO.
3. **Truthful State** – Only describe features that exist. Keep docs/tests aligned with code.
4. **Parity with OpenAI** – Prioritise coverage of current OpenAI endpoints, Responses-first ergonomics, and helper consistency.
5. **Publishing Ready** – Every change nudges crate toward crates.io readiness (docs, tests, CI, release automation).
6. **Parallel-Friendly** – When running multiple efforts concurrently, isolate work per git tree/branch so each Claude Code instance retains context without collisions.

## Key References
- `PLAN.md` – Long-form roadmap across phases.
- `TODO.md` – Current actionable checklist tracked alongside Claude sessions.
- `docs/research/repo_scaffolding.md` – Findings on existing automation/tooling patterns.
- `docs/research/openai_builders.md` – Audit of builders modules, known TODOs, and example coverage.
- `openai-experiment/crates/openai-builders` – API ergonomics reference.
- `langfuse-ergonomic` – Repo scaffolding, CI, CONTRIBUTING patterns.

## Agent Roster

### 1. Scaffolder Agent
- **Scope**: Repository bootstrapping, config files, tooling, workspace layout.
- **Inputs**: Target TODO entry, desired file tree, relevant snippets from `langfuse-ergonomic`.
- **Outputs**: Minimal compiling scaffolds, annotated diffs, follow-up TODO updates.

### 2. API Designer Agent
- **Scope**: Builder APIs, helper functions, constants, error layers.
- **Inputs**: `docs/design/api_surface.md` (once created), schemas from `openai-client-base`, existing module code.
- **Outputs**: Implemented builders/helpers with inline docs, adherence to naming conventions, unit tests.

### 3. Docs Agent
- **Scope**: README, docs.rs comments, `docs/` guides, examples.
- **Inputs**: Feature notes, diffs from implementation agents, documentation gaps flagged in TODO.
- **Outputs**: Updated README sections, new examples, expanded documentation, doctest coverage.

### 4. QA Agent
- **Scope**: Testing harness, regression coverage, CI guardrails.
- **Inputs**: Modules under test, expected behaviours, mock server requirements, feature flags.
- **Outputs**: Unit/integration tests, doctests, smoke-test toggles, CI workflow adjustments.

### 5. Release Agent
- **Scope**: CI/CD pipelines, release-plz config, dependency hygiene.
- **Inputs**: `release-plz.toml`, GitHub workflow templates, dependency constraints.
- **Outputs**: Actions workflows, publish checklist, dependency update guidance, Renovate rules.

### 6. Support Agent
- **Scope**: Issue triage, contributor onboarding, roadmap grooming.
- **Inputs**: Incoming issues/PRs, roadmap drafts, contributor feedback.
- **Outputs**: Updated `docs/roadmap.md`, clarified `CONTRIBUTING.md`, curated TODO backlog.

## Operating Procedure
1. **Review Context** – Read `PLAN.md`, `TODO.md`, relevant files from reference projects before editing.
2. **Draft Plan in Claude.ai** – Outline steps, risks, questions. Paste distilled plan into TODO entry if new.
3. **Seek Plan Approval** – In Claude Code, request a step-by-step approach; confirm before changes.
4. **Execute Incrementally** – Apply modifications in small chunks, run checks (`cargo fmt`, `cargo clippy`, `cargo test`) where applicable.
5. **Update Artefacts** – Mark TODOs, adjust docs/tests, summarise changes for PR preparation.
6. **Document Decisions** – Record notable choices under `docs/` (design notes, research findings) to keep context persistent.
7. **End-of-Session Review** – Before closing a session, ask Claude Code for a summary plus workflow improvement suggestions; capture outcomes in `TODO.md` or doc updates (including refinements to `CLAUDE.md`/`AGENTS.md`).

## Parallel Session Management
- Spin up separate Claude Code instances per long-running task (or per repository clone/worktree) to preserve context between sessions.
- Use dedicated git trees (e.g., `git worktree add ../openai-ergonomic-taskA ...`) or distinct clones so concurrent agents can commit independently.
- Synchronise frequently: merge or rebase when switching contexts to avoid drift across parallel branches.
- Note active sessions and their locations in `TODO.md` to make handoffs explicit.
- When re-opening a parked instance, skim the last end-of-session summary to restore momentum instantly.

## Quick Commands
```bash
# Format & lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Run tests (once implemented)
cargo test --all

# Run a specific example (placeholder)
cargo run --example responses_quickstart
```

## Change Review Checklist
- [ ] Plan/TODO entry created or updated before work.
- [ ] Tests/docs added or adjusted as needed.
- [ ] `cargo fmt` and `cargo clippy` clean.
- [ ] README/examples remain accurate.
- [ ] Summary prepared for eventual PR (follow Conventional Commits when committing).

## Contact & Ownership
- **Maintainer**: Tim Van Wassenhove (`@timvw`, github@timvw.be)
- **Related Repos**: `openai-client-base`, `openai-experiment`, `langfuse-ergonomic` (all under `/Users/tim.van.wassenhove/src/github`).

Keep this document updated as workflows evolve. When new recurring tasks emerge, add/adjust agent roles rather than overloading existing ones.
