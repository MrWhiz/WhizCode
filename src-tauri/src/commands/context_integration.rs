/// Phase 4: Context Integration
/// 
/// This module implements intelligent context integration by:
/// 1. Injecting learned patterns into task prompts
/// 2. Activating knowledge distillation
/// 3. Scoring context relevance
/// 4. Suggesting patterns proactively
///
/// This makes WhizCode learn from past experiences and apply them to new tasks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a learned pattern that can be applied to tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub pattern_id: String,
    pub pattern_type: String, // "code", "workflow", "error_recovery", "optimization"
    pub description: String,
    pub context: String,
    pub language: String,
    pub success_rate: f32,
    pub times_used: u32,
    pub last_used: u64,
    pub effectiveness_score: f32,
}

/// Represents a distilled piece of knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistilledKnowledge {
    pub knowledge_id: String,
    pub title: String,
    pub content: String,
    pub category: String, // "best_practice", "common_error", "optimization", "pattern"
    pub relevance_score: f32,
    pub confidence: f32,
    pub source_patterns: Vec<String>,
}

/// Represents context relevance scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRelevance {
    pub context_id: String,
    pub relevance_score: f32, // 0-1
    pub matching_patterns: Vec<String>,
    pub suggested_approaches: Vec<String>,
    pub confidence: f32,
}

/// Represents a proactive suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveSuggestion {
    pub suggestion_id: String,
    pub suggestion_type: String, // "pattern", "optimization", "error_prevention"
    pub title: String,
    pub description: String,
    pub confidence: f32,
    pub estimated_benefit: String, // "high", "medium", "low"
    pub action: String,
}

/// Context Integration Engine
pub struct ContextIntegrationEngine {
    learned_patterns: Vec<LearnedPattern>,
    distilled_knowledge: Vec<DistilledKnowledge>,
    pattern_cache: HashMap<String, Vec<LearnedPattern>>,
}

#[allow(dead_code)]
impl ContextIntegrationEngine {
    /// Create a new context integration engine
    pub fn new() -> Self {
        ContextIntegrationEngine {
            learned_patterns: Vec::new(),
            distilled_knowledge: Vec::new(),
            pattern_cache: HashMap::new(),
        }
    }

    /// Add a learned pattern
    pub fn add_pattern(&mut self, pattern: LearnedPattern) {
        self.learned_patterns.push(pattern);
        self.pattern_cache.clear(); // Invalidate cache
    }

    /// Get patterns relevant to a context
    pub fn get_relevant_patterns(&self, context: &str, language: Option<&str>) -> Vec<LearnedPattern> {
        let mut relevant = Vec::new();

        for pattern in &self.learned_patterns {
            // Check if pattern context matches
            if pattern.context.to_lowercase().contains(&context.to_lowercase()) {
                // Check language if specified
                if let Some(lang) = language {
                    if pattern.language.to_lowercase() == lang.to_lowercase() {
                        relevant.push(pattern.clone());
                    }
                } else {
                    relevant.push(pattern.clone());
                }
            }
        }

        // Sort by effectiveness score (highest first)
        relevant.sort_by(|a, b| b.effectiveness_score.partial_cmp(&a.effectiveness_score).unwrap());

        relevant
    }

    /// Score context relevance
    pub fn score_context_relevance(
        &self,
        task: &str,
        context: &str,
        language: Option<&str>,
    ) -> ContextRelevance {
        let matching_patterns = self.get_relevant_patterns(context, language);

        // Calculate relevance score based on matching patterns
        let relevance_score = if matching_patterns.is_empty() {
            0.0
        } else {
            let avg_effectiveness: f32 = matching_patterns.iter().map(|p| p.effectiveness_score).sum::<f32>()
                / matching_patterns.len() as f32;
            (avg_effectiveness * 0.7) + (matching_patterns.len() as f32 / 10.0 * 0.3).min(1.0)
        };

        let pattern_ids: Vec<String> = matching_patterns.iter().map(|p| p.pattern_id.clone()).collect();

        let suggested_approaches = matching_patterns
            .iter()
            .take(3)
            .map(|p| p.description.clone())
            .collect();

        ContextRelevance {
            context_id: format!("ctx_{}", task.len()),
            relevance_score: relevance_score.min(1.0),
            matching_patterns: pattern_ids,
            suggested_approaches,
            confidence: (matching_patterns.len() as f32 / 5.0).min(1.0),
        }
    }

    /// Generate proactive suggestions
    pub fn generate_suggestions(
        &self,
        _task: &str,
        context: &str,
        language: Option<&str>,
    ) -> Vec<ProactiveSuggestion> {
        let mut suggestions = Vec::new();
        let relevant_patterns = self.get_relevant_patterns(context, language);

        // Generate pattern-based suggestions
        for (idx, pattern) in relevant_patterns.iter().take(3).enumerate() {
            suggestions.push(ProactiveSuggestion {
                suggestion_id: format!("sug_pattern_{}", idx),
                suggestion_type: "pattern".to_string(),
                title: format!("Apply {} Pattern", pattern.pattern_type),
                description: format!(
                    "Based on {} similar tasks, applying the {} pattern has a {:.0}% success rate",
                    pattern.times_used, pattern.pattern_type, pattern.success_rate * 100.0
                ),
                confidence: pattern.effectiveness_score,
                estimated_benefit: if pattern.effectiveness_score > 0.8 {
                    "high".to_string()
                } else if pattern.effectiveness_score > 0.6 {
                    "medium".to_string()
                } else {
                    "low".to_string()
                },
                action: format!("Use {} approach", pattern.pattern_type),
            });
        }

        // Generate knowledge-based suggestions
        for (idx, knowledge) in self.distilled_knowledge.iter().take(2).enumerate() {
            if knowledge.relevance_score > 0.5 {
                suggestions.push(ProactiveSuggestion {
                    suggestion_id: format!("sug_knowledge_{}", idx),
                    suggestion_type: "optimization".to_string(),
                    title: knowledge.title.clone(),
                    description: knowledge.content.clone(),
                    confidence: knowledge.confidence,
                    estimated_benefit: if knowledge.relevance_score > 0.8 {
                        "high".to_string()
                    } else {
                        "medium".to_string()
                    },
                    action: format!("Apply: {}", knowledge.title),
                });
            }
        }

        suggestions
    }

    /// Inject patterns into task prompt
    pub fn inject_patterns_into_prompt(
        &self,
        _task: &str,
        context: &str,
        language: Option<&str>,
    ) -> String {
        let relevant_patterns = self.get_relevant_patterns(context, language);

        if relevant_patterns.is_empty() {
            return String::new();
        }

        let mut injection = String::from("\n\n<learned_patterns>\n");
        injection.push_str("Based on past successful tasks, consider these patterns:\n\n");

        for (idx, pattern) in relevant_patterns.iter().take(3).enumerate() {
            injection.push_str(&format!(
                "{}. {} Pattern ({}% success rate, used {} times):\n   {}\n\n",
                idx + 1,
                pattern.pattern_type,
                (pattern.success_rate * 100.0) as u32,
                pattern.times_used,
                pattern.description
            ));
        }

        injection.push_str("</learned_patterns>\n");
        injection
    }

    /// Activate knowledge distillation
    pub fn activate_knowledge_distillation(&mut self) -> Vec<DistilledKnowledge> {
        let mut distilled = Vec::new();

        // Analyze patterns to extract key knowledge
        let mut pattern_categories: HashMap<String, Vec<&LearnedPattern>> = HashMap::new();

        for pattern in &self.learned_patterns {
            pattern_categories
                .entry(pattern.pattern_type.clone())
                .or_insert_with(Vec::new)
                .push(pattern);
        }

        // Generate distilled knowledge from patterns
        for (category, patterns) in pattern_categories {
            if patterns.is_empty() {
                continue;
            }

            let avg_effectiveness: f32 = patterns.iter().map(|p| p.effectiveness_score).sum::<f32>()
                / patterns.len() as f32;
            let total_uses: u32 = patterns.iter().map(|p| p.times_used).sum();

            let knowledge = DistilledKnowledge {
                knowledge_id: format!("know_{}", category),
                title: format!("Best Practice: {}", category),
                content: format!(
                    "The {} pattern has been successfully applied {} times with {:.0}% effectiveness",
                    category, total_uses, avg_effectiveness * 100.0
                ),
                category: "best_practice".to_string(),
                relevance_score: avg_effectiveness,
                confidence: (patterns.len() as f32 / 10.0).min(1.0),
                source_patterns: patterns.iter().map(|p| p.pattern_id.clone()).collect(),
            };

            distilled.push(knowledge);
        }

        self.distilled_knowledge = distilled.clone();
        distilled
    }

    /// Get context injection for agent
    pub fn get_context_injection(
        &self,
        task: &str,
        context: &str,
        language: Option<&str>,
    ) -> String {
        let mut injection = String::new();

        // Add pattern injection
        injection.push_str(&self.inject_patterns_into_prompt(task, context, language));

        // Add suggestions
        let suggestions = self.generate_suggestions(task, context, language);
        if !suggestions.is_empty() {
            injection.push_str("\n<proactive_suggestions>\n");
            for suggestion in suggestions.iter().take(3) {
                injection.push_str(&format!(
                    "- {}: {} (confidence: {:.0}%)\n",
                    suggestion.title,
                    suggestion.description,
                    suggestion.confidence * 100.0
                ));
            }
            injection.push_str("</proactive_suggestions>\n");
        }

        injection
    }

    /// Record pattern usage
    pub fn record_pattern_usage(&mut self, pattern_id: &str, success: bool) {
        for pattern in &mut self.learned_patterns {
            if pattern.pattern_id == pattern_id {
                pattern.times_used += 1;
                if success {
                    pattern.effectiveness_score = (pattern.effectiveness_score * 0.9) + 0.1;
                } else {
                    pattern.effectiveness_score = (pattern.effectiveness_score * 0.95) - 0.05;
                }
                pattern.effectiveness_score = pattern.effectiveness_score.max(0.0).min(1.0);
                pattern.last_used = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                break;
            }
        }
        self.pattern_cache.clear(); // Invalidate cache
    }

    /// Get statistics
    pub fn get_statistics(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();

        stats.insert(
            "total_patterns".to_string(),
            serde_json::json!(self.learned_patterns.len()),
        );
        stats.insert(
            "total_knowledge".to_string(),
            serde_json::json!(self.distilled_knowledge.len()),
        );

        let avg_effectiveness: f32 = if self.learned_patterns.is_empty() {
            0.0
        } else {
            self.learned_patterns.iter().map(|p| p.effectiveness_score).sum::<f32>()
                / self.learned_patterns.len() as f32
        };

        stats.insert(
            "avg_effectiveness".to_string(),
            serde_json::json!(avg_effectiveness),
        );

        let total_uses: u32 = self.learned_patterns.iter().map(|p| p.times_used).sum();
        stats.insert("total_uses".to_string(), serde_json::json!(total_uses));

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_integration_engine_creation() {
        let engine = ContextIntegrationEngine::new();
        assert_eq!(engine.learned_patterns.len(), 0);
        assert_eq!(engine.distilled_knowledge.len(), 0);
    }

    #[test]
    fn test_add_and_retrieve_patterns() {
        let mut engine = ContextIntegrationEngine::new();

        let pattern = LearnedPattern {
            pattern_id: "pat_1".to_string(),
            pattern_type: "code".to_string(),
            description: "Use functional components".to_string(),
            context: "React".to_string(),
            language: "JavaScript".to_string(),
            success_rate: 0.9,
            times_used: 5,
            last_used: 0,
            effectiveness_score: 0.85,
        };

        engine.add_pattern(pattern);
        assert_eq!(engine.learned_patterns.len(), 1);

        let relevant = engine.get_relevant_patterns("React", Some("JavaScript"));
        assert_eq!(relevant.len(), 1);
    }

    #[test]
    fn test_context_relevance_scoring() {
        let mut engine = ContextIntegrationEngine::new();

        let pattern = LearnedPattern {
            pattern_id: "pat_1".to_string(),
            pattern_type: "optimization".to_string(),
            description: "Memoize expensive computations".to_string(),
            context: "performance".to_string(),
            language: "JavaScript".to_string(),
            success_rate: 0.95,
            times_used: 10,
            last_used: 0,
            effectiveness_score: 0.9,
        };

        engine.add_pattern(pattern);

        let relevance = engine.score_context_relevance("Optimize performance", "performance", Some("JavaScript"));
        assert!(relevance.relevance_score > 0.5);
        assert!(!relevance.matching_patterns.is_empty());
    }

    #[test]
    fn test_proactive_suggestions() {
        let mut engine = ContextIntegrationEngine::new();

        let pattern = LearnedPattern {
            pattern_id: "pat_1".to_string(),
            pattern_type: "workflow".to_string(),
            description: "Test-driven development".to_string(),
            context: "testing".to_string(),
            language: "Rust".to_string(),
            success_rate: 0.88,
            times_used: 8,
            last_used: 0,
            effectiveness_score: 0.85,
        };

        engine.add_pattern(pattern);

        let suggestions = engine.generate_suggestions("Write tests", "testing", Some("Rust"));
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].suggestion_type, "pattern");
    }
}
