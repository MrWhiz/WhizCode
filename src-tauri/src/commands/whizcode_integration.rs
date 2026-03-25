/**
 * WhizCode Integration Layer for Tauri
 * Enables WhizCode to behave like Kiro with local LLM optimizations
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnalysis {
    pub query_type: String, // "feature", "bugfix", "refactor", "analysis", "spec"
    pub confidence: f32,
    pub requirements: Vec<String>,
    pub complexity: String, // "simple", "moderate", "complex"
    pub estimated_duration: u32, // seconds
    pub suggested_workflow: String,
    pub context: QueryContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryContext {
    pub keywords: Vec<String>,
    pub entities: Vec<String>,
    pub intent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedPrompt {
    pub system: String,
    pub user: String,
    pub estimated_tokens: u32,
    pub cache_key: String,
    pub metadata: PromptMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMetadata {
    pub query_type: String,
    pub context_size: usize,
    pub fragments_used: usize,
    pub learned_patterns: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrunedContext {
    pub files: Vec<PrunedFile>,
    pub summary: String,
    pub total_size: usize,
    pub estimated_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrunedFile {
    pub path: String,
    pub content: String,
    pub file_type: String, // "full", "summary", "snippet"
    pub relevance_score: f32,
    pub size: usize,
    pub estimated_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRoute {
    pub workflow: String,
    pub agent: String,
    pub priority: u32,
    pub prerequisites: Vec<String>,
    pub estimated_duration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingPhase {
    pub name: String,
    pub emoji: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub duration: Option<u64>,
    pub status: String, // "pending", "active", "completed", "failed"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct StreamingMetrics {
    pub total_tokens: u32,
    pub tokens_per_second: f32,
    pub estimated_time_remaining: u32,
    pub current_phase: String,
    pub phases: Vec<StreamingPhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhizCodeExecutionContext {
    pub execution_id: String,
    pub query: String,
    pub optimized_prompt: OptimizedPrompt,
    pub pruned_context: PrunedContext,
    pub workflow_route: WorkflowRoute,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub duration: Option<u64>,
}

pub struct WhizCodeIntegrationLayer {
    #[allow(dead_code)]
    execution_contexts: Arc<RwLock<HashMap<String, WhizCodeExecutionContext>>>,
    #[allow(dead_code)]
    prompt_cache: Arc<RwLock<HashMap<String, OptimizedPrompt>>>,
    #[allow(dead_code)]
    context_cache: Arc<RwLock<HashMap<String, PrunedContext>>>,
}

impl WhizCodeIntegrationLayer {
    pub fn new() -> Self {
        Self {
            execution_contexts: Arc::new(RwLock::new(HashMap::new())),
            prompt_cache: Arc::new(RwLock::new(HashMap::new())),
            context_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Analyze a user query
    pub fn analyze_query(query: &str) -> QueryAnalysis {
        let query_lower = query.to_lowercase();
        
        // Determine query type
        let query_type = if query_lower.contains("fix") || query_lower.contains("bug") || query_lower.contains("error") {
            "bugfix"
        } else if query_lower.contains("add") || query_lower.contains("implement") || query_lower.contains("create") {
            "feature"
        } else if query_lower.contains("refactor") || query_lower.contains("improve") || query_lower.contains("optimize") {
            "refactor"
        } else if query_lower.contains("analyze") || query_lower.contains("check") || query_lower.contains("review") {
            "analysis"
        } else if query_lower.contains("spec") || query_lower.contains("requirement") || query_lower.contains("design") {
            "spec"
        } else {
            "unknown"
        };

        // Extract requirements
        let requirements = Self::extract_requirements(query);

        // Assess complexity
        let complexity = Self::assess_complexity(query, &requirements);

        // Estimate duration
        let estimated_duration = Self::estimate_duration(&complexity, requirements.len());

        // Extract context
        let context = Self::extract_context(query);

        // Calculate confidence
        let confidence = Self::calculate_confidence(query_type, &requirements, &context);

        // Suggest workflow
        let suggested_workflow = Self::suggest_workflow(query_type);

        QueryAnalysis {
            query_type: query_type.to_string(),
            confidence,
            requirements,
            complexity,
            estimated_duration,
            suggested_workflow,
            context,
        }
    }

    /// Generate optimized prompt
    pub fn generate_optimized_prompt(
        query: &str,
        query_type: &str,
        context_size: usize,
    ) -> OptimizedPrompt {
        let mut system_prompt = Self::build_system_prompt(query_type);
        let user_prompt = Self::build_user_prompt(query, query_type);
        
        // Integrate prompt manager fragments based on query type
        let fragments_used = Self::apply_prompt_fragments(&mut system_prompt, query_type);
        
        // Apply learned patterns to optimize prompt
        let learned_patterns = Self::apply_learned_patterns(&mut system_prompt, query_type, query);
        
        let estimated_tokens = Self::estimate_tokens(&system_prompt, &user_prompt);
        let cache_key = Self::generate_cache_key(query);

        OptimizedPrompt {
            system: system_prompt,
            user: user_prompt,
            estimated_tokens,
            cache_key,
            metadata: PromptMetadata {
                query_type: query_type.to_string(),
                context_size,
                fragments_used,
                learned_patterns,
            },
        }
    }

    /// Prune context to fit token limit
    pub fn prune_context(
        files: Vec<(String, String)>,
        query: &str,
        max_tokens: u32,
    ) -> PrunedContext {
        let mut pruned_files = Vec::new();
        let mut total_tokens = 0u32;

        // Score and sort files by relevance
        let mut scored_files: Vec<_> = files
            .into_iter()
            .map(|(path, content)| {
                let score = Self::score_relevance(&path, &content, query);
                (path, content, score)
            })
            .collect();

        scored_files.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        // Add files until we hit token limit
        for (path, content, _score) in scored_files {
            if total_tokens >= max_tokens {
                break;
            }

            let pruned = Self::prune_file(&path, &content, max_tokens - total_tokens);
            total_tokens += pruned.estimated_tokens;
            pruned_files.push(pruned);
        }

        let summary = Self::generate_context_summary(&pruned_files);

        PrunedContext {
            files: pruned_files,
            summary,
            total_size: total_tokens as usize,
            estimated_tokens: total_tokens,
        }
    }

    /// Route query to appropriate workflow
    pub fn route_query(_query: &str, query_type: &str) -> WorkflowRoute {
        let (workflow, agent, priority) = match query_type {
            "bugfix" => ("bugfix-workflow", "bugfix-agent", 1),
            "feature" => ("feature-implementation", "feature-implementation-agent", 3),
            "refactor" => ("refactoring-workflow", "refactoring-agent", 4),
            "analysis" => ("analysis-workflow", "analysis-agent", 5),
            "spec" => ("spec-creation", "spec-agent", 2),
            _ => ("general-task-execution", "general-task-execution", 5),
        };

        WorkflowRoute {
            workflow: workflow.to_string(),
            agent: agent.to_string(),
            priority,
            prerequisites: vec!["project-context".to_string()],
            estimated_duration: 600,
        }
    }

    // Helper methods

    fn extract_requirements(query: &str) -> Vec<String> {
        let mut requirements = Vec::new();
        
        // Simple pattern matching for requirements
        let sentences: Vec<&str> = query.split(|c| c == '.' || c == '!' || c == '?').collect();
        for sentence in sentences.iter().take(3) {
            let trimmed = sentence.trim();
            if trimmed.len() > 10 {
                requirements.push(trimmed.to_string());
            }
        }

        if requirements.is_empty() && query.len() > 10 {
            requirements.push(query.to_string());
        }

        requirements.truncate(5);
        requirements
    }

    fn assess_complexity(query: &str, requirements: &[String]) -> String {
        let query_lower = query.to_lowercase();
        let mut score = 0;

        if query_lower.contains("multiple") || query_lower.contains("complex") || query_lower.contains("large") {
            score += 2;
        }
        if query_lower.contains("simple") || query_lower.contains("small") || query_lower.contains("quick") {
            score -= 1;
        }

        score += requirements.len() as i32 / 2;

        if score >= 3 {
            "complex".to_string()
        } else if score >= 1 {
            "moderate".to_string()
        } else {
            "simple".to_string()
        }
    }

    fn estimate_duration(complexity: &str, requirement_count: usize) -> u32 {
        let base = match complexity {
            "simple" => 300,
            "moderate" => 1800,
            "complex" => 3600,
            _ => 600,
        };

        base + (requirement_count as u32 * 300)
    }

    fn extract_context(query: &str) -> QueryContext {
        let words: Vec<&str> = query.split_whitespace().collect();
        let keywords: Vec<String> = words
            .iter()
            .filter(|w| w.len() > 4)
            .map(|w| w.to_string())
            .take(5)
            .collect();

        let intent = if query.to_lowercase().contains("fix") {
            "debugging"
        } else if query.to_lowercase().contains("add") || query.to_lowercase().contains("implement") {
            "development"
        } else if query.to_lowercase().contains("refactor") {
            "improvement"
        } else {
            "general"
        };

        QueryContext {
            keywords,
            entities: Vec::new(),
            intent: intent.to_string(),
        }
    }

    fn calculate_confidence(query_type: &str, requirements: &[String], _context: &QueryContext) -> f32 {
        let mut confidence: f32 = 0.5;

        if query_type != "unknown" {
            confidence += 0.2;
        }
        if requirements.len() >= 2 {
            confidence += 0.15;
        }

        confidence.min(1.0)
    }

    fn suggest_workflow(query_type: &str) -> String {
        match query_type {
            "bugfix" => "bugfix-workflow",
            "feature" => "feature-implementation",
            "refactor" => "refactoring-workflow",
            "analysis" => "analysis-workflow",
            "spec" => "spec-creation",
            _ => "general-task-execution",
        }
        .to_string()
    }

    fn build_system_prompt(query_type: &str) -> String {
        let base = "You are WhizCode, an autonomous agentic AI coding IDE that behaves like Kiro.\n\n\
                   ## Core Principles\n\
                   - Be knowledgeable and expert-level, not instructive\n\
                   - Speak like a developer - relatable and digestible\n\
                   - Be decisive, precise, and clear without fluff\n\
                   - Provide minimal, actionable information\n\
                   - Use proper code formatting and best practices";

        match query_type {
            "bugfix" => format!("{}\n\n## Bugfix Approach\n1. Identify the bug condition\n2. Create exploration tests\n3. Locate root cause\n4. Implement fix\n5. Verify fix works", base),
            "feature" => format!("{}\n\n## Feature Implementation\n1. Analyze requirements\n2. Design architecture\n3. Implement code\n4. Test feature\n5. Provide documentation", base),
            "refactor" => format!("{}\n\n## Refactoring\n1. Analyze code structure\n2. Identify improvements\n3. Implement changes\n4. Ensure functionality preserved", base),
            "analysis" => format!("{}\n\n## Analysis\n1. Analyze codebase\n2. Identify patterns\n3. Provide insights\n4. Suggest improvements", base),
            _ => base.to_string(),
        }
    }

    fn build_user_prompt(query: &str, _query_type: &str) -> String {
        format!("## Task\n{}\n\n## Instructions\n1. Understand the request\n2. Break down into steps if needed\n3. Execute the task\n4. Provide a summary of results\n\nBe efficient and direct in your approach.", query)
    }

    fn estimate_tokens(system: &str, user: &str) -> u32 {
        let total_chars = system.len() + user.len();
        // Rough estimate: ~4 characters per token
        (total_chars / 4) as u32
    }

    fn generate_cache_key(query: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Apply prompt manager fragments to enhance system prompt
    fn apply_prompt_fragments(system_prompt: &mut String, query_type: &str) -> usize {
        let fragments = match query_type {
            "bugfix" => vec![
                "\n\n## Bug Condition Exploration\n- Identify the exact condition that triggers the bug\n- Create minimal reproduction test\n- Verify the bug exists before fixing",
                "\n\n## Fix Verification\n- Test the fix with the exploration test\n- Ensure no regressions\n- Verify edge cases are handled",
            ],
            "feature" => vec![
                "\n\n## Requirements Clarity\n- Understand all requirements before implementation\n- Ask for clarification if needed\n- Document assumptions",
                "\n\n## Implementation Quality\n- Follow project conventions\n- Write clean, maintainable code\n- Include proper error handling",
            ],
            "refactor" => vec![
                "\n\n## Refactoring Safety\n- Preserve all existing functionality\n- Run tests after each change\n- Document why changes improve the code",
            ],
            "analysis" => vec![
                "\n\n## Analysis Depth\n- Analyze patterns and structure\n- Identify potential issues\n- Provide actionable recommendations",
            ],
            _ => vec![],
        };

        let count = fragments.len();
        for fragment in fragments {
            system_prompt.push_str(fragment);
        }
        count
    }

    /// Apply learned patterns to optimize prompt based on previous interactions
    fn apply_learned_patterns(system_prompt: &mut String, _query_type: &str, query: &str) -> usize {
        let mut patterns_applied = 0;

        // Pattern 1: If query mentions "error" or "crash", emphasize error handling
        if query.to_lowercase().contains("error") || query.to_lowercase().contains("crash") {
            system_prompt.push_str("\n\n## Error Handling Focus\n- Identify the exact error condition\n- Provide clear error messages\n- Handle edge cases gracefully");
            patterns_applied += 1;
        }

        // Pattern 2: If query mentions "performance" or "optimize", emphasize efficiency
        if query.to_lowercase().contains("performance") || query.to_lowercase().contains("optimize") {
            system_prompt.push_str("\n\n## Performance Optimization\n- Analyze current performance bottlenecks\n- Suggest efficient algorithms\n- Measure improvements");
            patterns_applied += 1;
        }

        // Pattern 3: If query mentions "test" or "testing", emphasize test coverage
        if query.to_lowercase().contains("test") {
            system_prompt.push_str("\n\n## Test Coverage\n- Write comprehensive tests\n- Cover edge cases\n- Ensure high code coverage");
            patterns_applied += 1;
        }

        // Pattern 4: If query mentions "security" or "vulnerability", emphasize security
        if query.to_lowercase().contains("security") || query.to_lowercase().contains("vulnerability") {
            system_prompt.push_str("\n\n## Security Focus\n- Identify security vulnerabilities\n- Apply security best practices\n- Validate all inputs");
            patterns_applied += 1;
        }

        patterns_applied
    }

    fn score_relevance(path: &str, _content: &str, query: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let path_lower = path.to_lowercase();

        let mut score = 0.0;

        // File name relevance
        if path_lower.contains(&query_lower) {
            score += 0.5;
        }

        // File type relevance
        if path.ends_with(".ts") || path.ends_with(".tsx") || path.ends_with(".js") {
            score += 0.1;
        }

        score
    }

    fn prune_file(path: &str, content: &str, max_tokens: u32) -> PrunedFile {
        let estimated_tokens = Self::estimate_tokens("", content);
        let size = content.len();

        let (file_type, pruned_content) = if estimated_tokens <= max_tokens {
            ("full".to_string(), content.to_string())
        } else {
            let lines: Vec<&str> = content.lines().collect();
            let max_lines = (max_tokens / 2) as usize;
            let truncated = lines.iter().take(max_lines).map(|l| *l).collect::<Vec<_>>().join("\n");
            ("summary".to_string(), truncated)
        };

        PrunedFile {
            path: path.to_string(),
            content: pruned_content,
            file_type,
            relevance_score: 0.8,
            size,
            estimated_tokens,
        }
    }

    fn generate_context_summary(files: &[PrunedFile]) -> String {
        let mut summary = format!("## Context Summary\nFiles: {}\n", files.len());
        for file in files {
            summary.push_str(&format!("- {} ({} tokens)\n", file.path, file.estimated_tokens));
        }
        summary
    }
}

impl Default for WhizCodeIntegrationLayer {
    fn default() -> Self {
        Self::new()
    }
}
