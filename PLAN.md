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
- Initialize standalone crate structure (licenses, README, CONTRIBUTING, SECURITY, CHANGELOG templates).
- Configure `Cargo.toml` with metadata, dependencies, feature flags, and lint settings.
- Port shared tooling (`rustfmt.toml`, `deny.toml`, `release-plz.toml`, `renovate.json5`, GitHub workflows).
- Add `CLAUDE.md` explaining project context and workflow expectations.
- Use `langfuse-ergonomic` and `openai-client-base` as templates for `.github/settings.yml`, CODEOWNERS, CI/security/release/dependency workflows, release-plz, renovate, and dual MIT/Apache licensing.

## Phase 2 – Claude & Agent Workflow Enablement
- Define specialized Claude agents (Scaffolder, Docs, QA, Release) with scoped prompts and responsibilities.
- Document plan-first rituals: brainstorm/plan in Claude.ai, translate to Claude Code TODOs before edits.
- Record workflow expectations in `docs/workflow.md` (plan/TODO hygiene, diff reviews, step-by-step prompting).

## Phase 3 – API Surface Design
- Establish module map aligned with OpenAI endpoints (responses, chat, assistants, images, embeddings, audio, files, fine_tuning, batch, vector_stores, moderations, tools).
- Specify builder APIs: when to expose raw `bon::builder` wrappers vs convenience helpers.
- Enumerate constants/type aliases mirroring generated models and enums; plan updates for new schema changes.
- Design ergonomic error handling that wraps `openai-client-base` errors with richer context and validation.

## Phase 4 – Implementation Iterations
- Port builders/helpers from `openai-builders`, adapting for the standalone crate and improving docs/naming.
- Implement convenience functions with sensible defaults for common workflows (Responses-first approach).
- Create client configuration surface wrapping `openai-client-base` (keys, org, base URL, timeouts, tracing hooks).
- Add streaming helpers (SSE chunk parsing) if base client supports them; document usage patterns.
- Wire feature flags for TLS choices, experimental endpoints, optional integrations.

## Phase 5 – Testing & Quality
- Author unit tests per module to validate builder output and helper defaults.
- Add integration tests using `mockito` (or similar) to assert serialized requests match expectations.
- Maintain doctests for quickstart examples; ensure `cargo test --doc` passes.
- Introduce optional smoke tests gated by env vars for real API hits (disabled in CI by default).

## Phase 6 – Examples & Documentation
- Populate `examples/` (responses quickstart, chat migration, assistants with tools, streaming, embeddings batch, moderation, files).
- Expand README with messaging, install instructions, quickstart, feature matrix, migration guide from `openai-builders`.
- Add module-level documentation for builders/helpers; ensure docs.rs builds cleanly.
- Create deeper guides in `docs/` (responses workflow, tool usage, streaming, error handling).

## Phase 7 – CI/CD & Release Engineering
- Configure GitHub Actions (fmt, clippy, test matrix, cargo-deny, doc build, example compilation).
- Set up release-plz for versioning, changelog generation, crates.io publishing workflow.
- Configure Renovate for dependency updates, especially tracking `openai-client-base`.
- Document publish checklist covering dry-run, docs.rs verification, tagging, announcements.

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
- Confirm stakeholder expectations around agent workflow and deliverables.
- Continue Phase 0 research sessions; capture findings in `docs/research/`.
- Implement Phase 1 core examples based on PR #1 analysis.
- Fix API generation issues in `openai-client-base` blocking some examples.
- Monitor GitHub PR backlog; PR #1 reviewed 2025-09-20, examples analysis completed.
