use crate::storage::CommandExecution;
use crate::i18n::I18n;
use crate::fuzzy_matcher::FzfMatcher;
use similar::{ChangeTag, TextDiff};
use colored::*;
use chrono::{DateTime, Datelike};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal,
};
use std::io::{self, Write};
use std::sync::Once;

pub struct Differ;

impl Differ {
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
            i18n.t_format("diff_command", &[&later.record.command]).bold().cyan()
        ));

        let earlier_local = earlier.record.timestamp.with_timezone(&chrono::Local);
        let later_local = later.record.timestamp.with_timezone(&chrono::Local);
        output.push_str(&format!(
            "{} vs {}\n",
            i18n.t_format("diff_earlier", &[&earlier_local.format("%Y-%m-%d %H:%M:%S").to_string()]).yellow(),
            i18n.t_format("diff_later", &[&later_local.format("%Y-%m-%d %H:%M:%S").to_string()]).green()
        ));

        if earlier.record.exit_code != later.record.exit_code {
            output.push_str(&i18n.t_format("diff_exit_code", &[
                &earlier.record.exit_code.to_string(),
                &later.record.exit_code.to_string()
            ]));
            output.push('\n');
        }

        output.push_str(&i18n.t_format("diff_execution_time", &[
            &earlier.record.duration_ms.to_string(),
            &later.record.duration_ms.to_string()
        ]));
        output.push('\n');

        output.push_str("\n");

        if earlier.stdout != later.stdout {
            output.push_str(&format!("{}\n", i18n.t("stdout_diff").yellow().bold()));
            output.push_str(&Self::diff_text(&earlier.stdout, &later.stdout));
            output.push_str("\n");
        }

        if earlier.stderr != later.stderr {
            output.push_str(&format!("{}\n", i18n.t("stderr_diff").red().bold()));
            output.push_str(&Self::diff_text(&earlier.stderr, &later.stderr));
            output.push_str("\n");
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
    pub fn select_executions_for_diff(executions: &[CommandExecution], i18n: &I18n) -> Vec<CommandExecution> {
        if executions.len() <= 2 {
            return executions.to_vec();
        }

        println!("{}", i18n.t_format("select_executions", &[&executions.len().to_string()]));

        // Display all execution records
        for (i, exec) in executions.iter().enumerate() {
            let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
            let date_str = local_time.format("%Y-%m-%d %H:%M:%S");
            println!(
                "{}: {} (exit code: {}, time: {})",
                i + 1,
                exec.record.command,
                exec.record.exit_code,
                date_str
            );
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

        let indices: Vec<usize> = parts.iter()
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
        Self::start_interactive_selection_impl(executions, i18n, use_alt_screen, || executions.to_vec())
    }

    pub fn interactive_select_executions_with_loader<F>(
        executions: &[CommandExecution],
        i18n: &I18n,
        tui_simple: bool,
        use_alt_screen: bool,
        loader: F,
    ) -> Vec<CommandExecution>
    where
        F: FnMut() -> Vec<CommandExecution>,
    {
        if tui_simple {
            return Self::simple_select_executions(executions, i18n);
        }
        Self::start_interactive_selection_impl(executions, i18n, use_alt_screen, loader)
    }

    fn start_interactive_selection_impl<F>(
        executions: &[CommandExecution],
        i18n: &I18n,
        use_alt_screen: bool,
        mut loader: F,
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
        if let Err(_) = terminal::enable_raw_mode() {
            println!("{}", i18n.t("warning_interactive_failed"));
            return Self::simple_select_executions(executions, i18n);
        }

        if use_alt_screen { print!("\x1b[?1049h"); }
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
            // Use simple method to clear screen - avoid complex terminal control
            print!("\x1b[2J\x1b[H"); // ANSI转义码：清屏并将光标移动到顶部
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

            // Use fzf-style fuzzy matching to filter records
            let fuzzy_matcher = FzfMatcher::new();
            let filtered_indices: Vec<usize> = if filter_input.is_empty() {
                (0..display_executions.len()).collect()
            } else {
                let items: Vec<(usize, String)> = display_executions.iter().enumerate()
                    .map(|(i, exec)| {
                        let display_num = (i + 1).to_string();
                        let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
                        let date_str = local_time.format("%Y-%m-%d %H:%M:%S").to_string();

                        // Create searchable text containing serial number, time and command
                        let searchable_text = format!("{} {} {}", display_num, date_str, exec.record.command);
                        (i, searchable_text)
                    })
                    .collect();

                // Use fuzzy matching for filtering
                let matched_items = fuzzy_matcher.match_and_sort(&filter_input, items);
                matched_items.into_iter().map(|(i, _, _)| i).collect()
            };

            // Display filtered records
            if filtered_indices.is_empty() {
                print!("\x1b[31m{}\x1b[0m\r\n", i18n.t("no_matches")); // 红色文本
            } else {
                // Ensure current selection is within valid range
                if current_selection >= filtered_indices.len() {
                    current_selection = filtered_indices.len().saturating_sub(1);
                }

                // Determine viewport height from terminal size, reserve some lines for header/footer
                let (_cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                let reserved_lines = 6usize; // title + filter + blank + hint + margins
                let mut viewport = rows as usize;
                viewport = viewport.saturating_sub(reserved_lines);
                if viewport < 5 { viewport = 5; }

                // Adjust scroll window to keep current selection visible
                if current_selection < scroll_offset {
                    scroll_offset = current_selection;
                } else if current_selection >= scroll_offset + viewport {
                    scroll_offset = current_selection + 1 - viewport;
                }

                let end_pos = (scroll_offset + viewport).min(filtered_indices.len());
                for list_idx in scroll_offset..end_pos {
                    let original_i = filtered_indices[list_idx];
                    let exec = &display_executions[original_i];
                    let actual_index = original_i + 1;
                    let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
                    let date_str = local_time.format("%Y-%m-%d %H:%M:%S");

                    // Prefix: show mark if selected (by record_id)
                    let is_selected = selected_ids.iter().any(|id| id == &exec.record.record_id);
                    let prefix = if is_selected { "✓ " } else { "  " };
                    let line = format!(
                        "{}{}: {} (exit code: {}, time: {})",
                        prefix,
                        actual_index,
                        exec.record.command,
                        exec.record.exit_code,
                        date_str
                    );

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
            print!("\x1b[90m{}\x1b[0m\r\n", i18n.t("navigate_hint")); // 暗灰色文本

            // Refresh output
            stdout.flush().unwrap();

            // Read keyboard input
            if let Event::Key(key_event) = event::read().unwrap() {
                // Handle Ctrl+C / Ctrl+D for exit (modifiers or control chars)
                let is_ctrl_combo = key_event.modifiers.contains(KeyModifiers::CONTROL);
                let is_ctrl_char = match key_event.code {
                    KeyCode::Char(c) if c == '\u{3}' || c == '\u{4}' => true, // ETX (^C) or EOT (^D)
                    _ => false,
                };
                if is_ctrl_combo || is_ctrl_char {
                    let exit_match = match key_event.code {
                        KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Char('d') | KeyCode::Char('D') => true,
                        KeyCode::Char(c) if c == '\u{3}' || c == '\u{4}' => true,
                        _ => false,
                    };
                    if exit_match {
                        // Exit without selection (return empty result)
                        print!("\x1b[2J\x1b[H");
                        stdout.flush().unwrap();
                        // Restore terminal settings (alt screen if used)
                        if use_alt_screen { print!("\x1b[?1049l"); }
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
                        if current_selection > 0 {
                            current_selection -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                        // Move selection down
                        if !filtered_indices.is_empty() && current_selection < filtered_indices.len() - 1 {
                            current_selection += 1;
                        }
                    }
                    // Control keys
                    KeyCode::Enter => {
                        // Select current item
                        if !filtered_indices.is_empty() {
                            let selected_original_index = filtered_indices[current_selection];
                            let selected_exec = &display_executions[selected_original_index];
                            if !selected_ids.iter().any(|id| id == &selected_exec.record.record_id) {
                                selected_ids.push(selected_exec.record.record_id.clone());

                                if selected_ids.len() == 2 {
                                    // Selection complete
                                    print!("\x1b[2J\x1b[H"); // 清屏并移动光标到顶部
                                    print!("\x1b[32m{}\x1b[0m\r\n", i18n.t("selection_complete")); // 绿色文本
                                    stdout.flush().unwrap();

                                    // Restore terminal settings (alt screen if used), then exit raw mode
                                    if use_alt_screen { print!("\x1b[?1049l"); }
                                    print!("\x1b[?7h\x1b[?25h");
                                    stdout.flush().ok();
                                    terminal::disable_raw_mode().unwrap();

                                    // Return selected results
                                    let mut result = Vec::new();
                                    for sid in &selected_ids {
                                        if let Some(exec) = current_execs.iter().find(|e| &e.record.record_id == sid) {
                                            result.push(exec.clone());
                                        }
                                    }
                                    result.sort_by(|a, b| a.record.timestamp.cmp(&b.record.timestamp));
                                    return result;
                                }
                            }
                        }
                    }
                    KeyCode::Esc => {
                        // Exit and return default selection
                        print!("\x1b[2J\x1b[H"); // 清屏并移动光标到顶部
                        stdout.flush().unwrap();
                        // Restore terminal settings
                        if use_alt_screen { print!("\x1b[?1049l"); }
                        print!("\x1b[?7h\x1b[?25h");
                        stdout.flush().ok();
                        terminal::disable_raw_mode().unwrap();
                        return executions.iter().take(2).cloned().collect();
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

    fn simple_select_executions(executions: &[CommandExecution], i18n: &I18n) -> Vec<CommandExecution> {
        if executions.len() <= 2 {
            return executions.to_vec();
        }

        println!("{}", i18n.t_format("select_executions", &[&executions.len().to_string()]));

        // 显示所有执行记录
        for (i, exec) in executions.iter().enumerate() {
            let local_time = exec.record.timestamp.with_timezone(&chrono::Local);
            let date_str = local_time.format("%Y-%m-%d %H:%M:%S");
            println!(
                "{}: {} (exit code: {}, time: {})",
                i + 1,
                exec.record.command,
                exec.record.exit_code,
                date_str
            );
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

        let indices: Vec<usize> = parts.iter()
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

    fn is_date_filter_input(input: &str, i18n: &I18n) -> bool {
        // Check if input contains date format (e.g., 2024-01, 01-15, 2024, etc.)
        let date_patterns = [
            r"\d{4}-\d{2}",      // YYYY-MM
            r"\d{2}-\d{2}",      // MM-DD
            r"\d{4}",           // YYYY
            r"\d{1,2}/\d{1,2}",  // MM/DD
            r"\d{4}/\d{1,2}",    // YYYY/MM
        ];

        for pattern in &date_patterns {
            if regex::Regex::new(pattern).unwrap().is_match(input) {
                return true;
            }
        }

        // Check if it's month name
        let month_names_en = [
            "jan", "feb", "mar", "apr", "may", "jun",
            "jul", "aug", "sep", "oct", "nov", "dec"
        ];

        let mut month_names_cn = Vec::new();
        for i in 1..=12 {
            month_names_cn.push(i18n.t(&format!("month_{}", Self::get_month_en_name(i))));
        }

        let lower_input = input.to_lowercase();
        month_names_en.iter().any(|&month| lower_input.contains(month)) ||
        month_names_cn.iter().any(|month| lower_input.contains(month))
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
            _ => "unknown"
        }
    }

    fn filter_by_date(executions: &[CommandExecution], filter: &str, i18n: &I18n) -> Vec<CommandExecution> {
        let mut filtered: Vec<CommandExecution> = executions.iter()
            .filter(|exec| Self::matches_date_filter(&exec.record.timestamp, filter, i18n))
            .cloned()
            .collect();

        if filtered.len() < 2 {
            println!("{}", i18n.t("few_records_fallback"));
            return executions.iter().take(2).cloned().collect();
        }

        // 如果匹配的记录多于2个，选择最近的两个
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

        // 年份匹配
        if filter.len() == 4 {
            if let Ok(year) = filter.parse::<i32>() {
                return timestamp.year() == year;
            }
        }

        // 年月匹配 (YYYY-MM)
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

        // 月日匹配 (MM-DD)
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

        // 月份名称匹配
        let month_names_en = [
            "jan", "feb", "mar", "apr", "may", "jun",
            "jul", "aug", "sep", "oct", "nov", "dec"
        ];

        let mut month_names_cn = Vec::new();
        for i in 1..=12 {
            month_names_cn.push(i18n.t(&format!("month_{}", Self::get_month_en_name(i))));
        }

        for (i, month_name) in month_names_en.iter().enumerate() {
            if lower_filter.contains(month_name) {
                return timestamp.month() == (i + 1) as u32;
            }
        }

        for (i, month_name) in month_names_cn.iter().enumerate() {
            if lower_filter.contains(month_name) {
                return timestamp.month() == (i + 1) as u32;
            }
        }

        // 模糊匹配日期字符串
        date_str.to_lowercase().contains(&lower_filter)
    }
}
