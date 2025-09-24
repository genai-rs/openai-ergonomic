# Development Workflow Guide

This document provides detailed guidance on the development workflow for `openai-ergonomic`, including our agent-driven development approach, git practices, and collaboration patterns.

## Overview

The `openai-ergonomic` project uses a unique **agent-driven development** approach where specialized AI agents handle different aspects of development. This workflow emphasizes planning first, small incremental changes, and maintaining comprehensive documentation throughout the development process.

## Core Principles

### 1. Plan First
- **Always plan before coding**: For any significant change, create or update entries in `PLAN.md` and `TODO.md` before writing code
- **Use any***REMOVED*** agent for brainstorming**: Draft ideas in***REMOVED***.ai, then translate actionable steps to***REMOVED*** TODOs
- **Document decisions**: Record notable choices in `docs/` for persistent context

### 2. Small Diffs
- **Execute in tight loops**: plan → apply → review → update TODO
- **Make focused commits**: Each commit should have a clear, single purpose
- **Incremental progress**: Build features step by step rather than in large chunks

### 3. Truthful State
- **Only describe existing features**: Keep docs/tests aligned with actual code
- **Update artifacts continuously**: Maintain `PLAN.md` and `TODO.md` as work progresses
- **Honest status reporting**: Mark TODOs as completed only when fully implemented

### 4. Publishing Ready
- **Every change counts**: Each change should nudge the crate toward crates.io readiness
- **Maintain quality**: All changes must pass pre-commit checks
- **Keep examples working**: Ensure examples compile and demonstrate real functionality

## Agent-Driven Development

### Understanding Agent Roles

We have specialized agents for different development aspects (see [AGENTS.md](../AGENTS.md) for complete details):

1. **Scaffolder Agent**: Repository setup, config files, tooling
2. **API Designer Agent**: Builder APIs, helper functions, constants
3. **Docs Agent**: README, docs.rs comments, guides, examples
4. **QA Agent**: Testing harness, regression coverage, CI
5. **Release Agent**: CI/CD pipelines, release ***REMOVED***
6. **Support Agent**: Issue triage, contributor onboarding, roadmap maintenance
7. **Reviewer/Driver Agent**: PR monitoring, reviews, planning hygiene
8. **Agile Coach Agent**: Retrospectives, workflow optimization

### Choosing the Right Agent Approach

When contributing, consider which agent role best fits your work:

- **Adding new API endpoints**: Follow API Designer Agent guidelines
- **Writing documentation**: Use Docs Agent patterns
- **Fixing tests or CI**: Apply QA Agent approaches
- **Repository improvements**: Use Scaffolder Agent methods

### Agent Workflow Pattern

1. **Review Context**: Read `PLAN.md`, `TODO.md`, and relevant files before editing
2. **Draft Plan**: Outline steps, risks, and questions
3. **Seek Plan Approval**: When working in***REMOVED***, present approach and confirm before changes
4. **Execute Incrementally**: Apply modifications in small chunks with frequent checks
5. **Update Artifacts**: Mark TODOs, adjust docs/tests, summarize changes
6. **Document Decisions**: Record choices in appropriate `docs/` files

## Git Workflow

### Branch Management

#### Main Branch Development
```bash
# Basic development flow
git checkout main
git pull origin main
git checkout -b feature/your-feature-name

# Make changes following agent guidelines
# ... implement feature incrementally ...

# Pre-commit checks (mandatory)
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build

# Commit with conventional commit format
git add .
git commit -m "feat(scope): add new feature description"

# Push and create PR
git push origin feature/your-feature-name
```

#### Parallel Session Management

For complex or long-running tasks, use separate git worktrees to maintain context:

```bash
# Create dedicated worktree for task
git worktree add ../openai-ergonomic-taskA feature/task-a

# Work in dedicated directory
cd ../openai-ergonomic-taskA

# Each***REMOVED*** instance gets its own worktree
# This preserves context between sessions
```

**Benefits of worktrees:**
- Preserve***REMOVED*** context between sessions
- Enable parallel development without conflicts
- Allow concurrent agents to work independently
- Maintain separate development environments

### Commit Standards

#### Conventional Commits Format
```
type(scope): description

[optional body]

[optional footer]
```

**Types:**
- `feat`: New features
- `fix`: Bug fixes
- `docs`: Documentation changes
- `style`: Formatting changes
- `refactor`: Code refactoring
- `test`: Test additions or changes
- `chore`: Maintenance tasks

**Examples:**
```bash
git commit -m "feat(responses): add streaming support for chat completions"
git commit -m "fix(auth): handle expired API keys gracefully"
git commit -m "docs(readme): add quickstart example"
git commit -m "test(integration): add mock server tests"
```

#### AI Attribution Policy

**CRITICAL**: Never include AI attribution in commits:
- ❌ "Generated with***REMOVED***"
- ❌ "Co-authored-by:***REMOVED***"
- ❌ References to AI assistance in comments
- ❌ Any indication of AI generation

All code must appear as human-authored work.

## Pre-Commit Checklist

**MANDATORY** - Run these checks before every commit:

```bash
# 1. Format check - must pass
cargo fmt --all

# 2. Lint check - must pass with no warnings
cargo clippy --all-targets --all-features -- -D warnings

# 3. Test check - all tests must pass
cargo test --all

# 4. Build check - project must compile
cargo build

# 5. Attribution check - ensure no AI references
```

**Never assume code is "done" without running these checks. CI will fail if they don't pass.**

## Planning Artifact Management

### TODO.md Maintenance

The `TODO.md` file is the source of truth for current work:

```markdown
## Current Sprint
- [ ] Task description with clear acceptance criteria
- [x] Completed task with implementation notes
- [ ] Next task ready for pickup

## Backlog
- [ ] Future tasks organized by priority
- [ ] Research tasks for investigation
```

**Best Practices:**
- Update TODO status in real-time as you work
- Mark tasks complete immediately after finishing
- Add new tasks discovered during implementation
- Remove tasks that are no longer relevant

### PLAN.md Updates

Keep the long-term plan current:
- Update phase completion status
- Add new discoveries or blockers
- Record decision rationale
- Note review checkpoints

## Development Environment Setup

### Local Development

```bash
# Clone and setup
git clone https://github.com/your-username/openai-ergonomic.git
cd openai-ergonomic

# Install dependencies
cargo build

# Verify setup
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

### Environment Variables

```bash
# Copy example environment file
cp .env.example .env

# Edit with your API keys (for smoke tests)
OPENAI_API_KEY=your_key_here
```

### IDE Setup

Recommended tools:
- **rust-analyzer**: LSP support
- **rustfmt**: Code formatting
- **clippy**: Linting
- **cargo-watch**: Continuous testing

## Testing Strategy

### Test Categories

1. **Unit Tests**: Test individual functions and builders
2. **Integration Tests**: Test complete workflows with mocks
3. **Documentation Tests**: Verify examples in docs work
4. **Smoke Tests**: Optional real API tests (disabled in CI)

### Running Tests

```bash
# All tests
cargo test --all

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test integration

# Documentation tests
cargo test --doc

# With coverage
cargo test --all -- --nocapture
```

### Mock Testing

Use `mockito` for HTTP mocking:

```rust
use mockito::{Mock, Server};

#[tokio::test]
async fn test_api_call() {
    let mut server = Server::new();
    let mock = server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"choices": [{"message": {"content": "Hello!"}}]}"#)
        .create();

    // Test implementation
    mock.assert();
}
```

## Documentation Standards

### Module Documentation

```rust
//! Brief module description.
//!
//! Detailed explanation of the module's purpose, common usage patterns,
//! and any important considerations.
//!
//! # Examples
//!
//! ```rust
//! use openai_ergonomic::responses::ResponsesBuilder;
//!
//! let response = ResponsesBuilder::new()
//!     .model("gpt-4")
//!     .prompt("Hello world")
//!     .build()?;
//! ```

/// Brief function description.
///
/// Detailed explanation of the function's behavior, parameters,
/// and return values.
///
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// When this function returns an error
///
/// # Examples
///
/// ```rust
/// let result = function_name(param)?;
/// ```
pub fn function_name(param: Type) -> Result<Type, Error> {
    // Implementation
}
```

### Documentation Generation

```bash
# Generate docs (headless, no GUI)
cargo doc --no-deps

# View at: target/doc/openai_ergonomic/index.html

# Check all links work
cargo doc --document-private-items
```

## Release Process

The canonical minute-by-minute instructions live in
[`docs/workflow/publish_checklist.md`](workflow/publish_checklist.md). Review that
checklist before every release and record the outcome once finished.

### Version Management

We use `release-plz` to generate release pull requests and keep the
`Cargo.toml`/`CHANGELOG.md` entries in sync:

1. Merge work into `main`; CI must be green before proceeding
2. Run `release-plz release-pr` locally (optionally with `--dry-run`) to stage the release
3. Review the generated PR for version bump, changelog, and dependency touches
4. Merge the PR; ***REMOVED*** (or a manual follow-up) will publish to crates.io

### Publishing Phases (Quick Reference)

1. **Preflight** – make sure planning artifacts are current, run the full
   `cargo fmt`/`clippy`/`test`/`build`/`doc`/`build --examples` suite, and verify
   examples still compile
2. **Dry Run** – execute `cargo publish --dry-run`, `cargo doc --no-deps --all-features`
   with `RUSTFLAGS="-D warnings"`, and `release-plz ... --dry-run`
3. **Execution** – merge the release PR, tag the commit if needed, and run
   `cargo publish` (or let ***REMOVED*** finish the job)
4. **Post-release** – confirm the crate on crates.io, check docs.rs status, send
   announcements, and capture release notes in `TODO.md`/`PLAN.md`

Keep the quick reference small; adjust the dedicated checklist when the process evolves.

## Collaboration Patterns

### Parallel Development

When multiple contributors work simultaneously:

1. **Use separate branches**: Avoid conflicts in shared files
2. **Coordinate on planning**: Update `TODO.md` with active work
3. **Frequent synchronization**: Merge/rebase regularly to avoid drift
4. **Clear handoffs**: Document session context for continuation

### Code Review Process

#### For Authors
1. **Pre-submit checklist**: Run all pre-commit checks
2. **Planning hygiene**: Update TODO/PLAN as needed
3. **Clear PR description**: Explain changes and their purpose
4. **Responsive iteration**: Address feedback promptly

#### For Reviewers
1. **Check planning artifacts**: Ensure TODO/PLAN updates are included
2. **Verify pre-commit checks**: Confirm tests pass and code is formatted
3. **Review for AI attribution**: Ensure no AI references are present
4. **Focus on quality**: Code clarity, test coverage, documentation

## Troubleshooting

### Common Issues

#### Pre-commit Checks Failing
```bash
# Fix formatting
cargo fmt --all

# Check specific clippy errors
cargo clippy --all-targets --all-features

# Run tests with output
cargo test --all -- --nocapture
```

#### Git Worktree Issues
```bash
# List all worktrees
git worktree list

# Remove old worktree
git worktree remove path/to/worktree

# Prune deleted worktrees
git worktree prune
```

#### Documentation Build Failures
```bash
# Check for broken links
cargo doc --document-private-items

# Build with verbose output
cargo doc --verbose
```

### Getting Help

- **Workflow Questions**: Check this document and [AGENTS.md](../AGENTS.md)
- **Technical Issues**: Open issue with reproduction steps
- **Planning Questions**: Review `PLAN.md` and `TODO.md` first
- **Architecture**: See [docs/architecture.md](architecture.md)

## Workflow Evolution

This workflow document should evolve based on:
- **Retrospective findings**: Regular assessment of what works
- **Contributor feedback**: Input from new and experienced contributors
- **Tool improvements**: Updates to development tools and practices
- **Project growth**: Scaling practices as the project grows

Updates to workflow should be proposed via PR with rationale and examples of improved outcomes.
