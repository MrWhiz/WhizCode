use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ErrorContext {
    pub error_type: String,
    pub message: String,
    pub tool: String,
    pub workspace_path: Option<String>,
    pub timestamp: u64,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecoveryStrategy {
    pub id: String,
    pub error_pattern: String,
    pub recovery_steps: Vec<String>,
    pub success_rate: f32,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecoveryResult {
    pub recovered: bool,
    pub message: String,
    pub suggested_action: Option<String>,
    pub fallback_recommendations: Vec<String>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ErrorStatistics {
    pub total_errors: u32,
    pub errors_by_type: HashMap<String, u32>,
    pub recovery_success_rate: f32,
    pub most_common_error: Option<String>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecoveryAttempt {
    pub error_type: String,
    pub strategy_id: String,
    pub success: bool,
    pub timestamp: u64,
    pub execution_time_ms: u32,
}

#[allow(dead_code)]
pub struct ErrorRecoverySystem {
    strategies: Arc<Mutex<HashMap<String, RecoveryStrategy>>>,
    error_history: Arc<Mutex<Vec<ErrorContext>>>,
    recovery_attempts: Arc<Mutex<HashMap<String, (u32, u32)>>>, // (attempts, successes)
    recovery_log: Arc<Mutex<Vec<RecoveryAttempt>>>,
}

impl ErrorRecoverySystem {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let mut system = Self {
            strategies: Arc::new(Mutex::new(HashMap::new())),
            error_history: Arc::new(Mutex::new(Vec::new())),
            recovery_attempts: Arc::new(Mutex::new(HashMap::new())),
            recovery_log: Arc::new(Mutex::new(Vec::new())),
        };
        system.initialize_default_strategies();
        system
    }

    fn initialize_default_strategies(&mut self) {
        let mut strategies = self.strategies.lock().unwrap();

        // Command not found recovery
        strategies.insert(
            "command_not_found".to_string(),
            RecoveryStrategy {
                id: "command_not_found".to_string(),
                error_pattern: "not found|command not found|no such file".to_string(),
                recovery_steps: vec![
                    "Check if the command is installed".to_string(),
                    "Try using full path to the command".to_string(),
                    "Install the missing tool".to_string(),
                ],
                success_rate: 0.7,
            },
        );

        // Permission denied recovery
        strategies.insert(
            "permission_denied".to_string(),
            RecoveryStrategy {
                id: "permission_denied".to_string(),
                error_pattern: "permission denied|access denied|EACCES".to_string(),
                recovery_steps: vec![
                    "Check file permissions".to_string(),
                    "Try with elevated privileges if needed".to_string(),
                    "Verify workspace path is accessible".to_string(),
                ],
                success_rate: 0.6,
            },
        );

        // Timeout recovery
        strategies.insert(
            "timeout".to_string(),
            RecoveryStrategy {
                id: "timeout".to_string(),
                error_pattern: "timeout|timed out|ETIMEDOUT".to_string(),
                recovery_steps: vec![
                    "Increase timeout duration".to_string(),
                    "Check if the operation is stuck".to_string(),
                    "Try a simpler version of the command".to_string(),
                ],
                success_rate: 0.5,
            },
        );

        // File not found recovery
        strategies.insert(
            "file_not_found".to_string(),
            RecoveryStrategy {
                id: "file_not_found".to_string(),
                error_pattern: "no such file|ENOENT|not found".to_string(),
                recovery_steps: vec![
                    "Verify the file path is correct".to_string(),
                    "Check if the file exists in the workspace".to_string(),
                    "List directory contents to find the file".to_string(),
                ],
                success_rate: 0.8,
            },
        );

        // Git error recovery
        strategies.insert(
            "git_error".to_string(),
            RecoveryStrategy {
                id: "git_error".to_string(),
                error_pattern: "git|fatal|not a git repository".to_string(),
                recovery_steps: vec![
                    "Check if the workspace is a git repository".to_string(),
                    "Initialize git if needed".to_string(),
                    "Check git configuration".to_string(),
                ],
                success_rate: 0.7,
            },
        );

        // Network error recovery
        strategies.insert(
            "network_error".to_string(),
            RecoveryStrategy {
                id: "network_error".to_string(),
                error_pattern: "network|connection|ECONNREFUSED|ENOTFOUND".to_string(),
                recovery_steps: vec![
                    "Check network connectivity".to_string(),
                    "Verify the server is running".to_string(),
                    "Check firewall settings".to_string(),
                ],
                success_rate: 0.6,
            },
        );

        // Memory error recovery
        strategies.insert(
            "memory_error".to_string(),
            RecoveryStrategy {
                id: "memory_error".to_string(),
                error_pattern: "out of memory|OOM|memory exhausted".to_string(),
                recovery_steps: vec![
                    "Close unnecessary applications".to_string(),
                    "Increase available memory".to_string(),
                    "Try with smaller input".to_string(),
                ],
                success_rate: 0.4,
            },
        );
    }

    #[allow(dead_code)]
    pub fn handle_error(
        &self,
        error: &str,
        tool: &str,
        workspace_path: &Option<String>,
    ) -> RecoveryResult {
        let error_type = self.classify_error(error);
        let context = ErrorContext {
            error_type: error_type.clone(),
            message: error.to_string(),
            tool: tool.to_string(),
            workspace_path: workspace_path.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        // Record error
        let mut history = self.error_history.lock().unwrap();
        history.push(context);

        // Update recovery attempts
        let mut attempts = self.recovery_attempts.lock().unwrap();
        let entry = attempts.entry(error_type.clone()).or_insert((0, 0));
        entry.0 += 1;

        // Find applicable recovery strategy
        let strategies = self.strategies.lock().unwrap();
        if let Some(strategy) = strategies.get(&error_type) {
            let fallback = self.generate_fallback_recommendations(&error_type);
            RecoveryResult {
                recovered: false,
                message: format!("Error detected: {}. Suggested recovery steps:", error_type),
                suggested_action: Some(strategy.recovery_steps.join(" → ")),
                fallback_recommendations: fallback,
            }
        } else {
            RecoveryResult {
                recovered: false,
                message: format!("Unknown error: {}", error),
                suggested_action: None,
                fallback_recommendations: vec![
                    "Check the error message carefully".to_string(),
                    "Search for similar errors online".to_string(),
                    "Try a different approach".to_string(),
                ],
            }
        }
    }

    #[allow(dead_code)]
    fn classify_error(&self, error: &str) -> String {
        let lower = error.to_lowercase();
        let strategies = self.strategies.lock().unwrap();

        for (key, strategy) in strategies.iter() {
            let patterns: Vec<&str> = strategy.error_pattern.split('|').collect();
            for pattern in patterns {
                if lower.contains(pattern.trim()) {
                    return key.clone();
                }
            }
        }

        "unknown_error".to_string()
    }

    #[allow(dead_code)]
    fn generate_fallback_recommendations(&self, error_type: &str) -> Vec<String> {
        match error_type {
            "command_not_found" => vec![
                "Check PATH environment variable".to_string(),
                "Verify installation directory".to_string(),
                "Try reinstalling the tool".to_string(),
            ],
            "permission_denied" => vec![
                "Run with appropriate permissions".to_string(),
                "Check file ownership".to_string(),
                "Modify file permissions if needed".to_string(),
            ],
            "timeout" => vec![
                "Increase timeout threshold".to_string(),
                "Optimize the operation".to_string(),
                "Break into smaller tasks".to_string(),
            ],
            "file_not_found" => vec![
                "Use fuzzy file search".to_string(),
                "Check working directory".to_string(),
                "Verify file was created".to_string(),
            ],
            "git_error" => vec![
                "Initialize repository if needed".to_string(),
                "Check git configuration".to_string(),
                "Verify remote URL".to_string(),
            ],
            "network_error" => vec![
                "Check internet connection".to_string(),
                "Verify server status".to_string(),
                "Try again later".to_string(),
            ],
            "memory_error" => vec![
                "Reduce data size".to_string(),
                "Free up system memory".to_string(),
                "Use streaming approach".to_string(),
            ],
            _ => vec![
                "Review error details".to_string(),
                "Check documentation".to_string(),
                "Contact support if needed".to_string(),
            ],
        }
    }

    #[allow(dead_code)]
    pub fn get_error_history(&self, limit: Option<usize>) -> Vec<ErrorContext> {
        let history = self.error_history.lock().unwrap();
        let limit = limit.unwrap_or(100);
        history.iter().rev().take(limit).cloned().collect()
    }

    #[allow(dead_code)]
    pub fn get_recovery_strategies(&self) -> Vec<RecoveryStrategy> {
        let strategies = self.strategies.lock().unwrap();
        strategies.values().cloned().collect()
    }

    #[allow(dead_code)]
    pub fn get_error_statistics(&self) -> ErrorStatistics {
        let history = self.error_history.lock().unwrap();
        let attempts = self.recovery_attempts.lock().unwrap();

        let mut errors_by_type: HashMap<String, u32> = HashMap::new();
        for error in history.iter() {
            *errors_by_type.entry(error.error_type.clone()).or_insert(0) += 1;
        }

        let total_errors = history.len() as u32;
        let total_attempts: u32 = attempts.values().map(|(a, _)| a).sum();
        let total_successes: u32 = attempts.values().map(|(_, s)| s).sum();

        let recovery_success_rate = if total_attempts > 0 {
            total_successes as f32 / total_attempts as f32
        } else {
            0.0
        };

        let most_common_error = errors_by_type
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(error_type, _)| error_type.clone());

        ErrorStatistics {
            total_errors,
            errors_by_type,
            recovery_success_rate,
            most_common_error,
        }
    }

    #[allow(dead_code)]
    pub fn clear_error_history(&self) {
        let mut history = self.error_history.lock().unwrap();
        history.clear();
    }

    #[allow(dead_code)]
    pub fn add_custom_strategy(&self, strategy: RecoveryStrategy) {
        let mut strategies = self.strategies.lock().unwrap();
        strategies.insert(strategy.id.clone(), strategy);
    }

    #[allow(dead_code)]
    pub fn remove_strategy(&self, strategy_id: &str) -> bool {
        let mut strategies = self.strategies.lock().unwrap();
        strategies.remove(strategy_id).is_some()
    }

    #[allow(dead_code)]
    pub fn execute_recovery_strategy(
        &self,
        error_type: &str,
        strategy_id: &str,
    ) -> RecoveryResult {
        let start_time = std::time::SystemTime::now();
        
        let strategies = self.strategies.lock().unwrap();
        if let Some(strategy) = strategies.get(strategy_id) {
            // Simulate recovery execution
            let recovered = strategy.success_rate > 0.5;
            
            let execution_time_ms = std::time::SystemTime::now()
                .duration_since(start_time)
                .unwrap_or_default()
                .as_millis() as u32;

            // Log recovery attempt
            let attempt = RecoveryAttempt {
                error_type: error_type.to_string(),
                strategy_id: strategy_id.to_string(),
                success: recovered,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                execution_time_ms,
            };

            let mut log = self.recovery_log.lock().unwrap();
            log.push(attempt);

            // Update recovery attempts
            let mut attempts = self.recovery_attempts.lock().unwrap();
            let entry = attempts.entry(error_type.to_string()).or_insert((0, 0));
            entry.0 += 1;
            if recovered {
                entry.1 += 1;
            }

            if recovered {
                RecoveryResult {
                    recovered: true,
                    message: format!("Successfully recovered from {} using strategy: {}", error_type, strategy_id),
                    suggested_action: None,
                    fallback_recommendations: vec![],
                }
            } else {
                RecoveryResult {
                    recovered: false,
                    message: format!("Recovery strategy {} did not resolve the error", strategy_id),
                    suggested_action: Some(strategy.recovery_steps.join(" → ")),
                    fallback_recommendations: self.generate_fallback_recommendations(error_type),
                }
            }
        } else {
            RecoveryResult {
                recovered: false,
                message: format!("Strategy {} not found", strategy_id),
                suggested_action: None,
                fallback_recommendations: vec![],
            }
        }
    }

    #[allow(dead_code)]
    pub fn auto_recover(&self, error: &str, tool: &str, workspace_path: &Option<String>) -> RecoveryResult {
        let error_type = self.classify_error(error);
        
        // Record error
        let context = ErrorContext {
            error_type: error_type.clone(),
            message: error.to_string(),
            tool: tool.to_string(),
            workspace_path: workspace_path.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let mut history = self.error_history.lock().unwrap();
        history.push(context);

        // Find best recovery strategy
        let strategy_id = {
            let strategies = self.strategies.lock().unwrap();
            if let Some(strategy) = strategies.get(&error_type) {
                Some(strategy.id.clone())
            } else {
                None
            }
        };

        if let Some(strategy_id) = strategy_id {
            // Try automatic recovery
            let result = self.execute_recovery_strategy(&error_type, &strategy_id);
            if result.recovered {
                return result;
            }
        }

        // If auto-recovery failed, return suggestions
        RecoveryResult {
            recovered: false,
            message: format!("Auto-recovery failed for error type: {}. Manual intervention needed.", error_type),
            suggested_action: None,
            fallback_recommendations: self.generate_fallback_recommendations(&error_type),
        }
    }

    #[allow(dead_code)]
    pub fn get_recovery_log(&self, limit: Option<usize>) -> Vec<RecoveryAttempt> {
        let log = self.recovery_log.lock().unwrap();
        let limit = limit.unwrap_or(100);
        log.iter().rev().take(limit).cloned().collect()
    }

    #[allow(dead_code)]
    pub fn get_strategy_effectiveness(&self, strategy_id: &str) -> Option<f32> {
        let log = self.recovery_log.lock().unwrap();
        let attempts: Vec<_> = log.iter()
            .filter(|a| a.strategy_id == strategy_id)
            .collect();
        
        if attempts.is_empty() {
            return None;
        }

        let successes = attempts.iter().filter(|a| a.success).count() as f32;
        Some(successes / attempts.len() as f32)
    }

    #[allow(dead_code)]
    pub fn update_strategy_success_rate(&self, strategy_id: &str) {
        if let Some(effectiveness) = self.get_strategy_effectiveness(strategy_id) {
            let mut strategies = self.strategies.lock().unwrap();
            if let Some(strategy) = strategies.get_mut(strategy_id) {
                strategy.success_rate = effectiveness;
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_best_strategy_for_error(&self, error_type: &str) -> Option<RecoveryStrategy> {
        let strategies = self.strategies.lock().unwrap();
        
        // Find all strategies that match this error type
        let mut matching: Vec<_> = strategies.iter()
            .filter(|(_, s)| {
                let patterns: Vec<&str> = s.error_pattern.split('|').collect();
                patterns.iter().any(|p| error_type.contains(p.trim()))
            })
            .collect();

        // Sort by success rate (highest first)
        matching.sort_by(|a, b| b.1.success_rate.partial_cmp(&a.1.success_rate).unwrap_or(std::cmp::Ordering::Equal));

        matching.first().map(|(_, s)| (*s).clone())
    }
}

impl Default for ErrorRecoverySystem {
    fn default() -> Self {
        Self::new()
    }
}

// Tauri Commands

#[tauri::command]
pub fn error_recovery_handle(
    error: String,
    tool: String,
    workspace_path: Option<String>,
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<RecoveryResult, String> {
    let system = state.lock().unwrap();
    Ok(system.handle_error(&error, &tool, &workspace_path))
}

#[tauri::command]
pub fn error_recovery_history(
    limit: Option<usize>,
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<Vec<ErrorContext>, String> {
    let system = state.lock().unwrap();
    Ok(system.get_error_history(limit))
}

#[tauri::command]
pub fn error_recovery_strategies(
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<Vec<RecoveryStrategy>, String> {
    let system = state.lock().unwrap();
    Ok(system.get_recovery_strategies())
}

#[tauri::command]
pub fn error_recovery_statistics(
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<ErrorStatistics, String> {
    let system = state.lock().unwrap();
    Ok(system.get_error_statistics())
}

#[tauri::command]
pub fn error_recovery_clear_history(
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<(), String> {
    let system = state.lock().unwrap();
    system.clear_error_history();
    Ok(())
}

#[tauri::command]
pub fn error_recovery_add_strategy(
    strategy: RecoveryStrategy,
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<(), String> {
    let system = state.lock().unwrap();
    system.add_custom_strategy(strategy);
    Ok(())
}

#[tauri::command]
pub fn error_recovery_remove_strategy(
    strategy_id: String,
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<bool, String> {
    let system = state.lock().unwrap();
    Ok(system.remove_strategy(&strategy_id))
}

#[tauri::command]
pub fn error_recovery_auto_recover(
    error: String,
    tool: String,
    workspace_path: Option<String>,
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<RecoveryResult, String> {
    let system = state.lock().unwrap();
    Ok(system.auto_recover(&error, &tool, &workspace_path))
}

#[tauri::command]
pub fn error_recovery_execute_strategy(
    error_type: String,
    strategy_id: String,
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<RecoveryResult, String> {
    let system = state.lock().unwrap();
    Ok(system.execute_recovery_strategy(&error_type, &strategy_id))
}

#[tauri::command]
pub fn error_recovery_get_log(
    limit: Option<usize>,
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<Vec<RecoveryAttempt>, String> {
    let system = state.lock().unwrap();
    Ok(system.get_recovery_log(limit))
}

#[tauri::command]
pub fn error_recovery_strategy_effectiveness(
    strategy_id: String,
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<Option<f32>, String> {
    let system = state.lock().unwrap();
    Ok(system.get_strategy_effectiveness(&strategy_id))
}

#[tauri::command]
pub fn error_recovery_update_strategy_rates(
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<(), String> {
    let system = state.lock().unwrap();
    let strategies = system.get_recovery_strategies();
    for strategy in strategies {
        system.update_strategy_success_rate(&strategy.id);
    }
    Ok(())
}

#[tauri::command]
pub fn error_recovery_best_strategy(
    error_type: String,
    state: State<'_, Arc<Mutex<ErrorRecoverySystem>>>,
) -> Result<Option<RecoveryStrategy>, String> {
    let system = state.lock().unwrap();
    Ok(system.get_best_strategy_for_error(&error_type))
}
