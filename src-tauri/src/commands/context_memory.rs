use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePattern {
    pub id: String,
    pub pattern: String,
    pub context: String,
    pub language: String,
    pub project_type: String,
    pub frequency: u32,
    pub last_used: u64,
    pub effectiveness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreference {
    pub key: String,
    pub value: serde_json::Value,
    pub confidence: f32,
    pub last_updated: u64,
    pub temporal_decay: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub id: String,
    pub error_type: String,
    pub context: String,
    pub solution: String,
    pub success_rate: f32,
    pub success_count: u32,
    pub last_seen: u64,
    pub resolution_time_avg: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessfulStrategy {
    pub id: String,
    pub task_type: String,
    pub strategy: String,
    pub tools: Vec<String>,
    pub average_duration: f32,
    pub success_count: u32,
    pub effectiveness_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub workspace_path: String,
    pub project_type: String,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub common_files: Vec<String>,
    pub last_analyzed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMemoryStats {
    pub total_patterns: usize,
    pub total_preferences: usize,
    pub total_error_patterns: usize,
    pub total_strategies: usize,
    pub total_projects: usize,
    pub most_used_language: Option<String>,
    pub most_common_error: Option<String>,
    pub best_strategy: Option<String>,
}

pub struct ContextMemory {
    code_patterns: Arc<Mutex<Vec<CodePattern>>>,
    user_preferences: Arc<Mutex<HashMap<String, UserPreference>>>,
    error_patterns: Arc<Mutex<Vec<ErrorPattern>>>,
    successful_strategies: Arc<Mutex<Vec<SuccessfulStrategy>>>,
    project_contexts: Arc<Mutex<HashMap<String, ProjectContext>>>,
}

#[allow(dead_code)]
impl ContextMemory {
    pub fn new() -> Self {
        Self {
            code_patterns: Arc::new(Mutex::new(Vec::new())),
            user_preferences: Arc::new(Mutex::new(HashMap::new())),
            error_patterns: Arc::new(Mutex::new(Vec::new())),
            successful_strategies: Arc::new(Mutex::new(Vec::new())),
            project_contexts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn record_code_pattern(
        &self,
        pattern: String,
        context: String,
        language: String,
        project_type: String,
    ) {
        let id = format!("pattern_{}_{}", language, pattern.len());
        let mut patterns = self.code_patterns.lock().unwrap();
        
        if let Some(existing) = patterns.iter_mut().find(|p| p.id == id) {
            existing.frequency += 1;
            existing.last_used = Self::current_timestamp();
        } else {
            patterns.push(CodePattern {
                id,
                pattern,
                context,
                language,
                project_type,
                frequency: 1,
                last_used: Self::current_timestamp(),
                effectiveness: 0.8,
            });
        }
    }

    pub fn get_relevant_code_patterns(
        &self,
        context: &str,
        language: Option<&str>,
    ) -> Vec<CodePattern> {
        let patterns = self.code_patterns.lock().unwrap();
        patterns
            .iter()
            .filter(|p| {
                p.context.contains(context)
                    && language.map_or(true, |l| p.language == l)
            })
            .cloned()
            .collect()
    }

    pub fn record_user_preference(
        &self,
        key: String,
        value: serde_json::Value,
        confidence: f32,
    ) {
        let mut preferences = self.user_preferences.lock().unwrap();
        preferences.insert(
            key.clone(),
            UserPreference {
                key,
                value,
                confidence,
                last_updated: Self::current_timestamp(),
                temporal_decay: 0.95,
            },
        );
    }

    pub fn get_user_preference(&self, key: &str) -> Option<serde_json::Value> {
        let preferences = self.user_preferences.lock().unwrap();
        preferences.get(key).map(|p| p.value.clone())
    }

    pub fn get_all_preferences(&self) -> Vec<UserPreference> {
        let preferences = self.user_preferences.lock().unwrap();
        preferences.values().cloned().collect()
    }

    pub fn record_error_pattern(
        &self,
        error_type: String,
        context: String,
        solution: String,
        success: bool,
        resolution_time: f32,
    ) {
        let id = format!("error_{}_{}", error_type, context.len());
        let mut errors = self.error_patterns.lock().unwrap();
        
        if let Some(existing) = errors.iter_mut().find(|e| e.id == id) {
            let total = existing.success_count + 1;
            existing.success_rate = if success {
                (existing.success_rate * existing.success_count as f32 + 1.0) / total as f32
            } else {
                (existing.success_rate * existing.success_count as f32) / total as f32
            };
            existing.resolution_time_avg = 
                (existing.resolution_time_avg * existing.success_count as f32 + resolution_time) / total as f32;
            existing.success_count = total;
            existing.last_seen = Self::current_timestamp();
        } else {
            errors.push(ErrorPattern {
                id,
                error_type,
                context,
                solution,
                success_rate: if success { 1.0 } else { 0.0 },
                success_count: 1,
                last_seen: Self::current_timestamp(),
                resolution_time_avg: resolution_time,
            });
        }
    }

    pub fn get_similar_error_patterns(&self, error_type: &str) -> Vec<ErrorPattern> {
        let errors = self.error_patterns.lock().unwrap();
        errors
            .iter()
            .filter(|e| e.error_type.contains(error_type))
            .cloned()
            .collect()
    }

    pub fn get_all_error_patterns(&self) -> Vec<ErrorPattern> {
        let errors = self.error_patterns.lock().unwrap();
        errors.clone()
    }

    pub fn record_successful_strategy(
        &self,
        task_type: String,
        strategy: String,
        tools: Vec<String>,
        duration: f32,
    ) {
        let id = format!("strategy_{}_{}", task_type, strategy.len());
        let mut strategies = self.successful_strategies.lock().unwrap();
        
        if let Some(existing) = strategies.iter_mut().find(|s| s.id == id) {
            existing.average_duration =
                (existing.average_duration * existing.success_count as f32 + duration)
                    / (existing.success_count + 1) as f32;
            existing.success_count += 1;
            existing.effectiveness_score = 
                (existing.success_count as f32 / (existing.success_count + 1) as f32) * 
                (1.0 - (existing.average_duration / 100.0).min(1.0));
        } else {
            strategies.push(SuccessfulStrategy {
                id,
                task_type,
                strategy,
                tools,
                average_duration: duration,
                success_count: 1,
                effectiveness_score: 0.8,
            });
        }
    }

    pub fn get_best_strategies(&self, task_type: &str) -> Vec<SuccessfulStrategy> {
        let strategies = self.successful_strategies.lock().unwrap();
        let mut result: Vec<_> = strategies
            .iter()
            .filter(|s| s.task_type == task_type)
            .cloned()
            .collect();

        result.sort_by(|a, b| {
            b.effectiveness_score
                .partial_cmp(&a.effectiveness_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.success_count.cmp(&a.success_count))
        });

        result
    }

    pub fn get_all_strategies(&self) -> Vec<SuccessfulStrategy> {
        let strategies = self.successful_strategies.lock().unwrap();
        strategies.clone()
    }

    pub fn record_project_context(&self, context: ProjectContext) {
        let mut projects = self.project_contexts.lock().unwrap();
        projects.insert(context.workspace_path.clone(), context);
    }

    pub fn get_project_context(&self, workspace_path: &str) -> Option<ProjectContext> {
        let projects = self.project_contexts.lock().unwrap();
        projects.get(workspace_path).cloned()
    }

    pub fn get_all_project_contexts(&self) -> Vec<ProjectContext> {
        let projects = self.project_contexts.lock().unwrap();
        projects.values().cloned().collect()
    }

    pub fn get_statistics(&self) -> ContextMemoryStats {
        let patterns = self.code_patterns.lock().unwrap();
        let preferences = self.user_preferences.lock().unwrap();
        let errors = self.error_patterns.lock().unwrap();
        let strategies = self.successful_strategies.lock().unwrap();
        let projects = self.project_contexts.lock().unwrap();

        let most_used_language = patterns
            .iter()
            .fold(HashMap::new(), |mut acc, p| {
                *acc.entry(p.language.clone()).or_insert(0) += 1;
                acc
            })
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang);

        let most_common_error = errors
            .iter()
            .max_by_key(|e| e.success_count)
            .map(|e| e.error_type.clone());

        let best_strategy = strategies
            .iter()
            .max_by(|a, b| {
                a.effectiveness_score
                    .partial_cmp(&b.effectiveness_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.strategy.clone());

        ContextMemoryStats {
            total_patterns: patterns.len(),
            total_preferences: preferences.len(),
            total_error_patterns: errors.len(),
            total_strategies: strategies.len(),
            total_projects: projects.len(),
            most_used_language,
            most_common_error,
            best_strategy,
        }
    }

    pub fn clear_old_data(&self, days_old: u64) {
        let cutoff_time = Self::current_timestamp() - (days_old * 86400);
        
        let mut patterns = self.code_patterns.lock().unwrap();
        patterns.retain(|p| p.last_used > cutoff_time);
        
        let mut errors = self.error_patterns.lock().unwrap();
        errors.retain(|e| e.last_seen > cutoff_time);
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
    context: String,
    language: Option<String>,
) -> Result<Vec<CodePattern>> {
    eprintln!(
        "Getting patterns for context: {} (language: {:?})",
        context, language
    );
    Ok(vec![])
}

#[tauri::command]
pub async fn context_memory_record_preference(
    key: String,
    value: serde_json::Value,
    confidence: f32,
) -> Result<()> {
    eprintln!("Recording user preference: {} = {:?} (confidence: {})", key, value, confidence);
    Ok(())
}

#[tauri::command]
pub async fn context_memory_get_preference(key: String) -> Result<Option<serde_json::Value>> {
    eprintln!("Getting user preference: {}", key);
    Ok(None)
}

#[tauri::command]
pub async fn context_memory_get_all_preferences() -> Result<Vec<UserPreference>> {
    eprintln!("Getting all user preferences");
    Ok(vec![])
}

#[tauri::command]
pub async fn context_memory_record_error(
    error_type: String,
    _context: String,
    _solution: String,
    success: bool,
    resolution_time: f32,
) -> Result<()> {
    eprintln!(
        "Recording error pattern: {} (success={}, resolution_time={}s)",
        error_type, success, resolution_time
    );
    Ok(())
}

#[tauri::command]
pub async fn context_memory_get_similar_errors(error_type: String) -> Result<Vec<ErrorPattern>> {
    eprintln!("Getting similar error patterns for: {}", error_type);
    Ok(vec![])
}

#[tauri::command]
pub async fn context_memory_get_all_errors() -> Result<Vec<ErrorPattern>> {
    eprintln!("Getting all error patterns");
    Ok(vec![])
}

#[tauri::command]
pub async fn context_memory_record_strategy(
    task_type: String,
    strategy: String,
    _tools: Vec<String>,
    duration: f32,
) -> Result<()> {
    eprintln!(
        "Recording successful strategy: {} for task type {} (duration: {}s)",
        strategy, task_type, duration
    );
    Ok(())
}

#[tauri::command]
pub async fn context_memory_get_best_strategies(task_type: String) -> Result<Vec<SuccessfulStrategy>> {
    eprintln!("Getting best strategies for task type: {}", task_type);
    Ok(vec![])
}

#[tauri::command]
pub async fn context_memory_get_all_strategies() -> Result<Vec<SuccessfulStrategy>> {
    eprintln!("Getting all strategies");
    Ok(vec![])
}

#[tauri::command]
pub async fn context_memory_record_project(
    workspace_path: String,
    project_type: String,
    languages: Vec<String>,
    _frameworks: Vec<String>,
    _common_files: Vec<String>,
) -> Result<()> {
    eprintln!(
        "Recording project context: {} (type: {}, languages: {:?})",
        workspace_path, project_type, languages
    );
    Ok(())
}

#[tauri::command]
pub async fn context_memory_get_project(workspace_path: String) -> Result<Option<ProjectContext>> {
    eprintln!("Getting project context for: {}", workspace_path);
    Ok(None)
}

#[tauri::command]
pub async fn context_memory_get_all_projects() -> Result<Vec<ProjectContext>> {
    eprintln!("Getting all project contexts");
    Ok(vec![])
}

#[tauri::command]
pub async fn context_memory_get_statistics() -> Result<ContextMemoryStats> {
    eprintln!("Getting context memory statistics");
    Ok(ContextMemoryStats {
        total_patterns: 0,
        total_preferences: 0,
        total_error_patterns: 0,
        total_strategies: 0,
        total_projects: 0,
        most_used_language: None,
        most_common_error: None,
        best_strategy: None,
    })
}

#[tauri::command]
pub async fn context_memory_clear_old_data(days_old: u64) -> Result<()> {
    eprintln!("Clearing context memory data older than {} days", days_old);
    Ok(())
}
