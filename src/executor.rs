use crate::storage::{CommandExecution, CommandRecord};
use anyhow::{Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::process::Command;
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

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .context(i18n.t("error_execute_command"))?;

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
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms: duration.as_millis() as u64,
            record_id,
            short_code: None,
        };

        let execution = CommandExecution {
            record,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            stdout_path: None,
            stderr_path: None,
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
