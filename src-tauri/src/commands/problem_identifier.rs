/// Intelligent Problem Identifier
/// 
/// This module implements smart problem identification that:
/// 1. Parses the problem statement to extract keywords
/// 2. Uses targeted searches instead of reading all files
/// 3. Prioritizes files by relevance and impact
/// 4. Provides focused context for problem-solving

use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskWorkingState {
    pub task_fingerprint: String,
    pub task_kind: String,
    pub current_goal: String,
    pub suspected_files: Vec<String>,
    pub completed_checks: Vec<String>,
    pub pending_actions: Vec<String>,
    pub blockers: Vec<String>,
    pub research_summary: Option<String>,
    pub last_iteration: u32,
    pub last_tool: Option<String>,
    pub update_count: u32,
}

impl TaskWorkingState {
    pub fn to_prompt_block(&self) -> String {
        let mut block = String::new();
        block.push_str("\n<task_working_state>\n");
        block.push_str(&format!("fingerprint: {}\n", self.task_fingerprint));
        block.push_str(&format!("kind: {}\n", self.task_kind));
        block.push_str(&format!("current_goal: {}\n", self.current_goal));
        block.push_str("decision_mode: one_search_pass_one_read_pass_then_edit\n");
        block.push_str("discovery_budget: stop exploring once a likely implementation file is known\n");

        if !self.suspected_files.is_empty() {
            block.push_str("suspected_files:\n");
            for file in self.suspected_files.iter().take(5) {
                block.push_str(&format!("- {}\n", file));
            }
        }

        if !self.completed_checks.is_empty() {
            block.push_str("completed_checks:\n");
            for check in self.completed_checks.iter().rev().take(5).rev() {
                block.push_str(&format!("- {}\n", check));
            }
        }

        if !self.pending_actions.is_empty() {
            block.push_str("pending_actions:\n");
            for action in self.pending_actions.iter().take(5) {
                block.push_str(&format!("- {}\n", action));
            }
        }

        if !self.blockers.is_empty() {
            block.push_str("blockers:\n");
            for blocker in self.blockers.iter().take(5) {
                block.push_str(&format!("- {}\n", blocker));
            }
        }

        if let Some(summary) = &self.research_summary {
            if !summary.trim().is_empty() {
                let compact = summary.trim().chars().take(800).collect::<String>();
                block.push_str("recent_research:\n");
                block.push_str(&compact);
                if compact.len() < summary.trim().len() {
                    block.push_str("\n... (truncated)");
                }
                block.push('\n');
            }
        }

        block.push_str(&format!("last_iteration: {}\n", self.last_iteration));
        if let Some(tool) = &self.last_tool {
            block.push_str(&format!("last_tool: {}\n", tool));
        }
        block.push_str("</task_working_state>\n");
        block
    }

    pub fn note_iteration(&mut self, iteration: u32, tool: Option<&str>) {
        self.last_iteration = iteration;
        if let Some(tool) = tool {
            self.last_tool = Some(tool.to_string());
        }
        self.update_count = self.update_count.saturating_add(1);
    }

    pub fn record_tool_success(&mut self, tool_name: &str, result: &str) {
        self.note_iteration(self.last_iteration, Some(tool_name));

        let summary = if result.trim().is_empty() {
            format!("{} completed", tool_name)
        } else {
            format!("{} completed: {}", tool_name, result.trim().chars().take(120).collect::<String>())
        };
        self.completed_checks.push(summary);
        self.pending_actions
            .retain(|action| !action.to_lowercase().contains(&tool_name.to_lowercase()));

        if matches!(
            tool_name,
            "write_file" | "edit_file" | "multi_edit_file" | "create_file" | "delete_file" | "move_file" | "rename_file"
        ) {
            if !self.pending_actions.iter().any(|action| action == "verify_changes") {
                self.pending_actions.push("verify_changes".to_string());
            }
            self.current_goal = "Verify the changes and make sure the task is fully resolved.".to_string();
        }
    }

    pub fn record_tool_failure(&mut self, tool_name: &str, error: &str) {
        self.note_iteration(self.last_iteration, Some(tool_name));
        let blocker = format!(
            "{} failed: {}",
            tool_name,
            error.trim().chars().take(160).collect::<String>()
        );
        self.blockers.push(blocker);
        if !self.pending_actions.iter().any(|action| action == "recover_from_failure") {
            self.pending_actions.push("recover_from_failure".to_string());
        }
        self.current_goal = "Resolve the blocker with the smallest safe change, then retry verification.".to_string();
    }

    pub fn record_research(&mut self, summary: String) {
        self.research_summary = Some(summary.trim().chars().take(2000).collect::<String>());
        if !self.pending_actions.iter().any(|action| action == "use_research_findings") {
            self.pending_actions.push("use_research_findings".to_string());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemAnalysis {
    pub keywords: Vec<String>,
    pub file_patterns: Vec<String>,
    pub search_queries: Vec<SearchQuery>,
    pub suspected_files: Vec<SuspectedFile>,
    pub task_kind: String,
    pub focus_summary: String,
    pub investigation_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub pattern: String,
    pub file_pattern: String,
    pub priority: u32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SuspectedFile {
    pub path: String,
    pub relevance_score: u32,
    pub reason: String,
    pub file_type: String,
}

pub struct ProblemIdentifier;

impl ProblemIdentifier {
    /// Analyze a problem statement and generate targeted search queries
    pub fn analyze_problem(problem_statement: &str) -> ProblemAnalysis {
        let keywords = Self::extract_keywords(problem_statement);
        let file_patterns = Self::infer_file_patterns(&keywords);
        let search_queries = Self::generate_search_queries(&keywords, &file_patterns);
        let suspected_files = Self::infer_suspected_files(&keywords);
        let task_kind = Self::classify_task_kind(problem_statement, &keywords);
        let focus_summary = Self::build_focus_summary(&task_kind, &keywords, &suspected_files);
        let investigation_strategy = Self::generate_strategy(&keywords);

        ProblemAnalysis {
            keywords,
            file_patterns,
            search_queries,
            suspected_files,
            task_kind,
            focus_summary,
            investigation_strategy,
        }
    }

    pub fn build_working_state(
        task_statement: &str,
        workspace_path: Option<&str>,
        active_file: Option<&str>,
        analysis: &ProblemAnalysis,
    ) -> TaskWorkingState {
        let mut suspected_files: Vec<String> = analysis
            .suspected_files
            .iter()
            .take(5)
            .map(|file| file.path.clone())
            .collect();

        if let Some(active_file) = active_file {
            if !suspected_files.iter().any(|file| file == active_file) {
                suspected_files.insert(0, active_file.to_string());
            }
        }

        let fingerprint = Self::analysis_fingerprint(task_statement, workspace_path, active_file, analysis);
        let mut pending_actions = vec![
            "use_semantic_search_first".to_string(),
            "inspect_one_candidate_file".to_string(),
            "make_the_smallest_safe_change".to_string(),
            "verify_the_result".to_string(),
        ];

        if analysis.task_kind == "bug-fix" {
            pending_actions.insert(1, "confirm_repro_or_error_path".to_string());
        }

        TaskWorkingState {
            task_fingerprint: fingerprint,
            task_kind: analysis.task_kind.clone(),
            current_goal: format!(
                "{} After one focused discovery pass, commit to the smallest safe edit instead of continuing to search.",
                analysis.focus_summary
            ),
            suspected_files,
            completed_checks: Vec::new(),
            pending_actions,
            blockers: Vec::new(),
            research_summary: None,
            last_iteration: 0,
            last_tool: None,
            update_count: 0,
        }
    }

    pub fn analysis_fingerprint(
        task_statement: &str,
        workspace_path: Option<&str>,
        active_file: Option<&str>,
        analysis: &ProblemAnalysis,
    ) -> String {
        let mut hasher = DefaultHasher::new();
        Self::normalize_task_text(task_statement).hash(&mut hasher);
        workspace_path.unwrap_or("").hash(&mut hasher);
        active_file.unwrap_or("").hash(&mut hasher);
        analysis.task_kind.hash(&mut hasher);
        analysis.keywords.hash(&mut hasher);
        analysis
            .suspected_files
            .iter()
            .map(|file| &file.path)
            .collect::<Vec<_>>()
            .hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    fn normalize_task_text(task_statement: &str) -> String {
        task_statement
            .split_whitespace()
            .map(|part| part.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn classify_task_kind(task_statement: &str, keywords: &[String]) -> String {
        let lower = task_statement.to_lowercase();
        let keyword_match = |needle: &str| keywords.iter().any(|k| k == needle);

        if lower.contains("fix") || lower.contains("bug") || lower.contains("error") || keyword_match("error") {
            "bug-fix".to_string()
        } else if lower.contains("optimize") || lower.contains("performance") || lower.contains("faster") {
            "performance-improvement".to_string()
        } else if lower.contains("refactor") || lower.contains("improve") {
            "refactoring".to_string()
        } else if lower.contains("add") || lower.contains("implement") || lower.contains("create") {
            "feature-implementation".to_string()
        } else if lower.contains("analyze") || lower.contains("review") || lower.contains("inspect") {
            "analysis".to_string()
        } else if keyword_match("agent") || keyword_match("streaming") {
            "agent-flow".to_string()
        } else {
            "general".to_string()
        }
    }

    fn build_focus_summary(task_kind: &str, keywords: &[String], suspected_files: &[SuspectedFile]) -> String {
        let top_files = suspected_files
            .iter()
            .take(3)
            .map(|file| file.path.clone())
            .collect::<Vec<_>>();

        let keyword_hint = if keywords.is_empty() {
            "no strong keyword matches".to_string()
        } else {
            keywords.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
        };

        let file_hint = if top_files.is_empty() {
            "no direct file candidate".to_string()
        } else {
            top_files.join(", ")
        };

        format!(
            "{} task: focus on {}, then verify with the narrowest safe check.",
            task_kind, if top_files.is_empty() { keyword_hint } else { file_hint }
        )
    }

    /// Extract keywords from problem statement
    fn extract_keywords(problem_statement: &str) -> Vec<String> {
        let mut keywords = Vec::new();
        let lower = problem_statement.to_lowercase();

        // Extract explicit mentions (e.g., "ChatPanel.tsx", "agent_streaming.rs")
        let file_mention_regex = Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*\.(?:tsx?|rs|jsx?|py|java|go))").unwrap();
        for cap in file_mention_regex.captures_iter(problem_statement) {
            keywords.push(cap[1].to_string());
        }

        // Extract error/issue keywords
        let error_keywords = vec![
            "error", "bug", "crash", "fail", "broken", "blank", "missing", "not showing",
            "not working", "issue", "problem", "wrong", "incorrect", "syntax", "type",
            "undefined", "null", "reference", "import", "export", "module", "component",
            "function", "variable", "constant", "class", "interface", "type", "enum",
        ];

        for keyword in error_keywords {
            if lower.contains(keyword) {
                keywords.push(keyword.to_string());
            }
        }

        // Extract technology/framework keywords
        let tech_keywords = vec![
            "react", "typescript", "rust", "tauri", "json", "xml", "html", "css",
            "javascript", "node", "npm", "cargo", "build", "compile", "render",
            "streaming", "agent", "tool", "command", "file", "path", "regex",
        ];

        for keyword in tech_keywords {
            if lower.contains(keyword) {
                keywords.push(keyword.to_string());
            }
        }

        // Remove duplicates and sort by frequency
        let mut unique_keywords: Vec<String> = keywords.into_iter().collect::<HashSet<_>>().into_iter().collect();
        unique_keywords.sort();
        unique_keywords
    }

    /// Infer file patterns based on keywords
    fn infer_file_patterns(keywords: &[String]) -> Vec<String> {
        let mut patterns = Vec::new();

        for keyword in keywords {
            let lower = keyword.to_lowercase();

            // File-specific patterns
            if lower.contains("chatpanel") || lower.contains("chat") {
                patterns.push("**/Chat/**/*.tsx".to_string());
                patterns.push("**/Chat/**/*.ts".to_string());
            }
            if lower.contains("streaming") {
                patterns.push("**/agent_streaming.rs".to_string());
                patterns.push("**/streaming*.rs".to_string());
                patterns.push("**/streaming*.tsx".to_string());
            }
            if lower.contains("component") || lower.contains("react") {
                patterns.push("**/components/**/*.tsx".to_string());
                patterns.push("**/components/**/*.ts".to_string());
            }
            if lower.contains("agent") {
                patterns.push("**/commands/agent*.rs".to_string());
                patterns.push("**/commands/*agent*.rs".to_string());
            }
            if lower.contains("error") || lower.contains("recovery") {
                patterns.push("**/error_recovery.rs".to_string());
                patterns.push("**/diagnostics*.rs".to_string());
            }
            if lower.contains("prompt") {
                patterns.push("**/prompts.rs".to_string());
                patterns.push("**/prompt*.rs".to_string());
            }
            if lower.contains("hook") {
                patterns.push("**/hooks/**/*.ts".to_string());
                patterns.push("**/hooks/**/*.tsx".to_string());
            }
            if lower.contains("type") || lower.contains("interface") {
                patterns.push("**/types/**/*.ts".to_string());
                patterns.push("**/types/index.ts".to_string());
            }
        }

        // Remove duplicates
        patterns.sort();
        patterns.dedup();
        patterns
    }

    /// Generate targeted search queries
    fn generate_search_queries(keywords: &[String], _file_patterns: &[String]) -> Vec<SearchQuery> {
        let mut queries = Vec::new();
        let mut priority = 100;

        for keyword in keywords {
            let lower = keyword.to_lowercase();

            // High-priority searches for explicit issues
            if lower.contains("blank") || lower.contains("not showing") {
                queries.push(SearchQuery {
                    pattern: "setContent|setDisplay|setVisible|return null".to_string(),
                    file_pattern: "**/*.tsx".to_string(),
                    priority,
                    reason: "Searching for content clearing or visibility logic".to_string(),
                });
                priority = priority.saturating_sub(5);
            }

            if lower.contains("xml") || lower.contains("thought") || lower.contains("tag") {
                queries.push(SearchQuery {
                    pattern: r"<thought>|<think>|\[THOUGHT\]|\[REASONING\]".to_string(),
                    file_pattern: "**/*.rs".to_string(),
                    priority,
                    reason: "Searching for XML tag references".to_string(),
                });
                priority = priority.saturating_sub(5);

                queries.push(SearchQuery {
                    pattern: r"<thought>|<think>|\[THOUGHT\]|\[REASONING\]".to_string(),
                    file_pattern: "**/*.tsx".to_string(),
                    priority,
                    reason: "Searching for XML tag references in TypeScript".to_string(),
                });
                priority = priority.saturating_sub(5);
            }

            if lower.contains("error") || lower.contains("fail") {
                queries.push(SearchQuery {
                    pattern: "contains.*error|contains.*Error|has_errors".to_string(),
                    file_pattern: "**/*.rs".to_string(),
                    priority,
                    reason: "Searching for error detection logic".to_string(),
                });
                priority = priority.saturating_sub(5);
            }

            if lower.contains("json") {
                queries.push(SearchQuery {
                    pattern: r#""thought"|"tool"|"args""#.to_string(),
                    file_pattern: "**/*.rs".to_string(),
                    priority,
                    reason: "Searching for JSON key references".to_string(),
                });
                priority = priority.saturating_sub(5);
            }

            if lower.contains("streaming") {
                queries.push(SearchQuery {
                    pattern: "streaming|stream|token".to_string(),
                    file_pattern: "**/*.rs".to_string(),
                    priority,
                    reason: "Searching for streaming-related code".to_string(),
                });
                priority = priority.saturating_sub(5);
            }
        }

        // Sort by priority (highest first)
        queries.sort_by(|a, b| b.priority.cmp(&a.priority));
        queries
    }

    /// Infer suspected files based on keywords
    fn infer_suspected_files(keywords: &[String]) -> Vec<SuspectedFile> {
        let mut suspected = Vec::new();

        for keyword in keywords {
            let lower = keyword.to_lowercase();

            // Direct file mentions
            if lower.contains("chatpanel") {
                suspected.push(SuspectedFile {
                    path: "src/components/Chat/ChatPanel.tsx".to_string(),
                    relevance_score: 100,
                    reason: "Explicitly mentioned in problem statement".to_string(),
                    file_type: "component".to_string(),
                });
            }

            if lower.contains("streaming") && lower.contains("display") {
                suspected.push(SuspectedFile {
                    path: "src/components/Chat/StreamingDisplay.tsx".to_string(),
                    relevance_score: 95,
                    reason: "Streaming display component".to_string(),
                    file_type: "component".to_string(),
                });
            }

            if lower.contains("agent_streaming") || (lower.contains("agent") && lower.contains("streaming")) {
                suspected.push(SuspectedFile {
                    path: "src-tauri/src/commands/agent_streaming.rs".to_string(),
                    relevance_score: 95,
                    reason: "Agent streaming logic".to_string(),
                    file_type: "backend".to_string(),
                });
            }

            if lower.contains("prompt") {
                suspected.push(SuspectedFile {
                    path: "src-tauri/src/commands/prompts.rs".to_string(),
                    relevance_score: 90,
                    reason: "System prompts and configurations".to_string(),
                    file_type: "backend".to_string(),
                });
            }

            if lower.contains("hook") || lower.contains("listener") {
                suspected.push(SuspectedFile {
                    path: "src/hooks/useAppEventListeners.ts".to_string(),
                    relevance_score: 85,
                    reason: "Event listeners and hooks".to_string(),
                    file_type: "hook".to_string(),
                });
            }

            if lower.contains("error") || lower.contains("recovery") {
                suspected.push(SuspectedFile {
                    path: "src-tauri/src/commands/error_recovery.rs".to_string(),
                    relevance_score: 80,
                    reason: "Error recovery system".to_string(),
                    file_type: "backend".to_string(),
                });
            }

            if lower.contains("type") || lower.contains("interface") {
                suspected.push(SuspectedFile {
                    path: "src/types/index.ts".to_string(),
                    relevance_score: 75,
                    reason: "Type definitions".to_string(),
                    file_type: "types".to_string(),
                });
            }
        }

        // Sort by relevance score (highest first)
        suspected.sort_by(|a, b| b.relevance_score.cmp(&a.relevance_score));
        suspected.dedup_by(|a, b| a.path == b.path);
        suspected
    }

    /// Generate investigation strategy
    fn generate_strategy(keywords: &[String]) -> String {
        let mut strategy = String::new();
        let lower_keywords: Vec<String> = keywords.iter().map(|k| k.to_lowercase()).collect();

        strategy.push_str("## Investigation Strategy\n\n");

        // Phase 1: Targeted Search
        strategy.push_str("### Phase 1: Targeted Search\n");
        strategy.push_str("1. Start with workspace search (`semantic_search`) using the issue keywords to narrow likely files and code blocks\n");
        strategy.push_str("2. Use find_symbols when a function, class, or identifier is known\n");
        strategy.push_str("3. Use grepSearch/search_files only to confirm exact locations after the search space is narrowed\n");
        strategy.push_str("4. Focus on explicitly mentioned files first\n\n");

        // Phase 2: Context Analysis
        strategy.push_str("### Phase 2: Context Analysis\n");
        strategy.push_str("- Read only the most likely file or a narrow line window first\n");
        strategy.push_str("- Avoid rereading the same file repeatedly; reuse what you already saw and inspect related files or narrower ranges instead\n");
        if lower_keywords.iter().any(|k| k.contains("xml") || k.contains("tag")) {
            strategy.push_str("- Check for XML tag patterns and their usage\n");
        }
        if lower_keywords.iter().any(|k| k.contains("error")) {
            strategy.push_str("- Analyze error detection and handling logic\n");
        }
        if lower_keywords.iter().any(|k| k.contains("streaming")) {
            strategy.push_str("- Review streaming content handling\n");
        }
        strategy.push_str("\n");

        // Phase 3: Impact Assessment
        strategy.push_str("### Phase 3: Impact Assessment\n");
        strategy.push_str("1. Determine which files have functional impact\n");
        strategy.push_str("2. Prioritize by severity (UI > logic > logging)\n");
        strategy.push_str("3. Identify dependencies between files\n\n");

        // Phase 4: Targeted Fix
        strategy.push_str("### Phase 4: Targeted Fix\n");
        strategy.push_str("1. Fix high-impact files first\n");
        strategy.push_str("2. Verify changes don't break other files\n");
        strategy.push_str("3. Test systematically\n");

        strategy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords() {
        let problem = "ChatPanel.tsx has XML thought tags that need to be removed";
        let keywords = ProblemIdentifier::extract_keywords(problem);
        assert!(keywords.contains(&"ChatPanel.tsx".to_string()));
        assert!(keywords.contains(&"xml".to_string()));
        assert!(keywords.contains(&"thought".to_string()));
    }

    #[test]
    fn test_infer_file_patterns() {
        let keywords = vec!["streaming".to_string(), "agent".to_string()];
        let patterns = ProblemIdentifier::infer_file_patterns(&keywords);
        assert!(patterns.iter().any(|p| p.contains("streaming")));
        assert!(patterns.iter().any(|p| p.contains("agent")));
    }

    #[test]
    fn test_analyze_problem() {
        let problem = "agent_streaming.rs still has <thought> tag references";
        let analysis = ProblemIdentifier::analyze_problem(problem);
        assert!(!analysis.keywords.is_empty());
        assert!(!analysis.search_queries.is_empty());
        assert!(!analysis.suspected_files.is_empty());
        assert!(!analysis.task_kind.is_empty());
    }
}
