# Contributing to openai-ergonomic

Thank you for your interest in contributing to openai-ergonomic! This guide will help you get started.

## Development Setup

### Prerequisites

- Rust 1.82 or later
- Git

### Getting Started

1. Fork and clone the repository:
```bash
git clone https://github.com/YOUR_USERNAME/openai-ergonomic.git
cd openai-ergonomic
```

2. Create a `.env` file with your OpenAI credentials:
```bash
cp .env.example .env
# Edit .env with your credentials
```

## Development Workflow

### Making Changes

1. **Create a feature branch:**
```bash
git checkout -b feat/your-feature-name
# or: fix/your-bug-fix
# or: docs/your-docs-change
```

2. **Make your changes**
   - Follow the existing code style
   - Add tests for new functionality
   - Update documentation as needed

3. **Run pre-commit checks:**
```bash
# Run formatting, linting, and tests
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

4. **Commit your changes:**
```bash
git add -A
git commit -m "feat: your descriptive commit message"
```

Use [conventional commits](https://www.conventionalcommits.org/):
- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation changes
- `chore:` for maintenance tasks
- `test:` for test changes
- `refactor:` for code refactoring

5. **Push and create a PR:**
```bash
git push origin feat/your-feature-name
gh pr create  # or use GitHub web UI
```

## Code Guidelines

### Rust Code

- Follow Rust naming conventions
- Use `cargo fmt` for formatting
- Fix all `cargo clippy` warnings
- Write idiomatic Rust code
- Add documentation comments for public APIs
- No unsafe code (enforced by linting)

### Builder Pattern

When implementing new builders:
- Follow the existing builder pattern conventions
- Provide both builder methods and direct constructors where appropriate
- Include comprehensive documentation with examples
- Ensure type safety at compile time

### Examples

When adding new features, please include examples:

1. Create an example file in `examples/`
2. Test the example:
```bash
cargo run --example your_example
```
3. Update the README to reference your example

## Testing

### Unit Tests

Add unit tests for new functionality:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_your_feature() {
        // Your test
    }
}
```

### Integration Tests

For features that require API calls, add integration tests:

```rust
#[tokio::test]
async fn test_api_feature() {
    dotenv::dotenv().ok();
    let client = Client::from_env().unwrap();
    // Your test
}
```

### Doctests

Include examples in documentation that can be tested:

```rust
/// Creates a new chat completion
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use openai_ergonomic::Client;
///
/// let client = Client::new();
/// let response = client.chat()
///     .model("gpt-4")
///     .user("Hello!")
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
```

## Pull Request Process

1. **Ensure CI passes:** All GitHub Actions checks must pass
2. **Update documentation:** Include any necessary documentation updates
3. **Add tests:** New features should include tests
4. **Update CHANGELOG:** Note your changes if they're user-facing
5. **Request review:** Tag maintainers for review

## Project Structure

```
openai-ergonomic/
├── src/                    # Core library code
│   ├── builders/          # Builder pattern implementations
│   ├── models/            # Response models
│   └── lib.rs            # Main library entry
├── examples/              # Usage examples
├── tests/                # Integration tests
├── docs/                 # Additional documentation
│   ├── design/          # Design documents
│   └── research/        # Research notes
└── .github/workflows/    # CI/CD configuration
```

## Working with openai-client-base

This crate builds on top of `openai-client-base`, which provides the generated OpenAI API client. When implementing new features:

1. Check the available methods in `openai-client-base`
2. Create ergonomic wrappers following the builder pattern
3. Map between the base types and our ergonomic types
4. Hide complexity while maintaining flexibility

## Agent-Based Development

We use Claude agents for development. See `AGENTS.md` for:
- Agent roles and responsibilities
- Workflow procedures
- Planning and TODO management
- Parallel session management

## Release Process

Releases are automated using [release-plz](https://release-plz.ieni.dev/):

1. Merge changes to `main`
2. release-plz creates a release PR automatically
3. Review and merge the release PR
4. Packages are published to crates.io automatically

## Getting Help

- Open an [issue](https://github.com/timvw/openai-ergonomic/issues) for bugs or feature requests
- Check existing issues before creating a new one
- Review the [OpenAI API documentation](https://platform.openai.com/docs) for API details

## Code of Conduct

Please be respectful and constructive in all interactions. We aim to maintain a welcoming and inclusive community.

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).