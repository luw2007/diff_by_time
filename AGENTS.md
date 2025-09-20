# Repository Guidelines

## Project Structure & Module Organization
- `src/` Rust 2021 sources: `main.rs` (CLI via clap), `executor.rs` (run commands), `differ.rs` (diff + TUI), `store_manager.rs`/`storage.rs` (persistence under `~/.dt`), `config.rs` (TOML config), `i18n.rs` (EN/ZH strings), `fuzzy_matcher.rs` (Skim‑like matching, no fzf dep).
- `package/` packaging scripts and templates; `package.sh` builds distributables and generates usage/docs.
- `target/` Cargo build output. Runtime data lives in `~/.dt/`.
- Ad‑hoc helpers: `test_package.sh`, `test_interactive.rs`, `test_hash.rs` (may be pruned over time).

## Build, Test, and Development Commands
```bash
cargo build            # Debug build
cargo build --release  # Optimized binary
cargo run -- run "ls | head -5"   # Execute and record
cargo run -- diff "ls | head -5"  # Diff past executions
cargo run -- list --no-merge       # List records
cargo test             # Run unit tests
cargo fmt --all        # Format
cargo clippy -- -D warnings  # Lint as errors
./package.sh && ./test_package.sh  # Package + quick verify
```

## Coding Style & Naming Conventions
- Rust 2021; 4‑space indentation; run `cargo fmt` before pushing.
- Prefer `anyhow::Result` for fallible functions; avoid `unwrap()` in non‑test code.
- Naming: modules/files `snake_case`; types `PascalCase`; functions/vars `snake_case`; CLI flags kebab‑case (e.g., `--no-merge`).
- Localization: update EN and ZH entries in `src/i18n.rs` together. Comments must be English.

## Testing Guidelines
- Unit tests live beside code under `#[cfg(test)]`; name tests `test_*`.
- Focus on pure logic (diffing, matching, formatting); TUI flows require a TTY—skip/guard in CI.
- Aim to cover command normalization, hashing, date‑filter parsing, and diff edge cases.

## Commit & Pull Request Guidelines
- Use Conventional Commits (e.g., `feat:`, `fix:`, `docs:`). Keep messages concise and scoped.
- PRs must include: summary, rationale, before/after notes; link issues; screenshots/GIFs for TUI changes.
- Required checks: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`. Run packaging script for release‑related changes.

## Security & Configuration Tips
- Commands execute via `sh -c`; never pass untrusted input automatically. Document risky examples in tests.
- Data directory: `~/.dt/` (index + records). Config: `~/.dt/config.toml`. Env: `DT_TUI`, `DT_ALT_SCREEN`.
- Packaging generates third‑party notices via `cargo-about` or `cargo-license`; install them locally if missing.
- Status: not production‑ready; interfaces may change quickly—see README for current caveats.

