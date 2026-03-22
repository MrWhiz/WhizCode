use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningInsight {
    pub id: String,
    pub insight_type: String,
    pub description: String,
    pub confidence: f32,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationRule {
    pub id: String,
    pub name: String,
    pub condition: String,
    pub action: String,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningMetrics {
    pub total_interactions: u32,
    pub success_rate: f32,
    pub average_task_duration: f32,
    pub most_used_tools: Vec<String>,
    pub improvement_trend: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRecord {
    pub user_request: String,
    pub agent_response: String,
    pub tools_used: Vec<String>,
    pub success: bool,
    pub duration_ms: u32,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRecommendation {
    pub tool_name: String,
    pub confidence: f32,
    pub reason: String,
    pub success_rate: f32,
}

#[allow(dead_code)]
pub struct LearningSystem {
    insights: Arc<Mutex<Vec<LearningInsight>>>,
    rules: Arc<Mutex<Vec<AdaptationRule>>>,
    interactions: Arc<Mutex<Vec<InteractionRecord>>>,
    tool_effectiveness: Arc<Mutex<HashMap<String, (u32, u32)>>>, // (uses, successes)
}

impl LearningSystem {
    pub fn new() -> Self {
        Self {
            insights: Arc::new(Mutex::new(Vec::new())),
            rules: Arc::new(Mutex::new(Self::initialize_adaptation_rules())),
            interactions: Arc::new(Mutex::new(Vec::new())),
            tool_effectiveness: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn initialize_adaptation_rules() -> Vec<AdaptationRule> {
        vec![
            AdaptationRule {
                id: "rule_1".to_string(),
                name: "Prefer Efficient Tools".to_string(),
                condition: "tool_usage_frequency > 0.7".to_string(),
                action: "suggest_tool_in_similar_context".to_string(),
                priority: 1,
            },
            AdaptationRule {
                id: "rule_2".to_string(),
                name: "Learn from Errors".to_string(),
                condition: "error_pattern_detected".to_string(),
                action: "provide_alternative_approach".to_string(),
                priority: 2,
            },
            AdaptationRule {
                id: "rule_3".to_string(),
                name: "Optimize Task Duration".to_string(),
                condition: "task_duration > average * 1.5".to_string(),
                action: "suggest_faster_approach".to_string(),
                priority: 3,
            },
        ]
    }

    pub fn record_interaction(&self, record: InteractionRecord) {
        let mut interactions = self.interactions.lock().unwrap();
        interactions.push(record.clone());

        // Update tool effectiveness
        let mut effectiveness = self.tool_effectiveness.lock().unwrap();
        for tool in &record.tools_used {
            let entry = effectiveness.entry(tool.clone()).or_insert((0, 0));
            entry.0 += 1;
            if record.success {
                entry.1 += 1;
            }
        }
    }

    pub fn analyze_patterns(&self) -> Vec<LearningInsight> {
        let mut insights = Vec::new();

        // Analyze tool usage patterns
        let tool_usage = self.analyze_tool_usage();
        insights.extend(tool_usage);

        // Analyze success patterns
        let success_patterns = self.analyze_success_patterns();
        insights.extend(success_patterns);

        // Analyze error patterns
        let error_patterns = self.analyze_error_patterns();
        insights.extend(error_patterns);

        // Analyze performance trends
        let performance = self.analyze_performance_trends();
        insights.extend(performance);

        let mut stored_insights = self.insights.lock().unwrap();
        *stored_insights = insights.clone();
        insights
    }

    fn analyze_tool_usage(&self) -> Vec<LearningInsight> {
        let effectiveness = self.tool_effectiveness.lock().unwrap();
        let _interactions = self.interactions.lock().unwrap();

        effectiveness
            .iter()
            .map(|(tool, (uses, successes))| {
                let confidence = (*successes as f32) / (*uses as f32).max(1.0);
                LearningInsight {
                    id: format!("tool_usage_{}", tool),
                    insight_type: "tool_usage".to_string(),
                    description: format!(
                        "Tool '{}' used {} times with {:.1}% success rate",
                        tool,
                        uses,
                        confidence * 100.0
                    ),
                    confidence,
                    timestamp: chrono::Utc::now().timestamp(),
                }
            })
            .collect()
    }

    fn analyze_success_patterns(&self) -> Vec<LearningInsight> {
        let interactions = self.interactions.lock().unwrap();
        let successful: Vec<_> = interactions.iter().filter(|i| i.success).collect();
        let success_rate = (successful.len() as f32) / (interactions.len() as f32).max(1.0);

        vec![LearningInsight {
            id: "success_rate".to_string(),
            insight_type: "success_pattern".to_string(),
            description: format!("Overall success rate: {:.1}%", success_rate * 100.0),
            confidence: 1.0,
            timestamp: chrono::Utc::now().timestamp(),
        }]
    }

    fn analyze_error_patterns(&self) -> Vec<LearningInsight> {
        let interactions = self.interactions.lock().unwrap();
        let failed: Vec<_> = interactions.iter().filter(|i| !i.success).collect();

        if failed.is_empty() {
            return vec![];
        }

        vec![LearningInsight {
            id: "error_pattern".to_string(),
            insight_type: "error_pattern".to_string(),
            description: format!("{} failed interactions detected", failed.len()),
            confidence: 0.8,
            timestamp: chrono::Utc::now().timestamp(),
        }]
    }

    fn analyze_performance_trends(&self) -> Vec<LearningInsight> {
        let interactions = self.interactions.lock().unwrap();
        if interactions.len() < 2 {
            return vec![];
        }

        let first_half_avg = interactions[..interactions.len() / 2]
            .iter()
            .map(|i| i.duration_ms as f32)
            .sum::<f32>()
            / (interactions.len() / 2) as f32;

        let second_half_avg = interactions[interactions.len() / 2..]
            .iter()
            .map(|i| i.duration_ms as f32)
            .sum::<f32>()
            / (interactions.len() - interactions.len() / 2) as f32;

        let trend = (first_half_avg - second_half_avg) / first_half_avg.max(1.0);

        vec![LearningInsight {
            id: "performance_trend".to_string(),
            insight_type: "performance".to_string(),
            description: format!(
                "Performance trend: {:.1}% {}",
                trend.abs() * 100.0,
                if trend > 0.0 { "improvement" } else { "degradation" }
            ),
            confidence: 0.7,
            timestamp: chrono::Utc::now().timestamp(),
        }]
    }

    pub fn get_recommendations(&self, _task_type: &str) -> Vec<ToolRecommendation> {
        let effectiveness = self.tool_effectiveness.lock().unwrap();
        let mut recommendations: Vec<ToolRecommendation> = effectiveness
            .iter()
            .map(|(tool, (uses, successes))| {
                let success_rate = (*successes as f32) / (*uses as f32).max(1.0);
                ToolRecommendation {
                    tool_name: tool.clone(),
                    confidence: success_rate,
                    reason: format!("High success rate ({:.1}%)", success_rate * 100.0),
                    success_rate,
                }
            })
            .collect();

        recommendations.sort_by(|a, b| b.success_rate.partial_cmp(&a.success_rate).unwrap());
        recommendations.into_iter().take(5).collect()
    }

    pub fn calculate_metrics(&self) -> LearningMetrics {
        let interactions = self.interactions.lock().unwrap();
        let total_interactions = interactions.len() as u32;
        let successful = interactions.iter().filter(|i| i.success).count() as u32;
        let success_rate = if total_interactions > 0 {
            (successful as f32) / (total_interactions as f32)
        } else {
            0.0
        };

        let average_duration = if !interactions.is_empty() {
            interactions.iter().map(|i| i.duration_ms as f32).sum::<f32>()
                / interactions.len() as f32
        } else {
            0.0
        };

        let effectiveness = self.tool_effectiveness.lock().unwrap();
        let mut most_used_tools: Vec<_> = effectiveness.iter().collect();
        most_used_tools.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
        let most_used_tools: Vec<String> = most_used_tools
            .into_iter()
            .take(5)
            .map(|(tool, _)| tool.clone())
            .collect();

        // Calculate improvement trend
        let improvement_trend = if interactions.len() > 10 {
            let first_10_success = interactions[..10].iter().filter(|i| i.success).count() as f32 / 10.0;
            let last_10_success = interactions[interactions.len() - 10..]
                .iter()
                .filter(|i| i.success)
                .count() as f32
                / 10.0;
            last_10_success - first_10_success
        } else {
            0.0
        };

        LearningMetrics {
            total_interactions,
            success_rate,
            average_task_duration: average_duration,
            most_used_tools,
            improvement_trend,
        }
    }

    pub fn get_insights(&self) -> Vec<LearningInsight> {
        let insights = self.insights.lock().unwrap();
        insights.clone()
    }

    pub fn clear_history(&self) {
        let mut interactions = self.interactions.lock().unwrap();
        interactions.clear();
        let mut effectiveness = self.tool_effectiveness.lock().unwrap();
        effectiveness.clear();
    }
}

impl Default for LearningSystem {
    fn default() -> Self {
        Self::new()
    }
}

// Tauri Commands

#[tauri::command]
pub fn learning_analyze_patterns(
    state: State<'_, Arc<Mutex<LearningSystem>>>,
) -> Result<Vec<LearningInsight>, String> {
    let system = state.lock().unwrap();
    Ok(system.analyze_patterns())
}

#[tauri::command]
pub fn learning_get_recommendations(
    task_type: String,
    state: State<'_, Arc<Mutex<LearningSystem>>>,
) -> Result<Vec<ToolRecommendation>, String> {
    let system = state.lock().unwrap();
    Ok(system.get_recommendations(&task_type))
}

#[tauri::command]
pub fn learning_get_metrics(
    state: State<'_, Arc<Mutex<LearningSystem>>>,
) -> Result<LearningMetrics, String> {
    let system = state.lock().unwrap();
    Ok(system.calculate_metrics())
}

#[tauri::command]
pub fn learning_record_interaction(
    user_request: String,
    agent_response: String,
    tools_used: Vec<String>,
    success: bool,
    duration_ms: u32,
    state: State<'_, Arc<Mutex<LearningSystem>>>,
) -> Result<(), String> {
    let system = state.lock().unwrap();
    let record = InteractionRecord {
        user_request,
        agent_response,
        tools_used,
        success,
        duration_ms,
        timestamp: chrono::Utc::now().timestamp(),
    };
    system.record_interaction(record);
    Ok(())
}

#[tauri::command]
pub fn learning_get_insights(
    state: State<'_, Arc<Mutex<LearningSystem>>>,
) -> Result<Vec<LearningInsight>, String> {
    let system = state.lock().unwrap();
    Ok(system.get_insights())
}

#[tauri::command]
pub fn learning_clear_history(
    state: State<'_, Arc<Mutex<LearningSystem>>>,
) -> Result<(), String> {
    let system = state.lock().unwrap();
    system.clear_history();
    Ok(())
}
