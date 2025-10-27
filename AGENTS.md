# Repository Guidelines

## Project Structure & Module Organization

- **Source code**: `src/` - Core application modules including main CLI logic, TUI, storage, and command execution
- **Binary entry**: `src/main.rs` - Main application entry point with command parsing
- **Documentation**: `docs/` - Contains VHS tapes for demo GIF generation
- **Packaging**: `package/` - Scripts and templates for Homebrew formula generation
- **Build artifacts**: `target/` - Rust build outputs (gitignored)
- **Configuration**: `~/.dt/` - Runtime data storage and configuration files

## Build, Test, and Development Commands

- `cargo build` - Build debug binary
- `cargo build --release` - Build optimized release binary
- `cargo test` - Run all tests
- `cargo fmt --all -- --check` - Check code formatting
- `cargo clippy -- -D warnings` - Lint code with clippy
- `./package.sh` - Create distributable packages with binaries and docs
- `make gallery` - Generate demo GIFs from VHS tapes in `docs/vhs/`

## Coding Style & Naming Conventions

- **Language**: Rust 2021 edition
- **Indentation**: 4 spaces
- **Naming conventions**:
  - Modules and files: `snake_case`
  - Types and structs: `PascalCase`
  - Functions and variables: `snake_case`
- **Error handling**: Use `anyhow::Result` for error propagation, avoid `unwrap()` in non-test code
- **Documentation**: Comments in English, maintain i18n strings for Chinese/English support
- **Architecture**: Follow SOLID principles and domain-driven design patterns

## Testing Guidelines

- **Framework**: Built-in Rust `cargo test`
- **Test location**: Integration tests in project root, unit tests within modules
- **Coverage**: Ensure comprehensive coverage for core functionality
- **Test naming**: Use descriptive names following Rust conventions
- **Test data**: Use temporary directories and clean up after tests

## Commit & Pull Request Guidelines

- **Commit messages**: Use conventional commit format (feat:, fix:, docs:, etc.)
- **PR requirements**:
  - Clear description of changes
  - Link to relevant issues when applicable
  - Pass all CI checks (format, clippy, tests)
  - Update documentation for new features
- **Branch naming**: Use feature/ or fix/ prefixes
- **Review process**: At least one approval required for merging

## Security & Configuration Tips

- **Command execution**: Commands run via `sh -c` - never pass untrusted input
- **Data storage**: Runtime data stored in `~/.dt/` with configurable retention
- **Environment isolation**: Use `--data-dir` for testing/demo environments
- **Configuration**: Override display settings via environment variables (`DT_TUI`, `DT_ALT_SCREEN`)