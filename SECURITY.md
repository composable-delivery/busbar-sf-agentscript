# Security Policy

## Supported Versions

We provide security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please follow these steps:

### Private Disclosure

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please report security issues by:

1. **Email**: Contact the maintainers directly (check repository settings for contact info)
2. **GitHub Security Advisory**: Use GitHub's private vulnerability reporting feature
   - Go to the Security tab in the repository
   - Click "Report a vulnerability"
   - Fill out the form with details

### What to Include

When reporting a vulnerability, please include:

- **Description**: Clear description of the vulnerability
- **Impact**: What could an attacker do with this vulnerability?
- **Reproduction**: Step-by-step instructions to reproduce the issue
- **Environment**: Rust version, OS, and any relevant configuration
- **Suggested Fix**: If you have ideas on how to fix it (optional)

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 1 week
- **Status Updates**: Every week until resolved
- **Fix Release**: Depends on severity
  - Critical: Within days
  - High: Within 1-2 weeks
  - Medium: Within 1 month
  - Low: Next regular release

## Security Measures

### Automated Security

This project uses multiple layers of automated security:

1. **Daily Security Audits**
   - Automated scanning via GitHub Actions
   - Checks against RustSec Advisory Database
   - Runs every day at 00:00 UTC

2. **Dependency Vulnerability Scanning**
   - `cargo-audit`: Scans for known vulnerabilities
   - `cargo-deny`: Enforces dependency policies
   - Runs on every PR and push

3. **License Compliance**
   - Only approved open-source licenses
   - Automatic detection of license violations

4. **Continuous Integration**
   - All tests must pass before merge
   - Clippy lints catch potential bugs
   - Code coverage tracking

### Manual Security Practices

1. **Code Review**: All changes reviewed by maintainers
2. **Minimal Dependencies**: We keep dependencies minimal
3. **Dependency Updates**: Regular updates for security patches
4. **Safe Rust**: Minimize use of `unsafe` code
5. **Input Validation**: Strict validation of all inputs

### Running Security Checks Locally

Before submitting code, run:

```bash
# Install security tools
cargo install cargo-audit cargo-deny

# Run comprehensive security checks
cargo audit
cargo deny check advisories
cargo deny check licenses
cargo deny check bans
cargo deny check sources
```

## Known Security Considerations

### Parser Security

- **Input Validation**: All Agentscript input is validated
- **Resource Limits**: Parser has limits to prevent DoS
- **Error Handling**: Errors don't leak sensitive information

### Runtime Security

- **Sandboxing**: Runtime execution is isolated (future)
- **Resource Limits**: Memory and execution time limits (future)
- **Permission System**: Controlled access to system resources (future)

## Security Best Practices for Contributors

1. **Never commit secrets**: No API keys, passwords, tokens, etc.
2. **Validate all inputs**: Especially in parser and runtime
3. **Handle errors safely**: Don't expose internal details
4. **Use safe Rust**: Avoid `unsafe` unless absolutely necessary
5. **Document unsafe code**: If you must use `unsafe`, document why
6. **Check dependencies**: Review new dependencies for security
7. **Update dependencies**: Keep dependencies current

## Security-Related Configuration

### deny.toml

Our `cargo-deny` configuration:
- Denies known vulnerabilities
- Warns about unmaintained crates
- Denies yanked crates
- Enforces approved licenses only

### Cargo.toml

Security-related settings:
- Locked dependency versions
- Minimal dependencies
- Well-maintained dependencies only

## Public Disclosure

Once a vulnerability is fixed:

1. We will release a patched version
2. Credit will be given to the reporter (unless they prefer anonymity)
3. A security advisory will be published
4. Details will be added to CHANGELOG.md

## Questions?

If you have questions about security but don't have a vulnerability to report:
- Open a discussion on GitHub
- Check existing security documentation
- Review our CI/CD workflows for security practices
