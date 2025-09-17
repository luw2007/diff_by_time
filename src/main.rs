mod storage;
mod executor;
mod store_manager;
mod differ;
mod config;
mod i18n;
mod fuzzy_matcher;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::io::{self, Write};
use anyhow::Result;
use colored::*;
use sha2::{Sha256, Digest};

use executor::CommandExecutor;
use store_manager::StoreManager;
use differ::Differ;
use config::Config;
use i18n::I18n;

#[derive(Parser)]
#[command(name = "dt")]
#[command(about = "")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute command and record output
    Run {
        /// Command to execute (wrap commands with pipes in quotes)
        #[arg(required = true)]
        command: String,
        /// Diff with a specific short code after run
        #[arg(long = "diff-code", short = 'd')]
        diff_code: Option<String>,
    },
    /// Compare command output differences
    Diff {
        /// Command to compare (wrap commands with pipes in quotes)
        #[arg()]
        command: Option<String>,
        /// Maximum number of selection records to display [default: 20]
        #[arg(long)]
        max_shown: Option<usize>,
    },
    /// Clean history records
    Clean {
        /// Clean mode
        #[command(subcommand)]
        mode: CleanMode,
    },
}

#[derive(Subcommand)]
enum CleanMode {
    /// Clean by fuzzy search (command)
    #[command(alias = "prefix")] // backward-compat alias
    Search {
        /// Search query (optional; if omitted, opens an interactive selector)
        query: Option<String>,
    },
    /// Clean by file
    File {
        /// File path (optional, if not provided will show related files list)
        file: Option<PathBuf>,
    },
    /// Clean all records
    All,
}

fn main() -> Result<()> {
    // First try to parse arguments to check if it's a help request
    let args: Vec<String> = std::env::args().collect();

    // Check if help is requested
    let needs_help = args.contains(&"--help".to_string()) ||
                     args.contains(&"-h".to_string()) ||
                     args.len() == 1 ||
                     (args.len() >= 2 && args[1] == "help") ||
                     (args.len() >= 3 && args[1] == "clean" && args[2] == "help") ||
                     (args.len() >= 3 && args[1] == "clean" && args[2] == "--help") ||
                     (args.len() >= 3 && args[1] == "clean" && args[2] == "-h");

    if needs_help {
        // If it's a help request, load config first then display help
        let config = Config::new()?;
        let i18n = I18n::new(&config.get_effective_language());
        print_help(&i18n);
        return Ok(());
    }

    // If user typed `dt run` without a command, show the run help instead of an error.
    if args.len() == 2 && args[1] == "run" {
        let config = Config::new()?;
        let i18n = I18n::new(&config.get_effective_language());
        print_help(&i18n);
        return Ok(());
    }

    // Backward-compat: route `dt list` to interactive diff selector
    if args.len() >= 2 && args[1] == "list" {
        let config = Config::new()?;
        let i18n = I18n::new(&config.get_effective_language());
        let store = StoreManager::new_with_config(config.clone(), &i18n)?;

        println!("{}", i18n.t("list_removed_notice").yellow());

        // Resolve TUI settings (env overrides config if present)
        let tui_simple = std::env::var("DT_TUI").ok()
            .map(|v| { let v = v.to_lowercase(); v == "0" || v == "false" || v == "simple" })
            .unwrap_or_else(|| config.display.tui_mode.to_lowercase() == "simple");
        let use_alt_screen = std::env::var("DT_ALT_SCREEN").ok()
            .map(|v| { let v = v.to_lowercase(); !(v == "0" || v == "false") })
            .unwrap_or(config.display.alt_screen);

        Differ::command_then_diff_flow(&store, &i18n, tui_simple, use_alt_screen, None)?;
        return Ok(());
    }

    // Normal parsing and command processing
    let cli = Cli::parse();
    let config = Config::new()?;
    let i18n = I18n::new(&config.get_effective_language());
    let store = StoreManager::new_with_config(config.clone(), &i18n)?;

    match cli.command {
        Commands::Run { command, diff_code } => {
            let command_str = command;
            let command_hash = hash_command(&command_str);

            let mut execution = CommandExecutor::execute(&command_str, &i18n)?;
            // Assign minimal unused short code for this command
            store.assign_short_code(&mut execution.record, &i18n)?;

            println!("{}", i18n.t_format("command_completed", &[&execution.record.exit_code.to_string()]).green().bold());
            println!("{}: {}ms", i18n.t("execution_time").yellow(), execution.record.duration_ms.to_string().green());

            if !execution.stdout.is_empty() {
                println!("{}", i18n.t("stdout").cyan().bold());
                println!("{}", execution.stdout);
            }

            if !execution.stderr.is_empty() {
                println!("{}", i18n.t("stderr").red().bold());
                println!("{}", execution.stderr.red());
            }

            store.save_execution(&execution, &i18n)?;
            println!("{}", i18n.t("result_saved").green().bold());
            if let Some(code) = &execution.record.short_code {
                println!("{}", i18n.t_format("assigned_short_code", &[code]).yellow());
                println!("{}", i18n.t_format("hint_diff_with_code", &[code]).dimmed());
            }

            // If a diff target code is provided, show diff immediately
            if let Some(code) = diff_code {
                let executions = store.find_executions(&command_hash, &i18n)?;
                // Find target execution with the given short code, excluding the just-created record
                if let Some(target) = executions
                    .into_iter()
                    .filter(|e| e.record.record_id != execution.record.record_id)
                    .find(|e| e.record.short_code.as_deref() == Some(code.as_str()))
                {
                    let mut pair = vec![target, execution.clone()];
                    pair.sort_by(|a, b| a.record.timestamp.cmp(&b.record.timestamp));
                    if let Some(diff_output) = Differ::diff_executions(&pair, &i18n) {
                        print!("{}", diff_output);
                    }
                } else {
                    println!("{}", i18n.t_format("diff_code_not_found", &[&code]));
                }
            }
        }
        Commands::Diff { command, max_shown } => {
            // Resolve TUI settings (env overrides config if present)
            let tui_simple = std::env::var("DT_TUI").ok()
                .map(|v| { let v = v.to_lowercase(); v == "0" || v == "false" || v == "simple" })
                .unwrap_or_else(|| config.display.tui_mode.to_lowercase() == "simple");
            let use_alt_screen = std::env::var("DT_ALT_SCREEN").ok()
                .map(|v| { let v = v.to_lowercase(); !(v == "0" || v == "false") })
                .unwrap_or(config.display.alt_screen);

            if let Some(command_str) = command {
                let command_hash = hash_command(&command_str);
                let mut executions = store.find_executions(&command_hash, &i18n)?;
                if executions.len() < 2 {
                    println!("{}", i18n.t("need_at_least_two").red().bold());
                    return Ok(());
                }
                if executions.len() > 2 {
                    let hash_clone = command_hash.clone();
                    let store_ref = &store;
                    executions = Differ::interactive_select_executions_with_loader(
                        &executions,
                        &i18n,
                        tui_simple,
                        use_alt_screen,
                        max_shown,
                        || {
                            store_ref.find_executions(&hash_clone, &i18n).unwrap_or_default()
                        },
                    );
                }
                if let Some(diff_output) = Differ::diff_executions(&executions, &i18n) {
                    print!("{}", diff_output);
                }
            } else {
                // No command provided: open command selector, then enter diff selection flow.
                Differ::command_then_diff_flow(&store, &i18n, tui_simple, use_alt_screen, max_shown)?;
            }
        }
        Commands::Clean { mode } => {
            // Global flag for this invocation: if user typed ALL once, skip further confirms
            let mut skip_confirm_all = false;
            match mode {
                CleanMode::Search { query } => {
                    // Resolve TUI settings
                    let tui_simple = std::env::var("DT_TUI").ok()
                        .map(|v| { let v = v.to_lowercase(); v == "0" || v == "false" || v == "simple" })
                        .unwrap_or_else(|| config.display.tui_mode.to_lowercase() == "simple");
                    let use_alt_screen = std::env::var("DT_ALT_SCREEN").ok()
                        .map(|v| { let v = v.to_lowercase(); !(v == "0" || v == "false") })
                        .unwrap_or(config.display.alt_screen);

                    let chosen_query = if let Some(q) = query { Some(q) } else {
                        Differ::select_prefix_for_clean(&store, &i18n, tui_simple, use_alt_screen, None)?
                    };
                    if let Some(query_str) = chosen_query {
                        // Preview count and confirm
                        let all_records = store.get_all_records()?;
                        let count = {
                            let q = query_str.trim();
                            if q.is_empty() { 0 } else {
                                let ql = q.to_lowercase();
                                fn is_subsequence(needle: &str, haystack: &str) -> bool {
                                    let mut it = haystack.chars();
                                    for nc in needle.chars() {
                                        let mut found = false;
                                        for hc in it.by_ref() { if nc == hc { found = true; break; } }
                                        if !found { return false; }
                                    }
                                    true
                                }
                                all_records.iter().filter(|r| {
                                    let cmd = r.command.to_lowercase();
                                    cmd.contains(&ql) || is_subsequence(&ql, &cmd)
                                }).count()
                            }
                        };
                        if count == 0 {
                            println!("{}", i18n.t("delete_nothing").yellow());
                            return Ok(());
                        }
                        println!("{}", i18n.t_format("delete_summary_query", &[&count.to_string(), &query_str]));
                        if !confirm_delete(&i18n, &mut skip_confirm_all)? { println!("{}", i18n.t("confirm_clean_all_aborted").yellow()); return Ok(()); }
                        // Clean by search query: substring or simple fuzzy (subsequence)
                        let cleaned = store.clean_by_query(&query_str, &i18n)?;
                        println!("{}", i18n.t_format("cleaned_records", &[&cleaned.to_string()]));
                    }
                }
                CleanMode::File { file } => {
                    if let Some(file_path) = file {
                        // Preview and confirm
                        let all_records = store.get_all_records()?;
                        let target_path = match std::fs::canonicalize(&file_path) { Ok(p) => p, Err(_) => file_path.clone() };
                        let count = all_records.iter().filter(|record| {
                            let mut should = false;
                            if record.working_dir == target_path { should = true; }
                            let file_str = file_path.to_string_lossy();
                            let target_str = target_path.to_string_lossy();
                            if record.command.contains(file_str.as_ref()) || record.command.contains(target_str.as_ref()) { should = true; }
                            if let Some(rel_path) = pathdiff::diff_paths(&file_path, &record.working_dir) {
                                let rel_str = rel_path.to_string_lossy();
                                if record.command.contains(rel_str.as_ref()) { should = true; }
                            }
                            should
                        }).count();
                        if count == 0 {
                            println!("{}", i18n.t("delete_nothing").yellow());
                            return Ok(());
                        }
                        println!("{}", i18n.t_format("delete_summary_file", &[&count.to_string(), &file_path.display().to_string()]));
                        if !confirm_delete(&i18n, &mut skip_confirm_all)? { println!("{}", i18n.t("confirm_clean_all_aborted").yellow()); return Ok(()); }
                        let cleaned = store.clean_by_file(&file_path, &i18n)?;
                        println!("{}", i18n.t_format("cleaned_records", &[&cleaned.to_string()]));
                    } else {
                        // Resolve TUI settings
                        let tui_simple = std::env::var("DT_TUI").ok()
                            .map(|v| { let v = v.to_lowercase(); v == "0" || v == "false" || v == "simple" })
                            .unwrap_or_else(|| config.display.tui_mode.to_lowercase() == "simple");
                        let use_alt_screen = std::env::var("DT_ALT_SCREEN").ok()
                            .map(|v| { let v = v.to_lowercase(); !(v == "0" || v == "false") })
                            .unwrap_or(config.display.alt_screen);

                        let files = store.get_related_files()?;
                        if files.is_empty() {
                            println!("{}", i18n.t("no_related_files"));
                        } else if let Some(chosen) = Differ::select_file_for_clean(&files, &i18n, tui_simple, use_alt_screen, None)? {
                            // Preview and confirm
                            let all_records = store.get_all_records()?;
                            let target_path = match std::fs::canonicalize(&chosen) { Ok(p) => p, Err(_) => chosen.clone() };
                            let count = all_records.iter().filter(|record| {
                                let mut should = false;
                                if record.working_dir == target_path { should = true; }
                                let file_str = chosen.to_string_lossy();
                                let target_str = target_path.to_string_lossy();
                                if record.command.contains(file_str.as_ref()) || record.command.contains(target_str.as_ref()) { should = true; }
                                if let Some(rel_path) = pathdiff::diff_paths(&chosen, &record.working_dir) {
                                    let rel_str = rel_path.to_string_lossy();
                                    if record.command.contains(rel_str.as_ref()) { should = true; }
                                }
                                should
                            }).count();
                            if count == 0 { println!("{}", i18n.t("delete_nothing").yellow()); return Ok(()); }
                            println!("{}", i18n.t_format("delete_summary_file", &[&count.to_string(), &chosen.display().to_string()]));
                            if !confirm_delete(&i18n, &mut skip_confirm_all)? { println!("{}", i18n.t("confirm_clean_all_aborted").yellow()); return Ok(()); }
                            let cleaned = store.clean_by_file(&chosen, &i18n)?;
                            println!("{}", i18n.t_format("cleaned_records", &[&cleaned.to_string()]));
                        }
                    }
                }
                CleanMode::All => {
                    // Require explicit confirmation before destructive action
                    println!("{}", i18n.t("confirm_clean_all_title").red().bold());
                    // Show summary: how many different commands and total records
                    let all_records = store.get_all_records()?;
                    let mut unique = std::collections::HashSet::new();
                    for r in &all_records { unique.insert(r.command_hash.clone()); }
                    println!(
                        "{}",
                        i18n.t_format(
                            "clean_all_summary",
                            &[&unique.len().to_string(), &all_records.len().to_string()]
                        )
                    );
                    if !confirm_delete(&i18n, &mut skip_confirm_all)? {
                        println!("{}", i18n.t("confirm_clean_all_aborted").yellow());
                        return Ok(());
                    }

                    let _cleaned = store.clean_all(&i18n)?;
                    println!("{}", i18n.t("cleaned_all"));
                }
            }
        }
    }

    Ok(())
}

fn confirm_delete(i18n: &I18n, skip_confirm_all: &mut bool) -> Result<bool> {
    if *skip_confirm_all {
        return Ok(true);
    }
    print!("{}", i18n.t("confirm_delete_prompt").yellow());
    io::stdout().flush().ok();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return Ok(false);
    }
    let trimmed = input.trim().to_lowercase();
    if trimmed == "all" { *skip_confirm_all = true; return Ok(true); }
    Ok(trimmed == "yes")
}

fn format_command(command: &str) -> String {
    // Remove leading and trailing whitespace
    let trimmed = command.trim();

    // Normalize spaces: replace multiple consecutive spaces with single space, and handle spaces around pipe symbols
    let normalized = trimmed.chars().collect::<Vec<_>>();
    let mut result = String::new();
    let mut i = 0;

    while i < normalized.len() {
        let c = normalized[i];

        if c.is_whitespace() {
            // Skip consecutive whitespace characters
            while i < normalized.len() && normalized[i].is_whitespace() {
                i += 1;
            }
            // If next character is pipe symbol, don't add space
            if i < normalized.len() && normalized[i] == '|' {
                result.push('|');
                i += 1;
                // Skip all spaces after pipe symbol
                while i < normalized.len() && normalized[i].is_whitespace() {
                    i += 1;
                }
            } else {
                result.push(' ');
            }
        } else if c == '|' {
            // Handle pipe symbol, remove spaces before and after
            result.push('|');
            i += 1;
            // Skip following spaces
            while i < normalized.len() && normalized[i].is_whitespace() {
                i += 1;
            }
        } else {
            result.push(c);
            i += 1;
        }
    }

    result.trim().to_string()
}

fn hash_command(command: &str) -> String {
    let formatted_command = format_command(command);
    let mut hasher = Sha256::new();
    hasher.update(formatted_command.as_bytes());
    hex::encode(hasher.finalize())
}

fn print_help(i18n: &I18n) {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 || (args.len() >= 2 && (args[1] == "--help" || args[1] == "-h")) {
        // Main help
        println!("{}", i18n.t("help_about"));
        println!();
        println!("{} dt <COMMAND>", i18n.t("help_label_usage"));
        println!();
        println!("{}", i18n.t("help_label_commands"));
        println!("  {}    {}", "run".green(), i18n.t("help_run"));
        println!("  {}   {}", "diff".green(), i18n.t("help_diff"));
        println!("  {}  {}", "clean".green(), i18n.t("help_clean"));
        println!("  {}   Print this message or the help of the given subcommand(s)", "help".green());
        println!("{}", i18n.t_format("help_tip_run_diff_code", &[&i18n.t("help_run_diff_code")]));
        println!();
        println!("{}", i18n.t("help_label_options"));
        println!("  -h, --help  Print help");
        println!();
        println!("{}", i18n.t("help_config_section"));
        println!("  - {}", i18n.t("help_config_tui_mode"));
        println!("  - {}", i18n.t("help_config_alt_screen"));
    } else if args.len() >= 3 && args[1] == "clean" && (args[2] == "help" || args[2] == "--help" || args[2] == "-h") {
        // Clean subcommand help
        println!("{}", i18n.t("help_clean"));
        println!();
        println!("{} dt clean <COMMAND>", i18n.t("help_label_usage"));
        println!();
        println!("{}", i18n.t("help_label_commands"));
        println!("  {}  {}", "search".green(), i18n.t("help_clean_search"));
        println!("      {}", "(alias: prefix)".dimmed());
        println!("  {}    {}", "file".green(), i18n.t("help_clean_file"));
        println!("  {}     {}", "all".green(), i18n.t("help_clean_all"));
        println!("  {}    Print this message or the help of the given subcommand(s)", "help".green());
        println!();
        println!("{}", i18n.t("help_label_options"));
        println!("  -h, --help  Print help");
    } else if args.len() >= 3 && args[1] == "clean" {
        // Clean subcommand's subcommand help
        match args[2].as_str() {
            "search" | "prefix" => {
                println!("{}", i18n.t("help_clean_search"));
                println!();
                println!("{} dt clean search [QUERY]", i18n.t("help_label_usage"));
                println!();
                println!("{}", i18n.t("help_label_arguments"));
                println!("  [QUERY]  {}", i18n.t("help_clean_search_arg"));
                println!();
                println!("{}", i18n.t("help_label_options"));
                println!("  -h, --help  Print help");
            }
            "file" => {
                println!("{}", i18n.t("help_clean_file"));
                println!();
                println!("{} dt clean file [FILE]", i18n.t("help_label_usage"));
                println!();
                println!("{}", i18n.t("help_label_arguments"));
                println!("  [FILE]  {}", i18n.t("help_clean_file_arg"));
                println!();
                println!("{}", i18n.t("help_label_options"));
                println!("  -h, --help  Print help");
            }
            "all" => {
                println!("{}", i18n.t("help_clean_all"));
                println!();
                println!("{} dt clean all", i18n.t("help_label_usage"));
                println!();
                println!("{}", i18n.t("help_label_options"));
                println!("  -h, --help  Print help");
            }
            _ => {
                // Unknown subcommand, show clean main help
                println!("{}", i18n.t("help_clean"));
                println!();
                println!("{} dt clean <COMMAND>", i18n.t("help_label_usage"));
                println!();
                println!("{}", i18n.t("help_label_commands"));
                println!("  {}  {}", "search".green(), i18n.t("help_clean_search"));
                println!("      {}", "(alias: prefix)".dimmed());
                println!("  {}    {}", "file".green(), i18n.t("help_clean_file"));
                println!("  {}     {}", "all".green(), i18n.t("help_clean_all"));
                println!("  {}    Print this message or the help of the given subcommand(s)", "help".green());
                println!();
                println!("{}", i18n.t("help_label_options"));
                println!("  -h, --help  Print help");
            }
        }
    } else if args.len() >= 2 {
        match args[1].as_str() {
            "run" => {
                println!("{}", i18n.t("help_run"));
                println!();
                println!("{} dt run <COMMAND>", i18n.t("help_label_usage"));
                println!();
                println!("{}", i18n.t("help_label_arguments"));
                println!("  <COMMAND>  {}", i18n.t("help_run_command"));
                println!();
                println!("{}", i18n.t("help_label_options"));
                println!("  -d, --diff-code <CODE>  {}", i18n.t("help_run_diff_code"));
                println!("  -h, --help  Print help");
            }
            "diff" => {
                println!("{}", i18n.t("help_diff"));
                println!();
                println!("{} dt diff [OPTIONS] [COMMAND]", i18n.t("help_label_usage"));
                println!();
                println!("{}", i18n.t("help_label_arguments"));
                println!("  <COMMAND>  {}", i18n.t("help_diff_command"));
                println!();
                println!("{}", i18n.t("help_label_options"));
                println!("      --max-shown <MAX_SHOWN>  {}", i18n.t("help_diff_max_shown"));
                println!("  -h, --help                   Print help");
            }
            "clean" => {
                println!("{}", i18n.t("help_clean"));
                println!();
                println!("Usage: dt clean <COMMAND>");
                println!();
                println!("Commands:");
                println!("  {}  {}", "search".green(), i18n.t("help_clean_search"));
                println!("      {}", "(alias: prefix)".dimmed());
                println!("  {}    {}", "file".green(), i18n.t("help_clean_file"));
                println!("  {}     {}", "all".green(), i18n.t("help_clean_all"));
                println!("  {}    Print this message or the help of the given subcommand(s)", "help".green());
                println!();
                println!("Options:");
                println!("  -h, --help  Print help");
            }
            "list" => {
                println!("{}", i18n.t("list_removed_notice"));
                println!("\nUsage: dt diff [OPTIONS] [COMMAND]");
                println!();
                println!("Options:");
                println!("      --max-shown <MAX_SHOWN>  {}", i18n.t("help_diff_max_shown"));
                println!("  -h, --help                   Print help");
            }
            _ => {
                // Unknown subcommand, show main help
                println!("{}", i18n.t("help_about"));
                println!();
                println!("{} dt <COMMAND>", i18n.t("help_label_usage"));
                println!();
                println!("{}", i18n.t("help_label_commands"));
                println!("  {}    {}", "run".green(), i18n.t("help_run"));
                println!("  {}   {}", "diff".green(), i18n.t("help_diff"));
                println!("  {}  {}", "clean".green(), i18n.t("help_clean"));
                println!("  {}   Print this message or the help of the given subcommand(s)", "help".green());
                println!();
                println!("{}", i18n.t("help_label_options"));
                println!("  -h, --help  Print help");
            }
        }
    }
}
