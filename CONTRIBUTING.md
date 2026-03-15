# Contributing to Innu

Thank you for your interest in contributing to Innu! This document provides guidelines for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Environment](#development-environment)
- [Project Structure](#project-structure)
- [How to Contribute](#how-to-contribute)
  - [Reporting Bugs](#reporting-bugs)
  - [Suggesting Features](#suggesting-features)
  - [Pull Requests](#pull-requests)
- [Coding Standards](#coding-standards)
- [Commit Guidelines](#commit-guidelines)
- [Release Process](#release-process)

## Code of Conduct

This project adheres to the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/innu.git
   cd innu
   ```
3. Create a new branch for your feature or fix:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Environment

### Prerequisites

- **Rust toolchain** (latest stable version recommended)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **NetworkManager** running on your system
- **D-Bus** access
- A desktop environment with **Wayland or X11** support

### Building the Project

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

### Running the Application

```bash
# Development run
cargo run

# Release run
cargo run --release
```

## Project Structure

```
innu/
├── src/
│   ├── main.rs          # Application entry point
│   ├── lib.rs           # Library exports
│   ├── app.rs           # Main application logic
│   ├── model.rs         # Data models
│   ├── settings.rs      # Configuration management
│   ├── backend/         # Backend modules
│   ├── platform/        # Platform-specific code
│   └── ui/              # UI components (egui)
├── assets/              # Images and resources
├── Cargo.toml          # Rust dependencies and config
├── install.sh          # Installation script
└── README.md           # Project documentation
```

## How to Contribute

### Reporting Bugs

Before creating a bug report, please check if the issue already exists.

When reporting bugs, include:

- **Clear title and description**
- **Steps to reproduce** the bug
- **Expected behavior** vs actual behavior
- **Environment details**:
  - OS and version
  - Desktop environment (GNOME, KDE, etc.)
  - Rust version (`rustc --version`)
  - NetworkManager version
- **Screenshots** if applicable
- **Logs** (run with `RUST_LOG=debug cargo run`)

### Suggesting Features

Feature suggestions are welcome! When suggesting features:

- Use a clear, descriptive title
- Explain the use case and why it would be valuable
- Describe how you envision the feature working
- Consider potential UI/UX implications

### Pull Requests

1. **Open an issue first** for substantial changes (new features, major refactoring)
2. **Ensure your code compiles** and passes tests
3. **Follow coding standards** (see below)
4. **Update documentation** if needed
5. **Reference related issues** in your PR description

#### PR Process

1. Update your branch with the latest main:
   ```bash
   git fetch origin
   git rebase origin/main
   ```
2. Push your branch to your fork
3. Create a Pull Request on GitHub
4. Fill out the PR template (if provided) or include:
   - Description of changes
   - Related issue numbers
   - Testing performed
   - Screenshots for UI changes

## Coding Standards

### Rust Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` to format code:
  ```bash
  cargo fmt
  ```
- Use `cargo clippy` to catch common mistakes:
  ```bash
  cargo clippy -- -D warnings
  ```
- Keep functions focused and small
- Use meaningful variable and function names
- Add documentation comments (`///`) for public APIs

### Error Handling

- Use `thiserror` for custom error types
- Use `anyhow` for application-level error handling
- Avoid unwrap() in production code; use proper error handling

### Dependencies

- Keep dependencies minimal
- Document why each dependency is needed
- Update dependencies through PRs, not directly to main

### UI/UX Guidelines

- Follow egui best practices
- Ensure the UI is responsive
- Test on both Wayland and X11
- Consider accessibility (keyboard navigation, screen readers)

## Commit Guidelines

We follow [Conventional Commits](https://www.conventionalcommits.org/) for commit messages:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, no logic change)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Build process or auxiliary tool changes

### Examples

```
feat(ui): add password visibility toggle

fix(backend): handle NetworkManager disconnect race condition

docs: update README with uninstall instructions

refactor(model): simplify NetworkState enum variants
```

### Best Practices

- Use present tense ("add feature" not "added feature")
- Use imperative mood ("move cursor to..." not "moves cursor to...")
- Limit first line to 72 characters
- Reference issues and PRs in the footer when relevant

## Release Process

Releases are managed by maintainers:

1. Version bump in `Cargo.toml`
2. Update `CHANGELOG.md` (if maintained)
3. Create a git tag: `git tag -a vX.Y.Z -m "Release vX.Y.Z"`
4. Push tag: `git push origin vX.Y.Z`
5. GitHub Actions will build and create release artifacts

## Questions?

Feel free to open an issue for:
- Questions about the codebase
- Help with your contribution
- Clarification on guidelines

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (see [LICENSE](LICENSE)).

---

Thank you for contributing to Innu!
