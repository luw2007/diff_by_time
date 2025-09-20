mod bash_parser;
mod config;
mod differ;
mod executor;
mod fuzzy_matcher;
mod i18n;
mod storage;
mod store_manager;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};
use std::path::PathBuf;

use config::Config;
use differ::Differ;
use executor::CommandExecutor;
use i18n::I18n;
use std::fs;
use storage::CommandExecution;
use store_manager::StoreManager;

#[derive(Parser)]
#[command(name = "dt")]
#[command(about = "")]
struct Cli {
    /// Override data directory (default: ~/.dt)
    #[arg(long = "data-dir", global = true)]
    data_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute command and record output
    Run {
        /// Command to execute (wrap piped expressions in quotes)
        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
        /// Diff with a specific short code after run
        #[arg(long = "diff-code", short = 'd')]
        diff_code: Option<String>,
    },
    /// Compare command output differences
    Diff {
        /// Command to compare (wrap commands with pipes in quotes)
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
        /// Maximum number of selection records to display [default: 20]
        #[arg(long)]
        max_shown: Option<usize>,
        /// Compare strictly line-by-line (no cross-line alignment)
        #[arg(long = "linewise")]
        linewise: bool,
    },
    /// Clean history records
    Clean {
        /// Clean mode
        #[command(subcommand)]
        mode: CleanMode,
    },
    /// List records concisely (non-interactive)
    Ls {
        /// Optional query to filter (substring or subsequence)
        query: Option<String>,
        /// Output JSON instead of text
        #[arg(long = "json")]
        json: bool,
    },
    /// Parse a Bash snippet/file to AST (tree-sitter-bash)
    Parse {
        /// File path to parse; omit to read from STDIN
        #[arg()]
        file: Option<PathBuf>,
        /// Output JSON instead of outline
        #[arg(long = "json")]
        json: bool,
    },
}

#[derive(Subcommand)]
enum CleanMode {
    /// Clean by fuzzy search (command)
    Search {
        /// Search query (optional; if omitted, opens an interactive selector)
        query: Option<String>,
        /// List matches without deleting
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    /// Clean by file
    File {
        /// File path (optional, if not provided will show related files list)
        file: Option<PathBuf>,
        /// List matches without deleting
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    /// Clean all records
    All,
}

fn main() -> Result<()> {
    // First try to parse arguments to check if it's a help request
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 2 {
        match args[1].as_str() {
            "--version" | "-v" | "-V" => {
                println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => {}
        }
    }

    // Check if help is requested
    let needs_help = args.contains(&"--help".to_string())
        || args.contains(&"-h".to_string())
        || args.len() == 1
        || (args.len() >= 2 && args[1] == "help")
        || (args.len() >= 3 && args[1] == "clean" && args[2] == "help")
        || (args.len() >= 3 && args[1] == "clean" && args[2] == "--help")
        || (args.len() >= 3 && args[1] == "clean" && args[2] == "-h");

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

    if args.len() == 2 && args[1] == "clean" {
        let config = Config::new()?;
        let i18n = I18n::new(&config.get_effective_language());
        print_help(&i18n);
        return Ok(());
    }

    // Normal parsing and command processing
    let cli = Cli::parse();
    let config = Config::new()?;
    let i18n = I18n::new(&config.get_effective_language());
    let store =
        StoreManager::new_with_config_and_base_dir(config.clone(), &i18n, cli.data_dir.clone())?;

    match cli.command {
        Commands::Run { command, diff_code } => {
            let command_str = join_args_for_shell(&command);
            let command_hash = hash_command(&command_str);

            let mut execution = CommandExecutor::execute(&command_str, &i18n)?;
            // Assign minimal unused short code for this command
            store.assign_short_code(&mut execution.record, &i18n)?;

            println!(
                "{}",
                i18n.t_format(
                    "command_completed",
                    &[&execution.record.exit_code.to_string()]
                )
                .green()
                .bold()
            );
            println!(
                "{}: {}ms",
                i18n.t("execution_time").yellow(),
                execution.record.duration_ms.to_string().green()
            );

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
                    if let Some(diff_output) = Differ::diff_executions(&pair, &i18n, false) {
                        print!("{}", diff_output);
                    }
                } else {
                    println!("{}", i18n.t_format("diff_code_not_found", &[&code]));
                }
            }
        }
        Commands::Diff {
            command,
            max_shown,
            linewise,
        } => {
            // Resolve TUI settings (env overrides config if present)
            let tui_simple = std::env::var("DT_TUI")
                .ok()
                .map(|v| {
                    let v = v.to_lowercase();
                    v == "0" || v == "false" || v == "simple"
                })
                .unwrap_or_else(|| config.display.tui_mode.to_lowercase() == "simple");
            let use_alt_screen = std::env::var("DT_ALT_SCREEN")
                .ok()
                .map(|v| {
                    let v = v.to_lowercase();
                    !(v == "0" || v == "false")
                })
                .unwrap_or(config.display.alt_screen);

            if !command.is_empty() {
                let command_str = join_args_for_shell(&command);
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
                        linewise,
                        || {
                            store_ref
                                .find_executions(&hash_clone, &i18n)
                                .unwrap_or_default()
                        },
                        Some(|exec: &CommandExecution| store_ref.delete_execution(exec, &i18n)),
                    );
                }
                if let Some(diff_output) = Differ::diff_executions(&executions, &i18n, linewise) {
                    print!("{}", diff_output);
                }
            } else {
                // No command provided: open command selector, then enter diff selection flow.
                Differ::command_then_diff_flow(
                    &store,
                    &i18n,
                    tui_simple,
                    use_alt_screen,
                    max_shown,
                    linewise,
                )?;
            }
        }
        Commands::Ls { query, json } => {
            list_records_query(&store, &query.unwrap_or_default(), &i18n, json)?;
        }
        Commands::Parse { file, json } => {
            use bash_parser::{ast_outline, BashParser};
            let input = if let Some(p) = file {
                fs::read_to_string(&p).map_err(|e| anyhow::anyhow!("读取文件失败: {}", e))?
            } else {
                let mut buf = String::new();
                io::stdin()
                    .read_to_string(&mut buf)
                    .map_err(|e| anyhow::anyhow!("读取 STDIN 失败: {}", e))?;
                buf
            };
            let mut parser = BashParser::new()?;
            let ast = parser.parse_to_ast(&input)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&ast)?);
            } else {
                let mut outline = String::new();
                ast_outline(&ast, 0, &mut outline);
                print!("{}", outline);
            }
        }
        Commands::Clean { mode } => {
            // Global flag for this invocation: if user typed ALL once, skip further confirms
            let mut skip_confirm_all = false;
            match mode {
                CleanMode::Search { query, dry_run } => {
                    // Resolve TUI settings
                    let tui_simple = std::env::var("DT_TUI")
                        .ok()
                        .map(|v| {
                            let v = v.to_lowercase();
                            v == "0" || v == "false" || v == "simple"
                        })
                        .unwrap_or_else(|| config.display.tui_mode.to_lowercase() == "simple");
                    let use_alt_screen = std::env::var("DT_ALT_SCREEN")
                        .ok()
                        .map(|v| {
                            let v = v.to_lowercase();
                            !(v == "0" || v == "false")
                        })
                        .unwrap_or(config.display.alt_screen);

                    let chosen_query = if let Some(q) = query {
                        Some(q)
                    } else {
                        Differ::select_prefix_for_clean(
                            &store,
                            &i18n,
                            tui_simple,
                            use_alt_screen,
                            None,
                        )?
                    };
                    if let Some(query_str) = chosen_query {
                        // Preview count and confirm
                        let all_records = store.get_all_records()?;
                        let count = {
                            let q = query_str.trim();
                            if q.is_empty() {
                                0
                            } else {
                                let ql = q.to_lowercase();
                                fn is_subsequence(needle: &str, haystack: &str) -> bool {
                                    let mut it = haystack.chars();
                                    for nc in needle.chars() {
                                        let mut found = false;
                                        for hc in it.by_ref() {
                                            if nc == hc {
                                                found = true;
                                                break;
                                            }
                                        }
                                        if !found {
                                            return false;
                                        }
                                    }
                                    true
                                }
                                all_records
                                    .iter()
                                    .filter(|r| {
                                        let cmd = r.command.to_lowercase();
                                        cmd.contains(&ql) || is_subsequence(&ql, &cmd)
                                    })
                                    .count()
                            }
                        };
                        if count == 0 {
                            println!("{}", i18n.t("delete_nothing").yellow());
                            return Ok(());
                        }
                        if dry_run {
                            println!("{}", i18n.t_format("dry_run_total", &[&count.to_string()]));
                            return Ok(());
                        }
                        println!(
                            "{}",
                            i18n.t_format(
                                "delete_summary_query",
                                &[&count.to_string(), &query_str]
                            )
                        );
                        if !confirm_delete(&i18n, &mut skip_confirm_all)? {
                            println!("{}", i18n.t("confirm_clean_all_aborted").yellow());
                            return Ok(());
                        }
                        // Clean by search query: substring or simple fuzzy (subsequence)
                        let cleaned = store.clean_by_query(&query_str, &i18n)?;
                        println!(
                            "{}",
                            i18n.t_format("cleaned_records", &[&cleaned.to_string()])
                        );
                    }
                }
                CleanMode::File { file, dry_run } => {
                    if let Some(file_path) = file {
                        // Preview and confirm
                        let all_records = store.get_all_records()?;
                        let target_path = match std::fs::canonicalize(&file_path) {
                            Ok(p) => p,
                            Err(_) => file_path.clone(),
                        };
                        let count = all_records
                            .iter()
                            .filter(|record| {
                                let mut should = false;
                                if record.working_dir == target_path {
                                    should = true;
                                }
                                let file_str = file_path.to_string_lossy();
                                let target_str = target_path.to_string_lossy();
                                if record.command.contains(file_str.as_ref())
                                    || record.command.contains(target_str.as_ref())
                                {
                                    should = true;
                                }
                                if let Some(rel_path) =
                                    pathdiff::diff_paths(&file_path, &record.working_dir)
                                {
                                    let rel_str = rel_path.to_string_lossy();
                                    if record.command.contains(rel_str.as_ref()) {
                                        should = true;
                                    }
                                }
                                should
                            })
                            .count();
                        if count == 0 {
                            println!("{}", i18n.t("delete_nothing").yellow());
                            return Ok(());
                        }
                        if dry_run {
                            println!("{}", i18n.t_format("dry_run_total", &[&count.to_string()]));
                            return Ok(());
                        }
                        println!(
                            "{}",
                            i18n.t_format(
                                "delete_summary_file",
                                &[&count.to_string(), &file_path.display().to_string()]
                            )
                        );
                        if !confirm_delete(&i18n, &mut skip_confirm_all)? {
                            println!("{}", i18n.t("confirm_clean_all_aborted").yellow());
                            return Ok(());
                        }
                        let cleaned = store.clean_by_file(&file_path, &i18n)?;
                        println!(
                            "{}",
                            i18n.t_format("cleaned_records", &[&cleaned.to_string()])
                        );
                    } else {
                        // Resolve TUI settings
                        let tui_simple = std::env::var("DT_TUI")
                            .ok()
                            .map(|v| {
                                let v = v.to_lowercase();
                                v == "0" || v == "false" || v == "simple"
                            })
                            .unwrap_or_else(|| config.display.tui_mode.to_lowercase() == "simple");
                        let use_alt_screen = std::env::var("DT_ALT_SCREEN")
                            .ok()
                            .map(|v| {
                                let v = v.to_lowercase();
                                !(v == "0" || v == "false")
                            })
                            .unwrap_or(config.display.alt_screen);

                        let files = store.get_related_files()?;
                        if files.is_empty() {
                            println!("{}", i18n.t("no_related_files"));
                        } else if let Some(chosen) = Differ::select_file_for_clean(
                            &files,
                            &i18n,
                            tui_simple,
                            use_alt_screen,
                            None,
                        )? {
                            // Preview and confirm
                            let all_records = store.get_all_records()?;
                            let target_path = match std::fs::canonicalize(&chosen) {
                                Ok(p) => p,
                                Err(_) => chosen.clone(),
                            };
                            let count = all_records
                                .iter()
                                .filter(|record| {
                                    let mut should = false;
                                    if record.working_dir == target_path {
                                        should = true;
                                    }
                                    let file_str = chosen.to_string_lossy();
                                    let target_str = target_path.to_string_lossy();
                                    if record.command.contains(file_str.as_ref())
                                        || record.command.contains(target_str.as_ref())
                                    {
                                        should = true;
                                    }
                                    if let Some(rel_path) =
                                        pathdiff::diff_paths(&chosen, &record.working_dir)
                                    {
                                        let rel_str = rel_path.to_string_lossy();
                                        if record.command.contains(rel_str.as_ref()) {
                                            should = true;
                                        }
                                    }
                                    should
                                })
                                .count();
                            if count == 0 {
                                println!("{}", i18n.t("delete_nothing").yellow());
                                return Ok(());
                            }
                            println!(
                                "{}",
                                i18n.t_format(
                                    "delete_summary_file",
                                    &[&count.to_string(), &chosen.display().to_string()]
                                )
                            );
                            if !confirm_delete(&i18n, &mut skip_confirm_all)? {
                                println!("{}", i18n.t("confirm_clean_all_aborted").yellow());
                                return Ok(());
                            }
                            let cleaned = store.clean_by_file(&chosen, &i18n)?;
                            println!(
                                "{}",
                                i18n.t_format("cleaned_records", &[&cleaned.to_string()])
                            );
                        }
                    }
                }
                CleanMode::All => {
                    // Require explicit confirmation before destructive action
                    println!("{}", i18n.t("confirm_clean_all_title").red().bold());
                    // Show summary: how many different commands and total records
                    let all_records = store.get_all_records()?;
                    let mut unique = std::collections::HashSet::new();
                    for r in &all_records {
                        unique.insert(r.command_hash.clone());
                    }
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
    if trimmed == "all" {
        *skip_confirm_all = true;
        return Ok(true);
    }
    Ok(trimmed == "yes")
}

fn join_args_for_shell(args: &[String]) -> String {
    if args.len() == 1 {
        return args[0].clone();
    }

    args.iter()
        .map(|arg| shell_quote(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(arg: &str) -> String {
    if arg.is_empty() {
        return "''".to_string();
    }

    let simple = arg.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, '_' | '-' | '.' | '/' | ':' | ',' | '@' | '+' | '%' | '=')
    });
    if simple {
        arg.to_string()
    } else {
        let escaped = arg.replace('\'', "'\\''");
        format!("'{}'", escaped)
    }
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

#[cfg(test)]
mod tests {
    use super::join_args_for_shell;

    #[test]
    fn test_join_args_simple() {
        let args = vec!["ls".into(), "-al".into()];
        assert_eq!(join_args_for_shell(&args), "ls -al");
    }

    #[test]
    fn test_join_args_with_spaces() {
        let args = vec!["echo".into(), "hello world".into()];
        assert_eq!(join_args_for_shell(&args), "echo 'hello world'");
    }

    #[test]
    fn test_join_args_with_single_quote() {
        let args = vec!["printf".into(), "%s".into(), "it's ok".into()];
        let expected = ["printf", "%s", r"'it'\''s ok'"]
            .iter()
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(join_args_for_shell(&args), expected);
    }

    #[test]
    fn test_join_args_single_arg_passthrough() {
        let args = vec!["ls -l".into()];
        assert_eq!(join_args_for_shell(&args), "ls -l");
    }

    #[test]
    fn test_join_args_empty_token() {
        let args = vec!["printf".into(), "".into()];
        assert_eq!(join_args_for_shell(&args), "printf ''");
    }
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
        println!("  {}     {}", "ls".green(), i18n.t("help_ls"));
        println!("  {}  {}", "clean".green(), i18n.t("help_clean"));
        println!("  {}   {}", "parse".green(), i18n.t("help_parse"));
        println!(
            "  {}   Print this message or the help of the given subcommand(s)",
            "help".green()
        );
        println!(
            "{}",
            i18n.t_format("help_tip_run_diff_code", &[&i18n.t("help_run_diff_code")])
        );
        println!("{}", i18n.t("help_pipeline_tip"));
        println!("{}", i18n.t("help_subcommand_more"));
        println!();
        println!("{}", i18n.t("help_label_options"));
        println!("  -h, --help           Print help");
        println!("  -v, -V, --version    Print version info");
        println!("      --data-dir <DIR> Override data directory (default: ~/.dt)");
        println!();
        println!("{}", i18n.t("help_config_section"));
        println!("  - {}", i18n.t("help_config_tui_mode"));
        println!("  - {}", i18n.t("help_config_alt_screen"));
    } else if args.len() >= 3 && args[1] == "clean" {
        // Clean subcommand's subcommand help
        match args[2].as_str() {
            "search" => {
                println!("{}", i18n.t("help_clean_search"));
                println!();
                println!("{} dt clean search [QUERY]", i18n.t("help_label_usage"));
                println!();
                println!("{}", i18n.t("help_label_arguments"));
                println!("  [QUERY]  {}", i18n.t("help_clean_search_arg"));
                println!();
                println!("{}", i18n.t("help_label_options"));
                println!("      --dry-run  {}", i18n.t("help_clean_dry_run"));
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
                println!("      --dry-run  {}", i18n.t("help_clean_dry_run"));
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
                println!("  {}    {}", "file".green(), i18n.t("help_clean_file"));
                println!("  {}     {}", "all".green(), i18n.t("help_clean_all"));
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
                println!();
                println!("{}", i18n.t("help_pipeline_tip"));
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
                println!(
                    "      --max-shown <MAX_SHOWN>  {}",
                    i18n.t("help_diff_max_shown")
                );
                println!(
                    "      --linewise               {}",
                    i18n.t("help_diff_linewise")
                );
                println!("  -h, --help                   Print help");
                println!();
                println!("{}", i18n.t("help_pipeline_tip"));
            }
            "ls" | "list" => {
                println!("{}", i18n.t("help_ls"));
                println!();
                println!("{} dt ls [QUERY] [--json]", i18n.t("help_label_usage"));
                println!();
                println!("{}", i18n.t("help_label_arguments"));
                println!("  [QUERY]  {}", i18n.t("help_ls_query"));
                println!();
                println!("{}", i18n.t("help_label_options"));
                println!("      --json  {}", i18n.t("help_ls_json"));
                println!("  -h, --help  Print help");
            }
            "clean" => {
                println!("{}", i18n.t("help_clean"));
                println!();
                println!("Usage: dt clean <COMMAND>");
                println!();
                println!("Commands:");
                println!("  {}  {}", "search".green(), i18n.t("help_clean_search"));
                println!("  {}    {}", "file".green(), i18n.t("help_clean_file"));
                println!("  {}     {}", "all".green(), i18n.t("help_clean_all"));
                println!();
                println!("Options:");
                println!("  -h, --help  Print help");
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
                println!("  {}   {}", "parse".green(), i18n.t("help_parse"));
                println!(
                    "  {}   Print this message or the help of the given subcommand(s)",
                    "help".green()
                );
                println!();
                println!("{}", i18n.t("help_label_options"));
                println!("  -h, --help  Print help");
            }
        }
    }
}
fn list_records_query(store: &StoreManager, query: &str, _i18n: &I18n, json: bool) -> Result<()> {
    let mut records = store.get_all_records()?;
    let q = query.trim().to_lowercase();
    if !q.is_empty() {
        fn is_subsequence(needle: &str, haystack: &str) -> bool {
            let mut it = haystack.chars();
            for nc in needle.chars() {
                let mut found = false;
                for hc in it.by_ref() {
                    if nc == hc {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return false;
                }
            }
            true
        }
        records.retain(|r| {
            let cmd = r.command.to_lowercase();
            cmd.contains(&q) || is_subsequence(&q, &cmd)
        });
    }
    if json {
        let out: Vec<serde_json::Value> = records
            .iter()
            .map(|r| {
                serde_json::json!({
                    "timestamp": r.timestamp.to_rfc3339(),
                    "command": r.command,
                    "command_hash": r.command_hash,
                    "exit_code": r.exit_code,
                    "duration_ms": r.duration_ms,
                    "record_id": r.record_id,
                    "short_code": r.short_code,
                    "working_dir": r.working_dir,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        for r in records {
            let ts = r
                .timestamp
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S");
            if let Some(code) = r.short_code.as_deref() {
                println!(
                    "{} exit={} dur={}ms [code:{}] {}",
                    ts, r.exit_code, r.duration_ms, code, r.command
                );
            } else {
                println!(
                    "{} exit={} dur={}ms {}",
                    ts, r.exit_code, r.duration_ms, r.command
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
mod test_support_main {
    use super::*;

    pub fn list_records_file(
        store: &StoreManager,
        file_path: &std::path::Path,
        _i18n: &I18n,
        json: bool,
    ) -> Result<()> {
        let target_path = match std::fs::canonicalize(file_path) {
            Ok(p) => p,
            Err(_) => file_path.to_path_buf(),
        };
        let mut records = store.get_all_records()?;
        records.retain(|record| {
            let mut should = false;
            if record.working_dir == target_path {
                should = true;
            }
            let file_str = file_path.to_string_lossy();
            let target_str = target_path.to_string_lossy();
            if record.command.contains(file_str.as_ref())
                || record.command.contains(target_str.as_ref())
            {
                should = true;
            }
            if let Some(rel_path) = pathdiff::diff_paths(file_path, &record.working_dir) {
                let rel_str = rel_path.to_string_lossy();
                if record.command.contains(rel_str.as_ref()) {
                    should = true;
                }
            }
            should
        });
        if json {
            let out: Vec<serde_json::Value> = records
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "timestamp": r.timestamp.to_rfc3339(),
                        "command": r.command,
                        "command_hash": r.command_hash,
                        "exit_code": r.exit_code,
                        "duration_ms": r.duration_ms,
                        "record_id": r.record_id,
                        "short_code": r.short_code,
                        "working_dir": r.working_dir,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            for r in records {
                let ts = r
                    .timestamp
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M:%S");
                if let Some(code) = r.short_code.as_deref() {
                    println!(
                        "{} exit={} dur={}ms [code:{}] {}",
                        ts, r.exit_code, r.duration_ms, code, r.command
                    );
                } else {
                    println!(
                        "{} exit={} dur={}ms {}",
                        ts, r.exit_code, r.duration_ms, r.command
                    );
                }
            }
        }
        Ok(())
    }
}
