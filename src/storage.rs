use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandRecord {
    pub command: String,        // Command executed
    pub command_hash: String,   // SHA256 hash of the command
    pub timestamp: DateTime<Utc>, // Execution timestamp
    pub working_dir: PathBuf,   // Working directory
    pub exit_code: i32,         // Exit code
    pub duration_ms: u64,        // Execution duration (milliseconds)
    pub record_id: String,      // Record unique identifier
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandExecution {
    pub record: CommandRecord, // Command record
    pub stdout: String,        // Standard output
    pub stderr: String,        // Standard error output
}