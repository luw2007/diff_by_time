# CLAUDE.md (dt - diff_by_time)

This file provides guidance for development and architecture overview of the `dt` repository.

## Project Overview

`dt` is a Rust-based CLI tool designed to run shell commands, record their outputs over time, and provide an interactive TUI (Terminal User Interface) to compare results between different executions. It categorizes records by hashing the normalized command string.

## Common Development Commands

### Building and Testing
```bash
cargo build                      # Debug build
cargo build --release            # Release build
cargo test                       # Run all tests
cargo run -- run "ls -l"         # Run debug binary with command
cargo run -- diff "ls -l"        # Interactive diff selector
cargo run -- ls                  # List all recorded commands
cargo clippy -- -D warnings      # Lint code
cargo fmt --all                  # Format code
```

### Packaging and Maintenance
```bash
./package.sh                     # Create distributable packages
make gallery                     # Generate demo GIFs from VHS tapes
make tap-formula                 # Generate Homebrew formula
cargo run -- rebuild             # Rebuild index from records
```

## Architecture Overview

### Core Components

1.  **CLI Entry (`src/main.rs`)**: Uses `clap` for subcommands: `run`, `diff`, `show`, `ls`, `clean`, `parse`, `rebuild`.
2.  **Execution Engine (`src/executor.rs`)**: Spawns `sh -c` processes, captures and streams stdout/stderr live, records timing and exit codes.
3.  **Storage Layer (`src/storage.rs`, `src/store_manager.rs`)**: 
    - Data stored in `~/.dt/records/<command_hash>/`.
    - Each execution creates JSON metadata and separate text files for stdout/stderr.
    - Uses short codes (base62) for quick reference.
4.  **Interactive TUI (`src/differ.rs`)**: 
    - Built with `ratatui` and `crossterm`.
    - Features fuzzy search, dual-pane layout, and ANSI sanitization.
5.  **Diff Engine (`src/differ.rs`)**: Uses `similar` crate for text comparisons.
6.  **I18n (`src/i18n.rs`)**: Supports English and Chinese via `i18n.t()`.
7.  **Bash Parser (`src/bash_parser.rs`)**: Uses `tree-sitter-bash` to parse commands into AST.

### Key Design Patterns
- **Command Normalization**: Normalizes spaces and pipes before hashing.
- **Streaming Output**: Live streaming of stdout/stderr using threads.
- **Config Hierarchy**: Environment variables (`DT_TUI`, `DT_ALT_SCREEN`) > `config.toml` > defaults.

## Coding Style & Patterns
- **Language**: Rust (Edition 2021).
- **Error Handling**: Use `anyhow::Result` for propagation; avoid `unwrap()` in core logic.
- **Naming**: Standard Rust conventions (PascalCase for types, snake_case for others).
- **I18n**: All user-facing strings must be internationalized.

## Testing Guidelines
- Use `tempfile` for directory isolation.
- Integration tests should use `--data-dir` to avoid polluting user data.
- Unit tests are located in `#[cfg(test)] mod tests` blocks within modules.

## Security Considerations
- **Command Execution**: Commands are run via `sh -c` without sanitization. Do not pass untrusted input.
- **Data Privacy**: Records are stored locally in `~/.dt`. Be aware that sensitive outputs (passwords, tokens) will be persisted.
- **Isolation**: Use `--data-dir` to separate different environments or for testing.

## Configuration Details

Config file at `~/.dt/config.toml`:
```toml
[storage]
max_retention_days = 365
auto_archive = true

[display]
max_history_shown = 10
language = "auto"        # auto/en/zh
tui_mode = "interactive" # interactive|simple
alt_screen = true        # Use alternate screen
```

## Important File Locations
- Main Entry: `src/main.rs`
- TUI Logic: `src/differ.rs`
- Storage Manager: `src/store_manager.rs`
- Executor: `src/executor.rs`
- Models: `src/storage.rs`
