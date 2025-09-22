# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/genai-rs/openai-ergonomic/compare/v0.1.0...v0.2.0) - 2025-09-22

### Added

- add comprehensive testing harness and update documentation
- implement Phase 1 core examples
- implement comprehensive streaming responses example
- implement comprehensive chat completions example
- implement comprehensive responses API example
- implement core builder patterns and client wrapper

### Fixed

- apply cargo fmt formatting changes
- add missing stream error variant handlers in comprehensive example
- resolve clippy warnings in responses_comprehensive example
- resolve compilation errors in responses_comprehensive example
- resolve clippy warnings in responses_streaming example
- remove misplaced responses_comprehensive example from chat branch
- resolve clippy warnings in chat_comprehensive example
- resolve compilation errors in chat comprehensive example
- use proper structure for required_pull_request_reviews
- restore full settings.yml - Settings app is working
- simplify settings.yml format to match working repo
- update settings.yml with correct CI check names
- update docs agent to work headless without GUI programs
- resolve CI failures - compilation, clippy, and formatting issues

### Other

- mark responses_streaming.rs task complete in TODO
- minimal branch protection config
- trigger Settings app to create branch protection
- minimal settings.yml to debug Settings app
- trigger Settings app sync
- add Agile Coach Agent for workflow optimization

### Added
- Initial project scaffolding with Cargo.toml configuration
- Basic library structure with error handling foundation
- Comprehensive CI/CD pipeline with GitHub Actions
- Security auditing and dependency scanning workflows
- Automated release management with release-plz
- License compliance and supply chain security checks

### Changed
- N/A

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

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