# Example Porting Decisions

## Decision Matrix

| Original Example | Decision | New Example | Priority | Notes |
|-----------------|----------|-------------|----------|--------|
| **Responses API** |
| responses | MERGE | responses_comprehensive.rs | P1 | Core modern API |
| responses-basic | MERGE | responses_comprehensive.rs | P1 | Combine with main |
| responses-function-call | MERGE | responses_comprehensive.rs | P1 | Include function calling |
| responses-stream | PORT | responses_streaming.rs | P1 | Dedicated streaming |
| responses-web-search | MERGE | responses_comprehensive.rs | P2 | Advanced feature |
| **Chat API** |
| chat | MERGE | chat_comprehensive.rs | P1 | Legacy but needed |
| chat-store | MERGE | chat_comprehensive.rs | P1 | History management |
| chat-stream | MERGE | chat_comprehensive.rs | P1 | Streaming variant |
| **Assistants API** |
| assistants | PORT | assistants_basic.rs | P1 | Introduction example |
| assistants-code-interpreter | PORT | assistants_code_interpreter.rs | P2 | Distinct capability |
| assistants-file-search | PORT | assistants_file_search.rs | P2 | RAG patterns |
| assistants-func-call-stream | PORT | assistants_streaming.rs | P2 | Streaming functions |
| **Audio API** |
| audio-speech | MERGE | audio_speech.rs | P1 | TTS patterns |
| audio-speech-stream | MERGE | audio_speech.rs | P1 | Include streaming |
| audio-transcribe | MERGE | audio_transcription.rs | P1 | STT patterns |
| audio-translate | MERGE | audio_transcription.rs | P1 | Translation variant |
| **Images API** |
| create-image | MERGE | images_comprehensive.rs | P1 | All image ops |
| create-image-b64-json | MERGE | images_comprehensive.rs | P1 | Output format variant |
| create-image-edit | MERGE | images_comprehensive.rs | P1 | Editing capability |
| create-image-variation | MERGE | images_comprehensive.rs | P1 | Variation generation |
| images-edit-variation | DROP | - | - | Duplicate |
| **Embeddings** |
| embeddings | MERGE | embeddings.rs | P1 | Vector generation |
| embeddings-test | MERGE | embeddings.rs | P1 | Include test patterns |
| **Function/Tool Calling** |
| function-call | DROP | - | - | Legacy API |
| function-call-stream | DROP | - | - | Legacy API |
| tool-call | PORT | tool_calling.rs | P2 | Modern approach |
| tool-call-stream | MERGE | tool_calling.rs | P2 | Include streaming |
| **Structured Outputs** |
| structured-outputs | MERGE | structured_outputs.rs | P1 | JSON mode |
| structured-outputs-schemars | MERGE | structured_outputs.rs | P1 | Schema integration |
| **Completions** |
| completions | DROP | - | - | Deprecated API |
| completions-stream | DROP | - | - | Deprecated API |
| completions-web-search | DROP | - | - | Deprecated API |
| **Other Core** |
| moderations | PORT | moderations.rs | P2 | Content filtering |
| models | PORT | models.rs | P2 | Model listing |
| vision-chat | PORT | vision_chat.rs | P1 | Image understanding |
| vector-store-retrieval | PORT | vector_stores.rs | P2 | Search patterns |
| **Realtime** |
| realtime | DEFER | - | P4 | API not ready |
| realtime-chat | DEFER | - | P4 | API not ready |
| **Platform/Integration** |
| azure-openai-service | DROP | - | - | Platform-specific |
| gemini-openai-compatibility | DROP | - | - | Third-party |
| ollama-chat | DROP | - | - | Third-party |
| observability-langfuse | DROP | - | - | Separate concern |
| **Utilities** |
| middleware-demo | ADAPT | middleware_patterns.rs | P3 | Ergonomic patterns |
| mock-mode | ADAPT | testing_patterns.rs | P3 | Test strategies |
| streaming-helpers-demo | MERGE | responses_streaming.rs | P1 | Core streaming |
| model-helpers | DROP | - | - | Built into crate |
| bring-your-own-type | ADAPT | custom_types.rs | P3 | Advanced usage |
| in-memory-file | MERGE | file_handling.rs | P3 | File utilities |

## Priority Definitions

- **P1 (Must Have)**: Core functionality examples needed for v0.1.0
- **P2 (Should Have)**: Important features for complete coverage
- **P3 (Nice to Have)**: Advanced patterns and utilities
- **P4 (Future)**: Blocked on external dependencies or API readiness

## Consolidation Benefits

### Reduced Example Count
- From 49 examples to ~25 comprehensive examples
- Easier maintenance and documentation
- Clearer learning path for users

### Improved Organization
- Logical grouping by API feature
- Progressive complexity within examples
- Consistent naming and structure

### Better Coverage
- Fill gaps in authentication, error handling, retry patterns
- Add production-ready patterns
- Include testing strategies

## Implementation Order

1. **Phase 1** (11 examples): Core P1 examples for initial release
2. **Phase 2** (9 examples): P2 examples for feature completeness
3. **Phase 3** (5 examples): P3 advanced patterns
4. **Phase 4** (TBD): P4 when dependencies ready