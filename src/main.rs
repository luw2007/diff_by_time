mod storage;
mod executor;
mod store_manager;
mod differ;
mod config;
mod i18n;
mod fuzzy_matcher;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;
use colored::*;
use sha2::{Sha256, Digest};
use hex;

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
    },
    /// Compare command output differences
    Diff {
        /// Command to compare (wrap commands with pipes in quotes)
        #[arg(required = true)]
        command: String,
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
    /// List all history records
    List {
        /// Do not merge records with same commands [default: merge]
        #[arg(long)]
        no_merge: bool,
        /// Filter command string
        #[arg()]
        filter: Option<String>,
    },
}

#[derive(Subcommand)]
enum CleanMode {
    /// Clean by command prefix
    Prefix {
        /// Command prefix
        prefix: String,
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

    // Normal parsing and command processing
    let cli = Cli::parse();
    let config = Config::new()?;
    let i18n = I18n::new(&config.get_effective_language());
    let store = StoreManager::new_with_config(config.clone(), &i18n)?;

    match cli.command {
        Commands::Run { command } => {
            let command_str = command;
            let _command_hash = hash_command(&command_str);

            let execution = CommandExecutor::execute(&command_str, &i18n)?;

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
        }
        Commands::Diff { command, max_shown: _max_shown } => {
            let command_str = command;
            let command_hash = hash_command(&command_str);

            let mut executions = store.find_executions(&command_hash, &i18n)?;

            if executions.len() < 2 {
                println!("{}", i18n.t("need_at_least_two").red().bold());
                return Ok(());
            }

            if executions.len() > 2 {
                // Resolve TUI settings (env overrides config if present)
                let tui_simple = std::env::var("DT_TUI").ok()
                    .map(|v| { let v = v.to_lowercase(); v == "0" || v == "false" || v == "simple" })
                    .unwrap_or_else(|| config.display.tui_mode.to_lowercase() == "simple");
                let use_alt_screen = std::env::var("DT_ALT_SCREEN").ok()
                    .map(|v| { let v = v.to_lowercase(); !(v == "0" || v == "false") })
                    .unwrap_or(config.display.alt_screen);

                let hash_clone = command_hash.clone();
                let store_ref = &store;
                executions = Differ::interactive_select_executions_with_loader(
                    &executions,
                    &i18n,
                    tui_simple,
                    use_alt_screen,
                    || {
                        // Reload latest executions from index on each input
                        store_ref.find_executions(&hash_clone, &i18n).unwrap_or_default()
                    },
                );
            }

            if let Some(diff_output) = Differ::diff_executions(&executions, &i18n) {
                print!("{}", diff_output);
            }
        }
        Commands::Clean { mode } => {
            match mode {
                CleanMode::Prefix { prefix } => {
                    let cleaned = store.clean_by_prefix(&prefix, &i18n)?;
                    println!("{}", i18n.t_format("cleaned_records", &[&cleaned.to_string()]));
                }
                CleanMode::File { file } => {
                    if let Some(file_path) = file {
                        let cleaned = store.clean_by_file(&file_path, &i18n)?;
                        println!("{}", i18n.t_format("cleaned_records", &[&cleaned.to_string()]));
                    } else {
                        let files = store.get_related_files()?;
                        if files.is_empty() {
                            println!("{}", i18n.t("no_related_files"));
                        } else {
                            println!("{}", i18n.t("found_related_files"));
                            for (i, file_path) in files.iter().enumerate() {
                                println!("  {}: {}", i + 1, file_path.display());
                            }
                            println!("\n{}", i18n.t("clean_command"));
                            println!("  {}", i18n.t("clean_file_example"));
                        }
                    }
                }
                CleanMode::All => {
                    let _cleaned = store.clean_all(&i18n)?;
                    println!("{}", i18n.t("cleaned_all"));
                }
            }
        }
        Commands::List { no_merge, filter } => {
            let records = store.get_all_records()?;

            if records.is_empty() {
                println!("{}", i18n.t("no_records").yellow());
                return Ok(());
            }

            // Apply filtering
            let filtered_records = if let Some(filter_str) = &filter {
                records
                    .into_iter()
                    .filter(|record| record.command.to_lowercase().contains(&filter_str.to_lowercase()))
                    .collect()
            } else {
                records
            };

            if filtered_records.is_empty() {
                println!("{}", i18n.t("no_records").yellow());
                return Ok(());
            }

            if !no_merge {
                // Merge records with same commands
                let mut grouped: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();

                for record in filtered_records {
                    grouped.entry(record.command.clone()).or_insert_with(Vec::new).push(record);
                }

                println!("{}", i18n.t("history_records").cyan().bold());
                println!("{}", i18n.t_format("merged_commands", &[&grouped.len().to_string()]));

                for (_command, mut records) in grouped {
                    records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                    let latest = &records[0];
                    let count = records.len();

                    if count == 1 {
                        println!("{}", i18n.t_format("single_record", &[
                            &latest.command,
                            &latest.exit_code.to_string(),
                            &latest.duration_ms.to_string(),
                            &latest.timestamp.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string()
                        ]));
                    } else {
                        let earliest = &records[records.len() - 1];
                        println!("{}", i18n.t_format("multiple_records", &[
                            &latest.command,
                            &count.to_string(),
                            &latest.exit_code.to_string(),
                            &latest.duration_ms.to_string(),
                            &earliest.timestamp.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string(),
                            &latest.timestamp.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string()
                        ]));
                    }
                }
            } else {
                // Don't merge, show all records
                println!("{}", i18n.t("history_records").cyan().bold());
                println!("{}", i18n.t_format("showing_all", &[&filtered_records.len().to_string()]));

                for record in filtered_records {
                    println!("{}", i18n.t_format("all_records", &[
                        &record.command,
                        &record.exit_code.to_string(),
                        &record.duration_ms.to_string(),
                        &record.timestamp.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string()
                    ]));
                }
            }
        }
    }

    Ok(())
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
        println!("Usage: dt <COMMAND>");
        println!();
        println!("Commands:");
        println!("  {}    {}", "run".green(), i18n.t("help_run"));
        println!("  {}   {}", "diff".green(), i18n.t("help_diff"));
        println!("  {}  {}", "clean".green(), i18n.t("help_clean"));
        println!("  {}   {}", "list".green(), i18n.t("help_list"));
        println!("  {}   {}", "help".green(), "Print this message or the help of the given subcommand(s)");
        println!();
        println!("Options:");
        println!("  -h, --help  Print help");
        println!();
        println!("{}", i18n.t("help_config_section"));
        println!("  - {}", i18n.t("help_config_tui_mode"));
        println!("  - {}", i18n.t("help_config_alt_screen"));
    } else if args.len() >= 3 && args[1] == "clean" && (args[2] == "help" || args[2] == "--help" || args[2] == "-h") {
        // Clean subcommand help
        println!("{}", i18n.t("help_clean"));
        println!();
        println!("Usage: dt clean <COMMAND>");
        println!();
        println!("Commands:");
        println!("  {}  {}", "prefix".green(), i18n.t("help_clean_prefix"));
        println!("  {}    {}", "file".green(), i18n.t("help_clean_file"));
        println!("  {}     {}", "all".green(), i18n.t("help_clean_all"));
        println!("  {}    {}", "help".green(), "Print this message or the help of the given subcommand(s)");
        println!();
        println!("Options:");
        println!("  -h, --help  Print help");
    } else if args.len() >= 3 && args[1] == "clean" {
        // Clean subcommand's subcommand help
        match args[2].as_str() {
            "prefix" => {
                println!("{}", i18n.t("help_clean_prefix"));
                println!();
                println!("Usage: dt clean prefix <PREFIX>");
                println!();
                println!("Arguments:");
                println!("  <PREFIX>  {}", i18n.t("help_clean_prefix_arg"));
                println!();
                println!("Options:");
                println!("  -h, --help  Print help");
            }
            "file" => {
                println!("{}", i18n.t("help_clean_file"));
                println!();
                println!("Usage: dt clean file [FILE]");
                println!();
                println!("Arguments:");
                println!("  [FILE]  {}", i18n.t("help_clean_file_arg"));
                println!();
                println!("Options:");
                println!("  -h, --help  Print help");
            }
            "all" => {
                println!("{}", i18n.t("help_clean_all"));
                println!();
                println!("Usage: dt clean all");
                println!();
                println!("Options:");
                println!("  -h, --help  Print help");
            }
            _ => {
                // Unknown subcommand, show clean main help
                println!("{}", i18n.t("help_clean"));
                println!();
                println!("Usage: dt clean <COMMAND>");
                println!();
                println!("Commands:");
                println!("  {}  {}", "prefix".green(), i18n.t("help_clean_prefix"));
                println!("  {}    {}", "file".green(), i18n.t("help_clean_file"));
                println!("  {}     {}", "all".green(), i18n.t("help_clean_all"));
                println!("  {}    {}", "help".green(), "Print this message or the help of the given subcommand(s)");
                println!();
                println!("Options:");
                println!("  -h, --help  Print help");
            }
        }
    } else if args.len() >= 2 {
        match args[1].as_str() {
            "run" => {
                println!("{}", i18n.t("help_run"));
                println!();
                println!("Usage: dt run <COMMAND>");
                println!();
                println!("Arguments:");
                println!("  <COMMAND>  {}", i18n.t("help_run_command"));
                println!();
                println!("Options:");
                println!("  -h, --help  Print help");
            }
            "diff" => {
                println!("{}", i18n.t("help_diff"));
                println!();
                println!("Usage: dt diff [OPTIONS] <COMMAND>");
                println!();
                println!("Arguments:");
                println!("  <COMMAND>  {}", i18n.t("help_diff_command"));
                println!();
                println!("Options:");
                println!("      --max-shown <MAX_SHOWN>  {}", i18n.t("help_diff_max_shown"));
                println!("  -h, --help                   Print help");
            }
            "clean" => {
                println!("{}", i18n.t("help_clean"));
                println!();
                println!("Usage: dt clean <COMMAND>");
                println!();
                println!("Commands:");
                println!("  {}  {}", "prefix".green(), i18n.t("help_clean_prefix"));
                println!("  {}    {}", "file".green(), i18n.t("help_clean_file"));
                println!("  {}     {}", "all".green(), i18n.t("help_clean_all"));
                println!("  {}    {}", "help".green(), "Print this message or the help of the given subcommand(s)");
                println!();
                println!("Options:");
                println!("  -h, --help  Print help");
            }
            "list" => {
                println!("{}", i18n.t("help_list"));
                println!();
                println!("Usage: dt list [OPTIONS] [FILTER]");
                println!();
                println!("Arguments:");
                println!("  [FILTER]  {}", i18n.t("help_list_filter"));
                println!();
                println!("Options:");
                println!("      --no-merge  {}", i18n.t("help_list_no_merge"));
                println!("  -h, --help      Print help");
            }
            _ => {
                // Unknown subcommand, show main help
                println!("{}", i18n.t("help_about"));
                println!();
                println!("Usage: dt <COMMAND>");
                println!();
                println!("Commands:");
                println!("  {}    {}", "run".green(), i18n.t("help_run"));
                println!("  {}   {}", "diff".green(), i18n.t("help_diff"));
                println!("  {}  {}", "clean".green(), i18n.t("help_clean"));
                println!("  {}   {}", "list".green(), i18n.t("help_list"));
                println!("  {}   {}", "help".green(), "Print this message or the help of the given subcommand(s)");
                println!();
                println!("Options:");
                println!("  -h, --help  Print help");
            }
        }
    }
}
