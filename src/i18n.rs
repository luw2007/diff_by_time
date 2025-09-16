use std::collections::HashMap;

pub struct I18n {
    translations: HashMap<String, HashMap<String, String>>,
    current_lang: String,
}

impl I18n {
    pub fn new(lang: &str) -> Self {
        let mut translations = HashMap::new();

        // 英文
        let mut en = HashMap::new();
        // Help texts
        en.insert("help_about".to_string(), "Command execution time diff tool".to_string());
        en.insert("help_run".to_string(), "Execute command and record output".to_string());
        en.insert("help_run_command".to_string(), "Command to execute (wrap commands with pipes in quotes)".to_string());
        en.insert("help_diff".to_string(), "Compare command output differences".to_string());
        en.insert("help_diff_command".to_string(), "Command to compare (wrap commands with pipes in quotes)".to_string());
        en.insert("help_diff_max_shown".to_string(), "Maximum number of selection records to display [default: 20]".to_string());
        // Config help
        en.insert("help_config_section".to_string(), "Config (~/.dt/config.toml):".to_string());
        en.insert("help_config_tui_mode".to_string(), "display.tui_mode: interactive | simple (interactive by default)".to_string());
        en.insert("help_config_alt_screen".to_string(), "display.alt_screen: true | false (use alternate screen in interactive mode; default: false)".to_string());
        en.insert("help_clean".to_string(), "Clean history records".to_string());
        // removed: help_clean_mode (not used)
        en.insert("help_list".to_string(), "List all history records".to_string());
        en.insert("help_list_no_merge".to_string(), "Do not merge records with same commands [default: merge]".to_string());
        en.insert("help_list_filter".to_string(), "Filter command string".to_string());
        en.insert("help_clean_prefix".to_string(), "Clean by command prefix".to_string());
        en.insert("help_clean_prefix_arg".to_string(), "Command prefix".to_string());
        en.insert("help_clean_file".to_string(), "Clean by file".to_string());
        en.insert("help_clean_file_arg".to_string(), "File path (optional, if not provided will show related files list)".to_string());
        en.insert("help_clean_all".to_string(), "Clean all records".to_string());

        // Runtime messages
        // removed unused runtime prompts: continue_prompt, continue_hint, execution_cancelled
        en.insert("command_completed".to_string(), "Command completed, exit code: {0}".to_string());
        en.insert("execution_time".to_string(), "Execution time".to_string());
        en.insert("stdout".to_string(), "Standard output:".to_string());
        en.insert("stderr".to_string(), "Error output:".to_string());
        en.insert("result_saved".to_string(), "Result saved".to_string());
        en.insert("need_at_least_two".to_string(), "Need at least two execution records for comparison".to_string());
        en.insert("select_executions".to_string(), "Found {0} execution records, please select two to compare:".to_string());
        en.insert("input_numbers".to_string(), "Input two numbers (space separated, e.g., 1 2):".to_string());
        en.insert("invalid_input".to_string(), "Invalid input, will use the latest two records".to_string());

    // Interactive selection messages
    en.insert("interactive_filter".to_string(), "FZF Filter (type to fuzzy search, j/k to navigate, Enter to select, Delete to clear, Esc to quit):".to_string());
    en.insert("first_selection".to_string(), "Select first record (press Enter to confirm):".to_string());
    en.insert("second_selection".to_string(), "Select second record (press Enter to confirm):".to_string());
    en.insert("selection_complete".to_string(), "Selection complete, comparing records...".to_string());
    en.insert("no_matches".to_string(), "No matches found".to_string());
    en.insert("navigate_hint".to_string(), "FZF Navigation: j/k ↑/↓=move, Enter=select, Delete=clear, Esc=quit".to_string());
        en.insert("no_related_files".to_string(), "No related file records found".to_string());
        en.insert("found_related_files".to_string(), "Found following related files:".to_string());
        en.insert("clean_command".to_string(), "Use following command to clean specific file:".to_string());
        en.insert("cleaned_records".to_string(), "Cleaned {0} records".to_string());
        en.insert("cleaned_all".to_string(), "Clean completed".to_string());
        en.insert("no_records".to_string(), "No records found".to_string());
        en.insert("history_records".to_string(), "History records:".to_string());
        // removed unused/duplicate keys: total_records, filter_prompt, selected, first/second_selection (duplicates), no_matches (duplicate), exit_code, time
            en.insert("single_record".to_string(), "{0} (exit code: {1}, execution time: {2}ms, time: {3})".to_string());
        en.insert("multiple_records".to_string(), "{0} [{1} executions] (exit code: {2}, execution time: {3}ms, first: {4}, latest: {5})".to_string());
        en.insert("all_records".to_string(), "{0} (exit code: {1}, execution time: {2}ms, time: {3})".to_string());
        en.insert("merged_commands".to_string(), "(same commands merged, total {0} different commands)".to_string());
        en.insert("showing_all".to_string(), "(showing all {0} records)".to_string());

        // Diff output
        en.insert("diff_command".to_string(), "Command: {0}".to_string());
        en.insert("diff_earlier".to_string(), "Earlier ({0})".to_string());
        en.insert("diff_later".to_string(), "Later ({0})".to_string());
        en.insert("diff_exit_code".to_string(), "exit code: {0} -> {1}".to_string());
        en.insert("diff_execution_time".to_string(), "execution time: {0}ms -> {1}ms".to_string());
        en.insert("stdout_diff".to_string(), "stdout diff:".to_string());
        en.insert("stderr_diff".to_string(), "stderr diff:".to_string());
        en.insert("output_identical".to_string(), "output is identical".to_string());
        en.insert("warning_interactive_failed".to_string(), "Warning: Cannot enable interactive mode, falling back to simple selection mode".to_string());
        // removed unused month_* keys
        en.insert("few_records_fallback".to_string(), "Less than 2 matched records, using the latest two records".to_string());
        en.insert("using_filtered_records".to_string(), "Using the two filtered records for comparison:".to_string());

        // Error messages
        en.insert("error_create_dt_dir".to_string(), "Failed to create .dt directory".to_string());
        en.insert("error_create_records_dir".to_string(), "Failed to create records directory".to_string());
        en.insert("error_create_record_dir".to_string(), "Failed to create record directory".to_string());
        en.insert("error_save_metadata".to_string(), "Failed to save metadata".to_string());
        en.insert("error_save_stdout".to_string(), "Failed to save stdout".to_string());
        en.insert("error_save_stderr".to_string(), "Failed to save stderr".to_string());
        en.insert("error_read_stdout".to_string(), "Cannot read stdout".to_string());
        en.insert("error_read_stderr".to_string(), "Cannot read stderr".to_string());
        en.insert("error_update_index".to_string(), "Failed to update index".to_string());
        en.insert("error_save_archive".to_string(), "Failed to save {0} year archive".to_string());
        en.insert("error_rebuild_index".to_string(), "Failed to rebuild index".to_string());
        en.insert("error_execute_command".to_string(), "Failed to execute command".to_string());

        // Clean operation
        en.insert("clean_record".to_string(), "Cleaning record: {0} (time: {1})".to_string());
        en.insert("clean_file_example".to_string(), "dt clean file <file_path>".to_string());

        // 中文
        let mut zh = HashMap::new();
        // Help texts
        zh.insert("help_about".to_string(), "命令执行时间差比较工具".to_string());
        zh.insert("help_run".to_string(), "执行命令并记录输出".to_string());
        zh.insert("help_run_command".to_string(), "要执行的命令（用引号包裹包含管道的命令）".to_string());
        zh.insert("help_diff".to_string(), "比较命令输出差异".to_string());
        zh.insert("help_diff_command".to_string(), "要比较的命令（用引号包裹包含管道的命令）".to_string());
        zh.insert("help_diff_max_shown".to_string(), "最多显示的选择记录数 [默认: 20]".to_string());
        // Config help
        zh.insert("help_config_section".to_string(), "配置文件 (~/.dt/config.toml):".to_string());
        zh.insert("help_config_tui_mode".to_string(), "display.tui_mode: interactive | simple（默认 interactive，simple 为打印文本模式）".to_string());
        zh.insert("help_config_alt_screen".to_string(), "display.alt_screen: true | false（交互模式是否使用备用屏，默认 false）".to_string());
        zh.insert("help_clean".to_string(), "清理历史记录".to_string());
        // removed: help_clean_mode (not used)
        zh.insert("help_list".to_string(), "列出所有历史记录".to_string());
        zh.insert("help_list_no_merge".to_string(), "不合并相同命令的记录 [默认: 合并]".to_string());
        zh.insert("help_list_filter".to_string(), "过滤命令字符串".to_string());
        zh.insert("help_clean_prefix".to_string(), "按命令前缀清理".to_string());
        zh.insert("help_clean_prefix_arg".to_string(), "命令前缀".to_string());
        zh.insert("help_clean_file".to_string(), "按文件清理".to_string());
        zh.insert("help_clean_file_arg".to_string(), "文件路径 (可选，不提供则显示相关文件列表)".to_string());
        zh.insert("help_clean_all".to_string(), "清理所有记录".to_string());

        // Runtime messages
        // removed unused runtime prompts: continue_prompt, continue_hint, execution_cancelled
        zh.insert("command_completed".to_string(), "命令执行完成，退出码: {0}".to_string());
        zh.insert("execution_time".to_string(), "执行时间".to_string());
        zh.insert("stdout".to_string(), "标准输出:".to_string());
        zh.insert("stderr".to_string(), "错误输出:".to_string());
        zh.insert("result_saved".to_string(), "结果已保存".to_string());
        zh.insert("need_at_least_two".to_string(), "需要至少两个执行记录才能进行比较".to_string());
        zh.insert("select_executions".to_string(), "找到 {0} 个执行记录，请选择要比较的两个:".to_string());
        zh.insert("input_numbers".to_string(), "输入两个数字 (用空格分隔，例如: 1 2):".to_string());
        zh.insert("invalid_input".to_string(), "无效输入，将使用最新的两个记录".to_string());

    // Interactive selection messages
    zh.insert("interactive_filter".to_string(), "FZF过滤器 (输入模糊搜索，j/k导航，Enter选择，Delete清空，Esc退出):".to_string());
    zh.insert("first_selection".to_string(), "选择第一条记录 (按Enter确认):".to_string());
    zh.insert("second_selection".to_string(), "选择第二条记录 (按Enter确认):".to_string());
    zh.insert("selection_complete".to_string(), "选择完成，正在比较记录...".to_string());
    zh.insert("no_matches".to_string(), "没有找到匹配的记录".to_string());
    zh.insert("navigate_hint".to_string(), "FZF导航: j/k ↑/↓=移动, Enter=选择, Delete=清空, Esc=退出".to_string());
        zh.insert("no_related_files".to_string(), "没有找到相关的文件记录".to_string());
        zh.insert("found_related_files".to_string(), "找到以下相关文件:".to_string());
        zh.insert("clean_command".to_string(), "使用以下命令清理特定文件:".to_string());
        zh.insert("cleaned_records".to_string(), "清理了 {0} 条记录".to_string());
        zh.insert("cleaned_all".to_string(), "清理完成".to_string());
        zh.insert("no_records".to_string(), "没有找到任何记录".to_string());
        zh.insert("history_records".to_string(), "历史记录:".to_string());
        // removed unused/duplicate keys: total_records, filter_prompt, selected, first/second_selection (duplicates), no_matches (duplicate), exit_code, time
              zh.insert("single_record".to_string(), "{0} (退出码: {1}, 执行时间: {2}ms, 时间: {3})".to_string());
        zh.insert("multiple_records".to_string(), "{0} [{1}次执行] (退出码: {2}, 执行时间: {3}ms, 首次: {4}, 最新: {5})".to_string());
        zh.insert("all_records".to_string(), "{0} (退出码: {1}, 执行时间: {2}ms, 时间: {3})".to_string());
        zh.insert("merged_commands".to_string(), "(相同命令已合并，共 {0} 个不同命令)".to_string());
        zh.insert("showing_all".to_string(), "(显示所有 {0} 条记录)".to_string());

        // Diff output
        zh.insert("diff_command".to_string(), "命令: {0}".to_string());
        zh.insert("diff_earlier".to_string(), "较早 ({0})".to_string());
        zh.insert("diff_later".to_string(), "较晚 ({0})".to_string());
        zh.insert("diff_exit_code".to_string(), "退出码: {0} -> {1}".to_string());
        zh.insert("diff_execution_time".to_string(), "执行时间: {0}ms -> {1}ms".to_string());
        zh.insert("stdout_diff".to_string(), "标准输出差异:".to_string());
        zh.insert("stderr_diff".to_string(), "错误输出差异:".to_string());
        zh.insert("output_identical".to_string(), "输出完全一致".to_string());
        zh.insert("warning_interactive_failed".to_string(), "警告: 无法启用交互式模式，回退到简单选择模式".to_string());
        // removed unused month_* keys
        zh.insert("few_records_fallback".to_string(), "匹配的记录少于2个，将使用最新的两个记录".to_string());
        zh.insert("using_filtered_records".to_string(), "使用过滤后的两个记录进行比较:".to_string());

        // Error messages
        zh.insert("error_create_dt_dir".to_string(), "创建 .dt 目录失败".to_string());
        zh.insert("error_create_records_dir".to_string(), "创建 records 目录失败".to_string());
        zh.insert("error_create_record_dir".to_string(), "创建记录目录失败".to_string());
        zh.insert("error_save_metadata".to_string(), "保存元数据失败".to_string());
        zh.insert("error_save_stdout".to_string(), "保存标准输出失败".to_string());
        zh.insert("error_save_stderr".to_string(), "保存错误输出失败".to_string());
        zh.insert("error_read_stdout".to_string(), "无法读取标准输出".to_string());
        zh.insert("error_read_stderr".to_string(), "无法读取错误输出".to_string());
        zh.insert("error_update_index".to_string(), "更新索引失败".to_string());
        zh.insert("error_save_archive".to_string(), "保存 {0} 年归档失败".to_string());
        zh.insert("error_rebuild_index".to_string(), "重建索引失败".to_string());
        zh.insert("error_execute_command".to_string(), "执行命令失败".to_string());

        // Clean operation
        zh.insert("clean_record".to_string(), "清理记录: {0} (时间: {1})".to_string());
        zh.insert("clean_file_example".to_string(), "dt clean file <文件路径>".to_string());

        translations.insert("en".to_string(), en);
        translations.insert("zh".to_string(), zh);

        // 确定语言 - 支持多种语言代码格式
        let effective_lang = if lang.starts_with("zh") || lang == "cn" || lang == "chinese" {
            "zh"
        } else if lang.starts_with("en") || lang == "english" {
            "en"
        } else {
            // 默认使用英文
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
