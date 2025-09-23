# OpenAI Ergonomic – TODO List

> Maintain using Claude's plan-first workflow: brainstorm with any Claude agent, convert action items here before coding, check off via PRs.

- [ ] Maintenance: Monitor GitHub PR backlog (last checked 2025-09-23; Phase 2 examples implemented).

## Phase 0: Research & Discovery
- [x] Capture audit notes for `openai-client-base` in `docs/research/` (PR #2 merged 2025-09-20).
- [x] Capture audit notes for `openai-builders` and `langfuse-ergonomic` in `docs/research/` (completed 2025-09-21).
- [x] Document takeaway from `openai-experiment/examples/*` to shape ergonomic crate examples (PR #1).

## Examples Implementation (from PR #1 analysis)
### Phase 1: Core Examples (P1 - v0.1.0)
- [x] Create `examples/` directory structure (completed 2025-09-21)
- [x] Implement `quickstart.rs` - Basic getting started guide (completed 2025-09-21)
- [x] Implement `responses_comprehensive.rs` - Primary modern API with basic, function calling, web search (PR #23 completed 2025-09-22)
- [x] Implement `responses_streaming.rs` - Dedicated streaming patterns (PR #24 completed 2025-09-22)
- [x] Implement `chat_comprehensive.rs` - Chat completions with history and streaming (completed 2025-09-22)
- [x] Implement `assistants_basic.rs` - Assistant API introduction (completed 2025-09-22)
- [x] Implement `audio_speech.rs` - Text-to-speech with streaming (completed 2025-09-22)
- [x] Implement `audio_transcription.rs` - Speech-to-text and translation (completed 2025-09-22)
- [x] Implement `images_comprehensive.rs` - All image operations (completed 2025-09-22)
- [x] Implement `embeddings.rs` - Vector embeddings with testing patterns (completed 2025-09-22)
- [x] Implement `structured_outputs.rs` - JSON mode and schemas (completed 2025-09-22)
- [x] Implement `vision_chat.rs` - Image understanding (completed 2025-09-22)

### Phase 2: Extended Examples (P2) - COMPLETED
- [x] Implement `tool_calling.rs` - Modern tool/function calling with streaming (completed 2025-09-23)
- [ ] Implement `assistants_code_interpreter.rs` - Code execution capabilities (pending new builders)
- [ ] Implement `assistants_file_search.rs` - RAG patterns (pending new builders)
- [ ] Implement `vector_stores.rs` - Vector search patterns (pending new builders)
- [x] Implement `moderations.rs` - Content filtering (completed 2025-09-23)
- [x] Implement `models.rs` - Model listing and selection (completed 2025-09-23)
- [x] Implement `error_handling.rs` - Comprehensive error patterns (completed 2025-09-23)
- [x] Implement `retry_patterns.rs` - Resilience strategies (completed 2025-09-23)
- [x] Implement `auth_patterns.rs` - Authentication varieties (completed 2025-09-23)

### Phase 3: Advanced Examples (P3) - PENDING
- [ ] Implement `batch_processing.rs` - Batch API usage
- [ ] Implement `testing_patterns.rs` - Mock and test strategies
- [ ] Implement `middleware_patterns.rs` - Request/response interceptors
- [ ] Implement `caching_strategies.rs` - Response caching
- [ ] Implement `token_counting.rs` - Token estimation and budgeting

## Repository Setup
- [x] Phase 1: Scaffold repository (licenses, README stub, CONTRIBUTING, SECURITY, CHANGELOG templates) using `langfuse-ergonomic` / `openai-client-base` patterns (completed).
- [x] Phase 1: Port shared tooling (`rustfmt.toml`, `deny.toml`, `release-plz.toml`, `renovate.json5`, `.github/settings.yml`, CODEOWNERS, CI/security/release/dependency workflows) (completed).
- [x] Phase 1: Author initial `CLAUDE.md` describing agent expectations and workflow rituals (AGENTS.md created).
- [x] Phase 2: Define specialized Claude agents and prompt templates; documented in AGENTS.md (completed 2025-09-21).
- [ ] Phase 3: Produce API surface design spec (module map, builder naming, constants) under `docs/design/api_surface.md`.
- [x] Phase 4: Implement core Responses API builders/helpers (initial implementation completed 2025-09-21).
- [x] Phase 4: Implement client configuration wrapper over `openai-client-base` with error handling (initial implementation completed 2025-09-21).
- [x] Phase 5: Establish testing harness (unit, integration, doctest, smoke toggle) (PR #27 completed 2025-09-22).
- [x] Phase 6: Draft README quickstart + initial examples (completed with Phase 1 examples).
- [x] Phase 7: Configure CI/CD (fmt, clippy, test matrix, docs, cargo-deny) and release-plz workflow (PR #10, #16, #28 completed).
- [ ] Phase 8: Write contributor onboarding & operational playbook (`docs/workflow.md`, `CONTRIBUTING.md`).
- [ ] Phase 9: Define post-launch roadmap (coverage tooling, macros, integrations) in `docs/roadmap.md`.
