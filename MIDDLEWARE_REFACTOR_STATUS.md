# Middleware Refactor Status

## Summary
This refactor replaces the interceptor pattern with proper middleware to fix Langfuse integration and add full OpenTelemetry support.

## What's Been Completed
- ✅ Feature branch created: `feature/middleware-refactor`
- ✅ Core middleware trait defined (`src/middleware.rs`)
- ✅ MiddlewareChain and Next pattern implemented
- ✅ Fixed clippy warnings in langfuse_interceptor.rs
- ✅ Documented the issue in LANGFUSE_FIX_NEEDED.md

## What Needs To Be Done

### High Priority (Core Functionality)
1. **Implement LangfuseMiddleware** (`src/middleware/langfuse.rs`)
   - Proper OpenTelemetry context management
   - Root trace creation
   - Child span (observation) with full lifecycle
   - Langfuse-specific attributes
   - Estimated: 200-300 lines

2. **Implement OpenTelemetry Middlewares**
   - `src/middleware/opentelemetry.rs` - GenAI semantic conventions
   - `src/middleware/openai_telemetry.rs` - OpenAI-specific conventions
   - Estimated: 300-400 lines total

3. **Refactor Client** (`src/client.rs`)
   - Remove all interceptor code
   - Add MiddlewareChain field
   - Update all API methods to use middleware
   - Update helper methods
   - Estimated: 500+ lines changed

4. **Update Config** (`src/config.rs`)
   - Remove interceptor support
   - Add middleware support
   - Update builder
   - Estimated: 50-100 lines

### Medium Priority (Usability)
5. **Update Examples**
   - `examples/langfuse.rs` - use LangfuseMiddleware
   - Create `examples/opentelemetry.rs`
   - Update any interceptor examples
   - Estimated: 200-300 lines

6. **Update Tests**
   - Remove interceptor tests
   - Add middleware tests
   - Update integration tests
   - Estimated: 300-500 lines

### Low Priority (Documentation)
7. **Update Documentation**
   - `README.md` - remove interceptor references, add middleware
   - `docs/langfuse-integration.md` - new API
   - Create `docs/middleware-guide.md`
   - Update `docs/architecture.md`
   - Remove `LANGFUSE_FIX_NEEDED.md`
   - Estimated: 500+ lines

8. **PR & CI**
   - Create comprehensive PR description
   - Fix all clippy warnings
   - Ensure all tests pass
   - Fix any CI failures
   - Get approvals

## Estimated Total Effort
- **Lines of Code**: 2000-3000+
- **Files Modified**: 20-25
- **Time Required**: 10-20 hours of focused development
- **Complexity**: High (architectural change affecting entire codebase)

## Recommended Approach

### Option A: Complete It Yourself
Use this status document and MIDDLEWARE_REFACTOR_PLAN.md as a guide:
1. Implement middlewares using reqwest-openai-tracing as reference
2. Refactor Client systematically
3. Update examples one by one
4. Fix tests incrementally
5. Update docs last
6. Create PR and iterate on CI

### Option B: Collaborate
1. I implement core middleware (steps 1-2)
2. You review and test
3. I implement Client refactor (steps 3-4)
4. We iterate on examples/tests/docs together

### Option C: Incremental Migration
1. Keep interceptors, add middleware in parallel
2. Migrate to middleware gradually
3. Deprecate interceptors
4. Remove interceptors in next version

## Next Steps

The feature branch exists with:
- Core middleware trait ✅
- Foundation for implementation ✅
- Clear plan forward ✅

To continue:
```bash
git checkout feature/middleware-refactor
# Implement components from the plan
# Test each component
# Commit incrementally
# Push and create PR when ready
```

## Files Already Created
- `src/middleware.rs` - Core trait and chain
- `MIDDLEWARE_REFACTOR_PLAN.md` - Detailed implementation plan
- `MIDDLEWARE_REFACTOR_STATUS.md` - This file
- `LANGFUSE_FIX_NEEDED.md` - Problem analysis

## Key Reference
- https://github.com/timvw/reqwest-openai-tracing (working Langfuse implementation)
- https://opentelemetry.io/docs/specs/semconv/gen-ai/
- https://opentelemetry.io/docs/specs/semconv/gen-ai/openai/
