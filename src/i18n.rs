use std::collections::HashMap;

pub struct I18n {
    translations: HashMap<String, HashMap<String, String>>,
    current_lang: String,
}

impl I18n {
    pub fn new(lang: &str) -> Self {
        let mut translations = HashMap::new();

        // English translations
        let mut en = HashMap::new();
        // Help texts
        en.insert(
            "help_about".to_string(),
            "Command execution time diff tool".to_string(),
        );
        en.insert(
            "help_run".to_string(),
            "Execute command and record output".to_string(),
        );
        en.insert(
            "help_run_command".to_string(),
            "Command and arguments to execute (quote piped expressions)".to_string(),
        );
        en.insert(
            "help_run_diff_code".to_string(),
            "Show diff against an existing short code after run".to_string(),
        );
        en.insert(
            "help_diff".to_string(),
            "Compare command output differences".to_string(),
        );
        en.insert(
            "help_diff_command".to_string(),
            "Command and arguments to compare (quote piped expressions)".to_string(),
        );
        en.insert(
            "help_diff_max_shown".to_string(),
            "Selector viewport height (rows)".to_string(),
        );
        en.insert(
            "help_diff_linewise".to_string(),
            "Compare strictly line-by-line (no cross-line alignment)".to_string(),
        );
        en.insert(
            "help_pipeline_tip".to_string(),
            "When your command contains pipes, redirects, or shell operators, wrap the whole expression in quotes. Example: dt run 'ls -l | wc'."
                .to_string(),
        );
        // Tips
        en.insert(
            "help_tip_run_diff_code".to_string(),
            "Tip: dt run supports -d, --diff-code <CODE> — {0}".to_string(),
        );
        en.insert(
            "help_subcommand_more".to_string(),
            "Run `dt <COMMAND> --help` for more details on each subcommand.".to_string(),
        );
        // Legacy alias notices
        // Config help
        en.insert(
            "help_config_section".to_string(),
            "Config (~/.dt/config.toml):".to_string(),
        );
        en.insert(
            "help_config_tui_mode".to_string(),
            "display.tui_mode: interactive | simple (interactive by default)".to_string(),
        );
        en.insert("help_config_alt_screen".to_string(), "display.alt_screen: true | false (use alternate screen in interactive mode; default: false)".to_string());
        en.insert(
            "help_clean".to_string(),
            "Clean history records".to_string(),
        );
        // removed: help_clean_mode (not used)
        en.insert("short_code_label".to_string(), "code".to_string());
        en.insert("time_label".to_string(), "time".to_string());
        en.insert(
            "help_clean_search".to_string(),
            "Clean by command search".to_string(),
        );
        en.insert(
            "help_clean_search_arg".to_string(),
            "Search query".to_string(),
        );
        en.insert("help_clean_file".to_string(), "Clean by file".to_string());
        en.insert(
            "help_clean_file_arg".to_string(),
            "File path (optional, if not provided will show related files list)".to_string(),
        );
        en.insert(
            "help_clean_all".to_string(),
            "Clean all records".to_string(),
        );
        en.insert(
            "help_clean_backup".to_string(),
            "Backup matching records to current year's archive".to_string(),
        );
        en.insert(
            "help_clean_dry_run".to_string(),
            "List matches without deleting".to_string(),
        );
        en.insert(
            "help_ls".to_string(),
            "List records (non-interactive)".to_string(),
        );
        en.insert(
            "help_ls_query".to_string(),
            "Optional query to filter (substring/subsequence)".to_string(),
        );
        en.insert("help_ls_json".to_string(), "Output JSON".to_string());
        en.insert(
            "help_parse".to_string(),
            "Parse a Bash snippet/file to AST (tree-sitter-bash)".to_string(),
        );
        en.insert(
            "help_parse_file".to_string(),
            "File path to parse; omit to read from STDIN".to_string(),
        );
        en.insert(
            "help_parse_json".to_string(),
            "Output JSON instead of outline".to_string(),
        );
        // Dangerous command confirmations
        en.insert(
            "confirm_clean_all_title".to_string(),
            "Dangerous operation: this will delete ALL history".to_string(),
        );
        en.insert(
            "confirm_clean_all_prompt".to_string(),
            "Type YES to confirm: ".to_string(),
        );
        en.insert(
            "clean_all_summary".to_string(),
            "Summary: {0} different commands, {1} total records".to_string(),
        );
        en.insert(
            "confirm_clean_all_aborted".to_string(),
            "Aborted. No records were deleted.".to_string(),
        );
        // Generic deletion confirmations
        en.insert(
            "confirm_delete_prompt".to_string(),
            "Type YES to confirm (or ALL to confirm all deletions this session): ".to_string(),
        );
        en.insert(
            "delete_summary_query".to_string(),
            "About to delete {0} records matching: {1}".to_string(),
        );
        en.insert(
            "delete_summary_file".to_string(),
            "About to delete {0} records related to file: {1}".to_string(),
        );
        en.insert(
            "delete_nothing".to_string(),
            "No records matched; nothing to delete.".to_string(),
        );
        en.insert(
            "dry_run_total".to_string(),
            "Dry-run total: {0} records".to_string(),
        );
        en.insert(
            "backup_completed".to_string(),
            "Backed up records to index_{1}.json (total now: {0})".to_string(),
        );

        // Runtime messages
        // removed unused runtime prompts: continue_prompt, continue_hint, execution_cancelled
        en.insert(
            "command_completed".to_string(),
            "Command completed, exit code: {0}".to_string(),
        );
        en.insert("execution_time".to_string(), "Execution time".to_string());
        en.insert("stdout".to_string(), "Standard output:".to_string());
        en.insert("stderr".to_string(), "Error output:".to_string());
        en.insert("result_saved".to_string(), "Result saved".to_string());
        en.insert(
            "need_at_least_two".to_string(),
            "Need at least two execution records for comparison".to_string(),
        );
        en.insert(
            "diff_code_not_found".to_string(),
            "No record found with short code: {0}".to_string(),
        );
        en.insert(
            "select_executions".to_string(),
            "Found {0} execution records, please select two to compare:".to_string(),
        );
        en.insert(
            "input_numbers".to_string(),
            "Input two numbers (space separated, e.g., 1 2):".to_string(),
        );
        en.insert(
            "invalid_input".to_string(),
            "Invalid input, will use the latest two records".to_string(),
        );
        en.insert(
            "assigned_short_code".to_string(),
            "Short code: {0}".to_string(),
        );
        en.insert(
            "hint_diff_with_code".to_string(),
            "Tip: run again with --diff-code={0} to compare".to_string(),
        );
        en.insert(
            "select_command".to_string(),
            "Select a command to compare (Enter=open, Esc=quit):".to_string(),
        );
        en.insert("count_label".to_string(), "count".to_string());
        en.insert("latest_label".to_string(), "latest".to_string());

        // Interactive selection messages
        en.insert("interactive_filter".to_string(), "Filter (type to fuzzy search, j/k to navigate, Enter to select, Delete to clear, Esc to quit)".to_string());
        en.insert(
            "status_select_first".to_string(),
            "Select first".to_string(),
        );
        en.insert(
            "status_select_second".to_string(),
            "Select second".to_string(),
        );
        en.insert("status_filter".to_string(), "Filter".to_string());
        en.insert(
            "status_nav_narrow".to_string(),
            "jk/PgUp page · Space sel · Tab→Prev · ?=help".to_string(),
        );
        en.insert(
            "status_nav_medium".to_string(),
            "jk/PgUp/^f/^b page · Space/Enter sel · Tab→Prev · Enter diff · ?=help".to_string(),
        );
        en.insert(
            "status_nav_compact".to_string(),
            "Sel: type/jk, PgUp/PgDn or ^f/^b page, Space/Enter toggle, Tab→Preview | Prev: jk return, Shift/Ctrl+↑↓ scroll, Enter diff, q back, Q exit".to_string(),
        );
        en.insert(
            "delete_confirm_status".to_string(),
            "Press Shift+Backspace or Ctrl+X again to delete run from {0}.".to_string(),
        );
        en.insert(
            "delete_success_status".to_string(),
            "Deleted run from {0}.".to_string(),
        );
        en.insert(
            "delete_failed_status".to_string(),
            "Failed to delete run: {0}".to_string(),
        );
        en.insert(
            "first_selection".to_string(),
            "Select first record (press Enter to confirm):".to_string(),
        );
        en.insert(
            "second_selection".to_string(),
            "Select second record (press Enter to confirm):".to_string(),
        );
        en.insert(
            "selection_complete".to_string(),
            "Selected two records. Press Enter again to compare.".to_string(),
        );
        en.insert(
            "selection_limit_reached".to_string(),
            "Only two records can be compared at once. Deselect one to pick another.".to_string(),
        );
        en.insert("no_matches".to_string(), "No matches found".to_string());
        en.insert("navigate_hint".to_string(), "Navigation — Selection: type/jk, PgUp/PgDn or Ctrl+f/b page, Space/Enter toggle, Tab → preview; Preview: jk return, Shift/Ctrl+↑/↓ scroll, Enter toggle/diff, Space/b/F or PgUp/PgDn page, q back, Q exit; Esc back/exit".to_string());
        en.insert(
            "select_clean_command".to_string(),
            "Select a command to clean:".to_string(),
        );
        en.insert(
            "select_clean_file".to_string(),
            "Select a file to clean:".to_string(),
        );
        en.insert(
            "no_related_files".to_string(),
            "No related file records found".to_string(),
        );
        en.insert(
            "found_related_files".to_string(),
            "Found following related files:".to_string(),
        );
        en.insert(
            "clean_command".to_string(),
            "Use following command to clean specific file:".to_string(),
        );
        en.insert(
            "cleaned_records".to_string(),
            "Cleaned {0} records".to_string(),
        );
        en.insert("cleaned_all".to_string(), "Clean completed".to_string());
        en.insert("no_records".to_string(), "No records found".to_string());
        // removed unused/duplicate keys: total_records, filter_prompt, selected

        // Diff output
        en.insert("diff_command".to_string(), "Command: {0}".to_string());
        en.insert("diff_earlier".to_string(), "Earlier ({0})".to_string());
        en.insert("diff_later".to_string(), "Later ({0})".to_string());
        en.insert("diff_earlier_label".to_string(), "Earlier".to_string());
        en.insert("diff_later_label".to_string(), "Later".to_string());
        en.insert(
            "diff_exit_code".to_string(),
            "exit code: {0} -> {1}".to_string(),
        );
        en.insert(
            "diff_execution_time".to_string(),
            "execution time: {0}ms -> {1}ms".to_string(),
        );
        en.insert("stdout_diff".to_string(), "stdout diff:".to_string());
        en.insert(
            "preview_stdout_header".to_string(),
            "Preview: stdout".to_string(),
        );
        en.insert(
            "preview_stderr_header".to_string(),
            "Preview: stderr".to_string(),
        );
        en.insert(
            "preview_diff_stdout_header".to_string(),
            "Diff preview: stdout".to_string(),
        );
        en.insert(
            "preview_diff_stderr_header".to_string(),
            "Diff preview: stderr".to_string(),
        );
        en.insert(
            "preview_toggle_hint".to_string(),
            "Press o or ←/→ to toggle stdout/stderr".to_string(),
        );
        en.insert(
            "preview_toggle_short".to_string(),
            "o/←/→ to switch".to_string(),
        );
        en.insert("preview_path_label".to_string(), "Path: {0}".to_string());
        en.insert(
            "preview_path_missing".to_string(),
            "Path: unavailable".to_string(),
        );
        en.insert("preview_empty".to_string(), "Output is empty".to_string());
        en.insert(
            "preview_truncated_hint".to_string(),
            "… truncated".to_string(),
        );
        en.insert(
            "preview_no_selection".to_string(),
            "Select a record to preview output".to_string(),
        );
        en.insert(
            "preview_single_column_notice".to_string(),
            "Terminal too narrow, using single-column view".to_string(),
        );
        en.insert(
            "terminal_too_small".to_string(),
            "Terminal too small: need at least {0}x{1}, current {2}x{3}".to_string(),
        );
        // Preview help overlay
        en.insert("preview_help_title".to_string(), "Preview Help".to_string());
        en.insert(
            "preview_help_move".to_string(),
            "Change selection: j/k or Up/Down (returns to list and resets scroll)".to_string(),
        );
        en.insert(
            "preview_help_page".to_string(),
            "Scroll: Shift/Ctrl+Up/Down (line); Space or f (down), b or Backspace (up)".to_string(),
        );
        en.insert(
            "preview_help_half".to_string(),
            "Half page: d (down), u (up) or Alt+Up/Down".to_string(),
        );
        en.insert(
            "preview_help_top_bottom".to_string(),
            "Top/Bottom: g/G or Home/End (Fn+Left/Right)".to_string(),
        );
        en.insert(
            "preview_help_back".to_string(),
            "Back to selection: q (Esc also works)".to_string(),
        );
        en.insert(
            "preview_help_start_diff".to_string(),
            "Toggle selection / start diff (2 selected): Enter".to_string(),
        );
        en.insert(
            "preview_help_toggle".to_string(),
            "Toggle help: h / ?".to_string(),
        );
        en.insert("preview_help_quit".to_string(), "Quit app: Q".to_string());

        // Selection help
        en.insert("selection_help_title".to_string(), "Selection Help".to_string());
        en.insert(
            "selection_help_filter".to_string(),
            "Filter: Type to filter items (fuzzy matching)".to_string(),
        );
        en.insert(
            "selection_help_move".to_string(),
            "Move: j/k or Up/Down, Ctrl+p/Ctrl+n (emacs style)".to_string(),
        );
        en.insert(
            "selection_help_page".to_string(),
            "Page: PgUp/PgDn or Ctrl+b/Ctrl+f (dynamic based on terminal height)".to_string(),
        );
        en.insert(
            "selection_help_jump".to_string(),
            "Jump: Home/End or Ctrl+a/Ctrl+e (top/bottom)".to_string(),
        );
        en.insert(
            "selection_help_select".to_string(),
            "Select: Space or Enter to toggle (select 2 items to compare)".to_string(),
        );
        en.insert(
            "selection_help_preview".to_string(),
            "Preview: Tab to enter preview pane".to_string(),
        );
        en.insert(
            "selection_help_clear".to_string(),
            "Clear filter: Ctrl+u (clear all), Ctrl+w (delete word)".to_string(),
        );
        en.insert("stderr_diff".to_string(), "stderr diff:".to_string());
        en.insert(
            "output_identical".to_string(),
            "output is identical".to_string(),
        );
        en.insert(
            "warning_interactive_failed".to_string(),
            "Warning: Cannot enable interactive mode, falling back to simple selection mode"
                .to_string(),
        );
        // Month names for date filtering via i18n (both languages)
        en.insert("month_jan".to_string(), "Jan".to_string());
        en.insert("month_feb".to_string(), "Feb".to_string());
        en.insert("month_mar".to_string(), "Mar".to_string());
        en.insert("month_apr".to_string(), "Apr".to_string());
        en.insert("month_may".to_string(), "May".to_string());
        en.insert("month_jun".to_string(), "Jun".to_string());
        en.insert("month_jul".to_string(), "Jul".to_string());
        en.insert("month_aug".to_string(), "Aug".to_string());
        en.insert("month_sep".to_string(), "Sep".to_string());
        en.insert("month_oct".to_string(), "Oct".to_string());
        en.insert("month_nov".to_string(), "Nov".to_string());
        en.insert("month_dec".to_string(), "Dec".to_string());
        en.insert(
            "few_records_fallback".to_string(),
            "Less than 2 matched records, using the latest two records".to_string(),
        );
        en.insert(
            "using_filtered_records".to_string(),
            "Using the two filtered records for comparison:".to_string(),
        );

        // Error messages
        en.insert(
            "error_create_dt_dir".to_string(),
            "Failed to create .dt directory".to_string(),
        );
        en.insert(
            "error_create_records_dir".to_string(),
            "Failed to create records directory".to_string(),
        );
        en.insert(
            "error_create_record_dir".to_string(),
            "Failed to create record directory".to_string(),
        );
        en.insert(
            "error_save_metadata".to_string(),
            "Failed to save metadata".to_string(),
        );
        en.insert(
            "error_save_stdout".to_string(),
            "Failed to save stdout".to_string(),
        );
        en.insert(
            "error_save_stderr".to_string(),
            "Failed to save stderr".to_string(),
        );
        en.insert(
            "error_read_stdout".to_string(),
            "Cannot read stdout".to_string(),
        );
        en.insert(
            "error_read_stderr".to_string(),
            "Cannot read stderr".to_string(),
        );
        en.insert(
            "error_update_index".to_string(),
            "Failed to update index".to_string(),
        );
        en.insert(
            "error_save_archive".to_string(),
            "Failed to save {0} year archive".to_string(),
        );
        en.insert(
            "error_rebuild_index".to_string(),
            "Failed to rebuild index".to_string(),
        );
        en.insert(
            "error_execute_command".to_string(),
            "Failed to execute command".to_string(),
        );

        // Clean operation
        en.insert(
            "clean_record".to_string(),
            "Cleaning record: {0} (time: {1})".to_string(),
        );
        en.insert(
            "clean_file_example".to_string(),
            "dt clean file <file_path>".to_string(),
        );
        // Help section labels (en)
        en.insert("help_label_usage".to_string(), "Usage:".to_string());
        en.insert("help_label_commands".to_string(), "Commands:".to_string());
        en.insert("help_label_options".to_string(), "Options:".to_string());
        en.insert("help_label_arguments".to_string(), "Arguments:".to_string());

        // Chinese translations
        let mut zh = HashMap::new();
        // Help texts (zh)
        zh.insert(
            "help_about".to_string(),
            "命令执行时间差比较工具".to_string(),
        );
        zh.insert("help_run".to_string(), "执行命令并记录输出".to_string());
        zh.insert(
            "help_run_command".to_string(),
            "要执行的命令及其参数（包含管道时需整体加引号）".to_string(),
        );
        zh.insert("help_diff".to_string(), "比较命令输出差异".to_string());
        zh.insert(
            "help_diff_command".to_string(),
            "要比较的命令及其参数（包含管道时需整体加引号）".to_string(),
        );
        zh.insert(
            "help_diff_max_shown".to_string(),
            "选择器视口高度（行数）".to_string(),
        );
        zh.insert(
            "help_diff_linewise".to_string(),
            "逐行比较（不进行跨行对齐）".to_string(),
        );
        zh.insert(
            "help_pipeline_tip".to_string(),
            "命令包含管道、重定向或逻辑运算符时，必须用引号包裹整条命令，例如：dt run 'ls -l | wc'。"
                .to_string(),
        );
        // Tips (zh)
        zh.insert(
            "help_tip_run_diff_code".to_string(),
            "提示：dt run 支持 -d, --diff-code <CODE> — {0}".to_string(),
        );
        zh.insert(
            "help_subcommand_more".to_string(),
            "运行 `dt <命令> --help` 查看各子命令的详细说明。".to_string(),
        );
        // Config help
        zh.insert(
            "help_config_section".to_string(),
            "配置文件 (~/.dt/config.toml):".to_string(),
        );
        zh.insert(
            "help_config_tui_mode".to_string(),
            "display.tui_mode: interactive | simple（默认 interactive，simple 为打印文本模式）"
                .to_string(),
        );
        zh.insert(
            "help_config_alt_screen".to_string(),
            "display.alt_screen: true | false（交互模式是否使用备用屏，默认 false）".to_string(),
        );
        zh.insert("help_clean".to_string(), "清理历史记录".to_string());
        zh.insert(
            "help_run_diff_code".to_string(),
            "执行后与指定短码进行对比".to_string(),
        );
        // removed: help_clean_mode (not used)
        zh.insert("short_code_label".to_string(), "短码".to_string());
        zh.insert("time_label".to_string(), "时间".to_string());
        zh.insert(
            "help_clean_search".to_string(),
            "按命令搜索清理".to_string(),
        );
        zh.insert(
            "help_clean_search_arg".to_string(),
            "搜索关键词".to_string(),
        );
        zh.insert("help_clean_file".to_string(), "按文件清理".to_string());
        zh.insert(
            "help_clean_file_arg".to_string(),
            "文件路径 (可选，不提供则显示相关文件列表)".to_string(),
        );
        zh.insert("help_clean_all".to_string(), "清理所有记录".to_string());
        zh.insert(
            "help_clean_backup".to_string(),
            "备份匹配记录到当年的归档文件".to_string(),
        );
        zh.insert(
            "help_clean_dry_run".to_string(),
            "仅列出匹配结果，不执行删除".to_string(),
        );
        zh.insert("help_ls".to_string(), "简洁的非交互式列表".to_string());
        zh.insert(
            "help_ls_query".to_string(),
            "可选的查询（子串/子序列）".to_string(),
        );
        zh.insert("help_ls_json".to_string(), "输出 JSON".to_string());
        // Dangerous command confirmations (zh)
        zh.insert(
            "confirm_clean_all_title".to_string(),
            "危险操作：将删除所有历史记录".to_string(),
        );
        zh.insert(
            "confirm_clean_all_prompt".to_string(),
            "请输入 YES 以确认：".to_string(),
        );
        zh.insert(
            "clean_all_summary".to_string(),
            "汇总: {0} 个不同命令，{1} 条记录".to_string(),
        );
        zh.insert(
            "confirm_clean_all_aborted".to_string(),
            "已取消，未删除任何记录".to_string(),
        );
        // Generic deletion confirmations (zh)
        zh.insert(
            "confirm_delete_prompt".to_string(),
            "请输入 YES 确认（或输入 ALL 表示本次会话内不再提示）：".to_string(),
        );
        zh.insert(
            "delete_summary_query".to_string(),
            "将删除匹配“{1}”的 {0} 条记录".to_string(),
        );
        zh.insert(
            "delete_summary_file".to_string(),
            "将删除与文件相关的 {0} 条记录：{1}".to_string(),
        );
        zh.insert(
            "delete_nothing".to_string(),
            "没有匹配记录，无需删除。".to_string(),
        );
        zh.insert(
            "dry_run_total".to_string(),
            "试运行总计: {0} 条记录".to_string(),
        );
        zh.insert(
            "backup_completed".to_string(),
            "已备份记录到 index_{1}.json（当前总数：{0}）".to_string(),
        );
        // Help section labels (zh)
        zh.insert("help_label_usage".to_string(), "用法:".to_string());
        zh.insert("help_label_commands".to_string(), "命令:".to_string());
        zh.insert("help_label_options".to_string(), "选项:".to_string());
        zh.insert("help_label_arguments".to_string(), "参数:".to_string());

        // Runtime messages
        // removed unused runtime prompts: continue_prompt, continue_hint, execution_cancelled
        zh.insert(
            "command_completed".to_string(),
            "命令执行完成，退出码: {0}".to_string(),
        );
        zh.insert("execution_time".to_string(), "执行时间".to_string());
        zh.insert("stdout".to_string(), "标准输出:".to_string());
        zh.insert("stderr".to_string(), "错误输出:".to_string());
        zh.insert("result_saved".to_string(), "结果已保存".to_string());
        zh.insert(
            "need_at_least_two".to_string(),
            "需要至少两个执行记录才能进行比较".to_string(),
        );
        zh.insert(
            "diff_code_not_found".to_string(),
            "未找到短码为 {0} 的记录".to_string(),
        );
        zh.insert(
            "select_executions".to_string(),
            "找到 {0} 个执行记录，请选择要比较的两个:".to_string(),
        );
        zh.insert(
            "input_numbers".to_string(),
            "输入两个数字 (用空格分隔，例如: 1 2):".to_string(),
        );
        zh.insert(
            "invalid_input".to_string(),
            "无效输入，将使用最新的两个记录".to_string(),
        );
        zh.insert("assigned_short_code".to_string(), "短码: {0}".to_string());
        zh.insert(
            "hint_diff_with_code".to_string(),
            "提示: 再次使用 --diff-code={0} 可直接比较".to_string(),
        );
        zh.insert(
            "select_command".to_string(),
            "选择一个命令进行比较（Enter进入，Esc退出）：".to_string(),
        );
        zh.insert("count_label".to_string(), "数量".to_string());
        zh.insert("latest_label".to_string(), "最新".to_string());

        // Interactive selection messages
        zh.insert(
            "interactive_filter".to_string(),
            "过滤器 (输入模糊搜索，j/k 导航，Enter 选择，Delete 清空，Esc 退出)".to_string(),
        );
        zh.insert("status_select_first".to_string(), "选择首条".to_string());
        zh.insert("status_select_second".to_string(), "选择次条".to_string());
        zh.insert("status_filter".to_string(), "筛选".to_string());
        zh.insert(
            "status_nav_narrow".to_string(),
            "jk/PgUp翻页 · 空格选择 · Tab→预览 · ?=帮助".to_string(),
        );
        zh.insert(
            "status_nav_medium".to_string(),
            "jk/PgUp/^f/^b翻页 · 空格/Enter选择 · Tab→预览 · Enter比较 · ?=帮助".to_string(),
        );
        zh.insert(
            "status_nav_compact".to_string(),
            "选择: 输入/jk, PgUp/PgDn或^f/^b翻页, 空格/Enter切换, Tab入预览 | 预览: jk返回, Shift/Ctrl+↑↓逐行, Enter比较, q返回, Q退出".to_string(),
        );
        zh.insert(
            "delete_confirm_status".to_string(),
            "再次按 Shift+Backspace 或 Ctrl+X 删除 {0} 的记录。".to_string(),
        );
        zh.insert(
            "delete_success_status".to_string(),
            "已删除 {0} 的记录。".to_string(),
        );
        zh.insert(
            "delete_failed_status".to_string(),
            "删除失败：{0}".to_string(),
        );
        zh.insert(
            "first_selection".to_string(),
            "选择第一条记录 (按Enter确认):".to_string(),
        );
        zh.insert(
            "second_selection".to_string(),
            "选择第二条记录 (按Enter确认):".to_string(),
        );
        zh.insert(
            "selection_complete".to_string(),
            "已选择两条记录，再次按 Enter 立即对比。".to_string(),
        );
        zh.insert(
            "selection_limit_reached".to_string(),
            "一次最多比较两条记录，请先取消其中一条再选择其它记录。".to_string(),
        );
        zh.insert("no_matches".to_string(), "没有找到匹配的记录".to_string());
        zh.insert("navigate_hint".to_string(), "导航 — 选择: 输入/jk, PgUp/PgDn或Ctrl+f/b翻页, 空格/Enter 切换, Tab → 预览；预览: jk 返回, Shift/Ctrl+↑/↓ 逐行, Enter 切换/比较, 空格/b/F 或 PgUp/PgDn 翻页, q 返回, Q 退出；Esc 返回/退出".to_string());
        zh.insert(
            "select_clean_command".to_string(),
            "选择要清理的命令:".to_string(),
        );
        zh.insert(
            "select_clean_file".to_string(),
            "选择要清理的文件:".to_string(),
        );
        zh.insert(
            "no_related_files".to_string(),
            "没有找到相关的文件记录".to_string(),
        );
        zh.insert(
            "found_related_files".to_string(),
            "找到以下相关文件:".to_string(),
        );
        zh.insert(
            "clean_command".to_string(),
            "使用以下命令清理特定文件:".to_string(),
        );
        zh.insert(
            "cleaned_records".to_string(),
            "清理了 {0} 条记录".to_string(),
        );
        zh.insert("cleaned_all".to_string(), "清理完成".to_string());
        zh.insert("no_records".to_string(), "没有找到任何记录".to_string());
        // removed unused/duplicate keys: total_records, filter_prompt, selected

        // Diff output
        zh.insert("diff_command".to_string(), "命令: {0}".to_string());
        zh.insert("diff_earlier".to_string(), "较早 ({0})".to_string());
        zh.insert("diff_later".to_string(), "较晚 ({0})".to_string());
        zh.insert("diff_earlier_label".to_string(), "较早".to_string());
        zh.insert("diff_later_label".to_string(), "较晚".to_string());
        zh.insert(
            "diff_exit_code".to_string(),
            "退出码: {0} -> {1}".to_string(),
        );
        zh.insert(
            "diff_execution_time".to_string(),
            "执行时间: {0}ms -> {1}ms".to_string(),
        );
        zh.insert("stdout_diff".to_string(), "标准输出差异:".to_string());
        zh.insert(
            "preview_stdout_header".to_string(),
            "输出预览（stdout）".to_string(),
        );
        zh.insert(
            "preview_stderr_header".to_string(),
            "输出预览（stderr）".to_string(),
        );
        zh.insert(
            "preview_diff_stdout_header".to_string(),
            "差异预览（stdout）".to_string(),
        );
        zh.insert(
            "preview_diff_stderr_header".to_string(),
            "差异预览（stderr）".to_string(),
        );
        zh.insert(
            "preview_toggle_hint".to_string(),
            "按 o 或 ←/→ 切换 stdout/stderr".to_string(),
        );
        zh.insert("preview_toggle_short".to_string(), "o/←/→ 切换".to_string());
        zh.insert("preview_path_label".to_string(), "路径: {0}".to_string());
        zh.insert(
            "preview_path_missing".to_string(),
            "路径: 暂不可用".to_string(),
        );
        zh.insert("preview_empty".to_string(), "输出为空".to_string());
        zh.insert(
            "preview_truncated_hint".to_string(),
            "… 内容较长，已截断".to_string(),
        );
        zh.insert(
            "preview_no_selection".to_string(),
            "请选择记录以查看输出预览".to_string(),
        );
        zh.insert(
            "preview_single_column_notice".to_string(),
            "终端宽度不足，使用单列视图".to_string(),
        );
        zh.insert(
            "terminal_too_small".to_string(),
            "终端尺寸过小：至少需要 {0}x{1}，当前 {2}x{3}".to_string(),
        );
        // 预览帮助浮层
        zh.insert("preview_help_title".to_string(), "预览帮助".to_string());
        zh.insert(
            "preview_help_move".to_string(),
            "切换选中: j/k 或 上/下方向键（返回列表并重置滚动）".to_string(),
        );
        zh.insert(
            "preview_help_page".to_string(),
            "滚动: Shift/Ctrl+↑/↓ 逐行；空格 或 f 向下，b 或 退格 向上".to_string(),
        );
        zh.insert(
            "preview_help_half".to_string(),
            "半页: d 向下, u 向上 或 Alt+↑/↓".to_string(),
        );
        zh.insert(
            "preview_help_top_bottom".to_string(),
            "顶/底: g / G 或 Home/End (Fn+←/→)".to_string(),
        );
        zh.insert(
            "preview_help_back".to_string(),
            "返回选择: q（Esc 亦可）".to_string(),
        );
        zh.insert(
            "preview_help_start_diff".to_string(),
            "切换/对比（已选2条时）: Enter".to_string(),
        );
        zh.insert(
            "preview_help_toggle".to_string(),
            "切换帮助: h / ?".to_string(),
        );
        zh.insert("preview_help_quit".to_string(), "退出程序: Q".to_string());

        // Selection help (Chinese)
        zh.insert("selection_help_title".to_string(), "选择帮助".to_string());
        zh.insert(
            "selection_help_filter".to_string(),
            "筛选: 输入文字进行模糊匹配筛选".to_string(),
        );
        zh.insert(
            "selection_help_move".to_string(),
            "移动: j/k 或 上/下, Ctrl+p/Ctrl+n (emacs 风格)".to_string(),
        );
        zh.insert(
            "selection_help_page".to_string(),
            "翻页: PgUp/PgDn 或 Ctrl+b/Ctrl+f (根据终端高度动态调整)".to_string(),
        );
        zh.insert(
            "selection_help_jump".to_string(),
            "跳转: Home/End 或 Ctrl+a/Ctrl+e (跳到顶部/底部)".to_string(),
        );
        zh.insert(
            "selection_help_select".to_string(),
            "选择: 空格 或 Enter 切换选中 (选择2个项目进行比较)".to_string(),
        );
        zh.insert(
            "selection_help_preview".to_string(),
            "预览: Tab 进入预览面板".to_string(),
        );
        zh.insert(
            "selection_help_clear".to_string(),
            "清除筛选: Ctrl+u (清除全部), Ctrl+w (删除单词)".to_string(),
        );
        zh.insert("stderr_diff".to_string(), "错误输出差异:".to_string());
        zh.insert("output_identical".to_string(), "输出完全一致".to_string());
        zh.insert(
            "warning_interactive_failed".to_string(),
            "警告: 无法启用交互式模式，回退到简单选择模式".to_string(),
        );
        zh.insert(
            "few_records_fallback".to_string(),
            "匹配的记录少于2个，将使用最新的两个记录".to_string(),
        );
        zh.insert(
            "using_filtered_records".to_string(),
            "使用过滤后的两个记录进行比较:".to_string(),
        );
        // Parse help (new)
        zh.insert(
            "help_parse".to_string(),
            "解析 Bash 片段/文件为 AST（基于 tree-sitter-bash）".to_string(),
        );
        zh.insert(
            "help_parse_file".to_string(),
            "解析的文件路径；缺省则从 STDIN 读取".to_string(),
        );
        zh.insert(
            "help_parse_json".to_string(),
            "以 JSON 输出（默认为概要树）".to_string(),
        );

        // Error messages
        zh.insert(
            "error_create_dt_dir".to_string(),
            "创建 .dt 目录失败".to_string(),
        );
        zh.insert(
            "error_create_records_dir".to_string(),
            "创建 records 目录失败".to_string(),
        );
        zh.insert(
            "error_create_record_dir".to_string(),
            "创建记录目录失败".to_string(),
        );
        zh.insert(
            "error_save_metadata".to_string(),
            "保存元数据失败".to_string(),
        );
        zh.insert(
            "error_save_stdout".to_string(),
            "保存标准输出失败".to_string(),
        );
        zh.insert(
            "error_save_stderr".to_string(),
            "保存错误输出失败".to_string(),
        );
        zh.insert(
            "error_read_stdout".to_string(),
            "无法读取标准输出".to_string(),
        );
        zh.insert(
            "error_read_stderr".to_string(),
            "无法读取错误输出".to_string(),
        );
        zh.insert("error_update_index".to_string(), "更新索引失败".to_string());
        zh.insert(
            "error_save_archive".to_string(),
            "保存 {0} 年归档失败".to_string(),
        );
        zh.insert(
            "error_rebuild_index".to_string(),
            "重建索引失败".to_string(),
        );
        zh.insert(
            "error_execute_command".to_string(),
            "执行命令失败".to_string(),
        );

        // Clean operation
        zh.insert(
            "clean_record".to_string(),
            "清理记录: {0} (时间: {1})".to_string(),
        );
        zh.insert(
            "clean_file_example".to_string(),
            "dt clean file <文件路径>".to_string(),
        );

        translations.insert("en".to_string(), en);
        translations.insert("zh".to_string(), zh);

        // Determine effective language - support multiple language code forms
        let effective_lang = if lang.starts_with("zh") || lang == "cn" || lang == "chinese" {
            "zh"
        } else if lang.starts_with("en") || lang == "english" {
            "en"
        } else {
            // Default to English
            "en"
        };

        Self {
            translations,
            current_lang: effective_lang.to_string(),
        }
    }

    pub fn t(&self, key: &str) -> String {
        if let Some(lang_map) = self.translations.get(&self.current_lang) {
            if let Some(value) = lang_map.get(key) {
                return value.clone();
            }
        }
        key.to_string()
    }

    pub fn t_format(&self, key: &str, args: &[&str]) -> String {
        let template = self.t(key);
        let mut result = template;
        for (i, arg) in args.iter().enumerate() {
            result = result.replace(&format!("{{{}}}", i), arg);
        }
        result
    }
}
