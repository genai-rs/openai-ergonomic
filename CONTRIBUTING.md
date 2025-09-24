# Contributing to openai-ergonomic

Thank you for your interest in contributing to `openai-ergonomic`! This document provides comprehensive guidelines for contributing to the project, including our unique agent-driven development workflow.

## Getting Started

### Prerequisites

- Rust 1.82 or higher
- Git
- A GitHub account
- Familiarity with OpenAI API concepts (recommended)
- Understanding of our agent workflow (see [AGENTS.md](AGENTS.md) and [docs/workflow.md](docs/workflow.md))

### Setting up the Development Environment

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/your-username/openai-ergonomic.git
   cd openai-ergonomic
   ```

2. Install dependencies:
   ```bash
   cargo build
   ```

3. Run tests to ensure everything works:
   ```bash
   cargo test
   ```

## Development Workflow

We follow a **plan-first** development approach using specialized AI agents. Please read [AGENTS.md](AGENTS.md) for complete details on our agent-driven workflow.

### Quick Workflow Overview

1. **Plan First**: For any significant changes, create or update entries in `PLAN.md` and `TODO.md` before coding
2. **Use Appropriate Agent Role**: Follow the agent guidelines in [AGENTS.md](AGENTS.md) based on your contribution type
3. **Small, Focused Changes**: Make incremental commits with clear purposes
4. **Maintain Documentation**: Keep planning artifacts and documentation in sync with code changes

### Detailed Workflow Steps

1. **Review Context**: Read `PLAN.md`, `TODO.md`, and relevant reference files before making changes
2. **Draft Plan**: Outline your approach, identify risks, and capture the plan in appropriate TODO entries
3. **Execute Incrementally**: Apply changes in small chunks with frequent validation
4. **Update Artifacts**: Mark TODOs complete, adjust docs/tests, summarize changes
5. **Document Decisions**: Record notable choices in `docs/` for persistent context

### Code Style

We use standard Rust formatting and linting tools:

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --all
```

### Project Structure

- `src/` - Core library code
- `examples/` - Example usage patterns
- `docs/` - Documentation files
- `tests/` - Integration tests

### Making Changes

1. **Plan First**: For significant changes, create or update entries in `PLAN.md` and `TODO.md` before coding
2. **Follow Agent Guidelines**: Reference [AGENTS.md](AGENTS.md) for role-specific responsibilities and workflows
3. **Small Commits**: Make focused commits with clear messages following [Conventional Commits](https://www.conventionalcommits.org/)
4. **Test Coverage**: Add tests for new functionality
5. **Documentation**: Update documentation for API changes
6. **Maintain Planning Artifacts**: Keep `TODO.md` and `PLAN.md` current with your progress

### Commit Message Format

We follow the Conventional Commits specification:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New features
- `fix`: Bug fixes
- `docs`: Documentation changes
- `style`: Formatting changes
- `refactor`: Code refactoring
- `test`: Test additions or changes
- `chore`: Maintenance tasks

Examples:
```
feat(client): add streaming support for chat completions
fix(auth): handle expired API keys gracefully
docs(readme): add quickstart example
```

## Pull Request Process

### Before Creating a PR

**CRITICAL**: Run all pre-commit checks before submitting:

```bash
# Format check - must pass
cargo fmt --all

# Lint check - must pass with no warnings
cargo clippy --all-targets --all-features -- -D warnings

# Test check - all tests must pass
cargo test --all

# Build check - project must compile
cargo build
```

**Never submit a PR without running these checks first - CI will fail if they don't pass.**

1. **Create a Feature Branch**:
   ```bash
   git checkout -b feat/your-feature-name
   ```

2. **Make Your Changes**: Follow the development workflow above

3. **Update Documentation**: Ensure README, docs, and code comments are updated

4. **Update Planning Artifacts**:
   - Mark relevant TODO items as completed
   - Update `PLAN.md` if your changes affect the roadmap
   - Document any decisions made during implementation

5. **Run Pre-commit Checks** (see above - this is mandatory)

6. **Submit Pull Request**:
   - Provide a clear description of changes
   - Link to any related issues or TODO items
   - Include examples if adding new features
   - Reference which agent role guidelines were followed
   - Explain how the change fits into the overall project plan

### Pull Request Guidelines

- Keep PRs focused and reasonably sized (follow the small diffs principle from [AGENTS.md](AGENTS.md))
- Include tests for new functionality
- Update documentation for API changes
- **Mandatory**: Ensure all pre-commit checks pass before requesting review
- Update planning artifacts (`TODO.md`, `PLAN.md`) as needed
- Be responsive to feedback
- Follow agent-specific guidelines from [AGENTS.md](AGENTS.md)
- Never include AI attribution in commits or code (see policy below)

## Critical Policy: No AI Attribution

**ABSOLUTELY NO AI ATTRIBUTION**: Never include any of the following in commits, PRs, or code:
- "Generated with Claude Code" or similar
- "Co-authored-by: Claude" or any AI co-authorship
- References to AI/LLM assistance in comments or documentation
- Any indication that code was AI-generated

All code must appear as human-authored work. This is a hard requirement - violations will result in PR rejection.

## Testing

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration

# Documentation tests
cargo test --doc

# All tests with coverage
cargo test --all
```

### Writing Tests

- Add unit tests for individual functions
- Add integration tests for complete workflows
- Include doctests in public API documentation
- Use mock servers for testing HTTP interactions

## Documentation

### Types of Documentation

1. **API Documentation**: Rustdoc comments in code
2. **User Guides**: Files in `docs/` directory
3. **Examples**: Runnable examples in `examples/` directory
4. **README**: Project overview and quick start

### Documentation Guidelines

- Write clear, concise explanations
- Include code examples for complex concepts
- Keep examples up-to-date with API changes
- Use consistent terminology throughout

### Building Documentation

```bash
# Build API documentation (headless - no GUI)
cargo doc --no-deps

# View docs manually at: target/doc/openai_ergonomic/index.html

# Check documentation links
cargo doc --document-private-items
```

## Issue Guidelines

### Reporting Bugs

When reporting bugs, please include:

- Rust version
- Operating system
- Minimal reproducible example
- Expected vs actual behavior
- Error messages or logs

### Feature Requests

For feature requests, please:

- Explain the use case
- Provide examples of how it would be used
- Consider existing alternatives
- Be open to discussion about implementation

## Agent-Driven Development

This project uses specialized AI agents for different aspects of development. Contributors should:

1. **Understand Agent Roles**: Read [AGENTS.md](AGENTS.md) to understand which agent guidelines apply to your contribution
2. **Follow Workflow Patterns**: Use the plan-first approach and maintain planning artifacts
3. **Collaborate Effectively**: When working in parallel with others, use separate branches/worktrees as described in [docs/workflow.md](docs/workflow.md)
4. **Maintain Context**: Keep documentation and planning files updated for future contributors

See [docs/workflow.md](docs/workflow.md) for detailed guidance on development patterns.

## Code Review

### As a Reviewer

- Be constructive and respectful
- Focus on code quality, not personal preferences
- Ask questions to understand the reasoning
- Suggest improvements with explanations
- **Check planning hygiene**: Ensure TODO/PLAN updates are included when needed
- Verify pre-commit checks have been run
- Confirm no AI attribution is present in the changes

### As an Author

- Be receptive to feedback
- Explain your reasoning when requested
- Make requested changes promptly
- Ask for clarification if feedback is unclear
- Update planning artifacts when requested
- Ensure all pre-commit checks pass after making changes

## Release Process

The project uses automated releases through `release-plz`:

1. Changes are merged to `main`
2. `release-plz` automatically creates release PRs
3. Maintainers review and merge release PRs
4. Releases are automatically published to crates.io

## Community

### Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Please be respectful and inclusive in all interactions.

### Getting Help

- Open an issue for bugs or feature requests
- Start a discussion for questions or ideas
- Check existing issues before creating new ones

## License

By contributing to this project, you agree that your contributions will be licensed under the project's MIT OR Apache-2.0 license.

## Quick Commands Reference

```bash
# Format & lint (run before every commit)
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
cargo test --all

# Generate documentation (headless, no GUI)
cargo doc --no-deps

# Run a specific example
cargo run --example quickstart
```

## Getting Help

- **New Contributors**: Start with [docs/getting-started.md](docs/getting-started.md)
- **Development Workflow**: Read [docs/workflow.md](docs/workflow.md) and [AGENTS.md](AGENTS.md)
- **Architecture Questions**: Check [docs/architecture.md](docs/architecture.md)
- **Bug Reports**: Open an issue with reproduction steps
- **Feature Requests**: Check the roadmap in [docs/roadmap.md](docs/roadmap.md) first

## Additional Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [OpenAI API Documentation](https://platform.openai.com/docs/api-reference)
- [Project Architecture Guide](docs/architecture.md)
- [Getting Started Guide](docs/getting-started.md)
- [Development Workflow Guide](docs/workflow.md)
- [Project Roadmap](docs/roadmap.md)
- [Agent Guidelines](AGENTS.md)

Thank you for contributing to `openai-ergonomic`!