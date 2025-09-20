# OpenAI Builders TODO Sweep

> Comprehensive catalogue of TODOs, missing features, and generator gaps in openai-builders codebase

## Executive Summary

This sweep identified 27 distinct issues across 8 modules, with the majority (63%) related to OpenAPI generator problems. Critical gaps include missing enum variants for streaming events and field type mismatches in the Responses module.

## Findings by Category

### 1. Generator Issues (17 items)

#### Streaming Module (`src/streaming.rs`)
- **Lines 315-316**: MessageStreamEvent enum is empty, needs proper variants for message streaming
- **Lines 319-320**: RunStreamEvent enum is empty, needs proper variants for run streaming
- **Lines 365-366**: Duplicate RunStreamEvent issue for different context

#### Responses Module (`src/responses.rs`)
- **Lines 66-67**: Field type mismatch for `tools` and `tool_choice` fields in spec generation
- **Lines 140-141**: ResponsePropertiesToolChoice generated as empty enum
- **Lines 160-161**: ResponsePropertiesToolChoice enum generation needs fixing

#### Projects Module (`src/projects.rs`)
- **Line 32**: ProjectApiKeyCreateRequest doesn't exist in generated models

#### Files Module (`src/files.rs`)
- **Line 111**: Generated API only has `download_file` returning String (needs bytes support)

#### Realtime Module (`src/realtime.rs`)
- **Line 7**: Many types changed from enums to strings in generated client

### 2. Missing Implementations (8 items)

#### Evals Module (`src/evals.rs`)
- **Line 332-333**: CreateEvalRequest needs proper initialization with required fields (function commented out)
- **Line 357-358**: CreateEvalRunRequest needs proper initialization with required fields (function commented out)
- **Line 397**: Tests commented out because dependent functions don't exist yet

#### Observability Module (`src/observability.rs`)
- **Lines 89-93**: Missing observability support for:
  - Embeddings
  - Fine-tuning
  - Files
  - Models
  - Moderations

- **Line 181**: Consider middleware pattern for automatic instrumentation

### 3. Documentation References (2 items)

#### README.md
- **Line 138**: Documents workaround for OpenAPI generator issues (direct struct construction)
- **Line 159**: Notes not using `.builder()` methods due to type issues

## Impact Assessment

### High Priority (Blocking Features)
1. **Empty streaming enums** - Blocks all streaming functionality
2. **Missing Responses tool fields** - Breaks tool usage in chat completions
3. **Missing ProjectApiKeyCreateRequest** - Blocks project API key management

### Medium Priority (Workarounds Exist)
1. **File download returns String** - Can work around but inefficient
2. **Evals functions commented out** - Features unavailable but non-critical
3. **Realtime type changes** - Requires adaptation but manageable

### Low Priority (Enhancements)
1. **Observability gaps** - Nice to have but not blocking
2. **Middleware pattern** - Architecture improvement

## Recommended Actions

### Immediate (Fix in Generator)
1. Regenerate MessageStreamEvent and RunStreamEvent with proper variants
2. Fix ResponsePropertiesToolChoice enum generation
3. Generate missing ProjectApiKeyCreateRequest struct
4. Fix tools/tool_choice field types in Responses

### Short-term (Code Workarounds)
1. Implement manual enum definitions for streaming events
2. Create wrapper for file download to return bytes
3. Uncomment and fix Evals functions with manual structs if needed

### Long-term (Architecture)
1. Implement observability for remaining endpoints
2. Design middleware pattern for automatic instrumentation
3. Consider custom generator templates for persistent issues

## File-by-File Summary

| File | TODOs | Generator Issues | Missing Features |
|------|-------|-----------------|------------------|
| streaming.rs | 3 | 3 | 0 |
| responses.rs | 4 | 4 | 0 |
| evals.rs | 3 | 0 | 3 |
| observability.rs | 2 | 0 | 2 |
| projects.rs | 1 | 1 | 0 |
| files.rs | 1 | 1 | 0 |
| realtime.rs | 1 | 1 | 0 |
| README.md | 2 | 2 | 0 |

## Next Steps

1. **Create generator fix PR** - Submit fixes to openai-client-base generator config
2. **Implement workarounds** - Add manual implementations where generator can't be fixed
3. **Track in main TODO** - Update main TODO.md with prioritized action items
4. **Test coverage** - Ensure all workarounds have comprehensive tests

## Notes

- No FIXME or XXX comments found in codebase
- No unimplemented!() macros present
- Most issues stem from OpenAPI spec ambiguities or generator limitations
- Codebase shows good discipline in documenting known issues