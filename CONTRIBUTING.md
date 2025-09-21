# Contributing to openai-ergonomic

Thank you for your interest in contributing to `openai-ergonomic`! This document provides guidelines for contributing to the project.

## Getting Started

### Prerequisites

- Rust 1.82 or higher
- Git
- A GitHub account

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
2. **Small Commits**: Make focused commits with clear messages following [Conventional Commits](https://www.conventionalcommits.org/)
3. **Test Coverage**: Add tests for new functionality
4. **Documentation**: Update documentation for API changes

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

1. **Create a Feature Branch**:
   ```bash
   git checkout -b feat/your-feature-name
   ```

2. **Make Your Changes**: Follow the development workflow above

3. **Update Documentation**: Ensure README, docs, and code comments are updated

4. **Test Your Changes**:
   ```bash
   cargo test --all
   cargo clippy --all-targets --all-features -- -D warnings
   cargo fmt --all --check
   ```

5. **Submit Pull Request**:
   - Provide a clear description of changes
   - Link to any related issues
   - Include examples if adding new features

### Pull Request Guidelines

- Keep PRs focused and reasonably sized
- Include tests for new functionality
- Update documentation for API changes
- Ensure CI passes before requesting review
- Be responsive to feedback

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
# Build API documentation
cargo doc --open

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

## Code Review

### As a Reviewer

- Be constructive and respectful
- Focus on code quality, not personal preferences
- Ask questions to understand the reasoning
- Suggest improvements with explanations

### As an Author

- Be receptive to feedback
- Explain your reasoning when requested
- Make requested changes promptly
- Ask for clarification if feedback is unclear

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

## Additional Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [OpenAI API Documentation](https://platform.openai.com/docs/api-reference)
- [Project Architecture Guide](docs/architecture.md)
- [Getting Started Guide](docs/getting-started.md)

Thank you for contributing to `openai-ergonomic`!