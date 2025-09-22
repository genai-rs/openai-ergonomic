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
1. **Plan First** – Brainstorm with any Claude agent (Claude.ai, Claude Code, etc.), capture actionable steps in `PLAN.md`/`TODO.md` before touching code.
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
- **Headless Operation**:
  - Use `cargo doc` WITHOUT `--open` flag to prevent launching GUI programs
  - Never attempt to open HTML files with system defaults
  - View generated docs by navigating directly to `target/doc/` paths
  - Run all documentation generation in CI-friendly headless mode

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

### 7. Reviewer/Driver Agent
- **Scope**: Monitor PR backlog, perform reviews/merges, keep planning artefacts honest.
- **Inputs**: `gh pr list/view` output, latest `PLAN.md`/`TODO.md`, active worktree summary from teammates.
- **Outputs**: PR reviews/approvals, follow-up TODO items, plan updates noting review checkpoints, nudges when agents forget isolated worktrees.
- **Workflow Notes**:
  - Always confirm you are on a dedicated worktree/branch before committing review artifacts; never reuse the implementation branch under review.
  - Leave review comments when planning hygiene (TODO/PLAN updates) is missing; request artefact updates before approving.
  - When a PR is merge-ready, ensure `PLAN.md`/`TODO.md` reflect the new state before you hit merge.
  - Record review dates in `PLAN.md` or `TODO.md` so future reviewers know the backlog status.

### 8. Agile Coach Agent
- **Scope**: Post-task retrospectives, workflow optimization, agent instruction refinement.
- **Inputs**: Completed task logs, agent execution histories, workflow friction points, recurring issues.
- **Outputs**: Updated agent instructions, refined workflows, process improvements documented in `AGENTS.md`, retrospective notes in `docs/retrospectives/`.
- **Workflow Notes**:
  - Run after significant tasks or sprint-like cycles to assess what worked and what didn't.
  - Analyze patterns in agent failures or inefficiencies (e.g., missed edge cases, unclear instructions).
  - Update agent instructions based on lessons learned to prevent recurring issues.
  - Track improvement metrics: task completion time, rework frequency, instruction clarity scores.
  - Maintain a retrospective log with actionable improvements and their implementation status.
  - Focus on continuous improvement: each review should yield at least one actionable refinement.

## Operating Procedure
1. **Review Context** – Read `PLAN.md`, `TODO.md`, relevant files from reference projects before editing.
2. **Draft Plan** – Using any Claude agent, outline steps, risks, questions. Capture distilled plan in TODO entry if new.
3. **Seek Plan Approval** – When working in Claude Code, present a step-by-step approach; confirm before changes.
4. **Execute Incrementally** – Apply modifications in small chunks, run checks (`cargo fmt`, `cargo clippy`, `cargo test`) where applicable.
5. **Update Artefacts** – Mark TODOs, adjust docs/tests, summarise changes for PR preparation.
6. **Document Decisions** – Record notable choices under `docs/` (design notes, research findings) to keep context persistent.
7. **End-of-Session Review** – Before closing a session, ask Claude Code for a summary plus workflow improvement suggestions; capture outcomes in `TODO.md` or doc updates (including refinements to `CLAUDE.md`/`AGENTS.md`).

## Pre-Commit Checklist
**IMPORTANT**: Always run these checks before committing any code changes:
1. **Format check**: `cargo fmt --all` - Fix any formatting issues
2. **Lint check**: `cargo clippy --all-targets --all-features -- -D warnings` - Fix all clippy warnings
3. **Test check**: `cargo test --all` - Ensure all tests pass
4. **Build check**: `cargo build` - Verify the project compiles
5. **Attribution check**: Ensure NO Claude/AI attribution in commit messages or code

Never assume code is "done" without running these checks. CI will fail if these checks don't pass.

## Critical Policy: No AI Attribution
**ABSOLUTELY NO AI ATTRIBUTION**: Never include any of the following in commits, PRs, or code:
- "Generated with Claude Code" or similar
- "Co-authored-by: Claude" or any AI co-authorship
- References to AI/LLM assistance in comments or documentation
- Any indication that code was AI-generated

All code must appear as human-authored work. This is a hard requirement - violations will result in PR rejection.

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

# Generate documentation (headless, no GUI)
cargo doc --no-deps
# View docs manually at: target/doc/openai_ergonomic/index.html

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
