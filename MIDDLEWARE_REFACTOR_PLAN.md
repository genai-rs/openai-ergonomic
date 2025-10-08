# Middleware Refactor Implementation Plan

## Objective
Replace the interceptor pattern with a proper middleware pattern that:
- Supports full request/response lifecycle in one method
- Enables proper OpenTelemetry context management
- Fixes Langfuse integration
- Follows OpenTelemetry semantic conventions for GenAI and OpenAI

## Changes Required

### 1. Core Architecture
- [x] Create feature branch: `feature/middleware-refactor`
- [ ] Create `src/middleware.rs` with `Middleware` trait
- [ ] Implement `MiddlewareChain` and `Next` pattern
- [ ] Remove `src/interceptor.rs`
- [ ] Remove `src/langfuse_interceptor.rs`

### 2. New Middleware Implementations
- [ ] `src/middleware/langfuse.rs` - Proper Langfuse with root traces + observations
- [ ] `src/middleware/opentelemetry.rs` - Generic GenAI semantic conventions
- [ ] `src/middleware/openai_telemetry.rs` - OpenAI-specific conventions
- [ ] `src/middleware/mod.rs` - Module organization

### 3. Client Refactoring
- [ ] Remove all interceptor-related code from `src/client.rs`
- [ ] Add middleware chain support
- [ ] Update all API call methods to execute through middleware
- [ ] Update `Config` to accept middleware instead of interceptors

### 4. Examples Updates
- [ ] Update `examples/langfuse.rs` to use LangfuseMiddleware
- [ ] Create `examples/opentelemetry.rs` for GenAI conventions
- [ ] Update any other examples using interceptors
- [ ] Remove interceptor-specific examples

### 5. Tests Updates
- [ ] Remove all interceptor tests
- [ ] Add middleware tests in `tests/middleware/`
- [ ] Add Langfuse middleware integration tests
- [ ] Add OpenTelemetry middleware tests
- [ ] Ensure all existing tests still pass

### 6. Documentation Updates
- [ ] Update `README.md` - remove interceptor references
- [ ] Update `docs/langfuse-integration.md` - new middleware API
- [ ] Remove `LANGFUSE_FIX_NEEDED.md` (issue will be fixed)
- [ ] Update `docs/architecture.md`
- [ ] Create `docs/middleware-guide.md`

### 7. Dependency Updates
- [ ] Remove any interceptor-specific dependencies
- [ ] Ensure OpenTelemetry dependencies are correct
- [ ] Update `Cargo.toml` if needed

### 8. CI/PR
- [ ] Commit all changes
- [ ] Push feature branch
- [ ] Create PR with detailed description
- [ ] Fix clippy warnings
- [ ] Fix failing tests
- [ ] Ensure CI passes (build, test, clippy, fmt)
- [ ] Get to mergeable state

## Implementation Order

1. **Middleware Core** (foundation)
2. **Langfuse Middleware** (primary fix)
3. **OpenTelemetry Middlewares** (semantic conventions)
4. **Client Refactor** (integrate middleware)
5. **Examples & Tests** (demonstrate usage)
6. **Documentation** (explain changes)
7. **PR & CI** (get it merged)

## Success Criteria
- ✅ Langfuse traces appear correctly in Langfuse Cloud
- ✅ Proper parent-child trace hierarchy
- ✅ OpenTelemetry semantic conventions followed
- ✅ All tests pass
- ✅ All examples work
- ✅ CI green
- ✅ PR mergeable

## Estimated Complexity
- Files to create: ~8-10
- Files to modify: ~15-20
- Lines of code: ~2000-3000
- Time: 2-4 hours of focused work

Let's execute!
