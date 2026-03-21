use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePattern {
    pub id: String,
    pub pattern: String,
    pub context: String,
    pub language: String,
    pub project_type: String,
    pub frequency: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UserPreference {
    pub key: String,
    pub value: serde_json::Value,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub id: String,
    pub error_type: String,
    pub context: String,
    pub solution: String,
    pub success_rate: f32,
    pub success_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessfulStrategy {
    pub id: String,
    pub task_type: String,
    pub strategy: String,
    pub tools: Vec<String>,
    pub average_duration: f32,
    pub success_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProjectContext {
    pub workspace_path: String,
    pub project_type: String,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub common_files: Vec<String>,
}

pub struct ContextMemory {
    code_patterns: Vec<CodePattern>,
    user_preferences: HashMap<String, UserPreference>,
    error_patterns: Vec<ErrorPattern>,
    successful_strategies: Vec<SuccessfulStrategy>,
    project_contexts: HashMap<String, ProjectContext>,
}

#[allow(dead_code)]
impl ContextMemory {
    pub fn new() -> Self {
        Self {
            code_patterns: Vec::new(),
            user_preferences: HashMap::new(),
            error_patterns: Vec::new(),
            successful_strategies: Vec::new(),
            project_contexts: HashMap::new(),
        }
    }

    pub fn record_code_pattern(
        &mut self,
        pattern: String,
        context: String,
        language: String,
        project_type: String,
    ) {
        let id = format!("pattern_{}_{}", language, pattern.len());
        let existing = self
            .code_patterns
            .iter_mut()
            .find(|p| p.id == id);

        if let Some(existing) = existing {
            existing.frequency += 1;
        } else {
            self.code_patterns.push(CodePattern {
                id,
                pattern,
                context,
                language,
                project_type,
                frequency: 1,
            });
        }
    }

    pub fn get_relevant_code_patterns(
        &self,
        context: &str,
        language: Option<&str>,
    ) -> Vec<CodePattern> {
        self.code_patterns
            .iter()
            .filter(|p| {
                p.context.contains(context)
                    && language.map_or(true, |l| p.language == l)
            })
            .cloned()
            .collect()
    }

    pub fn record_user_preference(
        &mut self,
        key: String,
        value: serde_json::Value,
        confidence: f32,
    ) {
        self.user_preferences.insert(
            key.clone(),
            UserPreference {
                key,
                value,
                confidence,
            },
        );
    }

    pub fn get_user_preference(&self, key: &str) -> Option<serde_json::Value> {
        self.user_preferences.get(key).map(|p| p.value.clone())
    }

    pub fn record_error_pattern(
        &mut self,
        error_type: String,
        context: String,
        solution: String,
        success: bool,
    ) {
        let id = format!("error_{}_{}", error_type, context.len());
        let existing = self
            .error_patterns
            .iter_mut()
            .find(|e| e.id == id);

        if let Some(existing) = existing {
            let total = existing.success_count + 1;
            existing.success_rate = if success {
                (existing.success_rate * existing.success_count as f32 + 1.0) / total as f32
            } else {
                (existing.success_rate * existing.success_count as f32) / total as f32
            };
            existing.success_count = total;
        } else {
            self.error_patterns.push(ErrorPattern {
                id,
                error_type,
                context,
                solution,
                success_rate: if success { 1.0 } else { 0.0 },
                success_count: 1,
            });
        }
    }

    pub fn get_similar_error_patterns(&self, error_type: &str) -> Vec<ErrorPattern> {
        self.error_patterns
            .iter()
            .filter(|e| e.error_type.contains(error_type))
            .cloned()
            .collect()
    }

    pub fn record_successful_strategy(
        &mut self,
        task_type: String,
        strategy: String,
        tools: Vec<String>,
        duration: f32,
    ) {
        let id = format!("strategy_{}_{}", task_type, strategy.len());
        let existing = self
            .successful_strategies
            .iter_mut()
            .find(|s| s.id == id);

        if let Some(existing) = existing {
            existing.average_duration =
                (existing.average_duration * existing.success_count as f32 + duration)
                    / (existing.success_count + 1) as f32;
            existing.success_count += 1;
        } else {
            self.successful_strategies.push(SuccessfulStrategy {
                id,
                task_type,
                strategy,
                tools,
                average_duration: duration,
                success_count: 1,
            });
        }
    }

    pub fn get_best_strategies(&self, task_type: &str) -> Vec<SuccessfulStrategy> {
        let mut strategies: Vec<_> = self
            .successful_strategies
            .iter()
            .filter(|s| s.task_type == task_type)
            .cloned()
            .collect();

        strategies.sort_by(|a, b| {
            b.success_count
                .cmp(&a.success_count)
                .then_with(|| a.average_duration.partial_cmp(&b.average_duration).unwrap())
        });

        strategies
    }

    pub fn record_project_context(&mut self, context: ProjectContext) {
        self.project_contexts
            .insert(context.workspace_path.clone(), context);
    }

    pub fn get_project_context(&self, workspace_path: &str) -> Option<ProjectContext> {
        self.project_contexts.get(workspace_path).cloned()
    }
}

#[tauri::command]
pub async fn context_memory_record_pattern(
    pattern: String,
    _context: String,
    language: String,
    project_type: String,
) -> Result<()> {
    eprintln!(
        "Recording code pattern: {} in {} ({})",
        pattern, language, project_type
    );
    Ok(())
}

#[tauri::command]
pub async fn context_memory_get_patterns(
    _context: String,
    _language: Option<String>,
) -> Result<Vec<CodePattern>> {
    Ok(vec![])
}

#[tauri::command]
pub async fn context_memory_record_preference(
    key: String,
    value: serde_json::Value,
    _confidence: f32,
) -> Result<()> {
    eprintln!("Recording user preference: {} = {:?}", key, value);
    Ok(())
}

#[tauri::command]
pub async fn context_memory_get_preference(_key: String) -> Result<Option<serde_json::Value>> {
    Ok(None)
}

#[tauri::command]
pub async fn context_memory_record_error(
    error_type: String,
    _context: String,
    _solution: String,
    success: bool,
) -> Result<()> {
    eprintln!(
        "Recording error pattern: {} (success={})",
        error_type, success
    );
    Ok(())
}

#[tauri::command]
pub async fn context_memory_get_similar_errors(_error_type: String) -> Result<Vec<ErrorPattern>> {
    Ok(vec![])
}

#[tauri::command]
pub async fn context_memory_record_strategy(
    task_type: String,
    strategy: String,
    _tools: Vec<String>,
    _duration: f32,
) -> Result<()> {
    eprintln!(
        "Recording successful strategy: {} for task type {}",
        strategy, task_type
    );
    Ok(())
}

#[tauri::command]
pub async fn context_memory_get_best_strategies(_task_type: String) -> Result<Vec<SuccessfulStrategy>> {
    Ok(vec![])
}
