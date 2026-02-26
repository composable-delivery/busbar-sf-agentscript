# Contributing to sf-agentscript

Thank you for your interest in contributing to sf-agentscript! This document provides guidelines and best practices for contributing.

## Development Setup

1. **Install Rust**: Follow the instructions at https://rustup.rs/
2. **Clone the repository**:
   ```bash
   git clone https://github.com/composable-delivery/sf-agentscript.git
   cd sf-agentscript
   ```
3. **Install development tools**:
   ```bash
   cargo install cargo-audit cargo-deny cargo-llvm-cov
   ```

## Development Workflow

### Before Making Changes

1. **Create a new branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Ensure everything builds**:
   ```bash
   cargo build
   cargo test
   ```

### Making Changes

1. **Write code** following Rust conventions
2. **Add tests** for new functionality
3. **Update documentation** as needed
4. **Format code**:
   ```bash
   cargo fmt
   ```
5. **Check for issues**:
   ```bash
   cargo clippy --all-features --workspace -- -D warnings
   ```

### Before Committing

Run the complete check suite:

```bash
# Format check
cargo fmt --check

# Linting
cargo clippy --all-features --workspace -- -D warnings

# Tests
cargo test --all-features --workspace

# Security audit
cargo audit

# Dependency checks
cargo deny check

# Documentation
cargo doc --no-deps --all-features --workspace
```

### Commit Messages

Use clear, descriptive commit messages:
- Start with a verb in present tense (Add, Fix, Update, Remove)
- Keep the first line under 72 characters
- Add details in subsequent lines if needed

Examples:
```
Add parser support for conditional expressions

Fix runtime panic when handling empty arrays

Update documentation for parser module
```

## Code Standards

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting (configuration in `rustfmt.toml`)
- Address all `clippy` warnings
- Prefer standard library types when possible

### Testing

- Write unit tests for all public APIs
- Add integration tests for complex scenarios
- Test against examples in `agent-script-recipes/`
- Aim for high code coverage

### Documentation

- Add doc comments (`///`) for all public items
- Include examples in doc comments when helpful
- Keep documentation up-to-date with code changes
- Run `cargo doc` to verify documentation builds

### Security

- Never commit secrets or credentials
- Review dependencies for vulnerabilities
- Follow secure coding practices
- Report security issues privately (see SECURITY.md)

## Pull Request Process

1. **Push your branch** to your fork
2. **Create a Pull Request** with:
   - Clear title describing the change
   - Description of what changed and why
   - Reference to any related issues
3. **Wait for CI checks** to pass
4. **Address review feedback** if requested
5. **Squash commits** if asked (optional)

### PR Checklist

Before submitting, ensure:
- [ ] Code compiles without errors
- [ ] All tests pass
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings
- [ ] Security checks pass
- [ ] Documentation is updated
- [ ] New tests added for new functionality

## Getting Help

- **Questions**: Open a discussion on GitHub
- **Bugs**: Open an issue with reproduction steps
- **Features**: Open an issue to discuss before implementing

## Code of Conduct

Be respectful, inclusive, and professional in all interactions. We are committed to providing a welcoming environment for all contributors.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
