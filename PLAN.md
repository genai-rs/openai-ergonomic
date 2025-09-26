# OpenAI Ergonomic Crate – Project Plan

## Vision & Goals
- Deliver an ergonomic Rust wrapper around `openai-client-base`, mirroring the developer experience provided by `langfuse-ergonomic`.
- Provide comprehensive coverage of current OpenAI endpoints with type-safe builders, helpers, and sensible defaults.
- Ship a production-ready crate with documentation, examples, tests, and CI/CD tooling ready for crates.io publication.

## Phase 0 – Research & Discovery
- Audit `openai-experiment/crates/openai-builders` for module layout, builder patterns, naming, and gaps.
- Review `langfuse-ergonomic` for repository scaffolding (tooling, docs, CI, CLAUDE workflow).
- Inspect `openai-client-base` capabilities (auth, streaming, feature flags) to understand integration touchpoints.
- Inventory licensing, release tooling, and process assets we can reuse.
- ✅ **COMPLETE**: Catalogue existing builder implementations (e.g., responses/chat/assistants modules) and note outstanding TODOs/workarounds in `openai-builders`.
  - See `docs/research/openai_builders_todo_sweep.md` for comprehensive findings (27 issues catalogued)
  - See `docs/research/openai_builders_tasks.md` for actionable task breakdown
  - Critical blockers identified: empty streaming enums, Response field type mismatches
- ✅ Review `openai-experiment/examples/*` to understand coverage expectations (responses, streaming, assistants, audio, etc.) and plan equivalent ergonomic examples. **COMPLETED 2025-09-20** – See PR #1 with comprehensive analysis in `docs/research/`: mapped 49 examples to ~25 consolidated examples across 3 implementation phases.

## Phase 1 – Repository Bootstrap
- ✅ **COMPLETE**: Initialize standalone crate structure (licenses, README, CONTRIBUTING, SECURITY, CHANGELOG templates).
- ✅ **COMPLETE**: Configure `Cargo.toml` with metadata, dependencies, feature flags, and lint settings.
- ✅ **COMPLETE**: Port shared tooling (`rustfmt.toml`, `deny.toml`, `release-plz.toml`, `renovate.json5`, GitHub workflows).
- ✅ **COMPLETE**: Add `CLAUDE.md` explaining project context and workflow expectations (created as AGENTS.md).
- ✅ **COMPLETE**: Use `langfuse-ergonomic` and `openai-client-base` as templates for `.github/settings.yml`, CODEOWNERS, CI/security/release/dependency workflows, release-plz, renovate, and dual MIT/Apache licensing.

## Phase 2 – Claude & Agent Workflow Enablement
- ✅ **COMPLETE**: Define specialized Claude agents (Scaffolder, API Designer, Docs, QA, Release, Support, Reviewer/Driver, Agile Coach) with scoped prompts and responsibilities in AGENTS.md.
- ✅ **COMPLETE**: Document plan-first rituals: brainstorm/plan in Claude.ai, translate to Claude Code TODOs before edits.
- ✅ **COMPLETE**: Record workflow expectations in `docs/workflow.md` (expanded with publish checklist references).

## Phase 3 – API Surface Design
- Establish module map aligned with OpenAI endpoints (responses, chat, assistants, images, embeddings, audio, files, fine_tuning, batch, vector_stores, moderations, tools).
- Specify builder APIs: when to expose raw `bon::builder` wrappers vs convenience helpers.
- Enumerate constants/type aliases mirroring generated models and enums; plan updates for new schema changes.
- Design ergonomic error handling that wraps `openai-client-base` errors with richer context and validation.

## Phase 4 – Implementation Iterations
- ✅ **COMPLETE**: Port builders/helpers from `openai-builders`, adapting for the standalone crate and improving docs/naming (PR #20).
- ✅ **COMPLETE**: Implement convenience functions with sensible defaults for common workflows (Responses-first approach).
- ✅ **COMPLETE**: Create client configuration surface wrapping `openai-client-base` (keys, org, base URL, timeouts, tracing hooks).
- ✅ **COMPLETE**: Add streaming helpers (SSE chunk parsing) if base client supports them; document usage patterns.
- ✅ **COMPLETE**: Wire feature flags for TLS choices, experimental endpoints, optional integrations.

## Phase 5 – Testing & Quality
- ✅ **COMPLETE**: Author unit tests per module to validate builder output and helper defaults (PR #27).
- ✅ **COMPLETE**: Add integration tests using `mockito` to assert serialized requests match expectations.
- ✅ **COMPLETE**: Maintain doctests for quickstart examples; ensure `cargo test --doc` passes.
- ✅ **COMPLETE**: Introduce optional smoke tests gated by env vars for real API hits (disabled in CI by default).

## Phase 6 – Examples & Documentation
- ✅ **COMPLETE**: Populate `examples/` - Phase 1-3 complete (26 curated examples maintained).
- ✅ **COMPLETE**: Expand README with messaging, install instructions, quickstart, feature matrix.
- ✅ **COMPLETE**: Add module-level documentation for builders/helpers; ensure docs.rs builds cleanly.
- ✅ **COMPLETE**: Create deeper guides in `docs/` (responses-first workflows, tool orchestration, vector stores, migration guide).

## Phase 7 – CI/CD & Release Engineering
- ✅ **COMPLETE**: Configure GitHub Actions (fmt, clippy, test matrix, cargo-deny, doc build, example compilation) (PRs #10, #16, #28).
- ✅ **COMPLETE**: Set up release-plz for versioning, changelog generation, crates.io publishing workflow.
- ✅ **COMPLETE**: Configure Renovate for dependency updates, especially tracking `openai-client-base`.
- ✅ **COMPLETE**: Integrate Codecov for coverage reporting (PR #28).
- ✅ **COMPLETE**: Document publish checklist covering dry-run, docs.rs verification, tagging, announcements (`docs/workflow/publish_checklist.md`).

## Phase 8 – Operational Playbook
- Document onboarding (plan-first workflow, agent usage, TODO maintenance).
- Define contribution guidelines tying PRs to Claude plans/TODO updates.
- Maintain `TODO.md` (or equivalent project board) as source of truth for in-flight tasks; review weekly.
- Schedule API spec sync cadence to regenerate `openai-client-base` and queue ergonomic updates.

## Phase 9 – Roadmap Extensions
- Short-term: parity with existing OpenAI endpoints, polished helpers, `v0.1.0` release.
- Mid-term: coverage reporting, procedural macros to reduce boilerplate, CLI sample app.
- Long-term: optional integrations (tracing middleware, retries), community contribution pathways, auxiliary crates if scope expands.

## Immediate Next Steps
- ✅ Implemented remaining builder modules (embeddings, threads/uploads) to close API coverage gaps.
- ✅ Authored deep-dive documentation (responses-first workflow, tool orchestration, vector store RAG playbook, migration guide from `openai-builders`).
- ✅ Ran the publish checklist dry run for the upcoming v0.3.0 release candidate and captured outcomes.
- ✅ Kept examples aligned with new builder capabilities; added docs.rs snippets for new builders.
- Monitor GitHub PR backlog; checked 2025-09-26 — Closed stale PRs #18 (release v0.2.0) and #40 (embeddings helpers - already integrated into main).
