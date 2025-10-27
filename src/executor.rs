use crate::storage::{CommandExecution, CommandRecord};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Instant;

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

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn execute(command: &str, i18n: &crate::i18n::I18n) -> Result<CommandExecution> {
        let start_time = Instant::now();

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context(i18n.t("error_execute_command"))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!(i18n.t("error_read_stdout")))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!(i18n.t("error_read_stderr")))?;

        let stdout_handle = thread::spawn(move || -> Result<Vec<u8>> {
            let mut reader = stdout;
            let mut buffer = [0u8; 4096];
            let mut collected = Vec::new();
            loop {
                let bytes_read = reader.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                collected.extend_from_slice(&buffer[..bytes_read]);
                {
                    let mut handle = io::stdout();
                    handle.write_all(&buffer[..bytes_read])?;
                    handle.flush()?;
                }
            }
            Ok(collected)
        });

        let stderr_handle = thread::spawn(move || -> Result<Vec<u8>> {
            let mut reader = stderr;
            let mut buffer = [0u8; 4096];
            let mut collected = Vec::new();
            loop {
                let bytes_read = reader.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                collected.extend_from_slice(&buffer[..bytes_read]);
                {
                    let mut handle = io::stderr();
                    handle.write_all(&buffer[..bytes_read])?;
                    handle.flush()?;
                }
            }
            Ok(collected)
        });

        let status = child.wait().context(i18n.t("error_execute_command"))?;

        let stdout_bytes = stdout_handle
            .join()
            .map_err(|_| anyhow!(i18n.t("error_read_stdout")))??;
        let stderr_bytes = stderr_handle
            .join()
            .map_err(|_| anyhow!(i18n.t("error_read_stderr")))??;

        let duration = start_time.elapsed();

        let working_dir = std::env::current_dir()?;
        let formatted_command = format_command(command);
        let command_hash = Self::hash_command(&formatted_command);
        let timestamp = Utc::now();
        let record_id = format!("{}_{}", command_hash, timestamp.timestamp());

        let record = CommandRecord {
            command: formatted_command,
            command_hash,
            timestamp,
            working_dir,
            exit_code: status.code().unwrap_or(-1),
            duration_ms: duration.as_millis() as u64,
            record_id,
            short_code: None,
        };

        let execution = CommandExecution {
            record,
            stdout: String::from_utf8_lossy(&stdout_bytes).to_string(),
            stderr: String::from_utf8_lossy(&stderr_bytes).to_string(),
            stdout_path: None,
            stderr_path: None,
            streamed_stdout: true,
            streamed_stderr: true,
        };

        Ok(execution)
    }

    fn hash_command(command: &str) -> String {
        let formatted_command = format_command(command);
        let mut hasher = Sha256::new();
        hasher.update(formatted_command.as_bytes());
        hex::encode(hasher.finalize())
    }
}
