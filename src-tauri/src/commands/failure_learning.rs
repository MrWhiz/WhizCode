use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub pattern: String,
    pub occurrences: u32,
    pub last_seen: i64,
    pub recovery_strategies: Vec<String>,
    pub success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStrategyStats {
    pub strategy: String,
    pub total_attempts: u32,
    pub successful_attempts: u32,
    pub success_rate: f32,
    pub last_used: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    pub error_type: String,
    pub tool_name: String,
    pub frequency: u32,
    pub suggested_fixes: Vec<String>,
    pub effectiveness: f32,
}

pub struct FailureLearningEngine {
    error_patterns: HashMap<String, ErrorPattern>,
    strategy_stats: HashMap<String, RecoveryStrategyStats>,
    failure_history: Vec<FailurePattern>,
}

impl FailureLearningEngine {
    pub fn new() -> Self {
        Self {
            error_patterns: HashMap::new(),
            strategy_stats: HashMap::new(),
            failure_history: Vec::new(),
        }
    }

    /// Record a failure and learn from it
    #[allow(dead_code)]
    pub fn record_failure(
        &mut self,
        error: &str,
        tool_name: &str,
        strategy_used: Option<&str>,
        success: bool,
    ) {
        // Extract error pattern
        let pattern = self.extract_pattern(error);

        // Update error pattern
        let entry = self
            .error_patterns
            .entry(pattern.clone())
            .or_insert_with(|| ErrorPattern {
                pattern: pattern.clone(),
                occurrences: 0,
                last_seen: chrono::Utc::now().timestamp(),
                recovery_strategies: Vec::new(),
                success_rate: 0.0,
            });

        entry.occurrences += 1;
        entry.last_seen = chrono::Utc::now().timestamp();

        // Update strategy stats if strategy was used
        if let Some(strat) = strategy_used {
            if !entry.recovery_strategies.contains(&strat.to_string()) {
                entry.recovery_strategies.push(strat.to_string());
            }

            let stats = self
                .strategy_stats
                .entry(strat.to_string())
                .or_insert_with(|| RecoveryStrategyStats {
                    strategy: strat.to_string(),
                    total_attempts: 0,
                    successful_attempts: 0,
                    success_rate: 0.0,
                    last_used: chrono::Utc::now().timestamp(),
                });

            stats.total_attempts += 1;
            if success {
                stats.successful_attempts += 1;
            }
            stats.success_rate = stats.successful_attempts as f32 / stats.total_attempts as f32;
            stats.last_used = chrono::Utc::now().timestamp();
        }

        // Update overall success rate for pattern
        if !entry.recovery_strategies.is_empty() {
            let total: u32 = self
                .strategy_stats
                .values()
                .filter(|s| entry.recovery_strategies.contains(&s.strategy))
                .map(|s| s.total_attempts)
                .sum();

            let successful: u32 = self
                .strategy_stats
                .values()
                .filter(|s| entry.recovery_strategies.contains(&s.strategy))
                .map(|s| s.successful_attempts)
                .sum();

            if total > 0 {
                entry.success_rate = successful as f32 / total as f32;
            }
        }

        // Add to failure history
        self.failure_history.push(FailurePattern {
            error_type: pattern,
            tool_name: tool_name.to_string(),
            frequency: entry.occurrences,
            suggested_fixes: entry.recovery_strategies.clone(),
            effectiveness: entry.success_rate,
        });
    }

    /// Extract pattern from error message
    fn extract_pattern(&self, error: &str) -> String {
        let lower = error.to_lowercase();

        if lower.contains("filename") || lower.contains("syntax") {
            "syntax_error".to_string()
        } else if lower.contains("not found") || lower.contains("no such") {
            "path_not_found".to_string()
        } else if lower.contains("permission") || lower.contains("denied") {
            "permission_denied".to_string()
        } else if lower.contains("command not found") {
            "command_not_found".to_string()
        } else if lower.contains("timeout") {
            "timeout".to_string()
        } else if lower.contains("connection") {
            "connection_error".to_string()
        } else {
            "unknown_error".to_string()
        }
    }

    /// Get best strategy for error pattern
    #[allow(dead_code)]
    pub fn get_best_strategy(&self, error: &str) -> Option<String> {
        let pattern = self.extract_pattern(error);

        self.error_patterns
            .get(&pattern)
            .and_then(|ep| {
                ep.recovery_strategies
                    .iter()
                    .max_by(|a, b| {
                        let a_stats = self.strategy_stats.get(*a).map(|s| s.success_rate).unwrap_or(0.0);
                        let b_stats = self.strategy_stats.get(*b).map(|s| s.success_rate).unwrap_or(0.0);
                        a_stats.partial_cmp(&b_stats).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .cloned()
            })
    }

    /// Get error patterns sorted by frequency
    #[allow(dead_code)]
    pub fn get_top_error_patterns(&self, limit: usize) -> Vec<&ErrorPattern> {
        let mut patterns: Vec<_> = self.error_patterns.values().collect();
        patterns.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));
        patterns.into_iter().take(limit).collect()
    }

    /// Get strategy effectiveness
    #[allow(dead_code)]
    pub fn get_strategy_effectiveness(&self, strategy: &str) -> Option<f32> {
        self.strategy_stats
            .get(strategy)
            .map(|s| s.success_rate)
    }

    /// Get all patterns
    #[allow(dead_code)]
    pub fn get_all_patterns(&self) -> Vec<&ErrorPattern> {
        self.error_patterns.values().collect()
    }

    /// Get failure history
    #[allow(dead_code)]
    pub fn get_failure_history(&self) -> &[FailurePattern] {
        &self.failure_history
    }

    /// Clear old data (older than days)
    #[allow(dead_code)]
    pub fn cleanup_old_data(&mut self, days: i64) {
        let cutoff = chrono::Utc::now().timestamp() - (days * 86400);

        self.error_patterns.retain(|_, v| v.last_seen > cutoff);
        self.strategy_stats.retain(|_, v| v.last_used > cutoff);
        self.failure_history.retain(|f| {
            self.error_patterns
                .get(&f.error_type)
                .map(|p| p.last_seen > cutoff)
                .unwrap_or(false)
        });
    }
}

impl Default for FailureLearningEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_extraction() {
        let engine = FailureLearningEngine::new();
        assert_eq!(
            engine.extract_pattern("filename, directory name, or volume label syntax is incorrect"),
            "syntax_error"
        );
        assert_eq!(
            engine.extract_pattern("cannot find path"),
            "path_not_found"
        );
        assert_eq!(
            engine.extract_pattern("permission denied"),
            "permission_denied"
        );
    }

    #[test]
    fn test_record_failure() {
        let mut engine = FailureLearningEngine::new();
        engine.record_failure(
            "filename syntax error",
            "run_command",
            Some("fix_powershell_syntax"),
            true,
        );

        assert_eq!(engine.error_patterns.len(), 1);
        assert_eq!(engine.strategy_stats.len(), 1);
    }

    #[test]
    fn test_strategy_effectiveness() {
        let mut engine = FailureLearningEngine::new();

        // Record 3 successes and 1 failure
        for _ in 0..3 {
            engine.record_failure(
                "syntax error",
                "run_command",
                Some("fix_syntax"),
                true,
            );
        }
        engine.record_failure(
            "syntax error",
            "run_command",
            Some("fix_syntax"),
            false,
        );

        let effectiveness = engine.get_strategy_effectiveness("fix_syntax");
        assert_eq!(effectiveness, Some(0.75));
    }
}
