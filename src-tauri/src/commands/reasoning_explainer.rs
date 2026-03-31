use serde::{Deserialize, Serialize};

/// Reasoning explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningExplanation {
    pub action: String,
    pub why: Vec<String>,
    pub expected_outcome: String,
    pub alternatives: Vec<String>,
    pub risks: Vec<String>,
}

/// Reasoning Explainer Engine
pub struct ReasoningExplainerEngine;

impl ReasoningExplainerEngine {
    pub fn new() -> Self {
        Self
    }

    /// Generate reasoning explanation for a tool call
    pub fn explain_reasoning(
        &self,
        tool: &str,
        args: &serde_json::Value,
        task_context: &str,
        previous_results: &[String],
    ) -> ReasoningExplanation {
        let action = self.describe_action(tool, args);
        let why = self.explain_why(tool, args, task_context, previous_results);
        let expected_outcome = self.describe_expected_outcome(tool, args);
        let alternatives = self.suggest_alternatives(tool, args, task_context);
        let risks = self.identify_risks(tool, args);

        ReasoningExplanation {
            action,
            why,
            expected_outcome,
            alternatives,
            risks,
        }
    }

    /// Describe what action is being taken
    fn describe_action(&self, tool: &str, args: &serde_json::Value) -> String {
        match tool {
            "read_file" => {
                if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                    format!("Reading file: {}", path)
                } else {
                    "Reading file".to_string()
                }
            }
            "write_file" => {
                if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                    format!("Writing to file: {}", path)
                } else {
                    "Writing to file".to_string()
                }
            }
            "edit_file" => {
                if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                    format!("Editing file: {}", path)
                } else {
                    "Editing file".to_string()
                }
            }
            "run_command" => {
                if let Some(cmd) = args.get("command").and_then(|c| c.as_str()) {
                    format!("Running command: {}", cmd)
                } else {
                    "Running command".to_string()
                }
            }
            "grep_search" => {
                if let Some(query) = args.get("query").and_then(|q| q.as_str()) {
                    format!("Searching for: {}", query)
                } else {
                    "Searching".to_string()
                }
            }
            "semantic_search" => {
                if let Some(query) = args.get("query").and_then(|q| q.as_str()) {
                    format!("Semantic search for: {}", query)
                } else {
                    "Semantic search".to_string()
                }
            }
            _ => format!("Calling tool: {}", tool),
        }
    }

    /// Explain why this action is being taken
    fn explain_why(
        &self,
        tool: &str,
        _args: &serde_json::Value,
        task_context: &str,
        previous_results: &[String],
    ) -> Vec<String> {
        let mut reasons = Vec::new();

        match tool {
            "read_file" => {
                reasons.push("To understand the current state of the code".to_string());
                if task_context.to_lowercase().contains("understand") {
                    reasons.push("The task requires understanding the existing implementation".to_string());
                }
                if task_context.to_lowercase().contains("modify") {
                    reasons.push("Need to see the current content before making changes".to_string());
                }
            }
            "write_file" => {
                reasons.push("To create or update a file with new content".to_string());
                if task_context.to_lowercase().contains("create") {
                    reasons.push("Creating a new file as part of the task".to_string());
                }
                if task_context.to_lowercase().contains("implement") {
                    reasons.push("Implementing the required functionality".to_string());
                }
            }
            "run_command" => {
                reasons.push("To execute a command and see the results".to_string());
                if task_context.to_lowercase().contains("test") {
                    reasons.push("Running tests to verify the implementation".to_string());
                }
                if task_context.to_lowercase().contains("build") {
                    reasons.push("Building the project to check for errors".to_string());
                }
            }
            "grep_search" => {
                reasons.push("To find specific patterns in the codebase".to_string());
                if previous_results.is_empty() {
                    reasons.push("Starting the search for relevant code".to_string());
                } else {
                    reasons.push("Refining the search based on previous results".to_string());
                }
            }
            "semantic_search" => {
                reasons.push("To find semantically related code".to_string());
                reasons.push("This helps understand the broader context".to_string());
            }
            _ => {
                reasons.push(format!("To accomplish part of the task using {}", tool));
            }
        }

        reasons
    }

    /// Describe the expected outcome
    fn describe_expected_outcome(&self, tool: &str, args: &serde_json::Value) -> String {
        match tool {
            "read_file" => {
                if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                    format!("Will get the contents of {}", path)
                } else {
                    "Will get file contents".to_string()
                }
            }
            "write_file" => {
                if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                    format!("Will create or overwrite {}", path)
                } else {
                    "Will create or overwrite a file".to_string()
                }
            }
            "edit_file" => {
                if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                    format!("Will modify specific parts of {}", path)
                } else {
                    "Will modify a file".to_string()
                }
            }
            "run_command" => {
                if let Some(cmd) = args.get("command").and_then(|c| c.as_str()) {
                    format!("Will execute '{}' and return the output", cmd)
                } else {
                    "Will execute a command and return the output".to_string()
                }
            }
            "grep_search" => {
                "Will find all files containing the search pattern".to_string()
            }
            "semantic_search" => {
                "Will find semantically related code snippets".to_string()
            }
            _ => format!("Will execute {} and return results", tool),
        }
    }

    /// Suggest alternatives
    fn suggest_alternatives(&self, tool: &str, _args: &serde_json::Value, _task_context: &str) -> Vec<String> {
        match tool {
            "read_file" => {
                vec![
                    "Could use grep_search to find specific patterns".to_string(),
                    "Could use semantic_search for broader context".to_string(),
                ]
            }
            "write_file" => {
                vec![
                    "Could use edit_file for incremental changes".to_string(),
                    "Could use multi_edit_file for multiple changes".to_string(),
                ]
            }
            "run_command" => {
                vec![
                    "Could check documentation instead".to_string(),
                    "Could search for similar patterns in code".to_string(),
                ]
            }
            "grep_search" => {
                vec![
                    "Could use semantic_search for better results".to_string(),
                    "Could read specific files directly".to_string(),
                ]
            }
            _ => vec![],
        }
    }

    /// Identify potential risks
    fn identify_risks(&self, tool: &str, args: &serde_json::Value) -> Vec<String> {
        let mut risks = Vec::new();

        match tool {
            "write_file" => {
                risks.push("Will overwrite existing file content".to_string());
                if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                    if path.contains("package.json") || path.contains("config") {
                        risks.push("This is a critical configuration file".to_string());
                    }
                }
            }
            "run_command" => {
                risks.push("Command execution could have side effects".to_string());
                if let Some(cmd) = args.get("command").and_then(|c| c.as_str()) {
                    if cmd.contains("rm") || cmd.contains("delete") {
                        risks.push("This command could delete files".to_string());
                    }
                    if cmd.contains("npm install") || cmd.contains("pip install") {
                        risks.push("This could modify dependencies".to_string());
                    }
                }
            }
            _ => {}
        }

        risks
    }
}

/// Format reasoning for display
#[allow(dead_code)]
pub fn format_reasoning_for_display(reasoning: &ReasoningExplanation) -> String {
    format!(
        "[REASONING] {}\n\n\
         WHY:\n{}\n\n\
         EXPECTED OUTCOME:\n{}\n\n\
         ALTERNATIVES:\n{}\n\n\
         RISKS:\n{}",
        reasoning.action,
        reasoning
            .why
            .iter()
            .map(|w| format!("• {}", w))
            .collect::<Vec<_>>()
            .join("\n"),
        reasoning.expected_outcome,
        if reasoning.alternatives.is_empty() {
            "None".to_string()
        } else {
            reasoning
                .alternatives
                .iter()
                .map(|a| format!("• {}", a))
                .collect::<Vec<_>>()
                .join("\n")
        },
        if reasoning.risks.is_empty() {
            "None".to_string()
        } else {
            reasoning
                .risks
                .iter()
                .map(|r| format!("⚠️ {}", r))
                .collect::<Vec<_>>()
                .join("\n")
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain_read_file() {
        let engine = ReasoningExplainerEngine::new();
        let reasoning = engine.explain_reasoning(
            "read_file",
            &serde_json::json!({ "path": "src/main.rs" }),
            "Understand the main file",
            &[],
        );

        assert!(reasoning.action.contains("src/main.rs"));
        assert!(!reasoning.why.is_empty());
        assert!(!reasoning.expected_outcome.is_empty());
    }

    #[test]
    fn test_explain_write_file() {
        let engine = ReasoningExplainerEngine::new();
        let reasoning = engine.explain_reasoning(
            "write_file",
            &serde_json::json!({ "path": "src/new.rs" }),
            "Create a new file",
            &[],
        );

        assert!(reasoning.action.contains("src/new.rs"));
        assert!(!reasoning.risks.is_empty());
    }

    #[test]
    fn test_format_reasoning() {
        let reasoning = ReasoningExplanation {
            action: "Reading file".to_string(),
            why: vec!["To understand the code".to_string()],
            expected_outcome: "Will get file contents".to_string(),
            alternatives: vec!["Use grep search".to_string()],
            risks: vec![],
        };

        let formatted = format_reasoning_for_display(&reasoning);
        assert!(formatted.contains("REASONING"));
        assert!(formatted.contains("Reading file"));
    }
}
