use crate::storage::{self, CommandExecution, CommandRecord};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Instant;

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
        let formatted_command = storage::format_command(command);
        let command_hash = storage::hash_command(&formatted_command);
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
}
