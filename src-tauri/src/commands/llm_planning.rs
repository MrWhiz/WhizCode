/// LLM-Based Planning System
/// 
/// This module implements intelligent planning by asking the LLM to create
/// a detailed execution plan instead of using pattern matching.
/// 
/// Kiro-style planning:
/// 1. Ask LLM to analyze the task
/// 2. Ask LLM to create a step-by-step plan
/// 3. Validate the plan with acceptance criteria
/// 4. Execute step by step
/// 5. Adjust plan based on results

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub step_number: u32,
    pub title: String,
    pub description: String,
    pub tools_needed: Vec<String>,
    pub expected_outcome: String,
    pub success_criteria: Vec<String>,
    pub estimated_duration_seconds: u32,
    pub dependencies: Vec<u32>, // Step numbers this depends on
    pub is_critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMPlan {
    pub plan_id: String,
    pub task: String,
    pub objective: String,
    pub steps: Vec<PlanStep>,
    pub total_estimated_duration: u32,
    pub risks: Vec<String>,
    pub assumptions: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningPrompt {
    pub task: String,
    pub workspace_context: Option<String>,
    pub active_file: Option<String>,
}

/// Generate a planning prompt for the LLM
pub fn generate_planning_prompt(
    task: &str,
    workspace_context: Option<&str>,
    active_file: Option<&str>,
) -> String {
    let mut prompt = String::new();

    prompt.push_str("You are an expert task planner. Analyze the following task and create a detailed step-by-step execution plan.\n\n");
    
    prompt.push_str("TASK:\n");
    prompt.push_str(task);
    prompt.push_str("\n\n");

    if let Some(context) = workspace_context {
        prompt.push_str("WORKSPACE CONTEXT:\n");
        prompt.push_str(context);
        prompt.push_str("\n\n");
    }

    if let Some(file) = active_file {
        prompt.push_str("ACTIVE FILE:\n");
        prompt.push_str(file);
        prompt.push_str("\n\n");
    }

    prompt.push_str("Create a detailed plan with the following JSON structure:\n");
    prompt.push_str(r#"{
  "objective": "Clear, concise objective statement",
  "steps": [
    {
      "step_number": 1,
      "title": "Step title",
      "description": "Detailed description of what to do",
      "tools_needed": ["tool1", "tool2"],
      "expected_outcome": "What should happen after this step",
      "success_criteria": ["criterion1", "criterion2"],
      "estimated_duration_seconds": 30,
      "dependencies": [],
      "is_critical": true
    }
  ],
  "total_estimated_duration": 300,
  "risks": ["potential risk 1", "potential risk 2"],
  "assumptions": ["assumption 1", "assumption 2"],
  "acceptance_criteria": ["criterion 1", "criterion 2"]
}"#);

    prompt.push_str("\n\nIMPORTANT:\n");
    prompt.push_str("- Each step should be atomic and completable in one iteration\n");
    prompt.push_str("- Include dependencies between steps\n");
    prompt.push_str("- Mark critical steps that block other steps\n");
    prompt.push_str("- Provide clear success criteria for each step\n");
    prompt.push_str("- Estimate realistic durations\n");
    prompt.push_str("- Identify risks and assumptions upfront\n");
    prompt.push_str("- Output ONLY valid JSON, no other text\n");

    prompt
}

/// Parse LLM response into a structured plan
pub fn parse_llm_plan_response(response: &str, task: &str) -> Result<LLMPlan, String> {
    // Extract JSON from response
    let json_start = response.find('{').ok_or("No JSON found in response")?;
    let json_end = response.rfind('}').ok_or("Incomplete JSON in response")?;
    let json_str = &response[json_start..=json_end];

    // Parse JSON
    let plan_data: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Extract fields
    let objective = plan_data["objective"]
        .as_str()
        .ok_or("Missing objective")?
        .to_string();

    let steps: Vec<PlanStep> = plan_data["steps"]
        .as_array()
        .ok_or("Missing steps array")?
        .iter()
        .map(|step| {
            Ok(PlanStep {
                step_number: step["step_number"]
                    .as_u64()
                    .ok_or("Missing step_number")? as u32,
                title: step["title"]
                    .as_str()
                    .ok_or("Missing title")?
                    .to_string(),
                description: step["description"]
                    .as_str()
                    .ok_or("Missing description")?
                    .to_string(),
                tools_needed: step["tools_needed"]
                    .as_array()
                    .ok_or("Missing tools_needed")?
                    .iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect(),
                expected_outcome: step["expected_outcome"]
                    .as_str()
                    .ok_or("Missing expected_outcome")?
                    .to_string(),
                success_criteria: step["success_criteria"]
                    .as_array()
                    .ok_or("Missing success_criteria")?
                    .iter()
                    .filter_map(|c| c.as_str().map(|s| s.to_string()))
                    .collect(),
                estimated_duration_seconds: step["estimated_duration_seconds"]
                    .as_u64()
                    .ok_or("Missing estimated_duration_seconds")? as u32,
                dependencies: step["dependencies"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|d| d.as_u64().map(|n| n as u32))
                    .collect(),
                is_critical: step["is_critical"]
                    .as_bool()
                    .unwrap_or(false),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let total_estimated_duration = plan_data["total_estimated_duration"]
        .as_u64()
        .ok_or("Missing total_estimated_duration")? as u32;

    let risks: Vec<String> = plan_data["risks"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|r| r.as_str().map(|s| s.to_string()))
        .collect();

    let assumptions: Vec<String> = plan_data["assumptions"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|a| a.as_str().map(|s| s.to_string()))
        .collect();

    let acceptance_criteria: Vec<String> = plan_data["acceptance_criteria"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|c| c.as_str().map(|s| s.to_string()))
        .collect();

    let plan = LLMPlan {
        plan_id: format!(
            "plan_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ),
        task: task.to_string(),
        objective,
        steps,
        total_estimated_duration,
        risks,
        assumptions,
        acceptance_criteria,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    Ok(plan)
}

/// Format plan for display to user
pub fn format_plan_for_display(plan: &LLMPlan) -> String {
    let mut output = String::new();

    output.push_str("📋 EXECUTION PLAN\n");
    output.push_str("═══════════════════════════════════════\n\n");

    output.push_str(&format!("🎯 Objective: {}\n\n", plan.objective));

    output.push_str("📝 Steps:\n");
    for step in &plan.steps {
        let critical = if step.is_critical { "🔴 CRITICAL" } else { "⚪" };
        output.push_str(&format!(
            "\n{}  Step {}: {}\n",
            critical, step.step_number, step.title
        ));
        output.push_str(&format!("   Description: {}\n", step.description));
        output.push_str(&format!("   Expected Outcome: {}\n", step.expected_outcome));
        output.push_str(&format!("   Duration: ~{}s\n", step.estimated_duration_seconds));
        
        if !step.dependencies.is_empty() {
            output.push_str(&format!(
                "   Depends on: {}\n",
                step.dependencies
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    output.push_str(&format!(
        "\n⏱️  Total Estimated Duration: ~{}s ({:.1} min)\n",
        plan.total_estimated_duration,
        plan.total_estimated_duration as f32 / 60.0
    ));

    if !plan.risks.is_empty() {
        output.push_str("\n⚠️  Risks:\n");
        for risk in &plan.risks {
            output.push_str(&format!("   • {}\n", risk));
        }
    }

    if !plan.assumptions.is_empty() {
        output.push_str("\n📌 Assumptions:\n");
        for assumption in &plan.assumptions {
            output.push_str(&format!("   • {}\n", assumption));
        }
    }

    if !plan.acceptance_criteria.is_empty() {
        output.push_str("\n✅ Acceptance Criteria:\n");
        for criterion in &plan.acceptance_criteria {
            output.push_str(&format!("   • {}\n", criterion));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_planning_prompt() {
        let prompt = generate_planning_prompt(
            "Create a login form",
            Some("React project"),
            Some("App.jsx"),
        );
        assert!(prompt.contains("TASK:"));
        assert!(prompt.contains("Create a login form"));
        assert!(prompt.contains("WORKSPACE CONTEXT:"));
        assert!(prompt.contains("ACTIVE FILE:"));
    }

    #[test]
    fn test_parse_llm_plan_response() {
        let response = r#"{
  "objective": "Create a login form",
  "steps": [
    {
      "step_number": 1,
      "title": "Create form component",
      "description": "Create a new LoginForm component",
      "tools_needed": ["write_file"],
      "expected_outcome": "LoginForm.jsx created",
      "success_criteria": ["File exists", "Has form fields"],
      "estimated_duration_seconds": 60,
      "dependencies": [],
      "is_critical": true
    }
  ],
  "total_estimated_duration": 300,
  "risks": ["Styling issues"],
  "assumptions": ["React is installed"],
  "acceptance_criteria": ["Form renders", "Can submit"]
}"#;

        let plan = parse_llm_plan_response(response, "Create a login form").unwrap();
        assert_eq!(plan.objective, "Create a login form");
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].step_number, 1);
        assert_eq!(plan.total_estimated_duration, 300);
    }
}
