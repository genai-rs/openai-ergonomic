# Security Policy

## Supported Versions

We release patches for security vulnerabilities. Currently supported versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security vulnerability, please follow these steps:

1. **DO NOT** open a public issue
2. Email security details to: github@timvw.be
3. Include the following information:
   - Type of vulnerability
   - Full paths of source file(s) related to the vulnerability
   - Location of the affected source code (tag/branch/commit)
   - Step-by-step instructions to reproduce the issue
   - Proof-of-concept or exploit code (if possible)
   - Impact assessment

## Response Time

- **Initial Response**: Within 48 hours
- **Status Update**: Every 72 hours until resolved
- **Resolution Target**: Critical issues within 7 days

## Security Measures

This project implements several security measures:

### Code Security
- No unsafe code (enforced by `#![forbid(unsafe_code)]`)
- Regular dependency audits via `cargo-audit`
- Automated dependency updates via Renovate
- Security scanning in CI pipeline

### API Key Protection
- Never log or expose API keys
- Environment variable configuration
- Secure credential handling patterns
- Example files use placeholder keys

### Dependencies
- Minimal dependency footprint
- Regular security updates
- License compliance checks via `cargo-deny`

## Disclosure Policy

Once a security vulnerability has been resolved:

1. We will publish a security advisory
2. Credit will be given to the reporter (unless anonymity is requested)
3. Details will be shared to help the community

## Security Features

### Built-in Protections
- Input validation on all builder methods
- Safe serialization/deserialization
- No arbitrary code execution
- Memory-safe Rust guarantees

### Best Practices for Users
- Store API keys securely (use environment variables or secret management)
- Rotate API keys regularly
- Use minimal required permissions
- Monitor API usage for anomalies
- Keep the library updated

## Contact

For security concerns that shouldn't be public:
- Email: github@timvw.be
- GPG Key: [Available on request]

For general bugs and feature requests:
- Use [GitHub Issues](https://github.com/timvw/openai-ergonomic/issues)

## Acknowledgments

We appreciate the security research community and thank all individuals who responsibly disclose vulnerabilities.