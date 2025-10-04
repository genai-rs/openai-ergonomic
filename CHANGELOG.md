# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/genai-rs/openai-ergonomic/compare/v0.1.0...v0.2.0) - 2025-10-04

### Added

- complete all Phase 1 & 2 API implementations

### Fixed

- resolve all clippy warnings and formatting issues
- update release-plz workflow to use GitHub App token

### Other

- Fix clippy warnings
- Add tests and fix formatting for custom HTTP client feature
- Add support for custom HTTP client configuration
- fix rustdoc warnings for generic type parameters
- add API coverage tracking and examples index
- update openai-client-base to 0.4
- Disable required PR reviews in branch protection
- remove decorative README emoji
- migrate workflow instructions to automation terminology

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