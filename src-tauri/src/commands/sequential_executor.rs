use crate::commands::streaming_tool_executor::{ToolCall, ToolExecutionEvent, StreamingToolExecutor};
use tokio::sync::mpsc;

pub struct SequentialToolExecutor {
    executor: StreamingToolExecutor,
    llm_callback: Option<Box<dyn Fn(String) -> String + Send>>,
}

impl SequentialToolExecutor {
    #[allow(dead_code)]
    pub fn new(event_tx: mpsc::UnboundedSender<ToolExecutionEvent>) -> Self {
        Self {
            executor: StreamingToolExecutor::new(event_tx),
            llm_callback: None,
        }
    }

    /// Set the LLM callback for error recovery
    #[allow(dead_code)]
    pub fn set_llm_callback<F>(&mut self, callback: F)
    where
        F: Fn(String) -> String + Send + 'static,
    {
        self.llm_callback = Some(Box::new(callback));
    }

    /// Execute tools sequentially from streaming LLM response
    #[allow(dead_code)]
    pub async fn execute_from_stream(
        &mut self,
        stream_response: &str,
        tool_executor: impl Fn(&ToolCall) -> Result<String, String>,
    ) -> Result<Vec<(ToolCall, String)>, String> {
        // Parse streaming JSON and queue tools
        self.executor.parse_streaming_json(stream_response).await;

        let mut results = Vec::new();

        // Execute tools sequentially
        while let Some(tool) = self.executor.get_next_tool() {
            // Execute the tool
            match tool_executor(&tool) {
                Ok(result) => {
                    self.executor.mark_tool_completed(result.clone()).await;
                    results.push((tool, result));
                }
                Err(error) => {
                    self.executor.mark_tool_failed(error.clone()).await;

                    // Get LLM opinion on failure
                    if let Some(ref callback) = self.llm_callback {
                        let error_context = format!(
                            "Tool '{}' failed with error: {}\n\
                             Args were: {}\n\
                             What should I do? Should I retry, skip, or try a different approach?",
                            tool.tool,
                            error,
                            serde_json::to_string(&tool.args).unwrap_or_else(|_| "{}".to_string())
                        );

                        let llm_response = callback(error_context);
                        eprintln!("[SequentialExecutor] LLM recovery suggestion: {}", llm_response);

                        // Check if LLM suggests retry
                        if llm_response.to_lowercase().contains("retry") {
                            // Retry the tool
                            match tool_executor(&tool) {
                                Ok(result) => {
                                    self.executor.mark_tool_completed(result.clone()).await;
                                    results.push((tool, result));
                                }
                                Err(retry_error) => {
                                    self.executor.mark_tool_failed(retry_error.clone()).await;
                                    // Skip this tool and continue
                                    eprintln!("[SequentialExecutor] Retry failed, skipping tool");
                                }
                            }
                        } else if llm_response.to_lowercase().contains("skip") {
                            // Skip this tool
                            eprintln!("[SequentialExecutor] Skipping tool as suggested by LLM");
                        } else {
                            // Try alternative approach (for now, just skip)
                            eprintln!("[SequentialExecutor] Trying alternative approach (skipping for now)");
                        }
                    } else {
                        // No LLM callback, just skip
                        eprintln!("[SequentialExecutor] No LLM callback, skipping failed tool");
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get the executor for direct access
    #[allow(dead_code)]
    pub fn executor(&mut self) -> &mut StreamingToolExecutor {
        &mut self.executor
    }
}

/// Helper function to convert tool execution results to LLM-friendly format
#[allow(dead_code)]
pub fn format_tool_results(results: &[(ToolCall, String)]) -> String {
    let mut formatted = String::from("Tool execution results:\n\n");

    for (tool, result) in results {
        formatted.push_str(&format!(
            "[{}] (args: {})\n{}\n\n",
            tool.tool,
            serde_json::to_string(&tool.args).unwrap_or_else(|_| "{}".to_string()),
            result
        ));
    }

    formatted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sequential_execution() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut executor = SequentialToolExecutor::new(tx);

        let stream = r#"
{"tool": "read_file", "args": {"path": "/test.txt"}}
{"tool": "write_file", "args": {"path": "/out.txt", "content": "hello"}}
"#;

        let tool_executor = |tool: &ToolCall| -> Result<String, String> {
            match tool.tool.as_str() {
                "read_file" => Ok("file contents".to_string()),
                "write_file" => Ok("file written".to_string()),
                _ => Err("unknown tool".to_string()),
            }
        };

        let results = executor.execute_from_stream(stream, tool_executor).await;
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 2);
    }
}
