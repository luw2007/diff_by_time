use crate::fuzzy_matcher::SkimMatcher;
use crate::i18n::I18n;
use crate::storage::CommandExecution;
use crate::store_manager::StoreManager;
use anyhow::Result;
use chrono::{DateTime, Datelike};
use colored::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal,
};
use similar::{ChangeTag, TextDiff};
use std::io::{self, Write};
use std::sync::Once;

pub struct Differ;

struct CommandGroup {
    command_hash: String,
    command: String,
    count: usize,
    latest: chrono::DateTime<chrono::Utc>,
}

impl Differ {
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
            Self::interactive_select_command_string(&groups, i18n, use_alt_screen, max_viewport)
        };
        Ok(result)
    }

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
            print!("{}\r\n", i18n.t("select_clean_command"));
            print!("{}: ", i18n.t("interactive_filter"));
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
                    KeyCode::Backspace => {
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

    pub fn select_file_for_clean(
        files: &[std::path::PathBuf],
        i18n: &I18n,
        tui_simple: bool,
        use_alt_screen: bool,
        max_viewport: Option<usize>,
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

        // Interactive mode
        if terminal::enable_raw_mode().is_err() {
            return Ok(None);
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
            print!("{}\r\n", i18n.t("select_clean_file"));
            print!("{}: ", i18n.t("interactive_filter"));
            print!("{}\r\n\r\n", filter_input);

            let items: Vec<(usize, String)> = files
                .iter()
                .enumerate()
                .map(|(i, p)| (i, p.display().to_string()))
                .collect();
            let filtered_indices: Vec<usize> = if filter_input.is_empty() {
                (0..files.len()).collect()
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
                for (list_idx, i_ref) in filtered_indices
                    .iter()
                    .enumerate()
                    .skip(scroll_offset)
                    .take(end - scroll_offset)
                {
                    let i = *i_ref;
                    let line = format!("{}: {}", i + 1, files[i].display());
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
                        return Ok(None);
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
                            let i = filtered_indices[current_selection];
                            if use_alt_screen {
                                print!("\x1b[?1049l");
                            }
                            print!("\x1b[?7h\x1b[?25h");
                            stdout.flush().ok();
                            let _ = terminal::disable_raw_mode();
                            return Ok(Some(files[i].clone()));
                        }
                    }
                    KeyCode::Backspace => {
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
                        return Ok(None);
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
    pub fn command_then_diff_flow(
        store: &StoreManager,
        i18n: &I18n,
        tui_simple: bool,
        use_alt_screen: bool,
        max_viewport: Option<usize>,
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
                // Interactive record selection with back support
                let store_ref = store;
                let hash_clone = command_hash.clone();
                executions = Self::start_interactive_selection_impl(
                    &executions,
                    i18n,
                    use_alt_screen,
                    || {
                        store_ref
                            .find_executions(&hash_clone, i18n)
                            .unwrap_or_default()
                    },
                    true, // Esc returns empty => go back to command list
                    max_viewport,
                );
                if executions.is_empty() {
                    // Go back to command list
                    continue;
                }
            }

            if let Some(diff_output) = Self::diff_executions(&executions, i18n) {
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
                    KeyCode::Backspace => {
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
    pub fn diff_executions(executions: &[CommandExecution], i18n: &I18n) -> Option<String> {
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
            output.push_str(&Self::diff_text(&earlier.stdout, &later.stdout));
            output.push('\n');
        }

        if earlier.stderr != later.stderr {
            output.push_str(&format!("{}\n", i18n.t("stderr_diff").red().bold()));
            output.push_str(&Self::diff_text(&earlier.stderr, &later.stderr));
            output.push('\n');
        }

        if earlier.stdout == later.stdout && earlier.stderr == later.stderr {
            output.push_str(&format!("{}\n", i18n.t("output_identical").green().bold()));
        }

        Some(output)
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

    #[allow(dead_code)]
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

        // Display all execution records
        for (i, exec) in executions.iter().enumerate() {
            let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
            let date_str = local_time.format("%Y-%m-%d %H:%M:%S");
            if let Some(code) = &exec.record.short_code {
                println!(
                    "{}: {} (exit code: {}, time: {}) [{}: {}]",
                    i + 1,
                    exec.record.command,
                    exec.record.exit_code,
                    date_str,
                    i18n.t("short_code_label"),
                    code
                );
            } else {
                println!(
                    "{}: {} (exit code: {}, time: {})",
                    i + 1,
                    exec.record.command,
                    exec.record.exit_code,
                    date_str
                );
            }
        }

        println!("{}", i18n.t("input_numbers"));

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        // Check if it's date filter mode (supports fuzzy matching)
        let input = input.trim();
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

    #[allow(dead_code)]
    pub fn interactive_select_executions(
        executions: &[CommandExecution],
        i18n: &I18n,
        tui_simple: bool,
        use_alt_screen: bool,
    ) -> Vec<CommandExecution> {
        if executions.len() <= 2 {
            return executions.to_vec();
        }

        if tui_simple {
            return Self::simple_select_executions(executions, i18n);
        }

        // Start interactive selection interface
        Self::start_interactive_selection_impl(
            executions,
            i18n,
            use_alt_screen,
            || executions.to_vec(),
            false,
            None,
        )
    }

    pub fn interactive_select_executions_with_loader<F>(
        executions: &[CommandExecution],
        i18n: &I18n,
        tui_simple: bool,
        use_alt_screen: bool,
        max_viewport: Option<usize>,
        loader: F,
    ) -> Vec<CommandExecution>
    where
        F: FnMut() -> Vec<CommandExecution>,
    {
        if tui_simple {
            return Self::simple_select_executions(executions, i18n);
        }
        Self::start_interactive_selection_impl(
            executions,
            i18n,
            use_alt_screen,
            loader,
            false,
            max_viewport,
        )
    }

    fn start_interactive_selection_impl<F>(
        executions: &[CommandExecution],
        i18n: &I18n,
        use_alt_screen: bool,
        mut loader: F,
        on_escape_return_empty: bool,
        max_viewport: Option<usize>,
    ) -> Vec<CommandExecution>
    where
        F: FnMut() -> Vec<CommandExecution>,
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

        let mut selected_ids: Vec<String> = Vec::new();
        let mut filter_input = String::new();
        let mut current_selection = 0;
        let mut scroll_offset: usize = 0;

        // Dataset (may reload on input)
        let mut current_execs: Vec<CommandExecution> = executions.to_vec();
        let mut display_executions: Vec<&CommandExecution> = current_execs.iter().collect();

        loop {
            // Use simple ANSI escape to clear screen and move cursor to top
            print!("\x1b[2J\x1b[H");
            stdout.flush().unwrap();

            // Display title and input prompt
            if selected_ids.is_empty() {
                print!("{}\r\n", i18n.t("first_selection"));
            } else if selected_ids.len() == 1 {
                print!("{}\r\n", i18n.t("second_selection"));
            }

            // Display filter input line
            print!("{}: ", i18n.t("interactive_filter"));
            print!("{}\r\n", filter_input);
            print!("\r\n");

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
                    let (_cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                    let reserved_lines = 6usize; // title + filter + blank + hint + margins
                    let mut viewport = rows as usize;
                    viewport = viewport.saturating_sub(reserved_lines);
                    if viewport < 5 {
                        viewport = 5;
                    }
                    viewport
                };

                // Adjust scroll window to keep current selection visible
                if current_selection < scroll_offset {
                    scroll_offset = current_selection;
                } else if current_selection >= scroll_offset + viewport {
                    scroll_offset = current_selection + 1 - viewport;
                }

                let end_pos = (scroll_offset + viewport).min(filtered_indices.len());
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

                    // Prefix: show mark if selected (by record_id)
                    let is_selected = selected_ids.iter().any(|id| id == &exec.record.record_id);
                    let prefix = if is_selected { "✓ " } else { "  " };
                    let code = exec.record.short_code.as_deref();
                    let line = if let Some(code) = code {
                        format!(
                            "{}{}: {} (exit code: {}, time: {}) [{}: {}]",
                            prefix,
                            actual_index,
                            exec.record.command,
                            exec.record.exit_code,
                            date_str,
                            i18n.t("short_code_label"),
                            code
                        )
                    } else {
                        format!(
                            "{}{}: {} (exit code: {}, time: {})",
                            prefix,
                            actual_index,
                            exec.record.command,
                            exec.record.exit_code,
                            date_str
                        )
                    };

                    // Current highlight line: put reset code in same line to avoid polluting next line
                    if list_idx == current_selection {
                        print!("\x1b[44;37m{}\x1b[0m\x1b[K\r\n", line);
                    } else {
                        print!("{}\x1b[K\r\n", line);
                    }
                }
            }

            // Display navigation hints
            print!("\r\n");
            // Dim hint line
            print!("\x1b[90m{}\x1b[0m\r\n", i18n.t("navigate_hint"));

            // Refresh output
            stdout.flush().unwrap();

            // Read keyboard input
            if let Event::Key(key_event) = event::read().unwrap() {
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
                    } else {
                        selected_ids.push(ex.record.record_id.clone());
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
                        // Move selection up
                        current_selection = current_selection.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                        // Move selection down
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
                        // If already marked two, confirm and exit; otherwise mark current and exit if two.
                        if !filtered_indices.is_empty() {
                            if selected_ids.len() >= 2 {
                                // finalize immediately
                            } else {
                                let oi = filtered_indices[current_selection];
                                let ex = &display_executions[oi];
                                if !selected_ids.iter().any(|id| id == &ex.record.record_id) {
                                    selected_ids.push(ex.record.record_id.clone());
                                }
                            }

                            if selected_ids.len() >= 2 {
                                print!("\x1b[2J\x1b[H");
                                print!("\x1b[32m{}\x1b[0m\r\n", i18n.t("selection_complete"));
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
                        }
                    }
                    // Tab: toggle mark; BackTab: toggle and move down one
                    KeyCode::Tab | KeyCode::BackTab => {
                        let step_down = matches!(key_event.code, KeyCode::BackTab);
                        toggle_selection(step_down);
                    }
                    KeyCode::Char(' ') => {
                        let step_down = key_event.modifiers.contains(KeyModifiers::SHIFT);
                        toggle_selection(step_down);
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
                    KeyCode::Backspace => {
                        // Delete last character and reset view
                        filter_input.pop();
                        current_selection = 0;
                        scroll_offset = 0;
                        // Reload dataset on input
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
        }
    }

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

        // Display all execution records (with short code)
        for (i, exec) in executions.iter().enumerate() {
            let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
            let date_str = local_time.format("%Y-%m-%d %H:%M:%S");
            if let Some(code) = &exec.record.short_code {
                println!(
                    "{}: {} (exit code: {}, time: {}) [{}: {}]",
                    i + 1,
                    exec.record.command,
                    exec.record.exit_code,
                    date_str,
                    i18n.t("short_code_label"),
                    code
                );
            } else {
                println!(
                    "{}: {} (exit code: {}, time: {})",
                    i + 1,
                    exec.record.command,
                    exec.record.exit_code,
                    date_str
                );
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
