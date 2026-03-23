use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::error::{Result, ApiError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetrics {
    pub name: String,
    pub category: String,
    pub success_count: u32,
    pub failure_count: u32,
    pub total_executions: u32,
    pub success_rate: f32,
    pub avg_execution_time_ms: u32,
    pub min_execution_time_ms: u32,
    pub max_execution_time_ms: u32,
    pub failure_modes: Vec<String>,
    pub prerequisites: Vec<String>,
    pub post_conditions: Vec<String>,
    pub cost_estimate: u32,  // tokens
    pub last_used: u64,
    pub reliability_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub tool_name: String,
    pub timestamp: u64,
    pub duration_ms: u32,
    pub success: bool,
    pub error_message: Option<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRanking {
    pub tool_name: String,
    pub success_rate: f32,
    pub reliability_score: f32,
    pub avg_time_ms: u32,
    pub rank: u32,
    pub recommendation: String,
}

pub struct ToolMetricsSystem {
    metrics: Arc<RwLock<HashMap<String, ToolMetrics>>>,
    execution_history: Arc<RwLock<Vec<ToolExecution>>>,
}

impl ToolMetricsSystem {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn record_execution(&self, execution: ToolExecution) -> Result<()> {
        let mut metrics = self.metrics.write();
        let mut history = self.execution_history.write();

        // Update metrics
        let metric = metrics
            .entry(execution.tool_name.clone())
            .or_insert_with(|| ToolMetrics {
                name: execution.tool_name.clone(),
                category: "general".to_string(),
                success_count: 0,
                failure_count: 0,
                total_executions: 0,
                success_rate: 0.0,
                avg_execution_time_ms: 0,
                min_execution_time_ms: u32::MAX,
                max_execution_time_ms: 0,
                failure_modes: Vec::new(),
                prerequisites: Vec::new(),
                post_conditions: Vec::new(),
                cost_estimate: 0,
                last_used: 0,
                reliability_score: 0.0,
            });

        // Update counts
        if execution.success {
            metric.success_count += 1;
        } else {
            metric.failure_count += 1;
            if let Some(error) = &execution.error_message {
                if !metric.failure_modes.contains(error) {
                    metric.failure_modes.push(error.clone());
                }
            }
        }

        metric.total_executions += 1;
        metric.success_rate = metric.success_count as f32 / metric.total_executions as f32;

        // Update timing
        metric.avg_execution_time_ms = ((metric.avg_execution_time_ms as u64 * (metric.total_executions - 1) as u64
            + execution.duration_ms as u64)
            / metric.total_executions as u64) as u32;
        metric.min_execution_time_ms = metric.min_execution_time_ms.min(execution.duration_ms);
        metric.max_execution_time_ms = metric.max_execution_time_ms.max(execution.duration_ms);

        // Update reliability score
        metric.reliability_score = self.calculate_reliability_score(metric);
        metric.last_used = execution.timestamp;

        // Add to history
        history.push(execution);

        // Keep history size manageable (last 1000 executions)
        if history.len() > 1000 {
            history.remove(0);
        }

        Ok(())
    }

    fn calculate_reliability_score(&self, metric: &ToolMetrics) -> f32 {
        // Combine success rate (70%) and speed (30%)
        let success_score = metric.success_rate * 0.7;
        
        let speed_score = if metric.avg_execution_time_ms < 100 {
            0.3
        } else if metric.avg_execution_time_ms < 500 {
            0.25
        } else if metric.avg_execution_time_ms < 2000 {
            0.2
        } else {
            0.1
        };

        (success_score + speed_score).min(1.0).max(0.0)
    }

    pub fn get_tool_metrics(&self, tool_name: &str) -> Option<ToolMetrics> {
        self.metrics.read().get(tool_name).cloned()
    }

    pub fn get_all_metrics(&self) -> Vec<ToolMetrics> {
        self.metrics.read().values().cloned().collect()
    }

    pub fn rank_tools(&self) -> Vec<ToolRanking> {
        let mut rankings: Vec<ToolRanking> = self
            .metrics
            .read()
            .values()
            .map(|m| {
                let recommendation = if m.success_rate >= 0.9 {
                    "Highly recommended".to_string()
                } else if m.success_rate >= 0.7 {
                    "Recommended".to_string()
                } else if m.success_rate >= 0.5 {
                    "Use with caution".to_string()
                } else {
                    "Not recommended".to_string()
                };

                ToolRanking {
                    tool_name: m.name.clone(),
                    success_rate: m.success_rate,
                    reliability_score: m.reliability_score,
                    avg_time_ms: m.avg_execution_time_ms,
                    rank: 0,
                    recommendation,
                }
            })
            .collect();

        // Sort by reliability score (descending)
        rankings.sort_by(|a, b| b.reliability_score.partial_cmp(&a.reliability_score).unwrap());

        // Assign ranks
        for (i, ranking) in rankings.iter_mut().enumerate() {
            ranking.rank = (i + 1) as u32;
        }

        rankings
    }

    pub fn get_tool_recommendations(&self, _task_type: &str) -> Vec<ToolRanking> {
        let mut rankings = self.rank_tools();
        
        // Filter and limit to top 5 recommendations
        rankings.truncate(5);
        rankings
    }

    pub fn get_execution_history(&self, limit: Option<usize>) -> Vec<ToolExecution> {
        let history = self.execution_history.read();
        let limit = limit.unwrap_or(100);
        
        history
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn get_failure_analysis(&self, tool_name: &str) -> Option<serde_json::Value> {
        let metrics = self.get_tool_metrics(tool_name)?;
        let history = self.execution_history.read();

        let failures: Vec<_> = history
            .iter()
            .filter(|e| e.tool_name == tool_name && !e.success)
            .collect();

        let failure_rate = if metrics.total_executions > 0 {
            metrics.failure_count as f32 / metrics.total_executions as f32
        } else {
            0.0
        };

        Some(serde_json::json!({
            "tool_name": tool_name,
            "total_failures": metrics.failure_count,
            "failure_rate": failure_rate,
            "failure_modes": metrics.failure_modes,
            "recent_failures": failures.iter().map(|f| serde_json::json!({
                "timestamp": f.timestamp,
                "error": f.error_message,
                "duration_ms": f.duration_ms,
            })).collect::<Vec<_>>(),
        }))
    }

    pub fn clear_metrics(&self) -> Result<()> {
        self.metrics.write().clear();
        self.execution_history.write().clear();
        Ok(())
    }

    pub fn get_statistics(&self) -> serde_json::Value {
        let metrics = self.metrics.read();
        let history = self.execution_history.read();

        let total_executions: u32 = metrics.values().map(|m| m.total_executions).sum();
        let total_successes: u32 = metrics.values().map(|m| m.success_count).sum();
        let avg_success_rate = if total_executions > 0 {
            total_successes as f32 / total_executions as f32
        } else {
            0.0
        };

        let avg_execution_time: u32 = if !metrics.is_empty() {
            metrics.values().map(|m| m.avg_execution_time_ms).sum::<u32>() / metrics.len() as u32
        } else {
            0
        };

        serde_json::json!({
            "total_tools": metrics.len(),
            "total_executions": total_executions,
            "total_successes": total_successes,
            "total_failures": total_executions - total_successes,
            "average_success_rate": avg_success_rate,
            "average_execution_time_ms": avg_execution_time,
            "history_size": history.len(),
        })
    }
}

impl Default for ToolMetricsSystem {
    fn default() -> Self {
        Self::new()
    }
}

// Tauri Commands

#[tauri::command]
pub async fn tool_metrics_record_execution(
    tool_name: String,
    duration_ms: u32,
    success: bool,
    error_message: Option<String>,
    input_tokens: u32,
    output_tokens: u32,
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolMetricsSystem>>>,
) -> Result<()> {
    let system = state.lock().map_err(|e| ApiError::from(e.to_string()))?;
    system.record_execution(ToolExecution {
        tool_name,
        timestamp: chrono::Local::now().timestamp() as u64,
        duration_ms,
        success,
        error_message,
        input_tokens,
        output_tokens,
    })
}

#[tauri::command]
pub async fn tool_metrics_get_metrics(
    tool_name: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolMetricsSystem>>>,
) -> Result<Option<ToolMetrics>> {
    let system = state.lock().map_err(|e| ApiError::from(e.to_string()))?;
    Ok(system.get_tool_metrics(&tool_name))
}

#[tauri::command]
pub async fn tool_metrics_get_all(
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolMetricsSystem>>>,
) -> Result<Vec<ToolMetrics>> {
    let system = state.lock().map_err(|e| ApiError::from(e.to_string()))?;
    Ok(system.get_all_metrics())
}

#[tauri::command]
pub async fn tool_metrics_rank_tools(
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolMetricsSystem>>>,
) -> Result<Vec<ToolRanking>> {
    let system = state.lock().map_err(|e| ApiError::from(e.to_string()))?;
    Ok(system.rank_tools())
}

#[tauri::command]
pub async fn tool_metrics_get_recommendations(
    task_type: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolMetricsSystem>>>,
) -> Result<Vec<ToolRanking>> {
    let system = state.lock().map_err(|e| ApiError::from(e.to_string()))?;
    Ok(system.get_tool_recommendations(&task_type))
}

#[tauri::command]
pub async fn tool_metrics_get_history(
    limit: Option<usize>,
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolMetricsSystem>>>,
) -> Result<Vec<ToolExecution>> {
    let system = state.lock().map_err(|e| ApiError::from(e.to_string()))?;
    Ok(system.get_execution_history(limit))
}

#[tauri::command]
pub async fn tool_metrics_get_failure_analysis(
    tool_name: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolMetricsSystem>>>,
) -> Result<Option<serde_json::Value>> {
    let system = state.lock().map_err(|e| ApiError::from(e.to_string()))?;
    Ok(system.get_failure_analysis(&tool_name))
}

#[tauri::command]
pub async fn tool_metrics_get_statistics(
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolMetricsSystem>>>,
) -> Result<serde_json::Value> {
    let system = state.lock().map_err(|e| ApiError::from(e.to_string()))?;
    Ok(system.get_statistics())
}

#[tauri::command]
pub async fn tool_metrics_clear(
    state: tauri::State<'_, Arc<std::sync::Mutex<ToolMetricsSystem>>>,
) -> Result<()> {
    let system = state.lock().map_err(|e| ApiError::from(e.to_string()))?;
    system.clear_metrics()
}
