use kanban_core::KanbanError;
use kanban_core::KanbanResult;
use serde::de::DeserializeOwned;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, serde::Deserialize)]
pub struct CliResponse<T> {
    pub success: bool,
    #[allow(dead_code)]
    pub api_version: String,
    pub data: Option<T>,
    pub error: Option<String>,
}

pub struct SyncExecutor {
    kanban_path: String,
    data_file: String,
}

impl SyncExecutor {
    const DEFAULT_RETRY_COUNT: u32 = 3;
    const COMMAND_TIMEOUT_SECS: u64 = 30;

    pub fn new(data_file: String) -> Self {
        Self {
            kanban_path: "kanban".to_string(),
            data_file,
        }
    }

    pub fn with_kanban_path(mut self, path: String) -> Self {
        self.kanban_path = path;
        self
    }

    fn run_with_timeout(&self, args: &[&str]) -> KanbanResult<std::process::Output> {
        let mut child = Command::new(&self.kanban_path)
            .arg(&self.data_file)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| KanbanError::Internal(format!("Failed to execute kanban CLI: {}", e)))?;

        let start = Instant::now();
        let timeout = Duration::from_secs(Self::COMMAND_TIMEOUT_SECS);

        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    return child.wait_with_output().map_err(|e| {
                        KanbanError::Internal(format!("Failed to read CLI output: {}", e))
                    });
                }
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(KanbanError::Internal(format!(
                            "CLI command timed out after {}s",
                            Self::COMMAND_TIMEOUT_SECS
                        )));
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    return Err(KanbanError::Internal(format!(
                        "Failed to check CLI process status: {}",
                        e
                    )));
                }
            }
        }
    }

    pub fn execute<T: DeserializeOwned>(&self, args: &[&str]) -> KanbanResult<T> {
        let output = self.run_with_timeout(args)?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let response: CliResponse<T> = serde_json::from_str(&stdout)
            .or_else(|_| serde_json::from_str(&stderr))
            .or_else(|_| {
                let first_line = stderr.lines().next().unwrap_or("");
                serde_json::from_str(first_line)
            })
            .map_err(|e| {
                if stderr.is_empty() {
                    KanbanError::Serialization(format!(
                        "Failed to parse CLI response: {} (stdout: {})",
                        e, stdout
                    ))
                } else {
                    KanbanError::Serialization(format!(
                        "Failed to parse CLI response: {} (stdout: {}, stderr: {})",
                        e, stdout, stderr
                    ))
                }
            })?;

        if response.success {
            response
                .data
                .ok_or_else(|| KanbanError::Internal("Success response missing data".to_string()))
        } else {
            let error_msg = response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());

            if error_msg.contains("conflict") || error_msg.contains("modified by another") {
                Err(KanbanError::ConflictDetected {
                    path: self.data_file.clone(),
                    source: None,
                })
            } else if error_msg.contains("not found") {
                Err(KanbanError::NotFound(error_msg))
            } else {
                Err(KanbanError::Internal(error_msg))
            }
        }
    }

    pub fn execute_with_retry<T: DeserializeOwned>(&self, args: &[&str]) -> KanbanResult<T> {
        let max_attempts = Self::DEFAULT_RETRY_COUNT;
        let mut attempt = 0;
        let mut delay_ms = 50u64;

        loop {
            attempt += 1;
            match self.execute(args) {
                Ok(result) => return Ok(result),
                Err(KanbanError::ConflictDetected { .. }) if attempt < max_attempts => {
                    tracing::warn!(
                        "CLI command failed (attempt {}/{}): conflict. Retrying after {}ms...",
                        attempt,
                        max_attempts,
                        delay_ms
                    );
                    thread::sleep(Duration::from_millis(delay_ms));
                    delay_ms = (delay_ms * 2).min(1000);
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn execute_raw_stdout(&self, args: &[&str]) -> KanbanResult<String> {
        let output = self.run_with_timeout(args)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KanbanError::Internal(format!(
                "CLI command failed: {}",
                stderr
            )));
        }

        String::from_utf8(output.stdout)
            .map_err(|e| KanbanError::Internal(format!("Invalid UTF-8 in CLI output: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_success_response() {
        let json = r#"{"success":true,"api_version":"0.1.0","data":{"id":"123","name":"Test"}}"#;
        let response: CliResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert!(response.data.is_some());
        assert_eq!(response.data.unwrap()["id"], "123");
    }

    #[test]
    fn test_parse_error_response() {
        let json = r#"{"success":false,"api_version":"0.1.0","error":"Not found"}"#;
        let response: CliResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert_eq!(response.error, Some("Not found".to_string()));
    }

    #[test]
    fn test_parse_list_response() {
        let json = r#"{"success":true,"api_version":"0.1.0","data":{"items":[{"id":"1"},{"id":"2"}],"count":2}}"#;
        let response: CliResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(response.success);
        let data = response.data.unwrap();
        assert_eq!(data["count"], 2);
    }
}
