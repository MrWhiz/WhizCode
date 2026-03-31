use serde::{Deserialize, Serialize};

/// Detected loop pattern
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LoopPattern {
    ReadingSameFile(String),
    WritingSameFile(String),
    RunningCommand(String),
    SearchingPattern(String),
    CallingTool(String),
    Unknown,
}

impl std::fmt::Display for LoopPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopPattern::ReadingSameFile(path) => write!(f, "ReadingSameFile({})", path),
            LoopPattern::WritingSameFile(path) => write!(f, "WritingSameFile({})", path),
            LoopPattern::RunningCommand(cmd) => write!(f, "RunningCommand({})", cmd),
            LoopPattern::SearchingPattern(pattern) => write!(f, "SearchingPattern({})", pattern),
            LoopPattern::CallingTool(tool) => write!(f, "CallingTool({})", tool),
            LoopPattern::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
}

/// Loop recovery guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopRecoveryGuidance {
    pub pattern: String,
    pub analysis: String,
    pub suggestions: Vec<String>,
    pub next_step: String,
    pub confidence: f32,
}

/// Loop Recovery Engine
pub struct LoopRecoveryEngine;

impl LoopRecoveryEngine {
    pub fn new() -> Self {
        Self
    }

    /// Analyze loop pattern and generate recovery guidance
    pub fn analyze_and_recover(
        &self,
        tool_calls: &[ToolCall],
        tool_results: &[String],
        iteration: u32,
    ) -> LoopRecoveryGuidance {
        let pattern = self.detect_pattern(tool_calls);
        let analysis = self.analyze_pattern(&pattern, tool_calls, tool_results, iteration);
        let suggestions = self.generate_suggestions(&pattern, tool_calls, tool_results);
        let next_step = self.recommend_next_step(&pattern, &suggestions);
        let confidence = self.calculate_confidence(&pattern, &suggestions);

        LoopRecoveryGuidance {
            pattern: pattern.to_string(),
            analysis,
            suggestions,
            next_step,
            confidence,
        }
    }

    /// Detect the loop pattern
    fn detect_pattern(&self, tool_calls: &[ToolCall]) -> LoopPattern {
        if tool_calls.is_empty() {
            return LoopPattern::Unknown;
        }

        let tool = &tool_calls[0].tool;
        let args = &tool_calls[0].args;

        match tool.as_str() {
            "read_file" => {
                if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                    LoopPattern::ReadingSameFile(path.to_string())
                } else {
                    LoopPattern::Unknown
                }
            }
            "write_file" | "edit_file" | "multi_edit_file" => {
                if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                    LoopPattern::WritingSameFile(path.to_string())
                } else {
                    LoopPattern::Unknown
                }
            }
            "run_command" => {
                if let Some(cmd) = args.get("command").and_then(|c| c.as_str()) {
                    LoopPattern::RunningCommand(cmd.to_string())
                } else {
                    LoopPattern::Unknown
                }
            }
            "grep_search" | "search_files" | "semantic_search" => {
                if let Some(query) = args.get("query").and_then(|q| q.as_str()) {
                    LoopPattern::SearchingPattern(query.to_string())
                } else if let Some(pattern) = args.get("pattern").and_then(|p| p.as_str()) {
                    LoopPattern::SearchingPattern(pattern.to_string())
                } else {
                    LoopPattern::Unknown
                }
            }
            _ => LoopPattern::CallingTool(tool.to_string()),
        }
    }

    /// Analyze why the loop is happening
    fn analyze_pattern(
        &self,
        pattern: &LoopPattern,
        _tool_calls: &[ToolCall],
        _tool_results: &[String],
        iteration: u32,
    ) -> String {
        match pattern {
            LoopPattern::ReadingSameFile(path) => {
                format!(
                    "You've read {} {} times. The file content isn't changing, \
                     so reading again won't provide new information. This suggests you're \
                     looking for something that either:\n\
                     1. Doesn't exist in this file\n\
                     2. Requires understanding from other files\n\
                     3. Needs to be created or modified",
                    path, iteration
                )
            }
            LoopPattern::WritingSameFile(path) => {
                format!(
                    "You've written to {} {} times. Multiple writes to the same file suggest:\n\
                     1. Uncertainty about the correct changes\n\
                     2. Incremental changes that could be combined\n\
                     3. Need for validation before proceeding",
                    path, iteration
                )
            }
            LoopPattern::RunningCommand(cmd) => {
                format!(
                    "You've run '{}' {} times. The command output isn't changing, which means:\n\
                     1. The underlying issue hasn't been fixed\n\
                     2. The command needs different parameters\n\
                     3. A different approach is needed",
                    cmd, iteration
                )
            }
            LoopPattern::SearchingPattern(pattern) => {
                format!(
                    "You've searched for '{}' {} times. The search results aren't changing, \
                     which suggests:\n\
                     1. The pattern is too broad or too narrow\n\
                     2. The information doesn't exist in the codebase\n\
                     3. A different search strategy is needed",
                    pattern, iteration
                )
            }
            LoopPattern::CallingTool(tool) => {
                format!(
                    "You've called {} {} times. Repetition without progress suggests:\n\
                     1. The tool isn't providing the expected results\n\
                     2. The tool parameters need adjustment\n\
                     3. A different tool would be more effective",
                    tool, iteration
                )
            }
            LoopPattern::Unknown => {
                format!(
                    "You're repeating the same action {} times. This suggests the current \
                     approach isn't working and a different strategy is needed.",
                    iteration
                )
            }
        }
    }

    /// Generate specific suggestions for recovery
    fn generate_suggestions(
        &self,
        pattern: &LoopPattern,
        _tool_calls: &[ToolCall],
        _tool_results: &[String],
    ) -> Vec<String> {
        match pattern {
            LoopPattern::ReadingSameFile(path) => {
                vec![
                    format!("Analyze the content you already have from {} and make a change", path),
                    "Search for related files that might have the information you need".to_string(),
                    "Run a command to understand the codebase structure better".to_string(),
                    "Check if the information needs to be created or added to the file".to_string(),
                    "Move to a different file that might have the context you need".to_string(),
                ]
            }
            LoopPattern::WritingSameFile(path) => {
                vec![
                    format!("Verify the current content of {} is correct", path),
                    "Run tests to validate the changes work as expected".to_string(),
                    "Check if the changes are complete or if more modifications are needed".to_string(),
                    format!("Move to the next file if {} is done", path),
                    "Review the changes against the acceptance criteria".to_string(),
                ]
            }
            LoopPattern::RunningCommand(_cmd) => {
                vec![
                    "Analyze the command output you have and identify the root cause".to_string(),
                    "Try a different command to debug the issue".to_string(),
                    "Check if there's a configuration issue that needs to be fixed".to_string(),
                    "Modify the command parameters to get different output".to_string(),
                    "Try a completely different approach to solve the problem".to_string(),
                ]
            }
            LoopPattern::SearchingPattern(pattern) => {
                vec![
                    format!("Refine your search pattern to be more specific than '{}'", pattern),
                    "Search in a different location or with different scope".to_string(),
                    "Use a different search tool (semantic vs grep vs file search)".to_string(),
                    "Check if the information exists in the codebase at all".to_string(),
                    "Try searching for related terms or concepts".to_string(),
                ]
            }
            LoopPattern::CallingTool(_) => {
                vec![
                    "Try a different tool that might be more effective".to_string(),
                    "Modify the tool parameters to get different results".to_string(),
                    "Analyze the results you have and try a different approach".to_string(),
                    "Check if the tool is the right one for this task".to_string(),
                    "Break the task into smaller steps with different tools".to_string(),
                ]
            }
            LoopPattern::Unknown => {
                vec![
                    "Try a completely different approach".to_string(),
                    "Break the task into smaller, more manageable steps".to_string(),
                    "Review what you've learned so far and adjust strategy".to_string(),
                    "Check if there's a prerequisite step you're missing".to_string(),
                    "Consider if the task is achievable with current information".to_string(),
                ]
            }
        }
    }

    /// Recommend the next step
    fn recommend_next_step(&self, _pattern: &LoopPattern, suggestions: &[String]) -> String {
        if let Some(first_suggestion) = suggestions.first() {
            format!("Next step: {}", first_suggestion)
        } else {
            "Next step: Try a different approach".to_string()
        }
    }

    /// Calculate confidence in the recovery guidance
    fn calculate_confidence(&self, pattern: &LoopPattern, suggestions: &[String]) -> f32 {
        let base_confidence = match pattern {
            LoopPattern::ReadingSameFile(_) => 0.95,
            LoopPattern::WritingSameFile(_) => 0.90,
            LoopPattern::RunningCommand(_) => 0.85,
            LoopPattern::SearchingPattern(_) => 0.80,
            LoopPattern::CallingTool(_) => 0.75,
            LoopPattern::Unknown => 0.60,
        };

        let suggestion_factor = (suggestions.len() as f32 / 5.0).min(1.0);
        (base_confidence * suggestion_factor).min(1.0)
    }
}

/// Format guidance for display to the agent
pub fn format_guidance_for_agent(guidance: &LoopRecoveryGuidance) -> String {
    format!(
        "[SYSTEM] 🔄 LOOP DETECTED: {}\n\n\
         ANALYSIS:\n{}\n\n\
         SUGGESTIONS:\n{}\n\n\
         {}\n\n\
         Confidence: {:.0}%",
        guidance.pattern,
        guidance.analysis,
        guidance
            .suggestions
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}. ✅ {}", i + 1, s))
            .collect::<Vec<_>>()
            .join("\n"),
        guidance.next_step,
        guidance.confidence * 100.0
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_reading_same_file_pattern() {
        let engine = LoopRecoveryEngine::new();
        let tool_calls = vec![ToolCall {
            tool: "read_file".to_string(),
            args: serde_json::json!({ "path": "src/main.rs" }),
        }];

        let pattern = engine.detect_pattern(&tool_calls);
        assert_eq!(pattern, LoopPattern::ReadingSameFile("src/main.rs".to_string()));
    }

    #[test]
    fn test_detect_running_command_pattern() {
        let engine = LoopRecoveryEngine::new();
        let tool_calls = vec![ToolCall {
            tool: "run_command".to_string(),
            args: serde_json::json!({ "command": "npm run build" }),
        }];

        let pattern = engine.detect_pattern(&tool_calls);
        assert_eq!(pattern, LoopPattern::RunningCommand("npm run build".to_string()));
    }

    #[test]
    fn test_generate_recovery_guidance() {
        let engine = LoopRecoveryEngine::new();
        let tool_calls = vec![ToolCall {
            tool: "read_file".to_string(),
            args: serde_json::json!({ "path": "App.tsx" }),
        }];

        let guidance = engine.analyze_and_recover(&tool_calls, &[], 3);
        assert!(!guidance.suggestions.is_empty());
        assert!(guidance.confidence > 0.0);
    }

    #[test]
    fn test_format_guidance() {
        let guidance = LoopRecoveryGuidance {
            pattern: "ReadingSameFile(App.tsx)".to_string(),
            analysis: "Test analysis".to_string(),
            suggestions: vec!["Suggestion 1".to_string(), "Suggestion 2".to_string()],
            next_step: "Try suggestion 1".to_string(),
            confidence: 0.95,
        };

        let formatted = format_guidance_for_agent(&guidance);
        assert!(formatted.contains("LOOP DETECTED"));
        assert!(formatted.contains("Suggestion 1"));
        assert!(formatted.contains("95%"));
    }
}
