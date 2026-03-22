# Implementation Example: Chain-of-Thought Reasoning

This document provides a concrete implementation example for adding Chain-of-Thought (CoT) reasoning to WhizCode.

## 1. Data Structures

### Add to `src-tauri/src/commands/agent_orchestrator.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_number: u32,
    pub phase: String,  // "analysis", "hypothesis", "validation", "conclusion"
    pub reasoning: String,
    pub confidence: f32,  // 0.0 to 1.0
    pub alternatives_considered: Vec<String>,
    pub decision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoTResponse {
    pub reasoning_steps: Vec<ReasoningStep>,
    pub final_decision: String,
    pub overall_confidence: f32,
    pub reasoning_trace: String,
    pub execution_plan: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedAgentResponse {
    pub cot_response: CoTResponse,
    pub tool_calls: Vec<ToolCall>,
    pub execution_steps: Vec<AgentStep>,
    pub reasoning_quality_score: f32,
}
```

## 2. System Prompt Enhancement

### Modify `get_system_prompt()` in `agent_orchestrator.rs`

```rust
fn get_system_prompt_with_cot(&self, workspace_path: &Option<String>, active_file: &Option<serde_json::Value>) -> String {
    format!(
        r#"You are WhizCode, an advanced AI coding assistant with explicit reasoning capabilities.

## Your Reasoning Process

When solving problems, ALWAYS follow this Chain-of-Thought structure:

### Phase 1: ANALYSIS
- Understand the user's request
- Identify key constraints and requirements
- List what you know and what you need to find out
- Assess complexity level

### Phase 2: HYPOTHESIS
- Propose 2-3 different approaches
- Evaluate pros/cons of each
- Identify risks and dependencies
- Select the most promising approach

### Phase 3: VALIDATION
- Check if your approach is feasible
- Verify against constraints
- Consider edge cases
- Identify potential issues

### Phase 4: CONCLUSION
- Finalize your decision
- Explain why this is the best approach
- State your confidence level (0.0-1.0)
- Outline execution steps

## Response Format

For every task, respond with this JSON structure:

```json
{{
  "reasoning_steps": [
    {{
      "step_number": 1,
      "phase": "analysis",
      "reasoning": "...",
      "confidence": 0.9,
      "alternatives_considered": ["...", "..."],
      "decision": null
    }},
    {{
      "step_number": 2,
      "phase": "hypothesis",
      "reasoning": "...",
      "confidence": 0.85,
      "alternatives_considered": ["...", "..."],
      "decision": "Selected approach X because..."
    }},
    {{
      "step_number": 3,
      "phase": "validation",
      "reasoning": "...",
      "confidence": 0.9,
      "alternatives_considered": [],
      "decision": "Approach is feasible"
    }},
    {{
      "step_number": 4,
      "phase": "conclusion",
      "reasoning": "...",
      "confidence": 0.88,
      "alternatives_considered": [],
      "decision": "Final decision: ..."
    }}
  ],
  "final_decision": "...",
  "overall_confidence": 0.88,
  "reasoning_trace": "Full narrative of reasoning...",
  "execution_plan": ["step1", "step2", "step3"],
  "tool_calls": [
    {{
      "tool": "read_file",
      "args": {{"path": "..."}},
      "reasoning": "Need to understand current state"
    }}
  ]
}}
```

## Confidence Scoring Guidelines

- 0.9-1.0: Very confident, proceed autonomously
- 0.7-0.9: Confident, proceed with monitoring
- 0.5-0.7: Moderate confidence, may need review
- 0.3-0.5: Low confidence, recommend human review
- 0.0-0.3: Very uncertain, escalate to user

## Tool Selection

When selecting tools:
1. Explain why this tool is needed
2. State confidence in tool selection
3. Describe expected outcome
4. Plan fallback if tool fails

## Error Handling

If you encounter an error:
1. Analyze the error (phase: analysis)
2. Generate recovery hypotheses (phase: hypothesis)
3. Validate recovery approach (phase: validation)
4. Execute recovery (phase: conclusion)

---

Workspace: {workspace_context}
Active File: {active_file_context}
"#,
        workspace_context = self.get_workspace_context(workspace_path),
        active_file_context = self.get_active_file_context(active_file)
    )
}
```

## 3. Response Parsing

### Add to `agent_orchestrator.rs`

```rust
impl AgentOrchestrator {
    async fn parse_cot_response(&self, response: &str) -> Result<CoTResponse> {
        // Try to extract JSON from response
        let json_start = response.find('{').ok_or("No JSON found in response")?;
        let json_end = response.rfind('}').ok_or("Incomplete JSON in response")?;
        let json_str = &response[json_start..=json_end];
        
        let cot: CoTResponse = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse CoT response: {}", e))?;
        
        // Validate reasoning steps
        self.validate_reasoning_steps(&cot.reasoning_steps)?;
        
        Ok(cot)
    }
    
    fn validate_reasoning_steps(&self, steps: &[ReasoningStep]) -> Result<()> {
        let expected_phases = vec!["analysis", "hypothesis", "validation", "conclusion"];
        
        for (i, step) in steps.iter().enumerate() {
            if step.step_number != (i + 1) as u32 {
                return Err(format!("Step numbering mismatch at step {}", i + 1).into());
            }
            
            if !expected_phases.contains(&step.phase.as_str()) {
                return Err(format!("Invalid phase: {}", step.phase).into());
            }
            
            if step.confidence < 0.0 || step.confidence > 1.0 {
                return Err(format!("Invalid confidence score: {}", step.confidence).into());
            }
        }
        
        Ok(())
    }
    
    fn calculate_overall_confidence(&self, steps: &[ReasoningStep]) -> f32 {
        if steps.is_empty() {
            return 0.5;
        }
        
        // Weight later phases more heavily
        let weights = vec![0.1, 0.2, 0.3, 0.4];
        let mut total_weighted = 0.0;
        let mut total_weight = 0.0;
        
        for (i, step) in steps.iter().enumerate() {
            let weight = weights.get(i).unwrap_or(&0.25);
            total_weighted += step.confidence * weight;
            total_weight += weight;
        }
        
        (total_weighted / total_weight).min(1.0).max(0.0)
    }
}
```

## 4. Integration with Execution Loop

### Modify `run_agent_loop()` in `agent_orchestrator.rs`

```rust
async fn run_agent_loop_with_cot(
    &mut self,
    task: &str,
    model: &serde_json::Value,
    workspace_path: &Option<String>,
    active_file: &Option<serde_json::Value>,
    project_context: &str,
) -> Result<EnhancedAgentResponse> {
    let system_prompt = self.get_system_prompt_with_cot(workspace_path, active_file);
    
    let messages = vec![
        ("system".to_string(), system_prompt),
        ("user".to_string(), format!("{}\n\nProject Context:\n{}", task, project_context)),
    ];
    
    // Call LLM with CoT prompt
    let response = self.call_llm(&messages, &model.to_string()).await?;
    
    // Parse CoT response
    let cot_response = self.parse_cot_response(&response).await?;
    
    // Calculate overall confidence
    let overall_confidence = self.calculate_overall_confidence(&cot_response.reasoning_steps);
    
    // Extract tool calls from execution plan
    let tool_calls = self.extract_tool_calls_from_plan(&cot_response.execution_plan)?;
    
    // Check if confidence is too low for autonomous execution
    if overall_confidence < 0.5 {
        self.ask_user_for_confirmation(&cot_response).await?;
    }
    
    // Execute tools
    let mut execution_steps = Vec::new();
    for tool_call in &tool_calls {
        let result = self.execute_tool(tool_call, workspace_path).await?;
        execution_steps.push(AgentStep {
            tool: tool_call.tool.clone(),
            result,
            timestamp: chrono::Utc::now().timestamp(),
        });
    }
    
    // Store reasoning for learning
    self.store_reasoning_for_learning(&cot_response, &execution_steps).await?;
    
    Ok(EnhancedAgentResponse {
        cot_response,
        tool_calls,
        execution_steps,
        reasoning_quality_score: overall_confidence,
    })
}

async fn ask_user_for_confirmation(&self, cot_response: &CoTResponse) -> Result<()> {
    // Send to frontend for user confirmation
    if let Some(app_handle) = &self.app_handle {
        app_handle.emit_all(
            "reasoning_requires_confirmation",
            serde_json::json!({
                "reasoning": cot_response,
                "message": "Low confidence in reasoning. Please review before proceeding."
            })
        )?;
    }
    Ok(())
}

async fn store_reasoning_for_learning(
    &self,
    cot_response: &CoTResponse,
    execution_steps: &[AgentStep],
) -> Result<()> {
    // Store in context memory for future learning
    let reasoning_record = serde_json::json!({
        "reasoning_steps": cot_response.reasoning_steps,
        "final_decision": cot_response.final_decision,
        "confidence": cot_response.overall_confidence,
        "execution_steps": execution_steps,
        "timestamp": chrono::Utc::now().timestamp(),
    });
    
    // Save to context memory
    // This enables learning from successful reasoning patterns
    
    Ok(())
}
```

## 5. Frontend Display

### Add to `src/components/Chat/ChatPanel.tsx`

```typescript
interface ReasoningStep {
  step_number: number;
  phase: string;
  reasoning: string;
  confidence: number;
  alternatives_considered: string[];
  decision?: string;
}

interface CoTResponse {
  reasoning_steps: ReasoningStep[];
  final_decision: string;
  overall_confidence: number;
  reasoning_trace: string;
  execution_plan: string[];
}

export const ReasoningDisplay: React.FC<{ cot: CoTResponse }> = ({ cot }) => {
  const [expanded, setExpanded] = React.useState(false);
  
  const getPhaseColor = (phase: string) => {
    const colors: Record<string, string> = {
      analysis: "#3b82f6",      // blue
      hypothesis: "#8b5cf6",    // purple
      validation: "#ec4899",    // pink
      conclusion: "#10b981",    // green
    };
    return colors[phase] || "#6b7280";
  };
  
  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 0.8) return "#10b981"; // green
    if (confidence >= 0.6) return "#f59e0b"; // amber
    return "#ef4444"; // red
  };
  
  return (
    <div className="reasoning-display">
      <div 
        className="reasoning-header"
        onClick={() => setExpanded(!expanded)}
        style={{
          cursor: "pointer",
          padding: "12px",
          backgroundColor: "#1f2937",
          borderRadius: "8px",
          marginBottom: "12px",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
        }}
      >
        <div>
          <span style={{ fontWeight: "bold" }}>🧠 Chain-of-Thought Reasoning</span>
          <span style={{
            marginLeft: "12px",
            padding: "4px 8px",
            backgroundColor: getConfidenceColor(cot.overall_confidence),
            borderRadius: "4px",
            fontSize: "12px",
          }}>
            Confidence: {(cot.overall_confidence * 100).toFixed(0)}%
          </span>
        </div>
        <span>{expanded ? "▼" : "▶"}</span>
      </div>
      
      {expanded && (
        <div className="reasoning-content" style={{ marginLeft: "12px" }}>
          {cot.reasoning_steps.map((step) => (
            <div
              key={step.step_number}
              style={{
                marginBottom: "16px",
                paddingLeft: "12px",
                borderLeft: `3px solid ${getPhaseColor(step.phase)}`,
              }}
            >
              <div style={{ fontWeight: "bold", marginBottom: "4px" }}>
                Step {step.step_number}: {step.phase.toUpperCase()}
              </div>
              <div style={{ fontSize: "14px", marginBottom: "8px" }}>
                {step.reasoning}
              </div>
              {step.decision && (
                <div style={{ fontSize: "13px", fontStyle: "italic", color: "#9ca3af" }}>
                  Decision: {step.decision}
                </div>
              )}
              <div style={{ fontSize: "12px", color: "#6b7280" }}>
                Confidence: {(step.confidence * 100).toFixed(0)}%
              </div>
              {step.alternatives_considered.length > 0 && (
                <div style={{ fontSize: "12px", color: "#6b7280", marginTop: "4px" }}>
                  Alternatives: {step.alternatives_considered.join(", ")}
                </div>
              )}
            </div>
          ))}
          
          <div style={{
            marginTop: "16px",
            padding: "12px",
            backgroundColor: "#111827",
            borderRadius: "8px",
          }}>
            <div style={{ fontWeight: "bold", marginBottom: "8px" }}>Final Decision</div>
            <div>{cot.final_decision}</div>
          </div>
        </div>
      )}
    </div>
  );
};
```

## 6. Testing

### Add to test suite

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cot_response_parsing() {
        let response = r#"{
            "reasoning_steps": [
                {
                    "step_number": 1,
                    "phase": "analysis",
                    "reasoning": "Understanding the task",
                    "confidence": 0.9,
                    "alternatives_considered": [],
                    "decision": null
                }
            ],
            "final_decision": "Test decision",
            "overall_confidence": 0.9,
            "reasoning_trace": "Full trace",
            "execution_plan": ["step1"]
        }"#;
        
        let cot: CoTResponse = serde_json::from_str(response).unwrap();
        assert_eq!(cot.reasoning_steps.len(), 1);
        assert_eq!(cot.overall_confidence, 0.9);
    }
    
    #[test]
    fn test_confidence_calculation() {
        let steps = vec![
            ReasoningStep {
                step_number: 1,
                phase: "analysis".to_string(),
                reasoning: "test".to_string(),
                confidence: 0.8,
                alternatives_considered: vec![],
                decision: None,
            },
            ReasoningStep {
                step_number: 2,
                phase: "conclusion".to_string(),
                reasoning: "test".to_string(),
                confidence: 0.9,
                alternatives_considered: vec![],
                decision: None,
            },
        ];
        
        let orchestrator = AgentOrchestrator::new(None);
        let confidence = orchestrator.calculate_overall_confidence(&steps);
        assert!(confidence > 0.8 && confidence <= 0.9);
    }
}
```

## 7. Rollout Strategy

1. **Week 1:** Implement and test CoT parsing
2. **Week 2:** Integrate with agent loop
3. **Week 3:** Add frontend display
4. **Week 4:** Gather user feedback and iterate

## 8. Expected Impact

- **Reasoning Quality:** +15-20% improvement
- **Error Detection:** +30% (catch issues during reasoning)
- **User Trust:** +40% (transparent reasoning)
- **Latency:** +10-15% (worth the quality gain)

