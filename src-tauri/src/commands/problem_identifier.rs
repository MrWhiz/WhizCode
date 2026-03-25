/// Intelligent Problem Identifier
/// 
/// This module implements smart problem identification that:
/// 1. Parses the problem statement to extract keywords
/// 2. Uses targeted searches instead of reading all files
/// 3. Prioritizes files by relevance and impact
/// 4. Provides focused context for problem-solving

use std::collections::HashSet;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemAnalysis {
    pub keywords: Vec<String>,
    pub file_patterns: Vec<String>,
    pub search_queries: Vec<SearchQuery>,
    pub suspected_files: Vec<SuspectedFile>,
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
        let investigation_strategy = Self::generate_strategy(&keywords);

        ProblemAnalysis {
            keywords,
            file_patterns,
            search_queries,
            suspected_files,
            investigation_strategy,
        }
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
        strategy.push_str("1. Use grepSearch with high-priority patterns\n");
        strategy.push_str("2. Focus on explicitly mentioned files first\n");
        strategy.push_str("3. Identify exact locations of issues\n\n");

        // Phase 2: Context Analysis
        strategy.push_str("### Phase 2: Context Analysis\n");
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
    }
}
