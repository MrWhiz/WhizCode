/// Streaming Agent Flow Implementation
/// 
/// This module implements the proper streaming flow:
/// 1. Stream LLM response
/// 2. Parse JSON incrementally as it arrives
/// 3. Execute first tool immediately when identified
/// 4. Queue remaining tools
/// 5. Get LLM feedback on failures
/// 6. Continue with next tool

use serde_json::{json, Value};
use std::collections::VecDeque;


#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ToolCall {
    pub tool: String,
    pub args: Value,
    pub id: String,
}

#[allow(dead_code)]
pub struct StreamingAgentFlow {
    /// Queue of identified tools waiting to be executed
    tool_queue: VecDeque<ToolCall>,
    /// Tool counter for unique IDs
    tool_counter: u32,
    /// Incremental JSON parser
    json_parser: IncrementalJsonParser,
}

impl StreamingAgentFlow {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            tool_queue: VecDeque::new(),
            tool_counter: 0,
            json_parser: IncrementalJsonParser::new(),
        }
    }

    /// Process a chunk of streaming LLM response
    /// Returns newly identified tools
    #[allow(dead_code)]
    pub fn process_stream_chunk(&mut self, chunk: &str) -> Vec<ToolCall> {
        let mut new_tools = Vec::new();
        
        // Feed chunk to JSON parser
        let objects = self.json_parser.feed(chunk);
        
        // Extract tool calls from parsed objects
        for obj in objects {
            if let Some(tool_name) = obj.get("tool").and_then(|t| t.as_str()) {
                let args = obj.get("args").cloned().unwrap_or(json!({}));
                let tool_id = format!("tool_{}", self.tool_counter);
                self.tool_counter += 1;

                let tool_call = ToolCall {
                    tool: tool_name.to_string(),
                    args,
                    id: tool_id,
                };

                new_tools.push(tool_call.clone());
                self.tool_queue.push_back(tool_call);
            }
        }

        new_tools
    }

    /// Get the next tool to execute
    #[allow(dead_code)]
    pub fn get_next_tool(&mut self) -> Option<ToolCall> {
        self.tool_queue.pop_front()
    }

    /// Check if there are more tools to execute
    #[allow(dead_code)]
    pub fn has_more_tools(&self) -> bool {
        !self.tool_queue.is_empty()
    }

    /// Get all queued tools
    #[allow(dead_code)]
    pub fn get_queued_tools(&self) -> Vec<ToolCall> {
        self.tool_queue.iter().cloned().collect()
    }
}

/// Incremental JSON parser for streaming responses
pub struct IncrementalJsonParser {
    buffer: String,
}

impl IncrementalJsonParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Feed a chunk of data and extract any complete JSON objects
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
        } else {
            // No complete object found - check if buffer is getting too large (likely incomplete)
            // If buffer > 10KB and still no complete object, it's probably incomplete JSON
            if self.buffer.len() > 10000 {
                eprintln!("[JSONParser] ⚠️ Buffer exceeded 10KB without finding complete JSON object - likely incomplete response");
                // Try to extract what we have anyway (best effort)
                if let Ok(obj) = serde_json::from_str::<Value>(&self.buffer) {
                    self.buffer.clear();
                    return Some(obj);
                } else {
                    // Can't parse it - clear buffer to prevent repeated warnings
                    eprintln!("[JSONParser] ⚠️ Clearing unparseable buffer ({} bytes)", self.buffer.len());
                    self.buffer.clear();
                    return None;
                }
            }
        }

        // No complete object yet
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_flow() {
        let mut flow = StreamingAgentFlow::new();
        
        // Simulate streaming chunks
        let chunk1 = r#"{"tool": "read_file", "args": {"path": "/file.txt"}}"#;
        let tools1 = flow.process_stream_chunk(chunk1);
        assert_eq!(tools1.len(), 1);
        assert_eq!(tools1[0].tool, "read_file");

        let chunk2 = r#"
{"tool": "write_file", "args": {"path": "/out.txt", "content": "hello"}}
"#;
        let tools2 = flow.process_stream_chunk(chunk2);
        assert_eq!(tools2.len(), 1);
        assert_eq!(tools2[0].tool, "write_file");

        // Get tools in order
        let tool1 = flow.get_next_tool();
        assert!(tool1.is_some());
        assert_eq!(tool1.unwrap().tool, "read_file");

        let tool2 = flow.get_next_tool();
        assert!(tool2.is_some());
        assert_eq!(tool2.unwrap().tool, "write_file");

        assert!(!flow.has_more_tools());
    }

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
