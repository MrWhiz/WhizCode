use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningInsight {
    pub id: String,
    pub insight_type: String,
    pub description: String,
    pub confidence: f32,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct InteractionRecord {
    pub user_request: String,
    pub agent_response: String,
    pub tools_used: Vec<String>,
    pub success: bool,
    pub duration_ms: u32,
    pub timestamp: i64,
}

pub struct LearningSystem {
    insights: Vec<LearningInsight>,
    rules: Vec<AdaptationRule>,
    interactions: Vec<InteractionRecord>,
}

#[allow(dead_code)]
impl LearningSystem {
    pub fn new() -> Self {
        Self {
            insights: Vec::new(),
            rules: Self::initialize_adaptation_rules(),
            interactions: Vec::new(),
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

    pub fn record_interaction(&mut self, record: InteractionRecord) {
        self.interactions.push(record);
    }

    pub fn analyze_patterns(&mut self) -> Vec<LearningInsight> {
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

        self.insights = insights.clone();
        insights
    }

    fn analyze_tool_usage(&self) -> Vec<LearningInsight> {
        let mut tool_counts: HashMap<String, u32> = HashMap::new();

        for interaction in &self.interactions {
            for tool in &interaction.tools_used {
                *tool_counts.entry(tool.clone()).or_insert(0) += 1;
            }
        }

        tool_counts
            .into_iter()
            .map(|(tool, count)| {
                let confidence = (count as f32) / (self.interactions.len() as f32).max(1.0);
                LearningInsight {
                    id: format!("tool_usage_{}", tool),
                    insight_type: "tool_usage".to_string(),
                    description: format!("Tool '{}' used {} times", tool, count),
                    confidence,
                    timestamp: chrono::Local::now().timestamp(),
                }
            })
            .collect()
    }

    fn analyze_success_patterns(&self) -> Vec<LearningInsight> {
        let successful: Vec<_> = self.interactions.iter().filter(|i| i.success).collect();
        let success_rate = (successful.len() as f32) / (self.interactions.len() as f32).max(1.0);

        vec![LearningInsight {
            id: "success_rate".to_string(),
            insight_type: "success_pattern".to_string(),
            description: format!("Overall success rate: {:.1}%", success_rate * 100.0),
            confidence: 1.0,
            timestamp: chrono::Local::now().timestamp(),
        }]
    }

    fn analyze_error_patterns(&self) -> Vec<LearningInsight> {
        let failed: Vec<_> = self.interactions.iter().filter(|i| !i.success).collect();

        if failed.is_empty() {
            return vec![];
        }

        vec![LearningInsight {
            id: "error_pattern".to_string(),
            insight_type: "error_pattern".to_string(),
            description: format!("{} failed interactions detected", failed.len()),
            confidence: 0.8,
            timestamp: chrono::Local::now().timestamp(),
        }]
    }

    pub fn get_recommendations(&self, _task_type: &str) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Find relevant insights
        for insight in &self.insights {
            if insight.confidence > 0.6 {
                recommendations.push(format!(
                    "[{}] {}",
                    insight.insight_type, insight.description
                ));
            }
        }

        // Add rule-based recommendations
        for rule in &self.rules {
            if rule.priority <= 2 {
                recommendations.push(format!("Consider: {}", rule.action));
            }
        }

        recommendations
    }

    pub fn calculate_metrics(&self) -> LearningMetrics {
        let total_interactions = self.interactions.len() as u32;
        let successful = self.interactions.iter().filter(|i| i.success).count() as u32;
        let success_rate = if total_interactions > 0 {
            (successful as f32) / (total_interactions as f32)
        } else {
            0.0
        };

        let average_duration = if !self.interactions.is_empty() {
            self.interactions.iter().map(|i| i.duration_ms as f32).sum::<f32>()
                / self.interactions.len() as f32
        } else {
            0.0
        };

        let mut tool_counts: HashMap<String, u32> = HashMap::new();
        for interaction in &self.interactions {
            for tool in &interaction.tools_used {
                *tool_counts.entry(tool.clone()).or_insert(0) += 1;
            }
        }

        let mut most_used_tools: Vec<_> = tool_counts.into_iter().collect();
        most_used_tools.sort_by(|a, b| b.1.cmp(&a.1));
        let most_used_tools: Vec<String> = most_used_tools
            .into_iter()
            .take(5)
            .map(|(tool, _)| tool)
            .collect();

        LearningMetrics {
            total_interactions,
            success_rate,
            average_task_duration: average_duration,
            most_used_tools,
            improvement_trend: 0.0,
        }
    }
}

#[tauri::command]
pub async fn learning_analyze_patterns() -> Result<Vec<LearningInsight>> {
    Ok(vec![])
}

#[tauri::command]
pub async fn learning_get_recommendations(task_type: String) -> Result<Vec<String>> {
    Ok(vec![
        format!("Recommendation for task type: {}", task_type),
    ])
}

#[tauri::command]
pub async fn learning_get_metrics() -> Result<LearningMetrics> {
    Ok(LearningMetrics {
        total_interactions: 0,
        success_rate: 0.0,
        average_task_duration: 0.0,
        most_used_tools: vec![],
        improvement_trend: 0.0,
    })
}

#[tauri::command]
pub async fn learning_record_interaction(
    user_request: String,
    _agent_response: String,
    _tools_used: Vec<String>,
    success: bool,
    duration_ms: u32,
) -> Result<()> {
    eprintln!(
        "Recording interaction: request={}, success={}, duration={}ms",
        user_request, success, duration_ms
    );
    Ok(())
}
