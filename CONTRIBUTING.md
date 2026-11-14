# Contributing to openai-ergonomic

Thank you for your interest in contributing to `openai-ergonomic`! This page has the workflow,
the commit format and the pull request process; [development](docs/development.md) has the
commands (checks, tests, local CI) and [architecture](docs/architecture.md) how the crate is put
together.

## Getting started

### Prerequisites

- Rust at or above the minimum supported version, `rust-version` in [`Cargo.toml`](Cargo.toml)
  ([![MSRV](https://img.shields.io/crates/msrv/openai-ergonomic)](https://crates.io/crates/openai-ergonomic)),
  and stable for the everyday checks
- Git and a GitHub account
- Familiarity with the `OpenAI` API (recommended)

### Setting up

```bash
git clone https://github.com/your-username/openai-ergonomic.git
cd openai-ergonomic
cargo build
cargo test --all-features
./scripts/install-hooks.sh   # optional: the pre-commit hook in .githooks/
```

No API key is needed for the tests: the integration tests run against mock servers.

## Workflow

1. **Discuss larger changes first** in an issue, so that the approach is agreed before the code.
2. **Branch** from `main` (`feat/...`, `fix/...`, `docs/...`).
3. **Keep changes small and focused**: one concern per pull request, incremental commits.
4. **Test** new behaviour (unit tests next to the code, integration tests in `tests/`).
5. **Document** API changes: the rustdoc, the pages under `docs/`, the examples, and a
   [migration](docs/migration.md) note for a breaking change.

### Project structure

- `src/` - the library ([architecture](docs/architecture.md))
- `tests/` - integration tests (mockito servers) and the WebAssembly suite
- `examples/` - runnable examples ([index](docs/examples-index.md))
- `docs/` - the guides, compiled as doc tests
- `scripts/` - development scripts ([scripts](scripts/README.md))

### Commit messages

We follow [Conventional Commits](https://www.conventionalcommits.org/); release-plz derives the
version bump and the changelog from them:

```
type(scope): description

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `ci`. Mark a breaking
change with `!` (`feat!: ...`) or a `BREAKING CHANGE:` footer: the crate is on 0.x, so it bumps
the minor version.

```
feat(client): add streaming support for chat completions
fix(auth): handle expired API keys gracefully
docs: describe per-call options
```

## Pull requests

### Before opening one

Run the checks CI runs (see [development](docs/development.md#checks)); at least:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Maintainers can attest a commit with the genai-rs local CI script instead of waiting for the
heavy CI jobs (see [local CI](docs/development.md#local-ci)).

### Guidelines

- Keep pull requests focused and reasonably sized.
- Describe what changed and why; link related issues; include examples for new features.
- Include tests for new functionality and update the documentation for API changes.
- Make sure the checks pass before requesting review, and be responsive to feedback.
- Never include AI attribution in commits or code (see the policy below).

## Critical Policy: No AI Attribution

**ABSOLUTELY NO AI ATTRIBUTION**: Never include any of the following in commits, PRs, or code:
- "Generated with an AI assistant" or similar
- "Co-authored-by: <AI assistant>" or any AI co-authorship
- References to AI/LLM assistance in comments or documentation
- Any indication that code was AI-generated

All code must appear as human-authored work. This is a hard requirement - violations will result
in PR rejection. `scripts/check-attribution.sh` (run by the pre-commit hook) checks for it.

## Documentation

- **API documentation**: rustdoc comments in the code, with examples.
- **Guides**: the pages under `docs/`, linked from the [README](README.md). `src/lib.rs` includes
  them in a `#[cfg(doctest)]` module, so their Rust snippets compile with the code: use
  `no_run` for snippets that call the API, and `ignore` only when a snippet cannot compile here
  (for example because it needs a crate that is not a dependency), saying why.
- **Examples**: runnable programs in `examples/`.
- **No hard-coded versions**: install with `cargo add openai-ergonomic`, show versions with the
  crates.io badges or a placeholder such as `x.y`. Migration notes name the release they describe.

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features   # what CI checks
cargo test --doc --all-features                                  # the doc tests, guides included
```

## Issues

### Reporting bugs

Include the crate version, the Rust version, the operating system (or WebAssembly host), a
minimal reproducible example, the expected and actual behaviour, and error messages or logs
(without API keys).

### Feature requests

Explain the use case, show how the API would be used, and mention the alternatives you
considered.

## Code review

As a reviewer, be constructive and respectful, ask about the reasoning, explain suggestions, and
check that tests and documentation came along with the change. As an author, explain your
reasoning when asked and keep the checks green after each change.

## Releases

Releases are automated with [release-plz](https://release-plz.ieni.dev/): every push to `main`
opens or updates a release pull request (`chore: release version ...`) with the version bump and
the changelog. A maintainer reviews and merges it, then publishes it to crates.io with the
`Release-plz` workflow (`workflow_dispatch`, `publish`), which also tags the release and creates
the GitHub release.

## Community

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).
Please be respectful and inclusive. Open an issue for bugs, feature requests and questions, and
check the existing issues first.

## License

By contributing to this project, you agree that your contributions will be licensed under the
project's MIT OR Apache-2.0 license.
