# OpenAI Ergonomic – TODO List

> Maintain using***REMOVED***'s plan-first workflow: brainstorm with any***REMOVED*** agent, convert action items here before coding, check off via PRs.

- [ ] Maintenance: Monitor GitHub PR backlog (last checked 2025-09-20; PR #2 merged with openai-client-base analysis).

## Phase 0: Research & Discovery
- [x] Capture audit notes for `openai-client-base` in `docs/research/` (PR #2 merged 2025-09-20).
- [ ] Capture audit notes for `openai-builders` and `langfuse-ergonomic` in `docs/research/` (include module inventory & known TODOs).
- [x] Document takeaway from `openai-experiment/examples/*` to shape ergonomic crate examples (PR #1).

## Examples Implementation (from PR #1 analysis)
### Phase 1: Core Examples (P1 - v0.1.0)
- [ ] Create `examples/` directory structure
- [ ] Implement `responses_comprehensive.rs` - Primary modern API with basic, function calling, web search
- [ ] Implement `responses_streaming.rs` - Dedicated streaming patterns
- [ ] Implement `chat_comprehensive.rs` - Chat completions with history and streaming
- [ ] Implement `assistants_basic.rs` - Assistant API introduction
- [ ] Implement `audio_speech.rs` - Text-to-speech with streaming
- [ ] Implement `audio_transcription.rs` - Speech-to-text and translation
- [ ] Implement `images_comprehensive.rs` - All image operations
- [ ] Implement `embeddings.rs` - Vector embeddings with testing patterns
- [ ] Implement `structured_outputs.rs` - JSON mode and schemas
- [ ] Implement `vision_chat.rs` - Image understanding
- [ ] Implement `quickstart.rs` - 5-minute getting started guide

### Phase 2: Extended Examples (P2)
- [ ] Implement `tool_calling.rs` - Modern tool/function calling with streaming
- [ ] Implement `assistants_code_interpreter.rs` - Code execution capabilities
- [ ] Implement `assistants_file_search.rs` - RAG patterns
- [ ] Implement `vector_stores.rs` - Vector search patterns
- [ ] Implement `moderations.rs` - Content filtering
- [ ] Implement `models.rs` - Model listing and selection
- [ ] Implement `error_handling.rs` - Comprehensive error patterns
- [ ] Implement `retry_patterns.rs` - Resilience strategies
- [ ] Implement `auth_patterns.rs` - Authentication varieties

### Phase 3: Advanced Examples (P3)
- [ ] Implement `batch_processing.rs` - Batch API usage
- [ ] Implement `testing_patterns.rs` - Mock and test strategies
- [ ] Implement `middleware_patterns.rs` - Request/response interceptors
- [ ] Implement `caching_strategies.rs` - Response caching
- [ ] Implement `token_counting.rs` - Token estimation and budgeting

## Repository Setup
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
