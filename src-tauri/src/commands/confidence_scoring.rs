use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Confidence level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ConfidenceLevel {
    VeryLow,    // 0-20%
    Low,        // 20-40%
    Medium,     // 40-60%
    High,       // 60-80%
    VeryHigh,   // 80-100%
}

impl ConfidenceLevel {
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s < 0.2 => ConfidenceLevel::VeryLow,
            s if s < 0.4 => ConfidenceLevel::Low,
            s if s < 0.6 => ConfidenceLevel::Medium,
            s if s < 0.8 => ConfidenceLevel::High,
            _ => ConfidenceLevel::VeryHigh,
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            ConfidenceLevel::VeryLow => "❌",
            ConfidenceLevel::Low => "⚠️",
            ConfidenceLevel::Medium => "❓",
            ConfidenceLevel::High => "✅",
            ConfidenceLevel::VeryHigh => "✅✅",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ConfidenceLevel::VeryLow => "Very Low",
            ConfidenceLevel::Low => "Low",
            ConfidenceLevel::Medium => "Medium",
            ConfidenceLevel::High => "High",
            ConfidenceLevel::VeryHigh => "Very High",
        }
    }
}

/// Confidence score with reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    pub score: f32,
    pub level: String,
    pub emoji: String,
    pub reasons_for: Vec<String>,
    pub reasons_against: Vec<String>,
    pub recommendation: String,
}

/// Decision context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    pub tool: String,
    pub args: serde_json::Value,
    pub task_type: String,
    pub previous_success_rate: f32,
    pub similar_tasks_success_rate: f32,
    pub context_clarity: f32, // 0-1, how clear is the context
    pub is_new_tool: bool,
    pub has_error_history: bool,
}

/// Confidence Scoring Engine
pub struct ConfidenceScoringEngine {
    tool_success_rates: HashMap<String, f32>,
    task_type_success_rates: HashMap<String, f32>,
}

#[allow(dead_code)]
impl ConfidenceScoringEngine {
    pub fn new() -> Self {
        Self {
            tool_success_rates: HashMap::new(),
            task_type_success_rates: HashMap::new(),
        }
    }

    /// Score a decision
    pub fn score_decision(&self, context: &DecisionContext) -> ConfidenceScore {
        let mut score = 0.5; // Base confidence
        let mut reasons_for = Vec::new();
        let mut reasons_against = Vec::new();

        // Factor 1: Tool success rate
        let tool_success = self.tool_success_rates.get(&context.tool).copied().unwrap_or(0.5);
        score += (tool_success - 0.5) * 0.2;
        if tool_success > 0.7 {
            reasons_for.push(format!("Tool '{}' has high success rate ({:.0}%)", context.tool, tool_success * 100.0));
        } else if tool_success < 0.3 {
            reasons_against.push(format!("Tool '{}' has low success rate ({:.0}%)", context.tool, tool_success * 100.0));
        }

        // Factor 2: Task type success rate
        let task_success = self.task_type_success_rates.get(&context.task_type).copied().unwrap_or(0.5);
        score += (task_success - 0.5) * 0.15;
        if task_success > 0.7 {
            reasons_for.push(format!("Similar tasks have high success rate ({:.0}%)", task_success * 100.0));
        } else if task_success < 0.3 {
            reasons_against.push(format!("Similar tasks have low success rate ({:.0}%)", task_success * 100.0));
        }

        // Factor 3: Context clarity
        score += context.context_clarity * 0.15;
        if context.context_clarity > 0.8 {
            reasons_for.push("Context is clear and well-defined".to_string());
        } else if context.context_clarity < 0.3 {
            reasons_against.push("Context is ambiguous or unclear".to_string());
        }

        // Factor 4: Previous success
        score += context.previous_success_rate * 0.15;
        if context.previous_success_rate > 0.7 {
            reasons_for.push("Previous similar actions succeeded".to_string());
        } else if context.previous_success_rate < 0.3 {
            reasons_against.push("Previous similar actions failed".to_string());
        }

        // Factor 5: New tool penalty
        if context.is_new_tool {
            score -= 0.15;
            reasons_against.push("This is a new tool without prior success history".to_string());
        } else {
            reasons_for.push("Tool has been used successfully before".to_string());
        }

        // Factor 6: Error history
        if context.has_error_history {
            score -= 0.1;
            reasons_against.push("This tool has error history".to_string());
        } else {
            reasons_for.push("No error history with this tool".to_string());
        }

        // Clamp score to 0-1
        score = score.clamp(0.0, 1.0);

        let level = ConfidenceLevel::from_score(score);
        let recommendation = self.generate_recommendation(score, &reasons_against);

        ConfidenceScore {
            score,
            level: level.label().to_string(),
            emoji: level.emoji().to_string(),
            reasons_for,
            reasons_against,
            recommendation,
        }
    }

    /// Generate recommendation based on confidence
    fn generate_recommendation(&self, score: f32, reasons_against: &[String]) -> String {
        match score {
            s if s >= 0.8 => "Proceed with confidence".to_string(),
            s if s >= 0.6 => "Proceed, but monitor results".to_string(),
            s if s >= 0.4 => {
                if reasons_against.is_empty() {
                    "Proceed cautiously".to_string()
                } else {
                    format!("Consider alternatives: {}", reasons_against.join(", "))
                }
            }
            s if s >= 0.2 => {
                format!("Low confidence. Suggested alternatives: {}", reasons_against.join(", "))
            }
            _ => "Very low confidence. Strongly recommend reconsidering this approach".to_string(),
        }
    }

    /// Record tool success
    pub fn record_tool_success(&mut self, tool: &str, success: bool) {
        let current = self.tool_success_rates.get(tool).copied().unwrap_or(0.5);
        let new_rate = if success {
            (current * 0.9) + (1.0 * 0.1)
        } else {
            (current * 0.9) + (0.0 * 0.1)
        };
        self.tool_success_rates.insert(tool.to_string(), new_rate);
    }

    /// Record task type success
    pub fn record_task_success(&mut self, task_type: &str, success: bool) {
        let current = self.task_type_success_rates.get(task_type).copied().unwrap_or(0.5);
        let new_rate = if success {
            (current * 0.9) + (1.0 * 0.1)
        } else {
            (current * 0.9) + (0.0 * 0.1)
        };
        self.task_type_success_rates.insert(task_type.to_string(), new_rate);
    }

    /// Get tool success rate
    pub fn get_tool_success_rate(&self, tool: &str) -> f32 {
        self.tool_success_rates.get(tool).copied().unwrap_or(0.5)
    }

    /// Get task type success rate
    pub fn get_task_success_rate(&self, task_type: &str) -> f32 {
        self.task_type_success_rates.get(task_type).copied().unwrap_or(0.5)
    }
}

/// Format confidence for display
#[allow(dead_code)]
pub fn format_confidence_for_display(confidence: &ConfidenceScore) -> String {
    format!(
        "{} {} ({:.0}%)\n\n\
         REASONS FOR:\n{}\n\n\
         REASONS AGAINST:\n{}\n\n\
         RECOMMENDATION: {}",
        confidence.emoji,
        confidence.level,
        confidence.score * 100.0,
        if confidence.reasons_for.is_empty() {
            "None".to_string()
        } else {
            confidence
                .reasons_for
                .iter()
                .map(|r| format!("✓ {}", r))
                .collect::<Vec<_>>()
                .join("\n")
        },
        if confidence.reasons_against.is_empty() {
            "None".to_string()
        } else {
            confidence
                .reasons_against
                .iter()
                .map(|r| format!("✗ {}", r))
                .collect::<Vec<_>>()
                .join("\n")
        },
        confidence.recommendation
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_level_from_score() {
        assert_eq!(ConfidenceLevel::from_score(0.1), ConfidenceLevel::VeryLow);
        assert_eq!(ConfidenceLevel::from_score(0.3), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_score(0.5), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_score(0.7), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_score(0.9), ConfidenceLevel::VeryHigh);
    }

    #[test]
    fn test_score_decision() {
        let engine = ConfidenceScoringEngine::new();
        let context = DecisionContext {
            tool: "read_file".to_string(),
            args: serde_json::json!({}),
            task_type: "feature".to_string(),
            previous_success_rate: 0.8,
            similar_tasks_success_rate: 0.7,
            context_clarity: 0.9,
            is_new_tool: false,
            has_error_history: false,
        };

        let score = engine.score_decision(&context);
        assert!(score.score > 0.5);
        assert!(!score.reasons_for.is_empty());
    }

    #[test]
    fn test_record_tool_success() {
        let mut engine = ConfidenceScoringEngine::new();
        engine.record_tool_success("read_file", true);
        engine.record_tool_success("read_file", true);
        engine.record_tool_success("read_file", false);

        let rate = engine.get_tool_success_rate("read_file");
        assert!(rate > 0.5);
    }

    #[test]
    fn test_format_confidence() {
        let confidence = ConfidenceScore {
            score: 0.85,
            level: "High".to_string(),
            emoji: "✅".to_string(),
            reasons_for: vec!["Reason 1".to_string()],
            reasons_against: vec![],
            recommendation: "Proceed".to_string(),
        };

        let formatted = format_confidence_for_display(&confidence);
        assert!(formatted.contains("85%"));
        assert!(formatted.contains("High"));
    }
}
