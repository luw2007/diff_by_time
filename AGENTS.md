# Repository Guidelines

## Project Structure & Modules
- `src/` Rust 2021 source: `main.rs` (CLI entry via clap), `executor.rs` (runs commands), `differ.rs` (diff + TUI), `store_manager.rs` and `storage.rs` (persistence under `~/.dt`), `config.rs` (TOML config), `i18n.rs` (EN/ZH strings), `fuzzy_matcher.rs` (Skim-like matching, no external fzf dependency).
- `package/` packaging artifacts and release notes; `package.sh` builds a distributable and usage docs.
- `target/` Cargo build output. Runtime data is written to `~/.dt/`.
- Root helpers: `test_package.sh`, `test_interactive.rs`, `test_hash.rs` (ad‑hoc utilities).

## Build, Test, and Development
- Build debug/release:
  ```bash
  cargo build
  cargo build --release
  ```
- Run locally (examples):
  ```bash
  cargo run -- run "ls | head -5"
  cargo run -- diff "ls | head -5"
  cargo run -- list --no-merge
  ```
- Unit tests (in `src/`):
  ```bash
  cargo test
  ```
- Lint/format:
  ```bash
  cargo fmt --all
  cargo clippy -- -D warnings
  ```
- Package + quick verify:
  ```bash
  ./package.sh
  ./test_package.sh
  ```

## Coding Style & Naming
- Rust 2021; 4‑space indentation. Run `cargo fmt` before pushing.
- Keep `anyhow::Result` for fallible functions; avoid `unwrap()` in non‑test code.
- Names: modules/files `snake_case`, types `PascalCase`, functions/vars `snake_case`. CLI flags use kebab‑case (e.g., `--no-merge`).
- Localization: update both EN and ZH keys in `src/i18n.rs` together.
- Comments: English only for all source code comments (project override).

## Testing Guidelines
- Prefer unit tests co‑located under `#[cfg(test)]` (see `src/fuzzy_matcher.rs`). Use `test_*` naming.
- Interactive TUI paths require a TTY; focus tests on pure logic (diffing, matching, formatting). Skip or guard TUI flows in CI.
- Aim to cover command normalization, hashing, and diff output edge cases.

## Commit & Pull Request Guidelines
- Use Conventional Commits (e.g., `feat:`, `fix:`, `docs:`)—see repo history.
- PRs must include: concise summary, rationale, before/after notes; link issues; screenshots/GIFs for TUI changes.
- Required checks: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, and packaging script if release‑related.
- If CLI or config changes, update `package/USAGE.md` generation in `package.sh` and relevant messages in `i18n.rs`.

## Security & Configuration
- Commands execute via `sh -c`; never pass untrusted input automatically. Document risky examples in tests.
- Data lives in `~/.dt/` (index + records). Config at `~/.dt/config.toml`; env overrides: `DT_TUI`, `DT_ALT_SCREEN`.
