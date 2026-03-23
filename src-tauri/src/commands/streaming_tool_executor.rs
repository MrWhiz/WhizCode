use serde_json::{json, Value};
use std::collections::VecDeque;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub struct ToolCall {
    pub tool: String,
    pub args: Value,
    pub id: String,
}

#[derive(Clone, Debug)]
pub enum ToolExecutionEvent {
    /// Tool call identified from streaming JSON
    ToolIdentified {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        tool: String,
        #[allow(dead_code)]
        args: Value,
    },
    /// Tool execution started
    #[allow(dead_code)]
    ToolStarted {
        id: String,
    },
    /// Tool execution completed
    ToolCompleted {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        result: String,
    },
    /// Tool execution failed
    ToolFailed {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        error: String,
    },
    /// All tools from current LLM response have been identified
    AllToolsIdentified,
}

pub struct StreamingToolExecutor {
    /// Queue of tool calls waiting to be executed
    tool_queue: VecDeque<ToolCall>,
    /// Currently executing tool
    current_tool: Option<ToolCall>,
    /// Event sender for frontend updates
    event_tx: mpsc::UnboundedSender<ToolExecutionEvent>,
    /// Tool counter for unique IDs
    tool_counter: u32,
}

impl StreamingToolExecutor {
    pub fn new(event_tx: mpsc::UnboundedSender<ToolExecutionEvent>) -> Self {
        Self {
            tool_queue: VecDeque::new(),
            current_tool: None,
            event_tx,
            tool_counter: 0,
        }
    }

    /// Parse streaming JSON response and emit tool identified events as they arrive
    pub async fn parse_streaming_json(&mut self, stream: &str) {
        let lines = stream.lines();
        
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Try to parse as JSON
            if let Ok(json_obj) = serde_json::from_str::<Value>(trimmed) {
                if let Some(tool_name) = json_obj.get("tool").and_then(|t| t.as_str()) {
                    let args = json_obj.get("args").cloned().unwrap_or(json!({}));
                    let tool_id = format!("tool_{}", self.tool_counter);
                    self.tool_counter += 1;

                    let tool_call = ToolCall {
                        tool: tool_name.to_string(),
                        args: args.clone(),
                        id: tool_id.clone(),
                    };

                    // Emit "identified" event immediately
                    let _ = self.event_tx.send(ToolExecutionEvent::ToolIdentified {
                        id: tool_id,
                        tool: tool_name.to_string(),
                        args,
                    });

                    // Add to queue for sequential execution
                    self.tool_queue.push_back(tool_call);
                }
            }
        }

        // Signal that all tools from this response have been identified
        let _ = self.event_tx.send(ToolExecutionEvent::AllToolsIdentified);
    }

    /// Get the next tool to execute
    pub fn get_next_tool(&mut self) -> Option<ToolCall> {
        if self.current_tool.is_none() {
            self.current_tool = self.tool_queue.pop_front();
        }
        self.current_tool.clone()
    }

    /// Mark current tool as completed
    pub async fn mark_tool_completed(&mut self, result: String) {
        if let Some(tool) = &self.current_tool {
            let _ = self.event_tx.send(ToolExecutionEvent::ToolCompleted {
                id: tool.id.clone(),
                result,
            });
        }
        self.current_tool = None;
    }
    /// Mark current tool as failed
    pub async fn mark_tool_failed(&mut self, error: String) {
        if let Some(tool) = &self.current_tool {
            let _ = self.event_tx.send(ToolExecutionEvent::ToolFailed {
                id: tool.id.clone(),
                error,
            });
        }
        self.current_tool = None;
    }

    /// Check if there are more tools to execute
    #[allow(dead_code)]
    pub fn has_more_tools(&self) -> bool {
        !self.tool_queue.is_empty() || self.current_tool.is_some()
    }

    /// Get all queued tools (for debugging)
    #[allow(dead_code)]
    pub fn get_queued_tools(&self) -> Vec<ToolCall> {
        self.tool_queue.iter().cloned().collect()
    }
}

/// Incremental JSON parser for streaming responses
pub struct IncrementalJsonParser {
    #[allow(dead_code)]
    buffer: String,
    complete_objects: Vec<Value>,
}

impl IncrementalJsonParser {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            complete_objects: Vec::new(),
        }
    }

    /// Feed a chunk of data and extract any complete JSON objects
    #[allow(dead_code)]
    pub fn feed(&mut self, chunk: &str) -> Vec<Value> {
        self.buffer.push_str(chunk);
        let mut extracted = Vec::new();

        loop {
            // Try to find a complete JSON object
            if let Some(obj) = self.extract_next_object() {
                extracted.push(obj);
            } else {
                break;
            }
        }

        extracted
    }

    /// Extract the next complete JSON object from the buffer
    #[allow(dead_code)]
    fn extract_next_object(&mut self) -> Option<Value> {
        let trimmed = self.buffer.trim_start().to_string();
        if trimmed.is_empty() {
            self.buffer.clear();
            return None;
        }

        // Find the start of a JSON object
        if !trimmed.starts_with('{') {
            // Skip non-JSON content
            if let Some(pos) = trimmed.find('{') {
                self.buffer = trimmed[pos..].to_string();
            } else {
                self.buffer.clear();
                return None;
            }
        }

        // Try to find a complete object by counting braces
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut complete_pos = None;

        for (i, ch) in trimmed.chars().enumerate() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        complete_pos = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(i) = complete_pos {
            let obj_str = &trimmed[..=i];
            self.buffer = trimmed[i + 1..].to_string();
            
            if let Ok(obj) = serde_json::from_str::<Value>(obj_str) {
                return Some(obj);
            }
        }

        // No complete object yet
        None
    }

    /// Get all extracted objects
    #[allow(dead_code)]
    pub fn get_objects(&self) -> &[Value] {
        &self.complete_objects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incremental_parser() {
        let mut parser = IncrementalJsonParser::new();
        
        let chunk1 = r#"{"tool": "read_file", "args": {"path": "/file.txt"}}"#;
        let objects = parser.feed(chunk1);
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0]["tool"].as_str(), Some("read_file"));

        let chunk2 = r#"
{"tool": "write_file", "args": {"path": "/out.txt", "content": "hello"}}
{"tool": "done", "args": {}}
"#;
        let objects = parser.feed(chunk2);
        assert_eq!(objects.len(), 2);
    }
}
