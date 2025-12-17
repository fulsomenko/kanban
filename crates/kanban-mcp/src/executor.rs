use serde::de::DeserializeOwned;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

use rmcp::model::ErrorData as McpError;

/// Response format from kanban CLI (matches crates/kanban-cli/src/output.rs)
#[derive(Debug, serde::Deserialize)]
pub struct CliResponse<T> {
    pub success: bool,
    #[allow(dead_code)]
    pub api_version: String,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Executor that spawns kanban CLI subprocess for each operation
pub struct CliExecutor {
    kanban_path: String,
    data_file: String,
}

impl CliExecutor {
    /// Create a new executor
    ///
    /// # Arguments
    /// * `data_file` - Path to the kanban.json data file
    pub fn new(data_file: String) -> Self {
        Self {
            kanban_path: "kanban".to_string(), // Assumes in PATH via Nix wrapper
            data_file,
        }
    }

    /// Execute a kanban CLI command and parse the JSON response
    pub async fn execute<T: DeserializeOwned>(&self, args: &[&str]) -> Result<T, McpError> {
        let output = Command::new(&self.kanban_path)
            .arg(&self.data_file) // First arg is always the data file
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Failed to execute kanban CLI: {}", e), None)
            })?;

        // Parse stdout as JSON
        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: CliResponse<T> = serde_json::from_str(&stdout).map_err(|e| {
            McpError::internal_error(
                format!(
                    "Failed to parse CLI response: {} (output: {})",
                    e, stdout
                ),
                None,
            )
        })?;

        if response.success {
            response.data.ok_or_else(|| {
                McpError::internal_error("Success response missing data".to_string(), None)
            })
        } else {
            let error_msg = response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());

            // Check for conflict error (retryable)
            if error_msg.contains("conflict") || error_msg.contains("modified by another") {
                Err(McpError::internal_error(
                    error_msg,
                    Some(serde_json::json!({"retryable": true})),
                ))
            } else {
                Err(McpError::internal_error(error_msg, None))
            }
        }
    }

    /// Execute with retry on conflict (exponential backoff)
    pub async fn execute_with_retry<T: DeserializeOwned>(
        &self,
        args: &[&str],
        max_attempts: u32,
    ) -> Result<T, McpError> {
        let mut attempt = 0;
        let mut delay_ms = 50u64;

        loop {
            attempt += 1;
            match self.execute(args).await {
                Ok(result) => {
                    if attempt > 1 {
                        tracing::info!("CLI command succeeded after {} attempts", attempt);
                    }
                    return Ok(result);
                }
                Err(e) if Self::is_retryable(&e) && attempt < max_attempts => {
                    tracing::warn!(
                        "CLI command failed (attempt {}/{}): {}. Retrying after {}ms...",
                        attempt,
                        max_attempts,
                        e.message,
                        delay_ms
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(1000); // Exponential backoff, max 1s
                }
                Err(e) => {
                    if attempt > 1 {
                        tracing::error!(
                            "CLI command failed after {} attempts: {}",
                            attempt,
                            e.message
                        );
                    }
                    return Err(e);
                }
            }
        }
    }

    fn is_retryable(error: &McpError) -> bool {
        error
            .data
            .as_ref()
            .and_then(|d| d.get("retryable"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
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
