use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryAttempt {
    pub attempt_number: u32,
    pub error: String,
    pub timestamp: i64,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryHistory {
    pub tool_name: String,
    pub attempts: Vec<RetryAttempt>,
    pub final_success: bool,
    pub total_duration_ms: u128,
}

pub struct RetryManager {
    #[allow(dead_code)]
    config: RetryConfig,
    history: HashMap<String, RetryHistory>,
}

impl RetryManager {
    pub fn new(config: RetryConfig) -> Self {
        Self {
            config,
            history: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn calculate_delay(&self, attempt: u32) -> u64 {
        let delay = (self.config.initial_delay_ms as f64
            * self.config.backoff_multiplier.powi(attempt as i32)) as u64;
        delay.min(self.config.max_delay_ms)
    }

    #[allow(dead_code)]
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.config.max_retries
    }

    #[allow(dead_code)]
    pub fn record_attempt(
        &mut self,
        tool_name: String,
        attempt: u32,
        error: String,
        delay_ms: u64,
    ) {
        let entry = self
            .history
            .entry(tool_name.clone())
            .or_insert_with(|| RetryHistory {
                tool_name: tool_name.clone(),
                attempts: Vec::new(),
                final_success: false,
                total_duration_ms: 0,
            });

        entry.attempts.push(RetryAttempt {
            attempt_number: attempt,
            error,
            timestamp: chrono::Utc::now().timestamp(),
            delay_ms,
        });
    }

    #[allow(dead_code)]
    pub fn record_success(&mut self, tool_name: &str, total_duration_ms: u128) {
        if let Some(entry) = self.history.get_mut(tool_name) {
            entry.final_success = true;
            entry.total_duration_ms = total_duration_ms;
        }
    }

    #[allow(dead_code)]
    pub fn get_history(&self, tool_name: &str) -> Option<&RetryHistory> {
        self.history.get(tool_name)
    }

    #[allow(dead_code)]
    pub fn get_all_history(&self) -> Vec<&RetryHistory> {
        self.history.values().collect()
    }
}

// Auto-recovery strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStrategy {
    pub error_pattern: String,
    pub fix_description: String,
    pub fix_function: String,
}

pub struct AutoRecoveryEngine {
    strategies: Vec<RecoveryStrategy>,
}

impl AutoRecoveryEngine {
    pub fn new() -> Self {
        let mut strategies = Vec::new();

        // PowerShell syntax errors
        strategies.push(RecoveryStrategy {
            error_pattern: "filename, directory name, or volume label syntax is incorrect"
                .to_string(),
            fix_description: "Replace && with ; for PowerShell".to_string(),
            fix_function: "fix_powershell_syntax".to_string(),
        });

        // Path not found
        strategies.push(RecoveryStrategy {
            error_pattern: "cannot find path".to_string(),
            fix_description: "Create parent directories first".to_string(),
            fix_function: "create_parent_dirs".to_string(),
        });

        // Permission denied
        strategies.push(RecoveryStrategy {
            error_pattern: "permission denied".to_string(),
            fix_description: "Check file permissions".to_string(),
            fix_function: "check_permissions".to_string(),
        });

        // Command not found
        strategies.push(RecoveryStrategy {
            error_pattern: "command not found".to_string(),
            fix_description: "Install required tool or check PATH".to_string(),
            fix_function: "check_tool_installed".to_string(),
        });

        // File not found
        strategies.push(RecoveryStrategy {
            error_pattern: "no such file or directory".to_string(),
            fix_description: "Verify file path exists".to_string(),
            fix_function: "verify_file_exists".to_string(),
        });

        Self { strategies }
    }

    #[allow(dead_code)]
    pub fn find_strategy(&self, error: &str) -> Option<&RecoveryStrategy> {
        self.strategies
            .iter()
            .find(|s| error.to_lowercase().contains(&s.error_pattern.to_lowercase()))
    }

    #[allow(dead_code)]
    pub fn get_all_strategies(&self) -> &[RecoveryStrategy] {
        &self.strategies
    }
}

impl Default for AutoRecoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}
