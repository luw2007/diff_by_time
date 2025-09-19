use crate::storage::{CommandExecution, CommandRecord};
use anyhow::{Context, Result};
use chrono::{Datelike, Duration, Utc};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub struct StoreManager {
    base_dir: PathBuf,
    config: crate::config::Config,
}

impl StoreManager {
    pub fn new_with_config(
        config: crate::config::Config,
        i18n: &crate::i18n::I18n,
    ) -> Result<Self> {
        Self::new_with_config_and_base_dir(config, i18n, None)
    }

    pub fn new_with_config_and_base_dir(
        config: crate::config::Config,
        i18n: &crate::i18n::I18n,
        base_override: Option<PathBuf>,
    ) -> Result<Self> {
        let base_dir = if let Some(dir) = base_override {
            dir
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".dt")
        };

        fs::create_dir_all(&base_dir).context(i18n.t("error_create_dt_dir"))?;

        let records_dir = base_dir.join("records");
        fs::create_dir_all(&records_dir).context(i18n.t("error_create_records_dir"))?;

        Ok(Self { base_dir, config })
    }

    // Removed unused convenience constructor to avoid dead_code warnings.

    pub fn save_execution(
        &self,
        execution: &CommandExecution,
        i18n: &crate::i18n::I18n,
    ) -> Result<()> {
        let record_dir = self
            .base_dir
            .join("records")
            .join(&execution.record.command_hash);

        fs::create_dir_all(&record_dir).context(i18n.t("error_create_record_dir"))?;

        let meta_path = record_dir.join(format!(
            "meta_{}.json",
            execution.record.timestamp.timestamp()
        ));
        let stdout_path = record_dir.join(format!(
            "stdout_{}.txt",
            execution.record.timestamp.timestamp()
        ));
        let stderr_path = record_dir.join(format!(
            "stderr_{}.txt",
            execution.record.timestamp.timestamp()
        ));

        serde_json::to_writer_pretty(fs::File::create(&meta_path)?, &execution.record)
            .context(i18n.t("error_save_metadata"))?;

        fs::write(&stdout_path, &execution.stdout).context(i18n.t("error_save_stdout"))?;

        fs::write(&stderr_path, &execution.stderr).context(i18n.t("error_save_stderr"))?;

        self.update_index(&execution.record, i18n)?;

        Ok(())
    }

    /// Assign a minimal unused short code for the given record (per command hash).
    /// Codes are bijective base62 with alphabet a-zA-Z0-9, starting from 1 => 'a'.
    pub fn assign_short_code(
        &self,
        record: &mut CommandRecord,
        _i18n: &crate::i18n::I18n,
    ) -> Result<()> {
        let record_dir = self.base_dir.join("records").join(&record.command_hash);

        let mut used: HashSet<String> = HashSet::new();

        if record_dir.exists() {
            for entry in fs::read_dir(&record_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("json")
                    && path
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with("meta_")
                {
                    if let Ok(existing) =
                        serde_json::from_reader::<_, CommandRecord>(fs::File::open(&path)?)
                    {
                        if let Some(code) = existing.short_code {
                            used.insert(code);
                        }
                    }
                }
            }
        }

        // Find minimal unused n starting from 1
        let mut n: u64 = 1;
        loop {
            let code = Self::encode_bijective_base62(n);
            if !used.contains(&code) {
                record.short_code = Some(code);
                break;
            }
            n += 1;
        }

        Ok(())
    }

    fn encode_bijective_base62(mut n: u64) -> String {
        const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let base = ALPHABET.len() as u64; // 62
        let mut buf: Vec<u8> = Vec::new();
        while n > 0 {
            let mut rem = n % base;
            if rem == 0 {
                rem = base;
                n = n / base - 1;
            } else {
                n /= base;
            }
            let idx = (rem - 1) as usize;
            buf.push(ALPHABET[idx]);
        }
        buf.reverse();
        String::from_utf8(buf).unwrap()
    }

    pub fn find_executions(
        &self,
        command_hash: &str,
        i18n: &crate::i18n::I18n,
    ) -> Result<Vec<CommandExecution>> {
        let record_dir = self.base_dir.join("records").join(command_hash);

        if !record_dir.exists() {
            return Ok(Vec::new());
        }

        let mut executions = Vec::new();

        for entry in fs::read_dir(&record_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json")
                && path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("meta_")
            {
                if let Ok(execution) = self.load_execution_from_meta(&path, i18n) {
                    executions.push(execution);
                }
            }
        }

        executions.sort_by(|a, b| a.record.timestamp.cmp(&b.record.timestamp));
        Ok(executions)
    }

    fn load_execution_from_meta(
        &self,
        meta_path: &Path,
        i18n: &crate::i18n::I18n,
    ) -> Result<CommandExecution> {
        let record: CommandRecord = serde_json::from_reader(fs::File::open(meta_path)?)?;

        let timestamp = record.timestamp.timestamp();
        let record_dir = meta_path.parent().unwrap();

        let stdout_path = record_dir.join(format!("stdout_{}.txt", timestamp));
        let stderr_path = record_dir.join(format!("stderr_{}.txt", timestamp));

        let stdout =
            fs::read_to_string(&stdout_path).unwrap_or_else(|_| i18n.t("error_read_stdout"));

        let stderr =
            fs::read_to_string(&stderr_path).unwrap_or_else(|_| i18n.t("error_read_stderr"));

        Ok(CommandExecution {
            record,
            stdout,
            stderr,
            stdout_path: Some(stdout_path),
            stderr_path: Some(stderr_path),
        })
    }

    fn update_index(&self, record: &CommandRecord, i18n: &crate::i18n::I18n) -> Result<()> {
        let index_path = self.base_dir.join("index");

        let mut entries = Vec::new();
        if index_path.exists() {
            if let Ok(content) = fs::read_to_string(&index_path) {
                entries = serde_json::from_str(&content).unwrap_or_else(|_| Vec::new());
            }
        }

        // Check if archiving is needed
        if self.config.storage.auto_archive {
            self.check_and_archive(&mut entries, i18n)?;
        }

        entries.push(record.clone());

        // Apply retention days limit
        let cutoff_date =
            Utc::now() - Duration::days(self.config.storage.max_retention_days as i64);
        entries.retain(|r| r.timestamp > cutoff_date);

        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        serde_json::to_writer_pretty(fs::File::create(index_path)?, &entries)
            .context(i18n.t("error_update_index"))?;

        Ok(())
    }

    fn check_and_archive(
        &self,
        entries: &mut Vec<CommandRecord>,
        i18n: &crate::i18n::I18n,
    ) -> Result<()> {
        let cutoff_date =
            Utc::now() - Duration::days(self.config.storage.max_retention_days as i64);

        let to_archive: Vec<CommandRecord> = entries
            .iter()
            .filter(|r| r.timestamp <= cutoff_date)
            .cloned()
            .collect();

        if !to_archive.is_empty() {
            // Group by year for archiving
            let mut by_year: std::collections::HashMap<u32, Vec<CommandRecord>> =
                std::collections::HashMap::new();

            for record in to_archive {
                let year = record.timestamp.year() as u32;
                by_year.entry(year).or_default().push(record);
            }

            // Save to yearly archive file
            for (year, records) in by_year {
                let archive_path = self.base_dir.join(format!("index_{}.json", year));
                let mut existing_records = Vec::new();

                if archive_path.exists() {
                    if let Ok(content) = fs::read_to_string(&archive_path) {
                        existing_records =
                            serde_json::from_str(&content).unwrap_or_else(|_| Vec::new());
                    }
                }

                existing_records.extend(records);
                existing_records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

                serde_json::to_writer_pretty(fs::File::create(archive_path)?, &existing_records)
                    .context(i18n.t_format("error_save_archive", &[&year.to_string()]))?;
            }

            // Remove archived records from main index
            entries.retain(|r| r.timestamp > cutoff_date);
        }

        Ok(())
    }

    pub fn get_all_records(&self) -> Result<Vec<CommandRecord>> {
        let index_path = self.base_dir.join("index");

        if !index_path.exists() {
            return Ok(Vec::new());
        }

        let records: Vec<CommandRecord> = serde_json::from_reader(fs::File::open(&index_path)?)?;
        Ok(records)
    }

    #[allow(dead_code)]
    pub fn backup_by_file(&self, file_path: &Path, i18n: &crate::i18n::I18n) -> Result<usize> {
        let records = self.get_all_records()?;

        let target_path = match fs::canonicalize(file_path) {
            Ok(abs_path) => abs_path,
            Err(_) => file_path.to_path_buf(),
        };

        let mut to_backup: Vec<CommandRecord> = Vec::new();

        for record in records {
            let mut matched = false;
            if record.working_dir == target_path {
                matched = true;
            }
            let file_str = file_path.to_string_lossy();
            let target_str = target_path.to_string_lossy();
            if record.command.contains(file_str.as_ref())
                || record.command.contains(target_str.as_ref())
            {
                matched = true;
            }
            if let Some(rel_path) = pathdiff::diff_paths(file_path, &record.working_dir) {
                let rel_str = rel_path.to_string_lossy();
                if record.command.contains(rel_str.as_ref()) {
                    matched = true;
                }
            }
            if matched {
                to_backup.push(record);
            }
        }

        if to_backup.is_empty() {
            return Ok(0);
        }

        // Append to current year's archive (index_YYYY.json), deduplicating by record_id
        let year = chrono::Utc::now().year() as u32;
        let archive_path = self.base_dir.join(format!("index_{}.json", year));

        let mut existing: Vec<CommandRecord> = if archive_path.exists() {
            if let Ok(content) = fs::read_to_string(&archive_path) {
                serde_json::from_str(&content).unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let mut seen: std::collections::HashSet<String> =
            existing.iter().map(|r| r.record_id.clone()).collect();
        for r in to_backup {
            if seen.insert(r.record_id.clone()) {
                existing.push(r);
            }
        }

        existing.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        serde_json::to_writer_pretty(fs::File::create(&archive_path)?, &existing)
            .context(i18n.t_format("error_save_archive", &[&year.to_string()]))?;

        Ok(existing.len())
    }

    pub fn clean_by_query(&self, query: &str, i18n: &crate::i18n::I18n) -> Result<usize> {
        let records = self.get_all_records()?;
        let mut cleaned = 0;

        let q = query.trim();
        if q.is_empty() {
            return Ok(0);
        }
        let q_lower = q.to_lowercase();

        // simple fuzzy: subsequence match (skim-like loose match) fallback
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

        for record in records {
            let cmd_lower = record.command.to_lowercase();
            let substring = cmd_lower.contains(&q_lower);
            let fuzzy = !substring && is_subsequence(&q_lower, &cmd_lower);
            if substring || fuzzy {
                self.clean_record(&record)?;
                cleaned += 1;
            }
        }

        self.rebuild_index(i18n)?;
        Ok(cleaned)
    }

    pub fn clean_by_file(&self, file_path: &Path, i18n: &crate::i18n::I18n) -> Result<usize> {
        let records = self.get_all_records()?;
        let mut cleaned = 0;

        // Try to get absolute path, use original path if failed
        let target_path = match fs::canonicalize(file_path) {
            Ok(abs_path) => abs_path,
            Err(_) => file_path.to_path_buf(),
        };

        for record in records {
            let mut should_clean = false;

            // Check working directory
            if record.working_dir == target_path {
                should_clean = true;
            }

            // Check if command contains file path (multiple matching methods)
            let file_str = file_path.to_string_lossy();
            let target_str = target_path.to_string_lossy();

            if record.command.contains(file_str.as_ref())
                || record.command.contains(target_str.as_ref())
            {
                should_clean = true;
            }

            // If relative path, also try to match relative path
            if let Some(rel_path) = pathdiff::diff_paths(file_path, &record.working_dir) {
                let rel_str = rel_path.to_string_lossy();
                if record.command.contains(rel_str.as_ref()) {
                    should_clean = true;
                }
            }

            if should_clean {
                self.clean_record(&record)?;
                cleaned += 1;
                println!(
                    "{}",
                    i18n.t_format(
                        "clean_record",
                        &[
                            &record.command,
                            &record.timestamp.format("%Y-%m-%d %H:%M:%S").to_string()
                        ]
                    )
                );
            }
        }

        self.rebuild_index(i18n)?;
        Ok(cleaned)
    }

    pub fn get_related_files(&self) -> Result<Vec<PathBuf>> {
        let records = self.get_all_records()?;
        let mut files = std::collections::HashSet::new();

        for record in records {
            // Add working directory
            files.insert(record.working_dir);

            // Extract file paths from command
            self.extract_files_from_command(&record.command, &mut files);
        }

        let mut result: Vec<PathBuf> = files.into_iter().collect();
        result.sort();
        Ok(result)
    }

    fn extract_files_from_command(
        &self,
        command: &str,
        files: &mut std::collections::HashSet<PathBuf>,
    ) {
        // Simple file path extraction logic
        let tokens: Vec<&str> = command.split_whitespace().collect();

        for token in tokens {
            let path = PathBuf::from(token);

            // If it looks like a file path (contains / or . extension)
            if token.contains('/') || path.extension().is_some() || token == "ls" || token == "cat"
            {
                if path.exists() {
                    if let Ok(abs_path) = fs::canonicalize(&path) {
                        files.insert(abs_path);
                    }
                } else {
                    // Record possible path even if file doesn't exist
                    files.insert(path);
                }
            }
        }
    }

    pub fn clean_all(&self, _i18n: &crate::i18n::I18n) -> Result<usize> {
        let records_dir = self.base_dir.join("records");
        if records_dir.exists() {
            fs::remove_dir_all(&records_dir)?;
            fs::create_dir_all(&records_dir)?;
        }

        let index_path = self.base_dir.join("index");
        if index_path.exists() {
            fs::remove_file(&index_path)?;
        }

        Ok(0)
    }

    fn clean_record(&self, record: &CommandRecord) -> Result<()> {
        let record_dir = self.base_dir.join("records").join(&record.command_hash);

        let timestamp = record.timestamp.timestamp();
        let meta_path = record_dir.join(format!("meta_{}.json", timestamp));
        let stdout_path = record_dir.join(format!("stdout_{}.txt", timestamp));
        let stderr_path = record_dir.join(format!("stderr_{}.txt", timestamp));

        let _ = fs::remove_file(meta_path);
        let _ = fs::remove_file(stdout_path);
        let _ = fs::remove_file(stderr_path);

        Ok(())
    }

    pub fn delete_execution(
        &self,
        execution: &CommandExecution,
        i18n: &crate::i18n::I18n,
    ) -> Result<()> {
        self.clean_record(&execution.record)?;
        self.rebuild_index(i18n)?;
        Ok(())
    }

    fn rebuild_index(&self, i18n: &crate::i18n::I18n) -> Result<()> {
        let records_dir = self.base_dir.join("records");
        let mut all_records = Vec::new();

        if records_dir.exists() {
            for hash_dir in fs::read_dir(records_dir)? {
                let hash_dir = hash_dir?;
                let hash_dir_path = hash_dir.path();

                if hash_dir_path.is_dir() {
                    for entry in fs::read_dir(&hash_dir_path)? {
                        let entry = entry?;
                        let path = entry.path();

                        if path.extension().and_then(|s| s.to_str()) == Some("json")
                            && path
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .starts_with("meta_")
                        {
                            if let Ok(record) =
                                serde_json::from_reader::<_, CommandRecord>(fs::File::open(&path)?)
                            {
                                all_records.push(record);
                            }
                        }
                    }
                }
            }
        }

        all_records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let index_path = self.base_dir.join("index");
        serde_json::to_writer_pretty(fs::File::create(index_path)?, &all_records)
            .context(i18n.t("error_rebuild_index"))?;

        Ok(())
    }
}
