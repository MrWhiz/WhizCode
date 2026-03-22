# Next Improvements: Implementation Guide

## 1. Parallel Tool Execution (3-5x speedup)

### Location: `src-tauri/src/commands/agent_streaming.rs`

### Current Code (Sequential)
```rust
for tool_call in &tool_calls {
    let tool_result = self.execute_tool(tool_call, ...).await;
    // Process result
}
```

### Improved Code (Parallel)
```rust
// Identify independent tools (no shared args)
let independent_groups = identify_independent_tools(&tool_calls);

// Execute each group in parallel
for group in independent_groups {
    let futures: Vec<_> = group.iter().map(|tc| {
        self.execute_tool(tc, ...)
    }).collect();
    
    let results = futures::future::join_all(futures).await;
    // Process results
}
```

### Expected Impact
- 5 independent file reads: 2.5s → 500ms (5x faster)
- 3 git operations: 1.5s → 500ms (3x faster)

---

## 2. Exponential Backoff Retry (50% fewer failures)

### Location: `src-tauri/src/commands/agent_streaming.rs`

### Implementation
```rust
async fn execute_tool_with_retry(
    &self,
    tool_call: &ToolCall,
    max_retries: u32,
) -> Result<String> {
    let mut backoff = 1000; // 1 second
    
    for attempt in 0..=max_retries {
        match self.execute_tool(tool_call, ...).await {
            Ok(result) => return Ok(result),
            Err(e) if attempt < max_retries => {
                eprintln!("[Retry] Attempt {} failed, waiting {}ms", attempt + 1, backoff);
                tokio::time::sleep(Duration::from_millis(backoff)).await;
                backoff = (backoff * 2).min(30000); // Cap at 30s
            }
            Err(e) => return Err(e),
        }
    }
    
    Err("Max retries exceeded".into())
}
```

### Expected Impact
- Network timeouts: 80% failure → 20% failure
- Transient errors: 60% failure → 10% failure

---

## 3. Tool Composition (40% fewer LLM calls)

### Location: New file `src-tauri/src/commands/tool_pipeline.rs`

### DSL Example
```json
{
  "pipeline": [
    {
      "tool": "read_file",
      "args": { "path": "/workspace/src/main.rs" },
      "output": "file_content"
    },
    {
      "tool": "run_command",
      "args": { "command": "cargo check" },
      "condition": "file_content.contains('fn main')"
    }
  ]
}
```

### Implementation
```rust
pub struct ToolPipeline {
    steps: Vec<PipelineStep>,
}

impl ToolPipeline {
    pub async fn execute(&self, executor: &ToolExecutor) -> Result<PipelineResult> {
        let mut context = HashMap::new();
        
        for step in &self.steps {
            if let Some(condition) = &step.condition {
                if !evaluate_condition(condition, &context) {
                    continue;
                }
            }
            
            let result = executor.execute_tool(&step.tool, &step.args).await?;
            context.insert(step.output.clone(), result);
        }
        
        Ok(PipelineResult { context })
    }
}
```

---

## 4. Streaming Backpressure (Stable streaming)

### Location: `src-tauri/src/commands/agent_streaming.rs`

### Implementation
```rust
pub struct StreamingBuffer {
    tx: tokio::sync::mpsc::Sender<StreamToken>,
    rx: tokio::sync::mpsc::Receiver<StreamToken>,
    max_buffer: usize,
}

impl StreamingBuffer {
    pub async fn send(&self, token: StreamToken) -> Result<()> {
        if self.tx.capacity() < self.max_buffer / 2 {
            eprintln!("[Backpressure] Buffer at {}%", 
                (self.max_buffer - self.tx.capacity()) * 100 / self.max_buffer);
        }
        self.tx.send(token).await?;
        Ok(())
    }
}
```

---

## 5. Context Compression (50% token reduction)

### Location: New file `src-tauri/src/commands/context_compression.rs`

### Implementation
```rust
pub fn compress_context(context: &str, max_tokens: usize) -> String {
    if context.len() < max_tokens {
        return context.to_string();
    }
    
    // Summarize old context
    let summary = summarize_text(context, max_tokens / 2);
    
    // Keep recent context
    let recent = &context[context.len() - max_tokens / 2..];
    
    format!("[SUMMARY]\n{}\n\n[RECENT]\n{}", summary, recent)
}

fn summarize_text(text: &str, max_len: usize) -> String {
    // Use semantic compression or extractive summarization
    // For now, just take first and last parts
    let lines: Vec<&str> = text.lines().collect();
    let keep = max_len / 100; // Keep ~1% of lines
    
    let mut summary = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i < keep / 2 || i > lines.len() - keep / 2 {
            summary.push_str(line);
            summary.push('\n');
        }
    }
    summary
}
```

---

## Testing Strategy

### Parallel Execution
```bash
# Test with 5 independent file reads
# Expected: 2.5s → 500ms
cargo test test_parallel_tool_execution
```

### Retry Logic
```bash
# Test with flaky tool (50% failure rate)
# Expected: 80% success after retry
cargo test test_exponential_backoff
```

### Tool Composition
```bash
# Test pipeline with 3 steps
# Expected: 1 LLM call instead of 3
cargo test test_tool_pipeline
```

---

## Performance Benchmarks

### Before All Improvements
- Average task: 15-20 seconds
- Token usage: 60-100KB per request
- Tool execution: Sequential (2.5-10s for 5 tools)

### After All Improvements
- Average task: 5-8 seconds (2-3x faster)
- Token usage: 20-40KB per request (50-60% reduction)
- Tool execution: Parallel (500ms-2s for 5 tools)

---

## Priority Order

1. **Parallel Tool Execution** (Week 1) - Biggest impact, medium effort
2. **Exponential Backoff Retry** (Week 1) - Quick win, low effort
3. **Tool Composition** (Week 2) - Medium impact, medium effort
4. **Streaming Backpressure** (Week 2) - Stability improvement
5. **Context Compression** (Week 3) - Token reduction
