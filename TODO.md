# OpenAI Ergonomic – TODO List

> Maintain using***REMOVED***'s plan-first workflow: brainstorm with any***REMOVED*** agent, convert action items here before coding, check off via PRs.

- [ ] Phase 0: Capture audit notes for `openai-builders`, `langfuse-ergonomic`, and `openai-client-base` in `docs/research/` (include module inventory & known TODOs).
- [ ] Phase 0: Document takeaway from `openai-experiment/examples/*` to shape ergonomic crate examples.
- [ ] Phase 1: Scaffold repository (licenses, README stub, CONTRIBUTING, SECURITY, CHANGELOG templates) using `langfuse-ergonomic` / `openai-client-base` patterns.
- [ ] Phase 1: Port shared tooling (`rustfmt.toml`, `deny.toml`, `release-plz.toml`, `renovate.json5`, `.github/settings.yml`, CODEOWNERS, CI/security/release/dependency workflows).
- [ ] Phase 1: Author initial `CLAUDE.md` describing agent expectations and workflow rituals.
- [ ] Phase 2: Define specialized***REMOVED*** agents and prompt templates; document in `docs/workflow.md`.
- [ ] Phase 3: Produce API surface design spec (module map, builder naming, constants) under `docs/design/api_surface.md`.
- [ ] Phase 4: Implement core Responses API builders/helpers with tests.
- [ ] Phase 4: Implement client configuration wrapper over `openai-client-base` with error handling.
- [ ] Phase 5: Establish testing harness (unit, integration, doctest, smoke toggle).
- [ ] Phase 6: Draft README quickstart + initial examples (`examples/responses_quickstart.rs`, etc.).
- [ ] Phase 7: Configure CI/CD (fmt, clippy, test matrix, docs, cargo-deny) and release-plz workflow.
- [ ] Phase 8: Write contributor onboarding & operational playbook (`docs/workflow.md`, `CONTRIBUTING.md`).
- [ ] Phase 9: Define post-launch roadmap (coverage tooling, macros, integrations) in `docs/roadmap.md`.
