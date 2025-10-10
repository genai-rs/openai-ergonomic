# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1](https://github.com/genai-rs/openai-ergonomic/compare/v0.2.0...v0.2.1) - 2025-10-10

### Added

- add comprehensive Azure OpenAI API test
- add Azure OpenAI support

### Fixed

- resolve clippy warnings in azure_comprehensive example
- complete Azure OpenAI middleware implementation
- resolve clippy and formatting issues after emoji removal

### Other

- add Azure OpenAI support documentation to README
- remove emojis from code, scripts, and tests
- remove emojis from documentation

## [0.2.0](https://github.com/genai-rs/openai-ergonomic/compare/v0.1.0...v0.2.0) - 2025-10-09

### Added

- add reqwest-middleware support for HTTP client customization
- add context management to Langfuse interceptor
- add Langfuse integration for LLM observability
- leverage OpenTelemetry semantic conventions for operation names
- add interceptor hooks to 14 key API methods
- integrate interceptor hooks into API methods
- add interceptor/middleware infrastructure
- complete all Phase 1 & 2 API implementations
- Initial project scaffolding with Cargo.toml configuration
- Basic library structure with error handling foundation
- Comprehensive CI/CD pipeline with GitHub Actions
- Security auditing and dependency scanning workflows
- Automated release management with release-plz
- License compliance and supply chain security checks

### Fixed

- apply cargo fmt
- update reqwest-middleware to 0.4 to resolve version conflicts
- resolve clippy and format issues in example
- update langfuse_interceptor doctest for ClientBuilder API
- resolve CI failures from builder pattern refactoring
- set span status to error for proper Langfuse error flagging
- resolve clippy doc warnings in examples
- fix clippy
- make codecov checks informational only
- resolve clippy::future_not_send warnings properly
- resolve clippy documentation lints
- apply cargo fmt
- resolve final format! clippy lints
- resolve remaining clippy lints
- resolve all clippy lints for CI
- resolve clippy and formatting issues
- address formatting and clippy warnings
- resolve all clippy warnings and formatting issues
- update release-plz workflow to use GitHub App token

### Other

- [**breaking**] remove timeout_seconds from Config (breaking change)
- make cargo-deny checks informational only
- update all documentation to use ClientBuilder with .build()
- update examples to use ClientBuilder with .build()
- eliminate RwLock from Client using builder pattern
- simplify interceptor API by unifying with_interceptor methods
- use published opentelemetry-langfuse 0.6.0
- eliminate global state from Langfuse integration
- make interceptor system generic over state type
- separate OpenTelemetry setup from interceptor
- use metadata-based span tracking to eliminate user wrapper requirement
- update Langfuse examples to use span_storage pattern
- simplify LangfuseInterceptor to use task-local span storage
- restructure docs
- more cleanup of docs
- remove unused docs
- apply interceptor hooks to ALL API methods
- Fix clippy warnings
- Add tests and fix formatting for custom HTTP client feature
- Add support for custom HTTP client configuration
- fix rustdoc warnings for generic type parameters
- add API coverage tracking and examples index
- update openai-client-base to 0.4
- Disable required PR reviews in branch protection
- remove decorative README emoji
- migrate workflow instructions to automation terminology

### Security

- Implemented cargo-audit for vulnerability scanning
- Added cargo-deny for dependency policy enforcement
- Configured GitHub dependency review for pull requests

## [0.1.0] - TBD

### Added
- Initial release placeholder
- Project foundation and tooling setup

---

## Template for New Releases

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- New features

### Changed
- Changes in existing functionality

### Deprecated
- Soon-to-be removed features

### Removed
- Now removed features

### Fixed
- Bug fixes

### Security
- Vulnerability fixes
```