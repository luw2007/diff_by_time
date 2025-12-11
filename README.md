# dt — Command Execution Time Diff Tool

[![Update Homebrew Tap](https://github.com/luw2007/diff_by_time/actions/workflows/update-tap.yml/badge.svg)](https://github.com/luw2007/diff_by_time/actions/workflows/update-tap.yml)

dt lets you run shell commands, record their outputs over time, and compare results between runs. It provides a simple interactive selector (skim‑style fuzzy filtering) and colored diffs.

> Status: Not production‑ready yet. Interfaces, data layout and behavior may change frequently.
>
> 状态：当前为非正式发布版本，尚未达到生产可用级别；接口、数据结构与行为可能会频繁变更。

Generated previews:

![dt run GIF](docs/gallery/dt-run.gif)


![dt diff GIF](docs/gallery/dt-diff.gif)

## Quick Start

- Build (debug): `cargo build`
- Build (release): `cargo build --release`
- Run examples:
  - `cargo run -- run "ls | head -5"`
  - `cargo run -- diff "ls | head -5"`  # interactive selector + preview/diff
  - `cargo run -- ls`                    # non-interactive listing (use --json for scripts)

## CLI Overview

`dt run <COMMAND>`
- Executes the command (via `sh -c`) and records stdout, stderr, exit code, duration, and metadata.
- Options:
  - `-d, --diff-with <TARGET>`: After recording, immediately show a diff. TARGET can be:
    - `first`: Compare with the earliest execution
    - `last`: Compare with the most recent execution
    - `<CODE>`: Compare with a specific short code (e.g., `a`, `b`, `ab`)

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
  - `--linewise`: Compare strictly line-by-line (no cross-line alignment)

`dt ls [QUERY] [--json]`
- Non-interactive listing of stored command runs, sorted by most recent.
- Accepts optional substring or subsequence `QUERY` filters; omit to show all.
- Use `--json` for machine-readable output (records including timestamps and short codes).

`dt parse [FILE] [--json]`
- Parse a Bash snippet or file into an AST using tree-sitter-bash.
- If `FILE` is omitted, reads from STDIN. Use `--json` to output the AST as JSON; otherwise prints an outline.

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
- Preview scrollbar; help overlay (`h`/`?`); Enter-to-diff from either pane
- Toggle preview target: `o` or `←/→` switches stdout/stderr
- Per-command short codes for quick reference
- Multi-language messages (English/Chinese) with English as default in code and docs
- Packaging scripts and usage docs
  - Non-interactive listing for CI: `dt ls [QUERY] [--json]`
  - Safe cleaning with previews and confirmations; dry-run available for `dt clean search|file`

## Configuration

Config file is at `~/.dt/config.toml`. Environment variables can override display settings.

```toml
[storage]
max_retention_days = 365
auto_archive = true

[display]
max_history_shown = 10
language = "auto"        # auto/en/zh
tui_mode = "interactive" # interactive|simple
alt_screen = true         # Use alternate screen in interactive mode (vim-like)
```

Environment overrides:

```
DT_TUI=interactive|simple   # force TUI mode
DT_ALT_SCREEN=1              # use alt screen in interactive mode (prefer 1)
```

## Interactive UX

- Left panel shows compact items: `code:<a..z> time:<YYYY-MM-DD HH:MM:SS>`
- Right panel preview:
  - Header shows `Path: …` and `Preview: stdout|stderr`
  - Content area supports vertical scrolling with a visible scrollbar
  - Press `o` or `←/→` to toggle stdout/stderr
- Bottom status bar summarizes keys; press `h` or `?` for an overlay of preview shortcuts
- Fuzzy filter: type to filter; substring/prefix/number priority plus skim‑style fuzzy fallback
- Navigation: `j/k` or arrow keys; paging: `PgUp/PgDn` or `Ctrl+f`/`Ctrl+b` (selection list), `Space`/`f` down, `b`/`Backspace` up (preview); half pages: `d`/`u`; top/bottom: `g/G`, `Home/End`
- Selection: `Space`/`Enter` toggle the focused item (also works in preview); `Tab` enters preview; arrow keys in preview jump back to the list; once two items are selected, `Enter` runs the diff immediately
- Back/quit from preview: `q`; quit app: `Q`; global `Esc` backs/exits

## Data Storage

- Records live under `~/.dt/records/<command_hash>/`
- Index file `~/.dt/index` references all records
- Optional yearly archives `~/.dt/index_YYYY.json` when `auto_archive = true`

## Security Notes

- Commands run via `sh -c`. Do not pass untrusted input automatically.
- Examples in tests and docs may show pipelines; quote commands appropriately.

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


## Releasing to Homebrew

- This repo publishes a PR to `luw2007/homebrew-tap` on every GitHub Release via the workflow `update-tap.yml`.
- Configure a repo secret `TAP_PUSH_TOKEN` with write access to the tap repo. Optional variables: `TAP_REPO` (default `luw2007/homebrew-tap`), `TAP_DEFAULT_BRANCH` (default `main`).
- For manual steps and local verification, see `docs/homebrew_formula.md`.


## License

MIT. See `LICENSE` for details.

Third‑party licenses are in `THIRD_PARTY_NOTICES.md` (e.g., fuzzy‑matcher’s MIT license).
