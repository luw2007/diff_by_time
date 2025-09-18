# dt — Command Execution Time Diff Tool

dt lets you run shell commands, record their outputs over time, and compare results between runs. It provides a simple interactive selector (skim‑style fuzzy filtering) and colored diffs.

## Quick Start

- Build (debug): `cargo build`
- Build (release): `cargo build --release`
- Run examples:
  - `cargo run -- run "ls | head -5"`
  - `cargo run -- diff "ls | head -5"`
  - `cargo run -- list` (compat alias; opens interactive diff selector)

## Gallery

Generate demo GIFs locally using VHS (not stored in git):

```bash
brew install vhs            # or follow VHS installation guide
make gallery               # renders docs/vhs/*.tape to docs/gallery/*.gif
open docs/gallery          # view generated GIFs locally
```

Tapes:
- `docs/vhs/dt-diff.tape` → `docs/gallery/dt-diff.gif`
- `docs/vhs/dt-run.tape`  → `docs/gallery/dt-run.gif`

Notes:
- Generated media under `docs/gallery/` is ignored by git (`.gitignore`).
- Scripts fix terminal size and language for reproducible output.

## CLI Overview

`dt run <COMMAND>`
- Executes the command (via `sh -c`) and records stdout, stderr, exit code, duration, and metadata.
- Options:
  - `-d, --diff-code <CODE>`: After recording, immediately show a diff with the specified short code.

### Shell tip: handle commands with pipes

When a command includes pipes (`|`), redirection (`>`, `<`), or logical operators (`&&`, `||`), wrap the entire command in single or double quotes so the outer shell does not pre-parse it.

```bash
# Recommended
dt run 'ls -l | wc'

# Incorrect: outer shell captures the pipeline first
dt run ls -l | wc
```

`dt diff [OPTIONS] [COMMAND]`
- Interactive selection and comparison of historical runs.
- Options:
  - `--max-shown <N>`: Limit selector viewport rows.

`dt ls [QUERY] [--json]`
- Non-interactive listing of stored command runs, sorted by most recent.
- Accepts optional substring or subsequence `QUERY` filters; omit to show all.
- Use `--json` for machine-readable output (records including timestamps and short codes).

`dt clean <SUBCOMMAND>`
- Clean records by search, by file, or all.
- Subcommands:
  - `search [QUERY]`: Delete records matching QUERY (substring or subsequence). If missing, opens an interactive selector.
  - `file [PATH]`: Delete records related to PATH (matching working directory, absolute/relative occurrences in commands).
  - `all`: Delete all records.
- Safety confirmations:
  - All deletes require confirmation: type `YES`.
  - Type `ALL` to confirm and skip further confirmations during this dt process (session-wide).

Run `dt <COMMAND> --help` to see detailed usage for any subcommand.

## Features

- Skim‑style fuzzy search for interactive selection (no external `fzf` required)
- Colored diffs using `similar`
- Per-command short codes for quick reference
- Multi-language messages (English/Chinese) with English as default in code and docs
- Packaging scripts and usage docs
 - Non-interactive listing for CI: `dt ls [QUERY] [--json]`
 - Safe cleaning with previews and confirmations; dry-run available for `dt clean search|file`

## Configuration

Config file is at `~/.dt/config.toml`. Environment variables can override some display settings.

```toml
[storage]
max_retention_days = 365
auto_archive = true

[display]
max_history_shown = 10
language = "auto"        # auto/en/zh
tui_mode = "interactive" # interactive|simple
alt_screen = false        # Use alternate screen in interactive mode
```

## Interactive UX

- Left panel shows compact items: `code:<a..z> time:<YYYY-MM-DD HH:MM:SS>`
- Right panel preview:
  - Row 1 = `Path: …` (truncated to width)
  - Row 2 = `Preview: stdout|stderr`
  - File content follows; press `o` or `←/→` to toggle stdout/stderr
- Bottom status bar shows concise help: selection step, `Filter: <input>`, and `j/k ↑/↓ Enter Del Esc`
- Fuzzy filter: type to filter; uses substring/prefix/number priority plus skim fuzzy fallback
- Navigation: `j/k` or arrow keys; `PgUp/PgDn`; `Ctrl-a/e` (home/end)
- Selection: `Enter`
- Editing: `Backspace`, `Delete`, `Ctrl-u` (clear), `Ctrl-w` (delete word)
- Exit: `Esc`; `Ctrl-c/d` also exits gracefully from interactive views

## Data Storage

- Records live under `~/.dt/records/<command_hash>/`
- Index file `~/.dt/index` references all records
- Optional yearly archives `~/.dt/index_YYYY.json` when `auto_archive = true`

## Security Notes

- Commands run via `sh -c`. Do not pass untrusted input automatically.
- Examples in tests and docs may show pipelines; quote commands appropriately.

## License

MIT. See `LICENSE` for details.

Third‑party licenses are in `THIRD_PARTY_NOTICES.md` (e.g., fuzzy‑matcher’s MIT license).
