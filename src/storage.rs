use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub fn format_command(command: &str) -> String {
    // Remove leading and trailing whitespace and quotes
    let mut trimmed = command.trim();
    while (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('"') && trimmed.ends_with('"'))
    {
        if trimmed.len() < 2 {
            break;
        }
        trimmed = &trimmed[1..trimmed.len() - 1];
        trimmed = trimmed.trim();
    }

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

pub fn hash_command(command: &str) -> String {
    let formatted_command = format_command(command);
    let mut hasher = Sha256::new();
    hasher.update(formatted_command.as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandRecord {
    pub command: String,          // Command executed
    pub command_hash: String,     // SHA256 hash of the command
    pub timestamp: DateTime<Utc>, // Execution timestamp
    pub working_dir: PathBuf,     // Working directory
    pub exit_code: i32,           // Exit code
    pub duration_ms: u64,         // Execution duration (milliseconds)
    pub record_id: String,        // Record unique identifier
    #[serde(default)]
    pub short_code: Option<String>, // Short code for quick reference (per-command)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandExecution {
    pub record: CommandRecord,        // Command record
    pub stdout: String,               // Standard output
    pub stderr: String,               // Standard error output
    pub stdout_path: Option<PathBuf>, // Stored stdout file path
    pub stderr_path: Option<PathBuf>, // Stored stderr file path
    #[serde(skip)]
    pub streamed_stdout: bool, // Indicates stdout was streamed live during execution
    #[serde(skip)]
    pub streamed_stderr: bool, // Indicates stderr was streamed live during execution
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_command_trim_quotes() {
        assert_eq!(format_command("'ls -l'"), "ls -l");
        assert_eq!(format_command("\"ls -l\""), "ls -l");
        assert_eq!(format_command("  'ls -l'  "), "ls -l");
        assert_eq!(format_command("'  ls -l  '"), "ls -l");
        assert_eq!(format_command("\"'ls -l'\""), "ls -l");
        assert_eq!(format_command("'\"ls -l\"'"), "ls -l");
    }

    #[test]
    fn test_format_command_normalization() {
        assert_eq!(format_command("ls    -l"), "ls -l");
        assert_eq!(format_command("ls | grep a"), "ls|grep a");
        assert_eq!(format_command("ls  |  grep  a"), "ls|grep a");
    }

    #[test]
    fn test_hash_command_consistency() {
        let h1 = hash_command("ls -l");
        let h2 = hash_command("  'ls -l'  ");
        let h3 = hash_command("\"ls -l\"");
        assert_eq!(h1, h2);
        assert_eq!(h1, h3);
    }
}
