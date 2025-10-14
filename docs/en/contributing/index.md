---
title: Contributing
description: How to contribute to the Cheolsu Proxy project
---

# Contributing

Thank you for your interest in contributing to Cheolsu Proxy! We welcome contributions from the community to help improve this project. Whether you're interested in fixing bugs, adding new features, or improving documentation, there are many ways to get involved.

## How to Contribute

Here are some steps to get started with contributing to this project:

1. **Fork the repository**: Fork the repository on GitHub and clone it to your local machine
2. **Create a branch**: Create a new branch for your changes
3. **Make changes**: Make your changes and test them thoroughly
4. **Commit**: Commit your changes with a descriptive commit message
5. **Pull Request**: Push your changes to your fork and submit a pull request

We appreciate contributions of any size, from small bug fixes to major new features. If you're unsure about a change you'd like to make, feel free to open an issue first to discuss it with the maintainers.

## Types of Contributions

### 🐛 Bug Fixes

- Review and reproduce bug reports
- Analyze root causes
- Write test cases
- Implement fixes

### ✨ New Features

- Review feature requests
- Write design documents
- Implement and test
- Update documentation

### 📚 Documentation Improvements

- Write user guides
- Update API documentation
- Improve code comments
- Translation work

### 🧪 Testing

- Write unit tests
- Add integration tests
- Performance testing
- Security testing

## Development Environment Setup

### Prerequisites

- **Rust**: 1.70.0 or higher
- **Node.js**: 18.0.0 or higher
- **Tauri CLI**: Latest version
- **Git**: 2.0.0 or higher

### Rust Installation

```bash
# macOS/Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows
# Download rustup-init.exe from https://rustup.rs/ and run it
```

### Development Tools Installation

```bash
# Code formatting
rustup component add rustfmt

# Linter
rustup component add clippy

# Tauri CLI
cargo install tauri-cli
```

### Project Clone and Build

```bash
# Clone repository
git clone https://github.com/ohah/cheolsu-proxy.git
cd cheolsu-proxy

# Install dependencies
cargo build
cd tauri-ui && npm install
```

## Development Workflow

### 1. Check Issues

- Select an issue to work on from [GitHub Issues](https://github.com/ohah/cheolsu-proxy/issues)
- Assign yourself to the issue to indicate you're working on it
- Comment on the issue if you have questions or suggestions

### 2. Create Branch

```bash
# Update to latest code
git checkout main
git pull origin main

# Create new branch
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
```

### 3. Development and Testing

```bash
# Run development server
cd tauri-ui
npm run tauri dev

# Run tests
cargo test
cargo clippy
cargo fmt
```

### 4. Commit

```bash
# Stage changes
git add .

# Commit (follow commit message rules)
git commit -m "feat: add new feature"
git commit -m "fix: fix bug"
git commit -m "docs: update documentation"
```

### 5. Pull Request

```bash
# Push branch
git push origin feature/your-feature-name

# Create pull request on GitHub
```

## Coding Style

### Rust Coding Rules

- Follow **Rust standard coding conventions** (use rustfmt)
- **Error handling**: Use Result type for explicit error handling
- **Documentation comments**: Use `///`
- **Module structure**: Clearly separated
- **Testing**: Write appropriate tests for new features

### TypeScript/React Coding Rules

- Use **TypeScript strict mode**
- Use **functional components** and hooks
- **Type definitions**: Write explicitly
- **Components**: Follow single responsibility principle
- **Interfaces**: Use Interface when possible
- **Props**: Don't use inline format

### File Structure

- Follow **FSD (Feature-Sliced Design)** architecture
- **Naming conventions**:
  - Variables and functions: camelCase
  - Constants: UPPER_SNAKE_CASE
  - Components: PascalCase
  - File names: kebab-case

## Commit Message Rules

### Format

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code formatting, missing semicolons, etc.
- `refactor`: Code refactoring
- `test`: Add or modify tests
- `chore`: Build process, auxiliary tool changes

### Examples

```bash
feat(proxy): add TLS 1.3 support
fix(ui): improve text readability in dark theme
docs(guide): update certificate installation guide
refactor(ca): optimize certificate generation logic
```

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test integration_test
```

### UI Tests

```bash
# Test Tauri app
cd tauri-ui
npm run tauri test
```

### Manual Testing

```bash
# Install HTTP server and client
cargo install echo-server xh

# Run HTTP server
echo-server

# Test with HTTP client
xh --proxy http:http://127.0.0.1:8100 GET http://127.0.0.1:8080
```

## Pull Request Guidelines

### PR Title

- Write clearly and concisely
- Summarize changes in one line
- Include issue number (e.g., "Fix #123")

### PR Description

```markdown
## Changes

- Clearly describe what was changed

## Testing

- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist

- [ ] Code style guide followed
- [ ] Documentation updated
- [ ] No breaking changes

## Related Issues

Closes #123
```

### Review Process

1. **Automated checks**: CI/CD pipeline passes
2. **Code review**: At least one reviewer approval
3. **Testing**: All tests pass
4. **Merge**: Squash and merge recommended

## Troubleshooting

### Common Issues

**Build failure**:

```bash
# Update dependencies
cargo update
cd tauri-ui && npm update

# Clear cache
cargo clean
cd tauri-ui && rm -rf node_modules && npm install
```

**Test failure**:

```bash
# Check test environment
cargo test --verbose

# Debug specific test
cargo test test_name -- --nocapture
```

**Tauri build error**:

```bash
# Update Tauri CLI
cargo install tauri-cli --force

# Reinstall dependencies
cd tauri-ui && npm install
```

## Community

### Communication Channels

- **GitHub Issues**: Bug reports, feature requests
- **GitHub Discussions**: General questions, idea sharing
- **Pull Requests**: Code reviews, discussions

### Code of Conduct

All contributors must follow the [Code of Conduct](https://github.com/ohah/cheolsu-proxy/blob/master/CODE_OF_CONDUCT.md).

## License

All contributed code will be distributed under the same license as the project (MIT/Apache-2.0).

## Acknowledgments

Thank you to all contributors! Your contributions make Cheolsu Proxy a better tool.

---

For more detailed information, see [Development Setup](/en/contributing/development-setup).
