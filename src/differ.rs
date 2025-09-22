use crate::fuzzy_matcher::SkimMatcher;
use crate::i18n::I18n;
use crate::storage::CommandExecution;
use crate::store_manager::StoreManager;
use anyhow::Result;
use chrono::{DateTime, Datelike};
use colored::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Terminal,
};
use regex::Regex;
use similar::{ChangeTag, TextDiff};
use std::io::{self, Write};
use std::sync::OnceLock;
use unicode_width::UnicodeWidthChar;

pub struct Differ;

struct CommandGroup {
    command_hash: String,
    command: String,
    count: usize,
    latest: chrono::DateTime<chrono::Utc>,
}

// legacy terminal size constants removed (ratatui handles layout)

// legacy preview enums removed in ratatui rewrite

impl Differ {
    /// Sanitize text for safe TUI preview rendering:
    /// - Strip ANSI escape sequences (CSI/SGR common forms)
    /// - Convert carriage return `\r` to newline to preserve progress updates
    /// - Drop other C0 control chars (except `\n`), expand tabs to spaces
    fn sanitize_for_preview(text: &str) -> String {
        // Compile ANSI regex once
        static ANSI_RE: OnceLock<Regex> = OnceLock::new();
        let re = ANSI_RE.get_or_init(|| {
            // Covers ESC [ ... command (CSI sequences)
            let pattern = r"\x1B\[[0-?]*[ -/]*[@-~]";
            Regex::new(pattern).expect("valid ansi regex")
        });

        // Step 1: normalize carriage returns into newlines (typical for spinners/progress)
        let mut s = text.replace('\r', "\n");
        // Step 2: strip ANSI sequences
        s = re.replace_all(&s, "").into_owned();
        // Step 3: remove other control chars and expand tabs
        let mut cleaned = String::with_capacity(s.len());
        for ch in s.chars() {
            match ch {
                '\n' => cleaned.push('\n'),
                '\t' => cleaned.push_str("    "),
                _ if ch.is_control() => {
                    // drop other control chars such as \x08 (backspace), \x07 (bell), etc.
                }
                _ => cleaned.push(ch),
            }
        }
        cleaned
    }
    fn is_backspace_event(key: &KeyEvent) -> bool {
        matches!(key.code, KeyCode::Backspace)
            || matches!(key.code, KeyCode::Char(c) if c as u32 == 8 || c as u32 == 127)
    }

    // removed: shift+backspace handling from legacy UI

    pub fn select_prefix_for_clean(
        store: &StoreManager,
        i18n: &I18n,
        tui_simple: bool,
        use_alt_screen: bool,
        max_viewport: Option<usize>,
    ) -> Result<Option<String>> {
        let records = store.get_all_records()?;
        if records.is_empty() {
            println!("{}", i18n.t("no_records").yellow());
            return Ok(None);
        }

        let groups = Self::build_command_groups(&records);
        let result = if tui_simple {
            // simple: print list and ask index
            println!("{}", i18n.t("select_clean_command"));
            for (i, g) in groups.iter().enumerate() {
                let dt = g
                    .latest
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M:%S");
                println!(
                    "{}: {} ({}: {}, {}: {})",
                    i + 1,
                    g.command,
                    i18n.t("count_label"),
                    g.count,
                    i18n.t("latest_label"),
                    dt
                );
            }
            println!("{}", i18n.t("input_numbers"));
            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_err() {
                None
            } else {
                let s = input.trim();
                if let Ok(idx) = s.parse::<usize>() {
                    if idx > 0 && idx <= groups.len() {
                        Some(groups[idx - 1].command.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        } else {
            Self::interactive_select_command_string_ratatui(
                &groups,
                i18n,
                use_alt_screen,
                max_viewport,
            )
        };
        Ok(result)
    }

    fn interactive_select_command_string_ratatui(
        groups: &[CommandGroup],
        i18n: &I18n,
        use_alt_screen: bool,
        _max_viewport: Option<usize>,
    ) -> Option<String> {
        if terminal::enable_raw_mode().is_err() {
            return None;
        }
        let mut stdout = io::stdout();
        if use_alt_screen {
            let _ = crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen);
        }
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).ok()?;
        // Ensure we start from a clean frame even without alt-screen
        let _ = terminal.clear();

        let mut filter_input = String::new();
        let mut current_selection: usize = 0;

        let compute = |filter: &str| -> Vec<usize> {
            if filter.is_empty() {
                return (0..groups.len()).collect();
            }
            let items: Vec<(usize, String)> = groups
                .iter()
                .enumerate()
                .map(|(i, g)| {
                    let dt = g
                        .latest
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M:%S");
                    (i, format!("{} {} {} {}", g.command, g.count, dt, i + 1))
                })
                .collect();
            let m = SkimMatcher::new();
            m.match_and_sort(filter, items)
                .into_iter()
                .map(|(i, _, _)| i)
                .collect()
        };
        let mut filtered = compute("");

        let draw = |f: &mut ratatui::Frame,
                    i18n: &I18n,
                    groups: &[CommandGroup],
                    filter: &str,
                    filtered: &Vec<usize>,
                    sel: usize| {
            let root = f.size();
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(root);
            // Full-frame clear prevents artifacts mixing with prior shell output
            f.render_widget(Clear, root);
            let header = Paragraph::new(Line::from(vec![
                Span::raw(i18n.t("select_clean_command")),
                Span::raw("  |  "),
                Span::raw(i18n.t("status_filter")),
                Span::raw(": "),
                Span::raw(filter),
            ]));
            f.render_widget(header, rows[0]);

            let mut items: Vec<ListItem> = Vec::new();
            for (vis, &idx) in filtered.iter().enumerate() {
                let g = &groups[idx];
                let dt = g
                    .latest
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M:%S");
                let line = format!(
                    "{}: {} ({}: {}, {}: {})",
                    vis + 1,
                    g.command,
                    i18n.t("count_label"),
                    g.count,
                    i18n.t("latest_label"),
                    dt
                );
                items.push(ListItem::new(line));
            }
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(i18n.t("select_clean_command")),
                )
                .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));
            let mut state = ratatui::widgets::ListState::default();
            if !filtered.is_empty() {
                state.select(Some(sel));
            }
            f.render_stateful_widget(list, rows[1], &mut state);

            let foot =
                Paragraph::new(i18n.t("navigate_hint")).style(Style::default().fg(Color::Gray));
            f.render_widget(foot, rows[2]);
        };

        let res = loop {
            let _ = terminal
                .draw(|f| draw(f, i18n, groups, &filter_input, &filtered, current_selection));
            match event::read().ok()? {
                Event::Key(k) => {
                    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                    match k.code {
                        KeyCode::Esc => break None,
                        KeyCode::Up | KeyCode::Char('k') => {
                            current_selection = current_selection.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if current_selection + 1 < filtered.len() {
                                current_selection += 1;
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(&idx) = filtered.get(current_selection) {
                                break Some(groups[idx].command.clone());
                            }
                        }
                        KeyCode::Backspace | KeyCode::Delete => {
                            filter_input.pop();
                            filtered = compute(&filter_input);
                            if current_selection >= filtered.len() {
                                current_selection = filtered.len().saturating_sub(1);
                            }
                        }
                        KeyCode::Char(c) if !ctrl => {
                            filter_input.push(c);
                            filtered = compute(&filter_input);
                            current_selection = 0;
                        }
                        KeyCode::Char('c') if ctrl => break None,
                        _ => {}
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        };

        let mut out = io::stdout();
        if use_alt_screen {
            let _ = crossterm::execute!(out, crossterm::terminal::LeaveAlternateScreen);
        }
        let _ = terminal::disable_raw_mode();
        res
    }

    // removed legacy crossterm command selector block
    /*
    fn interactive_select_command_string(
        groups: &[CommandGroup],
        i18n: &I18n,
        use_alt_screen: bool,
        max_viewport: Option<usize>,
    ) -> Option<String> {
        if terminal::enable_raw_mode().is_err() {
            return None;
        }
        let mut stdout = io::stdout();
        if use_alt_screen {
            print!("\x1b[?1049h");
        }
        print!("\x1b[?7l\x1b[?25l");
        stdout.flush().ok();

        let mut filter_input = String::new();
        let mut current_selection = 0usize;
        let mut scroll_offset = 0usize;
        let fuzzy = SkimMatcher::new();

        loop {
            print!("\x1b[2J\x1b[H");
            // Enforce minimum terminal size before rendering UI
            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
            if cols < MIN_TERMINAL_COLS || rows < MIN_TERMINAL_ROWS {
                let warn = i18n.t_format(
                    "terminal_too_small",
                    &[
                        &MIN_TERMINAL_COLS.to_string(),
                        &MIN_TERMINAL_ROWS.to_string(),
                        &cols.to_string(),
                        &rows.to_string(),
                    ],
                );
                print!("{}\r\n", warn);
                stdout.flush().ok();
                std::thread::sleep(std::time::Duration::from_millis(300));
                continue;
            }
            print!("{}\r\n", i18n.t("select_clean_command"));
            // Terminal width for prompt truncation
            let (cols, _rows) = crossterm::terminal::size().unwrap_or((80, 24));
            let prompt =
                Self::truncate_for_column(&i18n.t("interactive_filter"), cols as usize - 2);
            print!("{}: ", prompt);
            print!("{}\r\n\r\n", filter_input);

            let items: Vec<(usize, String)> = groups
                .iter()
                .enumerate()
                .map(|(i, g)| {
                    let dt = g
                        .latest
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string();
                    let text = format!("{} {} {} {}", g.command, g.count, dt, i + 1);
                    (i, text)
                })
                .collect();
            let filtered_indices: Vec<usize> = if filter_input.is_empty() {
                (0..groups.len()).collect()
            } else {
                let matched = fuzzy.match_and_sort(&filter_input, items);
                matched.into_iter().map(|(i, _, _)| i).collect()
            };

            if filtered_indices.is_empty() {
                print!("\x1b[31m{}\x1b[0m\r\n", i18n.t("no_matches"));
            } else {
                if current_selection >= filtered_indices.len() {
                    current_selection = filtered_indices.len().saturating_sub(1);
                }
                let viewport = if let Some(v) = max_viewport {
                    v.max(3)
                } else {
                    let (_cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                    let reserved = 6usize;
                    let mut v = rows as usize;
                    v = v.saturating_sub(reserved);
                    if v < 5 {
                        v = 5;
                    }
                    v
                };
                if current_selection < scroll_offset {
                    scroll_offset = current_selection;
                } else if current_selection >= scroll_offset + viewport {
                    scroll_offset = current_selection + 1 - viewport;
                }
                let end = (scroll_offset + viewport).min(filtered_indices.len());
                for (list_idx, gi_ref) in filtered_indices
                    .iter()
                    .enumerate()
                    .skip(scroll_offset)
                    .take(end - scroll_offset)
                {
                    let gi = *gi_ref;
                    let g = &groups[gi];
                    let dt = g
                        .latest
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M:%S");
                    let line = format!(
                        "{}: {} ({}: {}, {}: {})",
                        gi + 1,
                        g.command,
                        i18n.t("count_label"),
                        g.count,
                        i18n.t("latest_label"),
                        dt
                    );
                    if list_idx == current_selection {
                        print!("\x1b[44;37m{}\x1b[0m\x1b[K\r\n", line);
                    } else {
                        print!("{}\x1b[K\r\n", line);
                    }
                }
            }

            print!("\r\n");
            let hint = Self::truncate_for_column(&i18n.t("navigate_hint"), cols as usize);
            print!("\x1b[90m{}\x1b[0m\r\n", hint);
            stdout.flush().ok();

            if let Ok(Event::Key(key)) = event::read() {
                let is_ctrl_combo = key.modifiers.contains(KeyModifiers::CONTROL);
                let is_ctrl_char =
                    matches!(key.code, KeyCode::Char(c) if c == '\u{3}' || c == '\u{4}');
                if is_ctrl_combo || is_ctrl_char {
                    let exit_match = match key.code {
                        KeyCode::Char('c')
                        | KeyCode::Char('C')
                        | KeyCode::Char('d')
                        | KeyCode::Char('D') => true,
                        KeyCode::Char(cc) if cc == '\u{3}' || cc == '\u{4}' => true,
                        _ => false,
                    };
                    if exit_match {
                        if use_alt_screen {
                            print!("\x1b[?1049l");
                        }
                        print!("\x1b[?7h\x1b[?25h");
                        stdout.flush().ok();
                        let _ = terminal::disable_raw_mode();
                        return None;
                    }
                }
                match key.code {
                    // Up/Down and vi-keys
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                        current_selection = current_selection.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                        if !filtered_indices.is_empty()
                            && current_selection < filtered_indices.len() - 1
                        {
                            current_selection += 1;
                        }
                    }
                    // Ctrl-p / Ctrl-n
                    KeyCode::Char('p') if is_ctrl_combo => {
                        current_selection = current_selection.saturating_sub(1);
                    }
                    KeyCode::Char('n') if is_ctrl_combo => {
                        if !filtered_indices.is_empty()
                            && current_selection < filtered_indices.len() - 1
                        {
                            current_selection += 1;
                        }
                    }
                    // PageDown / Ctrl-f, PageUp / Ctrl-b
                    KeyCode::PageDown | KeyCode::Char('f') if is_ctrl_combo => {
                        if !filtered_indices.is_empty() {
                            let (_cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                            let reserved = 6usize;
                            let mut viewport = rows as usize;
                            viewport = viewport.saturating_sub(reserved);
                            if viewport < 5 {
                                viewport = 5;
                            }
                            let max_idx = filtered_indices.len().saturating_sub(1);
                            current_selection = (current_selection + viewport).min(max_idx);
                        }
                    }
                    KeyCode::PageUp | KeyCode::Char('b') if is_ctrl_combo => {
                        if !filtered_indices.is_empty() {
                            let (_cols, rows) = terminal::size().unwrap_or((80, 24));
                            let reserved = 6usize;
                            let mut viewport = rows as usize;
                            viewport = viewport.saturating_sub(reserved);
                            if viewport < 5 {
                                viewport = 5;
                            }
                            current_selection = current_selection.saturating_sub(viewport);
                        }
                    }
                    // Home/End and Ctrl-a / Ctrl-e
                    KeyCode::Home | KeyCode::Char('a') if is_ctrl_combo => {
                        current_selection = 0;
                    }
                    KeyCode::End | KeyCode::Char('e') if is_ctrl_combo => {
                        if !filtered_indices.is_empty() {
                            current_selection = filtered_indices.len() - 1;
                        }
                    }
                    KeyCode::Enter => {
                        if !filtered_indices.is_empty() {
                            let gi = filtered_indices[current_selection];
                            if use_alt_screen {
                                print!("\x1b[?1049l");
                            }
                            print!("\x1b[?7h\x1b[?25h");
                            stdout.flush().ok();
                            let _ = terminal::disable_raw_mode();
                            return Some(groups[gi].command.clone());
                        }
                    }
                    // Clear query / delete word / delete to end
                    KeyCode::Char('u') if is_ctrl_combo => {
                        filter_input.clear();
                        current_selection = 0;
                        scroll_offset = 0;
                    }
                    KeyCode::Char('w') if is_ctrl_combo => {
                        while filter_input.ends_with(char::is_whitespace) {
                            filter_input.pop();
                        }
                        while !filter_input.is_empty()
                            && !filter_input.ends_with(char::is_whitespace)
                        {
                            filter_input.pop();
                        }
                        current_selection = 0;
                        scroll_offset = 0;
                    }
                    _ if Self::is_backspace_event(&key) => {
                        filter_input.pop();
                        current_selection = 0;
                        scroll_offset = 0;
                    }
                    KeyCode::Delete => {
                        filter_input.clear();
                        current_selection = 0;
                        scroll_offset = 0;
                    }
                    KeyCode::Esc => {
                        if use_alt_screen {
                            print!("\x1b[?1049l");
                        }
                        print!("\x1b[?7h\x1b[?25h");
                        stdout.flush().ok();
                        let _ = terminal::disable_raw_mode();
                        return None;
                    }
                    KeyCode::Char(c) => {
                        filter_input.push(c);
                        current_selection = 0;
                        scroll_offset = 0;
                    }
                    _ => {}
                }
            }
        }
    }
    */

    pub fn select_file_for_clean(
        files: &[std::path::PathBuf],
        i18n: &I18n,
        tui_simple: bool,
        use_alt_screen: bool,
        _max_viewport: Option<usize>,
    ) -> Result<Option<std::path::PathBuf>> {
        if files.is_empty() {
            return Ok(None);
        }
        if tui_simple {
            println!("{}", i18n.t("select_clean_file"));
            for (i, p) in files.iter().enumerate() {
                println!("{}: {}", i + 1, p.display());
            }
            println!("{}", i18n.t("input_numbers"));
            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_err() {
                return Ok(None);
            }
            let s = input.trim();
            if let Ok(idx) = s.parse::<usize>() {
                if idx > 0 && idx <= files.len() {
                    return Ok(Some(files[idx - 1].clone()));
                }
            }
            return Ok(None);
        }

        if terminal::enable_raw_mode().is_err() {
            return Ok(None);
        }
        let mut stdout = io::stdout();
        if use_alt_screen {
            let _ = crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen);
        }
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).ok().unwrap();

        let mut filter_input = String::new();
        let mut current_selection: usize = 0;

        let compute = |filter: &str| -> Vec<usize> {
            if filter.is_empty() {
                return (0..files.len()).collect();
            }
            let items: Vec<(usize, String)> = files
                .iter()
                .enumerate()
                .map(|(i, p)| (i, p.display().to_string()))
                .collect();
            let m = SkimMatcher::new();
            m.match_and_sort(filter, items)
                .into_iter()
                .map(|(i, _, _)| i)
                .collect()
        };
        let mut filtered = compute("");

        let draw = |f: &mut ratatui::Frame,
                    i18n: &I18n,
                    files: &[std::path::PathBuf],
                    filter: &str,
                    filtered: &Vec<usize>,
                    sel: usize| {
            let root = f.size();
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(root);
            let header = Paragraph::new(Line::from(vec![
                Span::styled(
                    i18n.t("select_clean_file"),
                    Style::default().fg(Color::Gray),
                ),
                Span::raw("  "),
                Span::raw(i18n.t("status_filter")),
                Span::raw(": "),
                Span::raw(filter),
            ]));
            f.render_widget(header, rows[0]);

            let mut items: Vec<ListItem> = Vec::new();
            for (vis, &idx) in filtered.iter().enumerate() {
                items.push(ListItem::new(format!(
                    "{}: {}",
                    vis + 1,
                    files[idx].display()
                )));
            }
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(i18n.t("select_clean_file")),
                )
                .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));
            let mut state = ratatui::widgets::ListState::default();
            if !filtered.is_empty() {
                state.select(Some(sel));
            }
            f.render_stateful_widget(list, rows[1], &mut state);

            let foot =
                Paragraph::new(i18n.t("navigate_hint")).style(Style::default().fg(Color::Gray));
            f.render_widget(foot, rows[2]);
        };

        let res = loop {
            let _ = terminal
                .draw(|f| draw(f, i18n, files, &filter_input, &filtered, current_selection));
            match event::read().ok() {
                Some(Event::Key(k)) => {
                    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                    match k.code {
                        KeyCode::Esc => break None,
                        KeyCode::Up | KeyCode::Char('k') => {
                            current_selection = current_selection.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if current_selection + 1 < filtered.len() {
                                current_selection += 1;
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(&idx) = filtered.get(current_selection) {
                                break Some(files[idx].clone());
                            }
                        }
                        KeyCode::Backspace | KeyCode::Delete => {
                            filter_input.pop();
                            filtered = compute(&filter_input);
                            if current_selection >= filtered.len() {
                                current_selection = filtered.len().saturating_sub(1);
                            }
                        }
                        KeyCode::Char(c) if !ctrl => {
                            filter_input.push(c);
                            filtered = compute(&filter_input);
                            current_selection = 0;
                        }
                        KeyCode::Char('c') if ctrl => break None,
                        _ => {}
                    }
                }
                Some(Event::Resize(_, _)) => {}
                _ => {}
            }
        };

        let mut out = io::stdout();
        if use_alt_screen {
            let _ = crossterm::execute!(out, crossterm::terminal::LeaveAlternateScreen);
        }
        let _ = terminal::disable_raw_mode();
        Ok(res)
    }
    pub fn command_then_diff_flow(
        store: &StoreManager,
        i18n: &I18n,
        tui_simple: bool,
        use_alt_screen: bool,
        max_viewport: Option<usize>,
        linewise: bool,
    ) -> Result<()> {
        // Build command groups from index
        let records = store.get_all_records()?;
        if records.is_empty() {
            println!("{}", i18n.t("no_records").yellow());
            return Ok(());
        }

        loop {
            // Select a command group
            let groups = Self::build_command_groups(&records);
            let selected_hash = if tui_simple {
                Self::simple_select_command(&groups, i18n)
            } else {
                Self::interactive_select_command(&groups, i18n, use_alt_screen)
            };

            let Some(command_hash) = selected_hash else {
                return Ok(());
            };

            // Load executions for the chosen command
            let mut executions = store.find_executions(&command_hash, i18n)?;
            if executions.len() < 2 {
                println!("{}", i18n.t("need_at_least_two").red().bold());
                continue;
            }

            if tui_simple {
                // Use simple two-number selection
                executions = Self::simple_select_executions(&executions, i18n);
            } else {
                // Interactive record selection (ratatui-based)
                let store_ref = store;
                let hash_clone = command_hash.clone();
                executions = Self::start_interactive_selection_ratatui(
                    &executions,
                    i18n,
                    use_alt_screen,
                    linewise,
                    || {
                        store_ref
                            .find_executions(&hash_clone, i18n)
                            .unwrap_or_default()
                    },
                    true, // Esc returns empty => go back to command list
                    max_viewport,
                    Some(|exec: &CommandExecution| store_ref.delete_execution(exec, i18n)),
                );
                if executions.is_empty() {
                    // Go back to command list
                    continue;
                }
            }

            if let Some(diff_output) = Self::diff_executions(&executions, i18n, linewise) {
                print!("{}", diff_output);
            }
            return Ok(());
        }
    }

    fn build_command_groups(records: &[crate::storage::CommandRecord]) -> Vec<CommandGroup> {
        use std::collections::HashMap;
        let mut map: HashMap<String, CommandGroup> = HashMap::new();
        for rec in records {
            let e = map
                .entry(rec.command_hash.clone())
                .or_insert_with(|| CommandGroup {
                    command_hash: rec.command_hash.clone(),
                    command: rec.command.clone(),
                    count: 0,
                    latest: rec.timestamp,
                });
            e.count += 1;
            if rec.timestamp > e.latest {
                e.latest = rec.timestamp;
                e.command = rec.command.clone();
            }
        }
        let mut groups: Vec<CommandGroup> = map.into_values().collect();
        groups.sort_by(|a, b| b.latest.cmp(&a.latest));
        groups
    }

    fn simple_select_command(groups: &[CommandGroup], i18n: &I18n) -> Option<String> {
        println!("{}", i18n.t("select_command"));
        for (i, g) in groups.iter().enumerate() {
            let dt = g
                .latest
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S");
            println!(
                "{}: {} ({}: {}, {}: {})",
                i + 1,
                g.command,
                i18n.t("count_label"),
                g.count,
                i18n.t("latest_label"),
                dt
            );
        }
        println!("{}", i18n.t("input_numbers"));
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return None;
        }
        let s = input.trim();
        if s.is_empty() {
            return None;
        }
        if let Ok(idx) = s.parse::<usize>() {
            if idx > 0 && idx <= groups.len() {
                return Some(groups[idx - 1].command_hash.clone());
            }
        }
        None
    }

    fn interactive_select_command(
        groups: &[CommandGroup],
        i18n: &I18n,
        use_alt_screen: bool,
    ) -> Option<String> {
        // Prepare terminal
        if terminal::enable_raw_mode().is_err() {
            return Self::simple_select_command(groups, i18n);
        }
        let mut stdout = io::stdout();
        if use_alt_screen {
            print!("\x1b[?1049h");
        }
        print!("\x1b[?7l\x1b[?25l");
        stdout.flush().ok();

        let mut filter_input = String::new();
        let mut current_selection = 0usize;
        let mut scroll_offset = 0usize;

        loop {
            print!("\x1b[2J\x1b[H");
            stdout.flush().ok();

            print!("{}\r\n", i18n.t("select_command"));
            print!("{}: ", i18n.t("interactive_filter"));
            print!("{}\r\n\r\n", filter_input);

            let fuzzy = SkimMatcher::new();
            let filtered_indices: Vec<usize> = if filter_input.is_empty() {
                (0..groups.len()).collect()
            } else {
                let items: Vec<(usize, String)> = groups
                    .iter()
                    .enumerate()
                    .map(|(i, g)| {
                        let dt = g
                            .latest
                            .with_timezone(&chrono::Local)
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string();
                        let text = format!("{} {} {} {}", g.command, g.count, dt, i + 1);
                        (i, text)
                    })
                    .collect();
                let matched = fuzzy.match_and_sort(&filter_input, items);
                matched.into_iter().map(|(i, _, _)| i).collect()
            };

            if filtered_indices.is_empty() {
                print!("\x1b[31m{}\x1b[0m\r\n", i18n.t("no_matches"));
            } else {
                if current_selection >= filtered_indices.len() {
                    current_selection = filtered_indices.len().saturating_sub(1);
                }
                let (_cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                let mut viewport = rows as usize;
                let reserved = 6usize;
                viewport = viewport.saturating_sub(reserved);
                if viewport < 5 {
                    viewport = 5;
                }
                if current_selection < scroll_offset {
                    scroll_offset = current_selection;
                } else if current_selection >= scroll_offset + viewport {
                    scroll_offset = current_selection + 1 - viewport;
                }
                let end = (scroll_offset + viewport).min(filtered_indices.len());
                for (list_idx, gi_ref) in filtered_indices
                    .iter()
                    .enumerate()
                    .skip(scroll_offset)
                    .take(end - scroll_offset)
                {
                    let gi = *gi_ref;
                    let g = &groups[gi];
                    let dt = g
                        .latest
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M:%S");
                    let line = format!(
                        "{}: {} ({}: {}, {}: {})",
                        gi + 1,
                        g.command,
                        i18n.t("count_label"),
                        g.count,
                        i18n.t("latest_label"),
                        dt
                    );
                    if list_idx == current_selection {
                        print!("\x1b[44;37m{}\x1b[0m\x1b[K\r\n", line);
                    } else {
                        print!("{}\x1b[K\r\n", line);
                    }
                }
            }

            print!("\r\n");
            print!("\x1b[90m{}\x1b[0m\r\n", i18n.t("navigate_hint"));
            stdout.flush().ok();

            if let Ok(Event::Key(key)) = event::read() {
                let is_ctrl_combo = key.modifiers.contains(KeyModifiers::CONTROL);
                let is_ctrl_char =
                    matches!(key.code, KeyCode::Char(c) if c == '\u{3}' || c == '\u{4}');
                if is_ctrl_combo || is_ctrl_char {
                    let exit_match = match key.code {
                        KeyCode::Char('c')
                        | KeyCode::Char('C')
                        | KeyCode::Char('d')
                        | KeyCode::Char('D') => true,
                        KeyCode::Char(cc) if cc == '\u{3}' || cc == '\u{4}' => true,
                        _ => false,
                    };
                    if exit_match {
                        if use_alt_screen {
                            print!("\x1b[?1049l");
                        }
                        print!("\x1b[?7h\x1b[?25h");
                        stdout.flush().ok();
                        let _ = terminal::disable_raw_mode();
                        return None;
                    }
                }
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                        current_selection = current_selection.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                        if !filtered_indices.is_empty()
                            && current_selection < filtered_indices.len() - 1
                        {
                            current_selection += 1;
                        }
                    }
                    KeyCode::Enter => {
                        if !filtered_indices.is_empty() {
                            // If the filter input is a pure number, allow direct selection by displayed index (gi + 1)
                            let trimmed = filter_input.trim();
                            let mut pick_gi: Option<usize> = None;
                            if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
                                if let Ok(n) = trimmed.parse::<usize>() {
                                    if n > 0 {
                                        for &gi_candidate in &filtered_indices {
                                            if gi_candidate + 1 == n {
                                                pick_gi = Some(gi_candidate);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            let gi = pick_gi.unwrap_or_else(|| filtered_indices[current_selection]);
                            if use_alt_screen {
                                print!("\x1b[?1049l");
                            }
                            print!("\x1b[?7h\x1b[?25h");
                            stdout.flush().ok();
                            let _ = terminal::disable_raw_mode();
                            return Some(groups[gi].command_hash.clone());
                        }
                    }
                    _ if Self::is_backspace_event(&key) => {
                        filter_input.pop();
                        current_selection = 0;
                        scroll_offset = 0;
                    }
                    KeyCode::Delete => {
                        filter_input.clear();
                        current_selection = 0;
                        scroll_offset = 0;
                    }
                    KeyCode::Esc => {
                        if use_alt_screen {
                            print!("\x1b[?1049l");
                        }
                        print!("\x1b[?7h\x1b[?25h");
                        stdout.flush().ok();
                        let _ = terminal::disable_raw_mode();
                        return None;
                    }
                    KeyCode::Char(c) => {
                        filter_input.push(c);
                        current_selection = 0;
                        scroll_offset = 0;
                    }
                    _ => {}
                }
            }
        }
    }
    pub fn diff_executions(
        executions: &[CommandExecution],
        i18n: &I18n,
        linewise: bool,
    ) -> Option<String> {
        if executions.len() < 2 {
            return None;
        }

        let executions: Vec<&CommandExecution> = executions.iter().collect();
        let earlier = &executions[0];
        let later = &executions[1];

        let mut output = String::new();

        output.push_str(&format!(
            "{}\n",
            i18n.t_format("diff_command", &[&later.record.command])
                .bold()
                .cyan()
        ));

        let earlier_local = earlier.record.timestamp.with_timezone(&chrono::Local);
        let later_local = later.record.timestamp.with_timezone(&chrono::Local);
        let earlier_code = earlier.record.short_code.as_deref().unwrap_or("");
        let later_code = later.record.short_code.as_deref().unwrap_or("");

        // Align timestamp column by padding label to same width
        let earlier_label = i18n.t("diff_earlier_label");
        let later_label = i18n.t("diff_later_label");
        let label_width = std::cmp::max(earlier_label.len(), later_label.len());
        let earlier_label_padded = format!("{:<width$}", earlier_label, width = label_width);
        let later_label_padded = format!("{:<width$}", later_label, width = label_width);
        let earlier_time = earlier_local.format("%Y-%m-%d %H:%M:%S").to_string();
        let later_time = later_local.format("%Y-%m-%d %H:%M:%S").to_string();

        let mut earlier_line = format!("- {}: {}", earlier_label_padded, earlier_time);
        if !earlier_code.is_empty() {
            earlier_line.push_str(&format!(
                " [{}: {}]",
                i18n.t("short_code_label"),
                earlier_code
            ));
        }
        let mut later_line = format!("+ {}: {}", later_label_padded, later_time);
        if !later_code.is_empty() {
            later_line.push_str(&format!(
                " [{}: {}]",
                i18n.t("short_code_label"),
                later_code
            ));
        }
        output.push_str(&format!("{}\n", earlier_line.red()));
        output.push_str(&format!("{}\n", later_line.green()));

        if earlier.record.exit_code != later.record.exit_code {
            output.push_str(&i18n.t_format(
                "diff_exit_code",
                &[
                    &earlier.record.exit_code.to_string(),
                    &later.record.exit_code.to_string(),
                ],
            ));
            output.push('\n');
        }

        output.push_str(&i18n.t_format(
            "diff_execution_time",
            &[
                &earlier.record.duration_ms.to_string(),
                &later.record.duration_ms.to_string(),
            ],
        ));
        output.push('\n');

        output.push('\n');

        if earlier.stdout != later.stdout {
            output.push_str(&format!("{}\n", i18n.t("stdout_diff").yellow().bold()));
            if linewise {
                output.push_str(&Self::diff_text_linewise(&earlier.stdout, &later.stdout));
            } else {
                output.push_str(&Self::diff_text(&earlier.stdout, &later.stdout));
            }
            output.push('\n');
        }

        if earlier.stderr != later.stderr {
            output.push_str(&format!("{}\n", i18n.t("stderr_diff").red().bold()));
            if linewise {
                output.push_str(&Self::diff_text_linewise(&earlier.stderr, &later.stderr));
            } else {
                output.push_str(&Self::diff_text(&earlier.stderr, &later.stderr));
            }
            output.push('\n');
        }

        if earlier.stdout == later.stdout && earlier.stderr == later.stderr {
            output.push_str(&format!("{}\n", i18n.t("output_identical").green().bold()));
        }

        Some(output)
    }

    // compute_preview_layout removed (ratatui handles layout)

    fn char_display_width(ch: char) -> usize {
        UnicodeWidthChar::width(ch).unwrap_or(1).max(1)
    }

    fn wrap_line_to_width(line: &str, width: usize) -> Vec<String> {
        if width == 0 {
            return vec![String::new()];
        }
        let mut result = Vec::new();
        let mut current = String::new();
        let mut current_width = 0usize;

        for ch in line.chars() {
            let ch_width = Self::char_display_width(ch);
            if current_width + ch_width > width && !current.is_empty() {
                result.push(current.clone());
                current.clear();
                current_width = 0;
            }
            if ch_width > width {
                // Character alone exceeds width; truncate and mark overflow
                if current.is_empty() {
                    current.push(ch);
                    result.push(current.clone());
                    current.clear();
                    current_width = 0;
                } else {
                    result.push(current.clone());
                    current.clear();
                    current.push(ch);
                    result.push(current.clone());
                    current.clear();
                    current_width = 0;
                }
                continue;
            }
            current.push(ch);
            current_width += ch_width;
            if current_width == width {
                result.push(current.clone());
                current.clear();
                current_width = 0;
            }
        }

        if !current.is_empty() {
            result.push(current);
        }

        if result.is_empty() {
            result.push(String::new());
        }

        result
    }

    // removed: truncate_for_column (ratatui handles wrapping/clipping)

    // Note: previous tail-focused truncation helper was removed as unused.

    // wrap_preview_content removed
    /* fn draw_preview_panel(
            stdout: &mut io::Stdout,
            layout: &PreviewLayout,
            i18n: &I18n,
            preview_target: PreviewTarget,
            focus_exec: Option<&CommandExecution>,
            diff_pair: Option<(&CommandExecution, &CommandExecution)>,
            linewise: bool,
        ) -> io::Result<()> {
            let max_rows = layout.total_rows.saturating_sub(1);
            if max_rows == 0 {
                return Ok(());
            }
            let max_rows = max_rows as usize;

            let toggle_hint = i18n.t("preview_toggle_short");
            let header_text = if diff_pair.is_some() {
                match preview_target {
                    PreviewTarget::Stdout => i18n.t("preview_diff_stdout_header"),
                    PreviewTarget::Stderr => i18n.t("preview_diff_stderr_header"),
                }
            } else {
                i18n.t(preview_target.label_key())
            };
            let header_display = Self::truncate_for_column(
                &format!("{}  ·  {}", header_text, toggle_hint),
                layout.preview_width,
            );

            enum PreviewBody {
                Text(String),
                Lines(Vec<String>),
            }

            let (metadata_lines, body) = if let Some((earlier, later)) = diff_pair {
                let earlier_local = earlier.record.timestamp.with_timezone(&chrono::Local);
                let later_local = later.record.timestamp.with_timezone(&chrono::Local);
                let short_code_label = i18n.t("short_code_label");
                let earlier_code = earlier
                    .record
                    .short_code
                    .clone()
                    .unwrap_or_else(|| "—".to_string());
                let later_code = later
                    .record
                    .short_code
                    .clone()
                    .unwrap_or_else(|| "—".to_string());
                let earlier_label = i18n.t("diff_earlier_label");
                let later_label = i18n.t("diff_later_label");
                let label_width = earlier_label
                    .chars()
                    .count()
                    .max(later_label.chars().count());
                let earlier_line = format!(
                    "- {:width$}: {} [{}: {}]",
                    earlier_label,
                    earlier_local.format("%Y-%m-%d %H:%M:%S"),
                    short_code_label,
                    earlier_code,
                    width = label_width
                );
                let later_line = format!(
                    "+ {:width$}: {} [{}: {}]",
                    later_label,
                    later_local.format("%Y-%m-%d %H:%M:%S"),
                    short_code_label,
                    later_code,
                    width = label_width
                );
                let exec_time_line = i18n.t_format(
                    "diff_execution_time",
                    &[
                        &earlier.record.duration_ms.to_string(),
                        &later.record.duration_ms.to_string(),
                    ],
                );
                let (old_text, new_text) = if preview_target.is_stdout() {
                    (
                        Self::sanitize_for_preview(earlier.stdout.as_str()),
                        Self::sanitize_for_preview(later.stdout.as_str()),
                    )
                } else {
                    (
                        Self::sanitize_for_preview(earlier.stderr.as_str()),
                        Self::sanitize_for_preview(later.stderr.as_str()),
                    )
                };
                let body_text = if old_text == new_text {
                    i18n.t("output_identical")
                } else if linewise {
                    Self::diff_preview_text_linewise(&old_text, &new_text)
                } else {
                    Self::diff_preview_text(&old_text, &new_text)
                };
                (
                    vec![earlier_line, later_line, exec_time_line],
                    PreviewBody::Text(body_text),
                )
            } else if let Some(execution) = focus_exec {
                let raw_path = if preview_target.is_stdout() {
                    execution.stdout_path.as_ref()
                } else {
                    execution.stderr_path.as_ref()
                };
                let path_text = raw_path
                    .map(|p| i18n.t_format("preview_path_label", &[&p.display().to_string()]))
                    .unwrap_or_else(|| i18n.t("preview_path_missing"));
                let content = if preview_target.is_stdout() {
                    Self::sanitize_for_preview(execution.stdout.as_str())
                } else {
                    Self::sanitize_for_preview(execution.stderr.as_str())
                };
                let body = if content.is_empty() {
                    PreviewBody::Lines(vec![i18n.t("preview_empty")])
                } else {
                    PreviewBody::Text(content)
                };
                (vec![path_text], body)
            } else {
                (
                    vec![i18n.t("preview_path_missing")],
                    PreviewBody::Lines(vec![i18n.t("preview_no_selection")]),
                )
            };

            let mut metadata_wrapped: Vec<String> = Vec::new();
            for line in metadata_lines {
                metadata_wrapped.extend(Self::wrap_line_to_width(&line, layout.preview_width));
            }

            let header_rows = 1usize;
            let available_body_lines = max_rows
                .saturating_sub(header_rows + metadata_wrapped.len())
                .max(1);

            let body_lines = match body {
                PreviewBody::Lines(lines) => lines,
                PreviewBody::Text(text) => {
                    if text.is_empty() {
                        vec![i18n.t("preview_empty")]
                    } else {
                        let (mut wrapped, was_truncated) = Self::wrap_preview_content(
                            &text,
                            layout.preview_width,
                            available_body_lines,
                        );
                        if wrapped.is_empty() {
                            wrapped.push(i18n.t("preview_empty"));
                        }
                        if was_truncated {
                            wrapped.push(i18n.t("preview_truncated_hint"));
                        }
                        wrapped
                    }
                }
            };

            // Draw vertical separator between panels
            let sep_col = layout.start_col.saturating_sub(1);
            for r in 0..max_rows {
                stdout.queue(MoveTo(sep_col, r as u16))?;
                stdout.queue(Print("│".to_string()))?;
            }

            let mut row = 0usize;
            stdout.queue(MoveTo(layout.start_col, row as u16))?;
            stdout.queue(Print(header_display))?;
            row += 1;

            for line in metadata_wrapped {
                if row >= max_rows {
                    break;
                }
                let display_line = Self::truncate_for_column(&line, layout.preview_width);
                stdout.queue(MoveTo(layout.start_col, row as u16))?;
                stdout.queue(Print(display_line))?;
                row += 1;
            }

            for line in body_lines {
                if row >= max_rows {
                    break;
                }
                let display_line = Self::truncate_for_column(&line, layout.preview_width);
                stdout.queue(MoveTo(layout.start_col, row as u16))?;
                stdout.queue(Print(format!("{}[K", display_line)))?;
                row += 1;
            }

            while row < max_rows {
                stdout.queue(MoveTo(layout.start_col, row as u16))?;
                stdout.queue(Print("[K".to_string()))?;
                row += 1;
            }

            Ok(())
        }
    */

    fn diff_preview_text(old: &str, new: &str) -> String {
        let diff = TextDiff::from_lines(old, new);
        let mut result = String::new();

        for change in diff.iter_all_changes() {
            let prefix = match change.tag() {
                ChangeTag::Delete => '-',
                ChangeTag::Insert => '+',
                ChangeTag::Equal => ' ',
            };
            result.push(prefix);
            result.push(' ');
            result.push_str(change.value());
        }

        result
    }

    fn diff_preview_text_linewise(old: &str, new: &str) -> String {
        let mut result = String::new();
        let old_lines: Vec<&str> = old.split('\n').collect();
        let new_lines: Vec<&str> = new.split('\n').collect();
        let max_len = old_lines.len().max(new_lines.len());
        for i in 0..max_len {
            let o = old_lines.get(i).copied();
            let n = new_lines.get(i).copied();
            match (o, n) {
                (Some(ol), Some(nl)) if ol == nl => {
                    result.push(' ');
                    result.push(' ');
                    result.push_str(ol);
                    result.push('\n');
                }
                (Some(ol), Some(nl)) => {
                    result.push('-');
                    result.push(' ');
                    result.push_str(ol);
                    result.push('\n');
                    result.push('+');
                    result.push(' ');
                    result.push_str(nl);
                    result.push('\n');
                }
                (Some(ol), None) => {
                    result.push('-');
                    result.push(' ');
                    result.push_str(ol);
                    result.push('\n');
                }
                (None, Some(nl)) => {
                    result.push('+');
                    result.push(' ');
                    result.push_str(nl);
                    result.push('\n');
                }
                (None, None) => {}
            }
        }
        result
    }

    fn diff_text(old: &str, new: &str) -> String {
        let diff = TextDiff::from_lines(old, new);

        let mut result = String::new();

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    result.push_str(&format!("{}{}", "-".red(), change.to_string().red()));
                }
                ChangeTag::Insert => {
                    result.push_str(&format!("{}{}", "+".green(), change.to_string().green()));
                }
                ChangeTag::Equal => {
                    result.push_str(&format!(" {}", change));
                }
            }
        }

        result
    }

    fn diff_text_linewise(old: &str, new: &str) -> String {
        let mut result = String::new();
        let old_lines: Vec<&str> = old.split('\n').collect();
        let new_lines: Vec<&str> = new.split('\n').collect();
        let max_len = old_lines.len().max(new_lines.len());
        for i in 0..max_len {
            let o = old_lines.get(i).copied();
            let n = new_lines.get(i).copied();
            match (o, n) {
                (Some(ol), Some(nl)) if ol == nl => {
                    result.push_str(&format!(" {}\n", ol));
                }
                (Some(ol), Some(nl)) => {
                    result.push_str(&format!("{}{}\n", "-".red(), ol.red()));
                    result.push_str(&format!("{}{}\n", "+".green(), nl.green()));
                }
                (Some(ol), None) => {
                    result.push_str(&format!("{}{}\n", "-".red(), ol.red()));
                }
                (None, Some(nl)) => {
                    result.push_str(&format!("{}{}\n", "+".green(), nl.green()));
                }
                (None, None) => {}
            }
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn start_interactive_selection_ratatui<F, D>(
        executions: &[CommandExecution],
        i18n: &I18n,
        use_alt_screen: bool,
        linewise: bool,
        _loader: F,
        _on_escape_return_empty: bool,
        _max_viewport: Option<usize>,
        _delete_action: Option<D>,
    ) -> Vec<CommandExecution>
    where
        F: FnMut() -> Vec<CommandExecution>,
        D: FnMut(&CommandExecution) -> Result<()>,
    {
        // 1) Prepare terminal
        if terminal::enable_raw_mode().is_err() {
            println!("{}", i18n.t("warning_interactive_failed"));
            return Self::simple_select_executions(executions, i18n);
        }
        let mut stdout = io::stdout();
        if use_alt_screen {
            let _ = crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen);
        }
        let _ = crossterm::execute!(stdout, crossterm::cursor::Hide);
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).expect("init terminal");
        // Clear once to avoid mixing with previous shell content when not using alt screen
        let _ = terminal.clear();

        // 2) App state
        let mut filter_input = String::new();
        let mut selected_ids: Vec<String> = Vec::new();
        let mut current_selection: usize = 0;
        // Preview shows stdout first, then stderr (stacked)
        let mut preview_offset: u16 = 0; // Paragraph vertical scroll offset
        let mut show_help: bool = false; // preview help overlay
                                         // Focus model: Selection (left list & filter) vs Preview (right pane scrolling)
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Focus {
            Selection,
            Preview,
        }
        let mut focus = Focus::Selection;

        let current_execs: Vec<CommandExecution> = executions.to_vec();
        let display_executions: Vec<&CommandExecution> = current_execs.iter().collect();

        let compute_filtered = |input: &str| -> Vec<usize> {
            if input.is_empty() {
                return (0..display_executions.len()).collect();
            }
            let items: Vec<(usize, String)> = display_executions
                .iter()
                .enumerate()
                .map(|(i, exec)| {
                    let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
                    let date_str = local_time.format("%Y-%m-%d %H:%M:%S");
                    let code = exec.record.short_code.clone().unwrap_or_default();
                    let searchable = if code.is_empty() {
                        format!("{} {} {}", i + 1, date_str, exec.record.command)
                    } else {
                        format!("{} {} {} {}", i + 1, date_str, exec.record.command, code)
                    };
                    (i, searchable)
                })
                .collect();
            let matcher = SkimMatcher::new();
            matcher
                .match_and_sort(input, items)
                .into_iter()
                .map(|(i, _, _)| i)
                .collect()
        };
        let mut filtered_indices = compute_filtered("");

        // Initial draw
        let _ = terminal.draw(|f| {
            Self::render_ratatui_frame(
                f,
                i18n,
                &filter_input,
                &selected_ids,
                current_selection,
                &mut preview_offset,
                &current_execs,
                &display_executions,
                &filtered_indices,
                linewise,
                matches!(focus, Focus::Preview),
                show_help,
            )
        });

        // 4) Event loop
        loop {
            if let Ok(ev) = event::read() {
                match ev {
                    Event::Key(key) => {
                        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                        if ctrl && matches!(key.code, KeyCode::Char('c')) {
                            break;
                        }
                        match (focus, key.code) {
                            // Global
                            (_, KeyCode::Esc) => {
                                if matches!(focus, Focus::Preview) {
                                    if show_help {
                                        show_help = false;
                                    } else {
                                        focus = Focus::Selection;
                                    }
                                } else {
                                    break;
                                }
                            }

                            // Selection focus: navigate & filter; Tab/Space/Enter to preview
                            (Focus::Selection, KeyCode::Up)
                            | (Focus::Selection, KeyCode::Char('k')) => {
                                current_selection = current_selection.saturating_sub(1);
                            }
                            (Focus::Selection, KeyCode::Down)
                            | (Focus::Selection, KeyCode::Char('j')) => {
                                if current_selection + 1 < filtered_indices.len() {
                                    current_selection += 1;
                                }
                            }
                            (Focus::Selection, KeyCode::Tab)
                            | (Focus::Selection, KeyCode::Enter)
                            | (Focus::Selection, KeyCode::Char(' ')) => {
                                // If already selected two, Enter finalizes directly
                                if matches!(key.code, KeyCode::Enter) && selected_ids.len() == 2 {
                                    break;
                                }
                                if let Some(&oi) = filtered_indices.get(current_selection) {
                                    // toggle selection on Space/Enter; Tab just switches focus
                                    if !matches!(key.code, KeyCode::Tab) {
                                        let id = display_executions[oi].record.record_id.clone();
                                        if let Some(pos) =
                                            selected_ids.iter().position(|x| x == &id)
                                        {
                                            selected_ids.remove(pos);
                                        } else if selected_ids.len() < 2 {
                                            selected_ids.push(id);
                                        }
                                    }
                                    // If two selected or user requested Tab/Space, go to preview
                                    preview_offset = 0;
                                    focus = Focus::Preview;
                                }
                            }
                            (Focus::Selection, KeyCode::Char(c)) if !ctrl => {
                                filter_input.push(c);
                                current_selection = 0;
                                preview_offset = 0;
                                filtered_indices = compute_filtered(&filter_input);
                            }
                            (Focus::Selection, KeyCode::Backspace)
                            | (Focus::Selection, KeyCode::Delete) => {
                                filter_input.pop();
                                current_selection = 0;
                                preview_offset = 0;
                                filtered_indices = compute_filtered(&filter_input);
                            }

                            // Preview focus: Up/Down switch selection; modifier keys keep preview scrolling
                            (Focus::Preview, KeyCode::Up)
                            | (Focus::Preview, KeyCode::Char('k')) => {
                                if key.modifiers.contains(KeyModifiers::ALT) {
                                    let half = terminal.size().map(|r| r.height / 2).unwrap_or(5);
                                    preview_offset = preview_offset.saturating_sub(half);
                                } else if key
                                    .modifiers
                                    .intersects(KeyModifiers::CONTROL | KeyModifiers::SHIFT)
                                {
                                    preview_offset = preview_offset.saturating_sub(1);
                                } else if !filtered_indices.is_empty() {
                                    let before = current_selection;
                                    current_selection = current_selection.saturating_sub(1);
                                    focus = Focus::Selection;
                                    show_help = false;
                                    if current_selection != before {
                                        preview_offset = 0;
                                    }
                                }
                            }
                            (Focus::Preview, KeyCode::Down)
                            | (Focus::Preview, KeyCode::Char('j')) => {
                                if key.modifiers.contains(KeyModifiers::ALT) {
                                    let half = terminal.size().map(|r| r.height / 2).unwrap_or(5);
                                    preview_offset = preview_offset.saturating_add(half);
                                } else if key
                                    .modifiers
                                    .intersects(KeyModifiers::CONTROL | KeyModifiers::SHIFT)
                                {
                                    preview_offset = preview_offset.saturating_add(1);
                                } else if !filtered_indices.is_empty() {
                                    if current_selection + 1 < filtered_indices.len() {
                                        current_selection += 1;
                                        preview_offset = 0;
                                    }
                                    focus = Focus::Selection;
                                    show_help = false;
                                }
                            }
                            (Focus::Preview, KeyCode::PageUp) => {
                                let page = terminal
                                    .size()
                                    .map(|r| r.height.saturating_sub(4))
                                    .unwrap_or(10);
                                preview_offset = preview_offset.saturating_sub(page);
                            }
                            (Focus::Preview, KeyCode::PageDown) => {
                                let page = terminal
                                    .size()
                                    .map(|r| r.height.saturating_sub(4))
                                    .unwrap_or(10);
                                preview_offset = preview_offset.saturating_add(page);
                            }
                            // less-like keys
                            (Focus::Preview, KeyCode::Char(' ')) => {
                                let page = terminal
                                    .size()
                                    .map(|r| r.height.saturating_sub(4))
                                    .unwrap_or(10);
                                preview_offset = preview_offset.saturating_add(page);
                            }
                            (Focus::Preview, KeyCode::Backspace)
                            | (Focus::Preview, KeyCode::Char('b')) => {
                                let page = terminal
                                    .size()
                                    .map(|r| r.height.saturating_sub(4))
                                    .unwrap_or(10);
                                preview_offset = preview_offset.saturating_sub(page);
                            }
                            (Focus::Preview, KeyCode::Char('f')) => {
                                let page = terminal
                                    .size()
                                    .map(|r| r.height.saturating_sub(4))
                                    .unwrap_or(10);
                                preview_offset = preview_offset.saturating_add(page);
                            }
                            (Focus::Preview, KeyCode::Char('d')) => {
                                let half = terminal.size().map(|r| r.height / 2).unwrap_or(5);
                                preview_offset = preview_offset.saturating_add(half);
                            }
                            (Focus::Preview, KeyCode::Char('u')) => {
                                let half = terminal.size().map(|r| r.height / 2).unwrap_or(5);
                                preview_offset = preview_offset.saturating_sub(half);
                            }
                            (Focus::Preview, KeyCode::Home) => {
                                preview_offset = 0;
                            }
                            (Focus::Preview, KeyCode::End) => {
                                preview_offset = u16::MAX;
                            }
                            (Focus::Preview, KeyCode::Char('g'))
                                if !ctrl && !key.modifiers.contains(KeyModifiers::ALT) =>
                            {
                                preview_offset = 0;
                            }
                            (Focus::Preview, KeyCode::Char('G'))
                                if !ctrl && !key.modifiers.contains(KeyModifiers::ALT) =>
                            {
                                preview_offset = u16::MAX;
                            }
                            // Toggle help overlay
                            (Focus::Preview, KeyCode::Char('?'))
                            | (Focus::Preview, KeyCode::Char('h'))
                                if !ctrl =>
                            {
                                show_help = !show_help;
                            }
                            // Quit/back keys: q back to Selection; Q quit all
                            (Focus::Preview, KeyCode::Char('q')) => {
                                focus = Focus::Selection;
                            }
                            (Focus::Preview, KeyCode::Char('Q')) => {
                                break;
                            }
                            // Enter in preview: toggle selection, finalize when two picked
                            (Focus::Preview, KeyCode::Enter) => {
                                if selected_ids.len() == 2 {
                                    break;
                                }
                                if let Some(&oi) = filtered_indices.get(current_selection) {
                                    let id = display_executions[oi].record.record_id.clone();
                                    if let Some(pos) = selected_ids.iter().position(|x| x == &id) {
                                        selected_ids.remove(pos);
                                    } else if selected_ids.len() < 2 {
                                        selected_ids.push(id);
                                    }
                                }
                            }
                            // Tab or Space in preview does nothing (Esc to go back)
                            _ => {}
                        }
                    }
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
            let _ = terminal.draw(|f| {
                Self::render_ratatui_frame(
                    f,
                    i18n,
                    &filter_input,
                    &selected_ids,
                    current_selection,
                    &mut preview_offset,
                    &current_execs,
                    &display_executions,
                    &filtered_indices,
                    linewise,
                    matches!(focus, Focus::Preview),
                    show_help,
                )
            });
        }

        // 5) Teardown
        let _ = terminal.show_cursor();
        let mut out = io::stdout();
        if use_alt_screen {
            let _ = crossterm::execute!(out, crossterm::terminal::LeaveAlternateScreen);
        } else {
            let _ = crossterm::execute!(
                out,
                crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
                crossterm::cursor::MoveTo(0, 0)
            );
        }
        let _ = terminal::disable_raw_mode();

        if selected_ids.len() == 2 {
            let mut pair: Vec<CommandExecution> = selected_ids
                .into_iter()
                .filter_map(|id| {
                    current_execs
                        .iter()
                        .find(|e| e.record.record_id == id)
                        .cloned()
                })
                .collect();
            if pair.len() == 2 {
                pair.sort_by(|a, b| a.record.timestamp.cmp(&b.record.timestamp));
                return pair;
            }
        }
        Vec::new()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_ratatui_frame(
        f: &mut ratatui::Frame,
        i18n: &I18n,
        filter_input: &str,
        selected_ids: &[String],
        current_selection: usize,
        preview_offset: &mut u16,
        current_execs: &[CommandExecution],
        display_executions: &Vec<&CommandExecution>,
        filtered_indices: &[usize],
        linewise: bool,
        preview_focused: bool,
        show_help: bool,
    ) {
        // Ensure the frame is fully cleared to avoid artifacts under the UI
        f.render_widget(Clear, f.size());
        let header_line = Line::from(vec![
            Span::styled(i18n.t("status_filter"), Style::default().fg(Color::Gray)),
            Span::raw(": "),
            Span::raw(filter_input),
        ]);

        let root = f.size();
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(root);

        f.render_widget(Paragraph::new(header_line), rows[0]);

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(rows[1]);

        // Left list
        let mut items: Vec<ListItem> = Vec::new();
        for (vis_idx, &orig_i) in filtered_indices.iter().enumerate() {
            let exec = display_executions[orig_i];
            let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
            let date_str = local_time.format("%Y-%m-%d %H:%M:%S");
            let mark = if selected_ids.iter().any(|id| id == &exec.record.record_id) {
                "✓ "
            } else {
                "  "
            };
            let line = if let Some(code) = exec.record.short_code.as_deref() {
                format!(
                    "{}{}: {}:{} {}: {}",
                    mark,
                    vis_idx + 1,
                    i18n.t("short_code_label"),
                    code,
                    i18n.t("time_label"),
                    date_str
                )
            } else {
                format!(
                    "{}{}: {}: {}",
                    mark,
                    vis_idx + 1,
                    i18n.t("time_label"),
                    date_str
                )
            };
            items.push(ListItem::new(line));
        }
        let list_title = i18n.t_format("select_executions", &[&current_execs.len().to_string()]);
        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if preview_focused {
                Style::default()
            } else {
                Style::default().fg(Color::Cyan)
            })
            .title(list_title);
        let list = List::new(items)
            .block(list_block)
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));
        let mut state = ratatui::widgets::ListState::default();
        if !filtered_indices.is_empty() {
            state.select(Some(current_selection));
        }
        f.render_stateful_widget(list, cols[0], &mut state);

        // Right preview
        let preview_pair = if selected_ids.len() == 2 {
            let mut pair: Vec<&CommandExecution> = selected_ids
                .iter()
                .filter_map(|id| current_execs.iter().find(|e| &e.record.record_id == id))
                .collect();
            if pair.len() == 2 {
                pair.sort_by(|a, b| a.record.timestamp.cmp(&b.record.timestamp));
                Some((pair[0], pair[1]))
            } else {
                None
            }
        } else {
            None
        };

        let focus_exec = filtered_indices
            .get(current_selection)
            .and_then(|&idx| display_executions.get(idx))
            .copied();

        // Build combined preview: stdout then stderr (with divider if both exist)
        let mut title = i18n.t("preview_stdout_header");
        let body = if let Some((earlier, later)) = preview_pair {
            let so_old = Self::sanitize_for_preview(&earlier.stdout);
            let so_new = Self::sanitize_for_preview(&later.stdout);
            let se_old = Self::sanitize_for_preview(&earlier.stderr);
            let se_new = Self::sanitize_for_preview(&later.stderr);
            let mut out = String::new();
            // stdout section
            if so_old == so_new {
                out.push_str(&i18n.t("output_identical"));
                out.push('\n');
            } else if linewise {
                out.push_str(&Self::diff_preview_text_linewise(&so_old, &so_new));
            } else {
                out.push_str(&Self::diff_preview_text(&so_old, &so_new));
            }
            // stderr section
            if !se_old.is_empty() || !se_new.is_empty() {
                title = format!(
                    "{}  |  {}",
                    i18n.t("preview_diff_stdout_header"),
                    i18n.t("preview_diff_stderr_header")
                );
                out.push_str("\n── stderr ─────────────────────────\n");
                if se_old == se_new {
                    out.push_str(&i18n.t("output_identical"));
                } else if linewise {
                    out.push_str(&Self::diff_preview_text_linewise(&se_old, &se_new));
                } else {
                    out.push_str(&Self::diff_preview_text(&se_old, &se_new));
                }
            } else {
                title = i18n.t("preview_diff_stdout_header");
            }
            out
        } else if let Some(exec) = focus_exec {
            let so = Self::sanitize_for_preview(&exec.stdout);
            let se = Self::sanitize_for_preview(&exec.stderr);
            let stdout_path_text = exec
                .stdout_path
                .as_ref()
                .map(|p| i18n.t_format("preview_path_label", &[&p.display().to_string()]))
                .unwrap_or_else(|| i18n.t("preview_path_missing"));
            let stderr_path_text = exec
                .stderr_path
                .as_ref()
                .map(|p| i18n.t_format("preview_path_label", &[&p.display().to_string()]))
                .unwrap_or_else(|| i18n.t("preview_path_missing"));
            let has_stderr_section = !se.is_empty();
            if has_stderr_section {
                title = format!(
                    "{}  |  {}",
                    i18n.t("preview_stdout_header"),
                    i18n.t("preview_stderr_header")
                );
            }
            let empty_label = i18n.t("preview_empty");
            let mut lines: Vec<String> = Vec::new();
            let stdout_heading = i18n.t("stdout");
            lines.push(format!("{} {}", stdout_heading, stdout_path_text));
            if so.is_empty() {
                lines.push(empty_label.clone());
            } else {
                lines.push(so);
            }
            if has_stderr_section {
                lines.push(String::new());
                let stderr_heading = i18n.t("stderr");
                lines.push(format!("{} {}", stderr_heading, stderr_path_text));
                lines.push(se);
            }
            lines.join("\n")
        } else {
            i18n.t("preview_no_selection")
        };

        let para_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if preview_focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            })
            .title(title);
        // Compute inner width/height for accurate wrapping & clamping
        let inner_w = cols[1].width.saturating_sub(2) as usize;
        let inner_h = cols[1].height.saturating_sub(2) as usize;

        // Count wrapped lines
        let total_lines = Self::count_wrapped_lines(&body, inner_w);
        let max_offset = total_lines.saturating_sub(inner_h);
        let clamped = (*preview_offset as usize).min(max_offset) as u16;
        *preview_offset = clamped;

        let para = Paragraph::new(body)
            .block(para_block)
            .wrap(Wrap { trim: false })
            .scroll((clamped, 0));
        let preview_area = cols[1];
        f.render_widget(para, preview_area);

        // Scrollbar (basic)
        let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        // If content fits within viewport, make the thumb 100% height
        let (sb_content_len, sb_pos) = if total_lines <= inner_h {
            (inner_h, 0usize)
        } else {
            (total_lines, clamped as usize)
        };
        let mut sb_state = ScrollbarState::new(sb_content_len)
            .position(sb_pos)
            .viewport_content_length(inner_h);
        f.render_stateful_widget(sb, preview_area, &mut sb_state);

        // Help overlay (Preview)
        if show_help {
            // Center a popup roughly 70% width, min 12 lines
            let popup = {
                let area = preview_area;
                let w = (area.width as f32 * 0.7) as u16;
                let h = 12u16;
                let x = area.x + (area.width.saturating_sub(w)) / 2;
                let y = area.y + (area.height.saturating_sub(h)) / 2;
                ratatui::layout::Rect {
                    x,
                    y,
                    width: w,
                    height: h,
                }
            };
            let lines = vec![
                i18n.t("preview_help_title"),
                String::new(),
                i18n.t("preview_help_move"),
                i18n.t("preview_help_page"),
                i18n.t("preview_help_half"),
                i18n.t("preview_help_top_bottom"),
                i18n.t("preview_help_back"),
                i18n.t("preview_help_start_diff"),
                format!(
                    "{}   {}",
                    i18n.t("preview_help_toggle"),
                    i18n.t("preview_help_quit")
                ),
            ];
            let help_text = Paragraph::new(lines.join("\n"))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(i18n.t("preview_help_title")),
                )
                .wrap(Wrap { trim: false });
            f.render_widget(Clear, popup);
            f.render_widget(help_text, popup);
        }

        // Footer
        let status = if selected_ids.is_empty() {
            i18n.t("status_select_first")
        } else if selected_ids.len() == 1 {
            i18n.t("status_select_second")
        } else {
            i18n.t("selection_complete")
        };
        let nav_hint = i18n.t("status_nav_compact");
        let mut footer_spans = vec![Span::raw(status)];
        if !nav_hint.is_empty() {
            footer_spans.push(Span::raw("  |  "));
            footer_spans.push(Span::raw(nav_hint));
        }
        f.render_widget(
            Paragraph::new(Line::from(footer_spans)).style(Style::default().fg(Color::Gray)),
            rows[2],
        );
    }

    fn count_wrapped_lines(text: &str, width: usize) -> usize {
        if width == 0 {
            return text.lines().count();
        }
        let mut total = 0usize;
        for line in text.split('\n') {
            let segs = Self::wrap_line_to_width(line, width);
            total += segs.len().max(1);
        }
        total
    }

    #[allow(clippy::too_many_arguments)]
    pub fn interactive_select_executions_with_loader<F, D>(
        executions: &[CommandExecution],
        i18n: &I18n,
        tui_simple: bool,
        use_alt_screen: bool,
        max_viewport: Option<usize>,
        linewise: bool,
        loader: F,
        delete_action: Option<D>,
    ) -> Vec<CommandExecution>
    where
        F: FnMut() -> Vec<CommandExecution>,
        D: FnMut(&CommandExecution) -> Result<()>,
    {
        if tui_simple {
            return Self::simple_select_executions(executions, i18n);
        }
        Self::start_interactive_selection_ratatui(
            executions,
            i18n,
            use_alt_screen,
            linewise,
            loader,
            false,
            max_viewport,
            delete_action,
        )
    }

    // legacy crossterm selection impl (removed)
    /* fn start_interactive_selection_impl<F, D>(
        executions: &[CommandExecution],
        i18n: &I18n,
        use_alt_screen: bool,
        linewise: bool,
        mut loader: F,
        on_escape_return_empty: bool,
        max_viewport: Option<usize>,
        mut delete_action: Option<D>,
    ) -> Vec<CommandExecution>
    where
        F: FnMut() -> Vec<CommandExecution>,
        D: FnMut(&CommandExecution) -> Result<()>,
    {
        let mut stdout = io::stdout();

        static INIT_CTRL_C: Once = Once::new();
        INIT_CTRL_C.call_once(|| {
            let _ = ctrlc::set_handler(move || {
                // Best-effort restore terminal state and exit with 130
                let _ = terminal::disable_raw_mode();
                print!("\x1b[?7h\x1b[?25h\x1b[?1049l");
                let _ = io::stdout().flush();
                std::process::exit(130);
            });
        });

        // Set terminal to raw mode, fallback to simple mode if failed
        if terminal::enable_raw_mode().is_err() {
            println!("{}", i18n.t("warning_interactive_failed"));
            return Self::simple_select_executions(executions, i18n);
        }

        if use_alt_screen {
            print!("\x1b[?1049h");
        }
        // Disable line wrap and hide cursor to avoid iTerm2 wrap glitches
        print!("\x1b[?7l\x1b[?25l");
        stdout.flush().ok();

        // Mouse features are disabled to keep UI simple and robust

        let mut selected_ids: Vec<String> = Vec::new();
        let mut filter_input = String::new();
        let mut current_selection = 0;
        let mut scroll_offset: usize = 0;
        let mut preview_target = PreviewTarget::Stdout;
        let mut pending_delete: Option<CommandExecution> = None;
        let mut last_action_message: Option<String> = None;

        // Dataset (may reload on input)
        let mut current_execs: Vec<CommandExecution> = executions.to_vec();
        let mut display_executions: Vec<&CommandExecution> = current_execs.iter().collect();

        loop {
            // Use simple ANSI escape to clear screen and move cursor to top
            print!("\x1b[2J\x1b[H");
            stdout.flush().unwrap();
            // Enforce minimum terminal size to avoid deformed layout on resize
            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
            if cols < MIN_TERMINAL_COLS || rows < MIN_TERMINAL_ROWS {
                let warn = i18n.t_format(
                    "terminal_too_small",
                    &[
                        &MIN_TERMINAL_COLS.to_string(),
                        &MIN_TERMINAL_ROWS.to_string(),
                        &cols.to_string(),
                        &rows.to_string(),
                    ],
                );
                print!("{}\r\n", warn);
                stdout.flush().ok();
                std::thread::sleep(std::time::Duration::from_millis(300));
                continue;
            }

            // Top prompts are removed; a compact bottom status bar shows select and filter hints

            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
            let mut layout = Self::compute_preview_layout(cols as usize, rows);
            if layout.is_none() {
                print!("{}\r\n", i18n.t("preview_single_column_notice"));
            }

            // Use skim-style fuzzy matching to filter records
            let fuzzy_matcher = SkimMatcher::new();
            let filtered_indices: Vec<usize> = if filter_input.is_empty() {
                (0..display_executions.len()).collect()
            } else {
                let items: Vec<(usize, String)> = display_executions
                    .iter()
                    .enumerate()
                    .map(|(i, exec)| {
                        let display_num = (i + 1).to_string();
                        let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
                        let date_str = local_time.format("%Y-%m-%d %H:%M:%S").to_string();

                        // Create searchable text containing serial number, time, command and short code
                        let code = exec.record.short_code.clone().unwrap_or_default();
                        let searchable_text = if code.is_empty() {
                            format!("{} {} {}", display_num, date_str, exec.record.command)
                        } else {
                            format!(
                                "{} {} {} {}",
                                display_num, date_str, exec.record.command, code
                            )
                        };
                        (i, searchable_text)
                    })
                    .collect();

                // Use fuzzy matching for filtering
                let matched_items = fuzzy_matcher.match_and_sort(&filter_input, items);
                matched_items.into_iter().map(|(i, _, _)| i).collect()
            };

            // After we have filtered items, adapt the left panel width
            if let Some(ref mut lay) = layout {
                // Measure max display width of visible lines (without truncation)
                use unicode_width::UnicodeWidthStr;
                let mut max_w = 0usize;
                let prefix_width = 2; // space for current selected mark "✓ " or two spaces
                let end_pos_probe = (scroll_offset
                    + if let Some(mv) = max_viewport {
                        mv.max(3)
                    } else {
                        rows as usize
                    })
                .min(filtered_indices.len());
                for original_i in filtered_indices
                    .iter()
                    .cloned()
                    .skip(scroll_offset)
                    .take(end_pos_probe.saturating_sub(scroll_offset))
                {
                    let exec = &display_executions[original_i];
                    let actual_index = original_i + 1;
                    let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
                    let date_str = local_time.format("%Y-%m-%d %H:%M:%S").to_string();
                    let code = exec.record.short_code.as_deref();
                    let raw_line = if let Some(code) = code {
                        format!(
                            "{}: {}:{} {}: {}",
                            actual_index,
                            i18n.t("short_code_label"),
                            code,
                            i18n.t("time_label"),
                            date_str,
                        )
                    } else {
                        format!("{}: {}: {}", actual_index, i18n.t("time_label"), date_str,)
                    };
                    let w = prefix_width + UnicodeWidthStr::width(raw_line.as_str());
                    if w > max_w {
                        max_w = w;
                    }
                }
                // Clamp left width between 20 and 45% of total columns, plus small padding
                let total_cols = cols as usize;
                let pad = 2usize;
                let min_left = PREVIEW_LEFT_MIN_WIDTH.max(20);
                let max_left = (total_cols as f32 * 0.45) as usize;
                let desired = (max_w + pad).max(min_left).min(max_left);
                // Ensure right side keeps its minimum width
                let allowed_left = desired.min(total_cols.saturating_sub(PREVIEW_RIGHT_MIN_WIDTH));
                lay.left_width = allowed_left;
                lay.start_col = (lay.left_width + 2).min(u16::MAX as usize) as u16;
                lay.preview_width = total_cols.saturating_sub(lay.left_width + 2);
            }

            // Display filtered records
            if filtered_indices.is_empty() {
                // Red text when nothing matches
                print!("\x1b[31m{}\x1b[0m\r\n", i18n.t("no_matches"));
            } else {
                // Ensure current selection is within valid range
                if current_selection >= filtered_indices.len() {
                    current_selection = filtered_indices.len().saturating_sub(1);
                }

                // Determine viewport height from terminal size, reserve some lines for header/footer
                let viewport = if let Some(mv) = max_viewport {
                    let mut v = mv;
                    if v < 3 {
                        v = 3;
                    }
                    v
                } else {
                    let reserved_lines = 5usize; // filter (row0) + header (row1) + bottom status + preview padding
                    let mut viewport = rows as usize;
                    viewport = viewport.saturating_sub(reserved_lines);
                    if viewport < 5 {
                        viewport = 5;
                    }
                    viewport
                };

                // Row 0: filter input prompt + current input
                print!("{}: {}\r\n", i18n.t("status_filter"), filter_input);
                // Row 1: compact header: show current index / total as "cmd i/total"
                let total = filtered_indices.len();
                let current_display = if total == 0 { 0 } else { current_selection + 1 };
                print!("(cmd {}/{})\r\n", current_display, total);

                // Adjust scroll window to keep current selection visible
                if current_selection < scroll_offset {
                    scroll_offset = current_selection;
                } else if current_selection >= scroll_offset + viewport {
                    scroll_offset = current_selection + 1 - viewport;
                }

                let end_pos = (scroll_offset + viewport).min(filtered_indices.len());
                // Keep the list simple; no visual scrollbar

                for (list_idx, original_i_ref) in filtered_indices
                    .iter()
                    .enumerate()
                    .skip(scroll_offset)
                    .take(end_pos - scroll_offset)
                {
                    let original_i = *original_i_ref;
                    let exec = &display_executions[original_i];
                    let actual_index = original_i + 1;
                    let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
                    let date_str = local_time.format("%Y-%m-%d %H:%M:%S");

                    // Fixed-width left padding; show checkmark for already selected entries
                    let is_selected = selected_ids.iter().any(|id| id == &exec.record.record_id);
                    let prefix = if is_selected { "✓ " } else { "  " };
                    let code = exec.record.short_code.as_deref();
                    let raw_line = if let Some(code) = code {
                        format!(
                            "{}{}: {}:{} {}: {}",
                            prefix,
                            actual_index,
                            i18n.t("short_code_label"),
                            code,
                            i18n.t("time_label"),
                            date_str,
                        )
                    } else {
                        format!(
                            "{}{}: {}: {}",
                            prefix,
                            actual_index,
                            i18n.t("time_label"),
                            date_str,
                        )
                    };
                    let line = if let Some(ref layout) = layout {
                        Self::truncate_for_column(&raw_line, layout.left_width.saturating_sub(1))
                    } else {
                        raw_line.clone()
                    };

                    // Current highlight line: put reset code in same line to avoid polluting next line
                    if list_idx == current_selection {
                        print!("\x1b[44;37m{}\x1b[0m\x1b[K\r\n", line);
                    } else if is_selected {
                        print!("\x1b[42;30m{}\x1b[0m\x1b[K\r\n", line);
                    } else {
                        print!("{}\x1b[K\r\n", line);
                    }
                }
            }

            // Bottom status bar: compact messages to avoid overflow (no filter echo here to avoid duplication)
            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
            let select_msg = if selected_ids.is_empty() {
                i18n.t("status_select_first")
            } else if selected_ids.len() == 1 {
                i18n.t("status_select_second")
            } else {
                i18n.t("selection_complete")
            };
            let select_part = Self::truncate_for_column(&select_msg, (cols as usize) / 3);
            let mut status = format!("{} | {}", select_part, i18n.t("status_nav_compact"));
            if let Some(msg) = &last_action_message {
                status.push_str(" | ");
                status.push_str(msg);
            }
            // Move cursor to last line, clear to end, then draw the status bar
            print!(
                "\x1b[{};1H\x1b[90m{}\x1b[0m\x1b[K",
                rows,
                Self::truncate_for_column(&status, cols as usize),
            );

            if let Some(ref layout) = layout {
                let preview_exec = filtered_indices
                    .get(current_selection)
                    .map(|&idx| display_executions[idx]);
                let selected_pair = if selected_ids.len() == 2 {
                    let mut pair: Vec<&CommandExecution> = selected_ids
                        .iter()
                        .filter_map(|id| {
                            current_execs
                                .iter()
                                .find(|exec| &exec.record.record_id == id)
                        })
                        .collect();
                    if pair.len() == 2 {
                        pair.sort_by(|a, b| a.record.timestamp.cmp(&b.record.timestamp));
                        Some((pair[0], pair[1]))
                    } else {
                        None
                    }
                } else {
                    None
                };
                let _ = Self::draw_preview_panel(
                    &mut stdout,
                    layout,
                    i18n,
                    preview_target,
                    preview_exec,
                    selected_pair,
                    linewise,
                );
            }

            // Refresh output
            stdout.flush().unwrap();

            // Read keyboard input
            let mut clear_pending_delete = |last_action: &mut Option<String>| {
                if pending_delete.is_some() {
                    pending_delete = None;
                    *last_action = None;
                }
            };

            match event::read().unwrap() {
                Event::Key(key_event) => {
                    let mut toggle_selection = |step_down: bool| {
                        if filtered_indices.is_empty() {
                            return;
                        }
                        let oi = filtered_indices[current_selection];
                        let ex = &display_executions[oi];
                        if let Some(pos) = selected_ids
                            .iter()
                            .position(|id| id == &ex.record.record_id)
                        {
                            selected_ids.remove(pos);
                            last_action_message = None;
                        } else if selected_ids.len() >= 2 {
                            last_action_message = Some(i18n.t("selection_limit_reached"));
                            return;
                        } else {
                            selected_ids.push(ex.record.record_id.clone());
                            if selected_ids.len() == 2 {
                                last_action_message = Some(i18n.t("selection_complete"));
                            } else {
                                last_action_message = None;
                            }
                        }
                        if step_down {
                            let max_idx = filtered_indices.len().saturating_sub(1);
                            if current_selection < max_idx {
                                current_selection += 1;
                            }
                        }
                    };

                    // Handle Ctrl+C / Ctrl+D for exit (modifiers or control chars)
                    let is_ctrl_combo = key_event.modifiers.contains(KeyModifiers::CONTROL);
                    let is_ctrl_char = match key_event.code {
                        KeyCode::Char(c) if c == '\u{3}' || c == '\u{4}' => true, // ETX (^C) or EOT (^D)
                        _ => false,
                    };
                    if is_ctrl_combo || is_ctrl_char {
                        let exit_match = match key_event.code {
                            KeyCode::Char('c')
                            | KeyCode::Char('C')
                            | KeyCode::Char('d')
                            | KeyCode::Char('D') => true,
                            KeyCode::Char(c) if c == '\u{3}' || c == '\u{4}' => true,
                            _ => false,
                        };
                        if exit_match {
                            // Exit without selection (return empty result)
                            print!("\x1b[2J\x1b[H");
                            stdout.flush().unwrap();
                            // Restore terminal settings (alt screen if used)
                            // Mouse capture not enabled; nothing to disable here
                            if use_alt_screen {
                                print!("\x1b[?1049l");
                            }
                            print!("\x1b[?7h\x1b[?25h");
                            stdout.flush().ok();
                            let _ = terminal::disable_raw_mode();
                            return Vec::new();
                        }
                    }

                    match key_event.code {
                        // Navigation keys
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                            clear_pending_delete(&mut last_action_message);
                            // Move selection up
                            current_selection = current_selection.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                            clear_pending_delete(&mut last_action_message);
                            // Move selection down
                            if !filtered_indices.is_empty()
                                && current_selection < filtered_indices.len() - 1
                            {
                                current_selection += 1;
                            }
                        }
                        // Ctrl-p / Ctrl-n
                        KeyCode::Char('p') if is_ctrl_combo => {
                            clear_pending_delete(&mut last_action_message);
                            current_selection = current_selection.saturating_sub(1);
                        }
                        KeyCode::Char('n') if is_ctrl_combo => {
                            clear_pending_delete(&mut last_action_message);
                            if !filtered_indices.is_empty()
                                && current_selection < filtered_indices.len() - 1
                            {
                                current_selection += 1;
                            }
                        }
                        // PageDown / Ctrl-f, PageUp / Ctrl-b
                        KeyCode::PageDown | KeyCode::Char('f') if is_ctrl_combo => {
                            clear_pending_delete(&mut last_action_message);
                            if !filtered_indices.is_empty() {
                                let viewport = if let Some(mv) = max_viewport {
                                    mv.max(3)
                                } else {
                                    let (_c, r) = crossterm::terminal::size().unwrap_or((80, 24));
                                    let mut v = r as usize;
                                    v = v.saturating_sub(6);
                                    if v < 5 {
                                        v = 5;
                                    }
                                    v
                                };
                                let max_idx = filtered_indices.len().saturating_sub(1);
                                current_selection = (current_selection + viewport).min(max_idx);
                            }
                        }
                        KeyCode::PageUp | KeyCode::Char('b') if is_ctrl_combo => {
                            clear_pending_delete(&mut last_action_message);
                            if !filtered_indices.is_empty() {
                                let viewport = if let Some(mv) = max_viewport {
                                    mv.max(3)
                                } else {
                                    let (_c, r) = crossterm::terminal::size().unwrap_or((80, 24));
                                    let mut v = r as usize;
                                    v = v.saturating_sub(6);
                                    if v < 5 {
                                        v = 5;
                                    }
                                    v
                                };
                                current_selection = current_selection.saturating_sub(viewport);
                            }
                        }
                        // Home/End and Ctrl-a / Ctrl-e
                        KeyCode::Home | KeyCode::Char('a') if is_ctrl_combo => {
                            current_selection = 0;
                        }
                        KeyCode::End | KeyCode::Char('e') if is_ctrl_combo => {
                            if !filtered_indices.is_empty() {
                                current_selection = filtered_indices.len() - 1;
                            }
                        }
                        // Control keys
                        KeyCode::Enter => {
                            // Simple behavior: if two items are selected, execute immediately
                            if selected_ids.len() == 2 {
                                // Exit TUI and return the two selected records regardless of cursor position
                                print!("\x1b[2J\x1b[H");
                                stdout.flush().unwrap();
                                if use_alt_screen {
                                    print!("\x1b[?1049l");
                                }
                                print!("\x1b[?7h\x1b[?25h");
                                stdout.flush().ok();
                                terminal::disable_raw_mode().unwrap();

                                let mut result = Vec::new();
                                for sid in &selected_ids {
                                    if let Some(exec) =
                                        current_execs.iter().find(|e| &e.record.record_id == sid)
                                    {
                                        result.push(exec.clone());
                                    }
                                }
                                result.sort_by(|a, b| a.record.timestamp.cmp(&b.record.timestamp));
                                return result;
                            }

                            // Otherwise, treat Enter as a toggle/select on the current line
                            if filtered_indices.is_empty() {
                                continue;
                            }
                            let oi = filtered_indices[current_selection];
                            let ex = &display_executions[oi];
                            let record_id = &ex.record.record_id;

                            if selected_ids.iter().any(|id| id == record_id) {
                                // Already selected and not yet two items selected: no-op
                            } else if selected_ids.len() >= 2 {
                                // Should not happen due to early return, but keep message for safety
                                last_action_message = Some(i18n.t("selection_limit_reached"));
                            } else {
                                selected_ids.push(record_id.clone());
                                if selected_ids.len() == 2 {
                                    last_action_message = Some(i18n.t("selection_complete"));
                                } else {
                                    last_action_message = None;
                                }
                            }
                        }
                        // Tab: toggle mark; BackTab: toggle and move down one
                        KeyCode::Tab | KeyCode::BackTab => {
                            let step_down = matches!(key_event.code, KeyCode::BackTab);
                            toggle_selection(step_down);
                        }
                        KeyCode::Char(' ') => {
                            toggle_selection(false);
                        }
                        KeyCode::Esc => {
                            // Exit and return default selection
                            // Clear screen and move cursor to top
                            print!("\x1b[2J\x1b[H");
                            stdout.flush().unwrap();
                            // Restore terminal settings
                            if use_alt_screen {
                                print!("\x1b[?1049l");
                            }
                            print!("\x1b[?7h\x1b[?25h");
                            stdout.flush().ok();
                            terminal::disable_raw_mode().unwrap();
                            if on_escape_return_empty {
                                return Vec::new();
                            } else {
                                return executions.iter().take(2).cloned().collect();
                            }
                        }
                        // Clear/kill editing bindings
                        KeyCode::Char('u') if is_ctrl_combo => {
                            filter_input.clear();
                            current_selection = 0;
                            scroll_offset = 0;
                            current_execs = loader();
                            display_executions = current_execs.iter().collect();
                        }
                        KeyCode::Char('w') if is_ctrl_combo => {
                            while filter_input.ends_with(char::is_whitespace) {
                                filter_input.pop();
                            }
                            while !filter_input.is_empty()
                                && !filter_input.ends_with(char::is_whitespace)
                            {
                                filter_input.pop();
                            }
                            current_selection = 0;
                            scroll_offset = 0;
                            current_execs = loader();
                            display_executions = current_execs.iter().collect();
                        }
                        _ if Self::is_shift_backspace_event(&key_event) => {
                            if delete_action.is_none() {
                                clear_pending_delete(&mut last_action_message);
                                filter_input.pop();
                                current_selection = 0;
                                scroll_offset = 0;
                                current_execs = loader();
                                display_executions = current_execs.iter().collect();
                            } else if filtered_indices.is_empty() {
                                clear_pending_delete(&mut last_action_message);
                                last_action_message = Some(i18n.t("delete_nothing"));
                            } else {
                                let oi = filtered_indices[current_selection];
                                let target_ref = display_executions[oi];
                                let timestamp_display = target_ref
                                    .record
                                    .timestamp
                                    .with_timezone(&chrono::Local)
                                    .format("%Y-%m-%d %H:%M:%S")
                                    .to_string();
                                let record_id = target_ref.record.record_id.clone();
                                let target_exec = (*target_ref).clone();

                                if pending_delete.as_ref().map(|p| p.record.record_id.as_str())
                                    == Some(record_id.as_str())
                                {
                                    if let Some(delete_fn) = delete_action.as_mut() {
                                        match delete_fn(&target_exec) {
                                            Ok(()) => {
                                                selected_ids.retain(|id| id != &record_id);
                                                pending_delete = None;
                                                last_action_message = Some(i18n.t_format(
                                                    "delete_success_status",
                                                    &[&timestamp_display],
                                                ));
                                                current_execs = loader();
                                                display_executions = current_execs.iter().collect();
                                                let total_items = display_executions.len();
                                                if total_items == 0 {
                                                    current_selection = 0;
                                                } else if current_selection >= total_items {
                                                    current_selection = total_items - 1;
                                                }
                                                scroll_offset = 0;
                                            }
                                            Err(err) => {
                                                pending_delete = Some(target_exec);
                                                last_action_message = Some(i18n.t_format(
                                                    "delete_failed_status",
                                                    &[&err.to_string()],
                                                ));
                                            }
                                        }
                                    }
                                } else {
                                    pending_delete = Some(target_exec);
                                    last_action_message =
                                        Some(i18n.t_format(
                                            "delete_confirm_status",
                                            &[&timestamp_display],
                                        ));
                                }
                            }
                        }
                        // Ctrl+X behaves like Shift+Backspace (delete current record with confirm)
                        KeyCode::Char('x') if is_ctrl_combo => {
                            if delete_action.is_none() {
                                clear_pending_delete(&mut last_action_message);
                                // When deletion is not available in this screen, treat as no-op
                            } else if filtered_indices.is_empty() {
                                clear_pending_delete(&mut last_action_message);
                                last_action_message = Some(i18n.t("delete_nothing"));
                            } else {
                                let oi = filtered_indices[current_selection];
                                let target_ref = display_executions[oi];
                                let timestamp_display = target_ref
                                    .record
                                    .timestamp
                                    .with_timezone(&chrono::Local)
                                    .format("%Y-%m-%d %H:%M:%S")
                                    .to_string();
                                let record_id = target_ref.record.record_id.clone();
                                let target_exec = (*target_ref).clone();

                                if pending_delete.as_ref().map(|p| p.record.record_id.as_str())
                                    == Some(record_id.as_str())
                                {
                                    if let Some(delete_fn) = delete_action.as_mut() {
                                        match delete_fn(&target_exec) {
                                            Ok(()) => {
                                                selected_ids.retain(|id| id != &record_id);
                                                pending_delete = None;
                                                last_action_message = Some(i18n.t_format(
                                                    "delete_success_status",
                                                    &[&timestamp_display],
                                                ));
                                                current_execs = loader();
                                                display_executions = current_execs.iter().collect();
                                                let total_items = display_executions.len();
                                                if total_items == 0 {
                                                    current_selection = 0;
                                                } else if current_selection >= total_items {
                                                    current_selection = total_items - 1;
                                                }
                                                scroll_offset = 0;
                                            }
                                            Err(err) => {
                                                pending_delete = Some(target_exec);
                                                last_action_message = Some(i18n.t_format(
                                                    "delete_failed_status",
                                                    &[&err.to_string()],
                                                ));
                                            }
                                        }
                                    }
                                } else {
                                    pending_delete = Some(target_exec);
                                    last_action_message =
                                        Some(i18n.t_format(
                                            "delete_confirm_status",
                                            &[&timestamp_display],
                                        ));
                                }
                            }
                        }
                        _ if Self::is_backspace_event(&key_event) => {
                            clear_pending_delete(&mut last_action_message);
                            filter_input.pop();
                            current_selection = 0;
                            scroll_offset = 0;
                            current_execs = loader();
                            display_executions = current_execs.iter().collect();
                        }
                        KeyCode::Delete => {
                            // Clear filter input and reset view
                            filter_input.clear();
                            current_selection = 0;
                            scroll_offset = 0;
                            // Reload dataset on input
                            current_execs = loader();
                            display_executions = current_execs.iter().collect();
                        }
                        KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Char('o')
                        | KeyCode::Char('O') => {
                            preview_target = preview_target.toggle();
                        }
                        // Character input
                        KeyCode::Char(c) => {
                            // Add character to filter input and reset view
                            filter_input.push(c);
                            current_selection = 0;
                            scroll_offset = 0;
                            // Reload dataset on input
                            current_execs = loader();
                            display_executions = current_execs.iter().collect();
                        }
                        _ => {}
                    }
                }

                Event::Resize(_, _) => { /* next loop will re-render with new size */ }
                Event::FocusGained | Event::FocusLost => { /* ignore */ }
                Event::Paste(_) => { /* ignore */ }
                Event::Mouse(_) => { /* ignore */ }
            }
        }
    }
    */

    fn simple_select_executions(
        executions: &[CommandExecution],
        i18n: &I18n,
    ) -> Vec<CommandExecution> {
        if executions.len() <= 2 {
            return executions.to_vec();
        }

        println!(
            "{}",
            i18n.t_format("select_executions", &[&executions.len().to_string()])
        );

        // Render records (compact: short code and time only)
        for (i, exec) in executions.iter().enumerate() {
            let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
            let date_str = local_time.format("%Y-%m-%d %H:%M:%S");
            if let Some(code) = &exec.record.short_code {
                println!(
                    "{}: {}:{} {}: {}",
                    i + 1,
                    i18n.t("short_code_label"),
                    code,
                    i18n.t("time_label"),
                    date_str,
                );
            } else {
                println!("{}: {}: {}", i + 1, i18n.t("time_label"), date_str,);
            }
        }

        println!("{}", i18n.t("input_numbers"));

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        // Check if it's short-code filter mode (e.g., ab or ab cd)
        let input = input.trim();
        if let Some(codes) = Self::is_code_filter_input(input, executions) {
            return Self::filter_by_code(executions, &codes);
        }

        // Check if it's date filter mode (supports fuzzy matching)
        if Self::is_date_filter_input(input, i18n) {
            return Self::filter_by_date(executions, input, i18n);
        }

        // Number selection mode
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() != 2 {
            println!("{}", i18n.t("invalid_input"));
            return executions.iter().take(2).cloned().collect();
        }

        let indices: Vec<usize> = parts
            .iter()
            .filter_map(|s| s.parse::<usize>().ok())
            .filter(|&i| i > 0 && i <= executions.len())
            .collect();

        if indices.len() != 2 {
            println!("{}", i18n.t("invalid_input"));
            return executions.iter().take(2).cloned().collect();
        }

        let mut selected = Vec::new();
        for &i in &indices {
            selected.push(executions[i - 1].clone());
        }

        selected.sort_by(|a, b| a.record.timestamp.cmp(&b.record.timestamp));
        selected
    }

    fn is_code_filter_input(input: &str, executions: &[CommandExecution]) -> Option<Vec<String>> {
        if input.is_empty() {
            return None;
        }
        // Split by whitespace or comma
        let tokens: Vec<&str> = input
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .collect();

        if tokens.is_empty() || tokens.len() > 2 {
            return None;
        }

        // Gather available short codes
        use std::collections::HashSet;
        let available: HashSet<String> = executions
            .iter()
            .filter_map(|e| e.record.short_code.clone())
            .collect();

        // Validate tokens: must be strictly base62 and exist in available
        let mut picked: Vec<String> = Vec::new();
        for t in tokens {
            if !t.chars().all(|c| c.is_ascii_alphanumeric()) {
                return None;
            }
            if available.contains(t) {
                picked.push(t.to_string());
            }
        }

        if picked.is_empty() {
            None
        } else {
            Some(picked)
        }
    }

    fn filter_by_code(executions: &[CommandExecution], codes: &[String]) -> Vec<CommandExecution> {
        // Try to pick records by provided codes. If only one code, pair with the latest other record.
        let mut map: std::collections::HashMap<String, CommandExecution> =
            std::collections::HashMap::new();
        for e in executions.iter() {
            if let Some(code) = &e.record.short_code {
                map.insert(code.clone(), e.clone());
            }
        }

        let mut selected: Vec<CommandExecution> = Vec::new();

        if !codes.is_empty() {
            if let Some(e) = map.get(&codes[0]) {
                selected.push(e.clone());
            }
        }
        if codes.len() >= 2 {
            if let Some(e) = map.get(&codes[1]) {
                selected.push(e.clone());
            }
        }

        // If only one picked or duplicates, add another record (latest not the same)
        if selected.len() < 2 {
            let mut sorted = executions.to_vec();
            sorted.sort_by(|a, b| b.record.timestamp.cmp(&a.record.timestamp));
            for e in sorted {
                if selected
                    .iter()
                    .all(|s| s.record.record_id != e.record.record_id)
                {
                    selected.push(e);
                    break;
                }
            }
        }

        if selected.len() < 2 {
            // Fallback to first two
            return executions.iter().take(2).cloned().collect();
        }

        selected.sort_by(|a, b| a.record.timestamp.cmp(&b.record.timestamp));
        selected
    }

    fn is_date_filter_input(input: &str, i18n: &I18n) -> bool {
        // Check if input contains date format (e.g., 2024-01, 01-15, 2024, etc.)
        let date_patterns = [
            r"\d{4}-\d{2}",     // YYYY-MM
            r"\d{2}-\d{2}",     // MM-DD
            r"\d{4}",           // YYYY
            r"\d{1,2}/\d{1,2}", // MM/DD
            r"\d{4}/\d{1,2}",   // YYYY/MM
        ];

        for pattern in &date_patterns {
            if regex::Regex::new(pattern).unwrap().is_match(input) {
                return true;
            }
        }

        // Check if it's month name (supports English abbr and common Chinese names)
        let month_names_en = [
            "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
        ];
        let mut month_names_cn: Vec<String> = Vec::new();
        for i in 1..=12 {
            month_names_cn.push(i18n.t(&format!("month_{}", Self::get_month_en_name(i))));
        }

        let lower_input = input.to_lowercase();
        month_names_en
            .iter()
            .any(|&month| lower_input.contains(month))
            || month_names_cn.iter().any(|month| input.contains(month))
    }

    fn filter_by_date(
        executions: &[CommandExecution],
        filter: &str,
        i18n: &I18n,
    ) -> Vec<CommandExecution> {
        let mut filtered: Vec<CommandExecution> = executions
            .iter()
            .filter(|exec| Self::matches_date_filter(&exec.record.timestamp, filter, i18n))
            .cloned()
            .collect();

        if filtered.len() < 2 {
            println!("{}", i18n.t("few_records_fallback"));
            return executions.iter().take(2).cloned().collect();
        }

        // If more than two matches, choose the latest two
        filtered.sort_by(|a, b| b.record.timestamp.cmp(&a.record.timestamp));
        filtered.truncate(2);
        filtered.sort_by(|a, b| a.record.timestamp.cmp(&b.record.timestamp));

        println!("{}", i18n.t("using_filtered_records"));
        for exec in &filtered {
            println!("  - {}", exec.record.timestamp.format("%Y-%m-%d %H:%M:%S"));
        }

        filtered
    }

    fn matches_date_filter(timestamp: &DateTime<chrono::Utc>, filter: &str, i18n: &I18n) -> bool {
        let date_str = timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let lower_filter = filter.to_lowercase();

        // Year match
        if filter.len() == 4 {
            if let Ok(year) = filter.parse::<i32>() {
                return timestamp.year() == year;
            }
        }

        // Year-month match (YYYY-MM)
        if filter.len() == 7 && filter.contains('-') {
            let parts: Vec<&str> = filter.split('-').collect();
            if parts.len() == 2 {
                if let Ok(year) = parts[0].parse::<i32>() {
                    if let Ok(month) = parts[1].parse::<u32>() {
                        return timestamp.year() == year && timestamp.month() == month;
                    }
                }
            }
        }

        // Month-day match (MM-DD)
        if filter.len() == 5 && filter.contains('-') {
            let parts: Vec<&str> = filter.split('-').collect();
            if parts.len() == 2 {
                if let Ok(month) = parts[0].parse::<u32>() {
                    if let Ok(day) = parts[1].parse::<u32>() {
                        return timestamp.month() == month && timestamp.day() == day;
                    }
                }
            }
        }

        // Month name match (English abbr and Chinese names)
        let month_names_en = [
            "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
        ];
        let mut month_names_cn: Vec<String> = Vec::new();
        for i in 1..=12 {
            month_names_cn.push(i18n.t(&format!("month_{}", Self::get_month_en_name(i))));
        }

        for (i, month_name) in month_names_en.iter().enumerate() {
            if lower_filter.contains(month_name) {
                return timestamp.month() == (i + 1) as u32;
            }
        }

        for (i, month_name) in month_names_cn.iter().enumerate() {
            if filter.contains(month_name) {
                return timestamp.month() == (i + 1) as u32;
            }
        }

        // Fuzzy match date string
        date_str.to_lowercase().contains(&lower_filter)
    }

    fn get_month_en_name(month: u32) -> &'static str {
        match month {
            1 => "jan",
            2 => "feb",
            3 => "mar",
            4 => "apr",
            5 => "may",
            6 => "jun",
            7 => "jul",
            8 => "aug",
            9 => "sep",
            10 => "oct",
            11 => "nov",
            12 => "dec",
            _ => "unknown",
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test_support {
    use super::*;

    impl Differ {
        pub fn select_executions_for_diff(
            executions: &[CommandExecution],
            i18n: &I18n,
        ) -> Vec<CommandExecution> {
            if executions.len() <= 2 {
                return executions.to_vec();
            }

            println!(
                "{}",
                i18n.t_format("select_executions", &[&executions.len().to_string()])
            );

            for (i, exec) in executions.iter().enumerate() {
                let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
                let date_str = local_time.format("%Y-%m-%d %H:%M:%S");
                if let Some(code) = &exec.record.short_code {
                    println!(
                        "{}: {}:{} {}: {}",
                        i + 1,
                        i18n.t("short_code_label"),
                        code,
                        i18n.t("time_label"),
                        date_str,
                    );
                } else {
                    println!("{}: {}: {}", i + 1, i18n.t("time_label"), date_str,);
                }
            }

            println!("{}", i18n.t("input_numbers"));

            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();

            let input = input.trim();
            if Self::is_date_filter_input(input, i18n) {
                return Self::filter_by_date(executions, input, i18n);
            }

            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() != 2 {
                println!("{}", i18n.t("invalid_input"));
                return executions.iter().take(2).cloned().collect();
            }

            let indices: Vec<usize> = parts
                .iter()
                .filter_map(|s| s.parse::<usize>().ok())
                .filter(|&i| i > 0 && i <= executions.len())
                .collect();

            if indices.len() != 2 {
                println!("{}", i18n.t("invalid_input"));
                return executions.iter().take(2).cloned().collect();
            }

            let mut selected = Vec::new();
            for &i in &indices {
                selected.push(executions[i - 1].clone());
            }

            selected.sort_by(|a, b| a.record.timestamp.cmp(&b.record.timestamp));
            selected
        }

        pub fn interactive_select_executions(
            executions: &[CommandExecution],
            i18n: &I18n,
            tui_simple: bool,
            use_alt_screen: bool,
            linewise: bool,
        ) -> Vec<CommandExecution> {
            if executions.len() <= 2 {
                return executions.to_vec();
            }

            if tui_simple {
                return Self::simple_select_executions(executions, i18n);
            }

            Self::start_interactive_selection_ratatui(
                executions,
                i18n,
                use_alt_screen,
                linewise,
                || executions.to_vec(),
                false,
                None,
                None::<fn(&CommandExecution) -> Result<()>>,
            )
        }
    }
}
