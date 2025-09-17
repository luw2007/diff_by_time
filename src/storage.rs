use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    pub record: CommandRecord, // Command record
    pub stdout: String,        // Standard output
    pub stderr: String,        // Standard error output
}
