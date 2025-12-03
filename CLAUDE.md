# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`dt` is a Rust CLI tool that runs shell commands, records their outputs over time, and provides interactive comparison between runs. It features a sophisticated TUI built with ratatui, fuzzy search, and colored diffs.

## Common Development Commands

### Building and Testing
```bash
cargo build                      # Debug build
cargo build --release            # Release build
cargo test                       # Run all tests
cargo run -- run "ls -l"         # Run debug binary with command
cargo run -- diff "ls -l"        # Interactive diff selector
cargo run -- ls                  # List all recorded commands
```

### Code Quality
```bash
cargo fmt --all -- --check       # Check formatting
cargo clippy -- -D warnings      # Lint with warnings as errors
```

### Packaging and Distribution
```bash
./package.sh                     # Create distributable packages
make gallery                     # Generate demo GIFs from VHS tapes
make tap-formula                 # Generate Homebrew formula
```

### Testing Specific Commands
```bash
# Run with custom data directory (useful for testing)
cargo run -- --data-dir /tmp/dt-test run "echo test"

# Test with environment overrides
DT_TUI=simple cargo run -- diff
DT_ALT_SCREEN=0 cargo run -- diff
```

## Architecture Overview

### Core Components

1. **Command Execution (`executor.rs`)**
   - Spawns `sh -c <command>` processes
   - Streams stdout/stderr live to terminal using threads
   - Records timing, exit codes, and working directory
   - Normalizes commands by formatting (removes extra spaces around pipes)

2. **Storage Layer (`store_manager.rs`, `storage.rs`)**
   - Data stored in `~/.dt/records/<command_hash>/`
   - Each execution creates 3 files: `meta_<timestamp>.json`, `stdout_<timestamp>.txt`, `stderr_<timestamp>.txt`
   - Maintains a global index file at `~/.dt/index`
   - Optional yearly archives when `auto_archive = true`
   - Short codes are bijective base62 (a-z, A-Z, 0-9), assigned per command hash
   - Commands are hashed using SHA256 after normalization

3. **Interactive TUI (`differ.rs`)**
   - Built with ratatui and crossterm
   - Two-pane layout: left = selection list, right = preview pane
   - Fuzzy search using custom skim-style matcher (`fuzzy_matcher.rs`)
   - Supports stdout/stderr toggle with `o` or arrow keys
   - Sanitizes ANSI escape sequences and control characters for safe preview
   - Two modes: "interactive" (full TUI with alternate screen) and "simple" (minimal)
   - Environment variables `DT_TUI` and `DT_ALT_SCREEN` override config settings
   - **Paging shortcuts**:
     - Selection list: `PageUp/PageDown` or `Ctrl+f`/`Ctrl+b` (dynamic page size based on terminal height)
     - Preview pane: `Space`/`f` (down), `b`/`Backspace` (up), `d`/`u` (half page), `g`/`G` (top/bottom)

4. **Diff Engine (`differ.rs`)**
   - Uses `similar` crate for text diffing
   - Two diff modes: default (cross-line alignment) and `--linewise` (strict line-by-line)
   - Color-coded output: green for additions, red for deletions, dimmed for context
   - Diff is computed between two selected executions sorted by timestamp

5. **Internationalization (`i18n.rs`)**
   - Supports English and Chinese
   - Language detection from `LANG` env var or config
   - All user-facing messages go through `i18n.t()` or `i18n.t_format()`

6. **Configuration (`config.rs`)**
   - Config file at `~/.dt/config.toml`
   - Settings: retention days, auto-archive, language, TUI mode, alt screen
   - Auto-creates default config on first run

### Data Flow

```
User runs: dt run "ls -l"
    ↓
executor.rs: spawns sh -c "ls -l", streams output
    ↓
storage.rs: CommandExecution created with stdout/stderr/metadata
    ↓
store_manager.rs: assigns short code, saves to ~/.dt/records/<hash>/
    ↓
Updates global index ~/.dt/index

User runs: dt diff "ls -l"
    ↓
differ.rs: loads all executions for command hash
    ↓
Interactive TUI: user selects two executions
    ↓
similar crate: computes diff
    ↓
Colored output printed to terminal
```

### Key Design Patterns

1. **Command Normalization**: Commands are normalized before hashing to ensure `ls|wc` and `ls | wc` produce the same hash
2. **Streaming Output**: stdout/stderr are streamed live during execution, not buffered
3. **Short Codes**: Minimal unique codes (a, b, c, ..., aa, ab, ...) assigned per command for easy reference
4. **Safe Preview**: All text in TUI preview is sanitized to strip ANSI codes and control chars
5. **Config Hierarchy**: Environment variables > config file > defaults

## Testing Patterns

- Use `tempfile` for creating temporary directories in tests
- Integration tests use `--data-dir` to isolate test data
- Test helper functions live in `#[cfg(test)] mod tests` blocks
- Mock I18n with default English language for consistent test output

## Important File Locations

- Main entry: `src/main.rs` - CLI parsing and command dispatch
- TUI logic: `src/differ.rs` - All interactive selection and diff display (~3500 lines)
- Storage: `src/store_manager.rs` - File I/O and indexing
- Execution: `src/executor.rs` - Command spawning and streaming
- Data models: `src/storage.rs` - Structs for CommandRecord and CommandExecution

## Shell Command Handling

Commands execute via `sh -c`, which means:
- Pipes, redirects, and logical operators work naturally when quoted
- Users must quote complex commands: `dt run 'ls | wc'` not `dt run ls | wc`
- Security: never pass untrusted input to `dt run`

## Configuration Details

Config lives at `~/.dt/config.toml`:
```toml
[storage]
max_retention_days = 365
auto_archive = true

[display]
max_history_shown = 10
language = "auto"        # auto/en/zh
tui_mode = "interactive" # interactive|simple
alt_screen = true        # Use alternate screen (vim-like)
```

Environment overrides:
- `DT_TUI=interactive|simple` - Force TUI mode
- `DT_ALT_SCREEN=1|0` - Enable/disable alternate screen
- `LANG` - Auto-detects language for i18n

## Bash Parser (`bash_parser.rs`)

- Uses `tree-sitter-bash` to parse shell commands
- Available via `dt parse [FILE]` command
- Outputs AST as JSON with `--json` flag
- Used for potential future features (not core to current functionality)
