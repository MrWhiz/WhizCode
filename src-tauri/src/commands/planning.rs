use serde::{Deserialize, Serialize};
use crate::error::Result;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlanTask {
    pub id: String,
    pub description: String,
    pub task_type: String, // analysis, design, edit, command, review, spec
    pub priority: u32,
    pub dependencies: Vec<String>,
    pub estimated_duration: u32, // in seconds
    pub owner_agent: String,
    pub deliverable: String,
    pub acceptance_criteria: Vec<String>,
    pub requires_write: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutionPlan {
    pub id: String,
    pub objective: String,
    pub spec_summary: String,
    pub tasks: Vec<PlanTask>,
    pub parallel_groups: Vec<Vec<PlanTask>>,
    pub estimated_duration: u32,
    pub risk_level: String, // low, medium, high
    pub fallback_strategies: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub assumptions: Vec<String>,
    pub definition_of_done: Vec<String>,
    pub execution_strategy: String,
}

impl ExecutionPlan {
    pub fn to_prompt_block(&self) -> String {
        let mut block = String::new();
        block.push_str("<execution_plan>\n");
        block.push_str(&format!("objective: {}\n", self.objective));
        block.push_str(&format!("spec_summary: {}\n", self.spec_summary));
        block.push_str(&format!("risk_level: {}\n", self.risk_level));
        block.push_str(&format!("execution_strategy: {}\n", self.execution_strategy));

        if !self.acceptance_criteria.is_empty() {
            block.push_str("acceptance_criteria:\n");
            for item in &self.acceptance_criteria {
                block.push_str(&format!("- {}\n", item));
            }
        }

        if !self.assumptions.is_empty() {
            block.push_str("assumptions:\n");
            for item in &self.assumptions {
                block.push_str(&format!("- {}\n", item));
            }
        }

        if !self.definition_of_done.is_empty() {
            block.push_str("definition_of_done:\n");
            for item in &self.definition_of_done {
                block.push_str(&format!("- {}\n", item));
            }
        }

        block.push_str("planned_tasks:\n");
        for task in &self.tasks {
            block.push_str(&format!(
                "- [{}] {} | owner={} | type={} | requires_write={} | deliverable={}\n",
                task.id,
                task.description,
                task.owner_agent,
                task.task_type,
                task.requires_write,
                task.deliverable
            ));
            if !task.acceptance_criteria.is_empty() {
                for criterion in &task.acceptance_criteria {
                    block.push_str(&format!("  acceptance: {}\n", criterion));
                }
            }
        }

        block.push_str("</execution_plan>\n");
        block
    }
}

pub struct PlanningSystem {
    plan_history: Vec<ExecutionPlan>,
}

impl PlanningSystem {
    pub fn new() -> Self {
        Self {
            plan_history: Vec::new(),
        }
    }

    pub fn create_plan(&mut self, user_request: &str, workspace_path: &Option<String>) -> ExecutionPlan {
        let objective = self.extract_objective(user_request);
        let task_type = self.classify_request(user_request);
        let acceptance_criteria = self.derive_acceptance_criteria(user_request, &task_type);
        let assumptions = self.derive_assumptions(user_request, workspace_path);
        let definition_of_done = self.build_definition_of_done(&task_type, &acceptance_criteria);
        let spec_summary = self.build_spec_summary(user_request, &task_type, &acceptance_criteria);
        let execution_strategy = self.build_execution_strategy(&task_type);

        let tasks = match task_type.as_str() {
            "bug-fix" => self.plan_bug_fix(&acceptance_criteria),
            "feature-implementation" => self.plan_feature_implementation(&acceptance_criteria),
            "refactoring" => self.plan_refactoring(&acceptance_criteria),
            "analysis" => self.plan_analysis(&acceptance_criteria),
            _ => self.plan_generic_task(&acceptance_criteria),
        };

        let parallel_groups = self.optimize_parallel_execution(&tasks);
        let estimated_duration = tasks.iter().map(|t| t.estimated_duration).sum();
        let risk_level = self.assess_risk(&tasks);
        let fallback_strategies = self.generate_fallback_strategies(&tasks);

        let plan = ExecutionPlan {
            id: format!(
                "plan_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            objective,
            spec_summary,
            tasks,
            parallel_groups,
            estimated_duration,
            risk_level,
            fallback_strategies,
            acceptance_criteria,
            assumptions,
            definition_of_done,
            execution_strategy,
        };

        self.plan_history.push(plan.clone());
        plan
    }

    fn extract_objective(&self, request: &str) -> String {
        request.lines().next().unwrap_or("").trim().to_string()
    }

    fn classify_request(&self, request: &str) -> String {
        let lower = request.to_lowercase();
        if lower.contains("fix") || lower.contains("bug") || lower.contains("error") {
            "bug-fix".to_string()
        } else if lower.contains("add") || lower.contains("implement") || lower.contains("create") {
            "feature-implementation".to_string()
        } else if lower.contains("refactor") || lower.contains("improve") || lower.contains("optimize") {
            "refactoring".to_string()
        } else if lower.contains("analyze") || lower.contains("check") || lower.contains("review") {
            "analysis".to_string()
        } else {
            "generic".to_string()
        }
    }

    fn derive_acceptance_criteria(&self, request: &str, task_type: &str) -> Vec<String> {
        let mut criteria = vec![
            "The implementation must address the user's stated request, not just explore the codebase.".to_string(),
            "The final result must be validated with the narrowest safe verification step.".to_string(),
        ];

        let lower = request.to_lowercase();
        if task_type == "feature-implementation" {
            criteria.push("A visible or behavioral feature change must exist in the code, not just planning notes.".to_string());
            if lower.contains("dashboard") {
                criteria.push("The UI should expose a dashboard-style summary instead of only raw transaction entry.".to_string());
            }
            if lower.contains("categor") {
                criteria.push("Transactions should support categories and the UI should surface category information.".to_string());
            }
        }

        if task_type == "bug-fix" {
            criteria.push("The specific broken behavior should be reproducibly addressed.".to_string());
        }

        if lower.contains("test") {
            criteria.push("Relevant tests should be added or updated if the project already supports them.".to_string());
        }

        criteria
    }

    fn derive_assumptions(&self, request: &str, workspace_path: &Option<String>) -> Vec<String> {
        let mut assumptions = Vec::new();
        if workspace_path.is_some() {
            assumptions.push("The best implementation path should be inferred from the current repository structure.".to_string());
        }
        if !request.to_lowercase().contains("spec") {
            assumptions.push("If requirements are underspecified, choose the highest-value, smallest safe implementation that fits the existing app.".to_string());
        }
        assumptions.push("Verification should happen only after a meaningful code change has been made.".to_string());
        assumptions
    }

    fn build_definition_of_done(&self, task_type: &str, acceptance_criteria: &[String]) -> Vec<String> {
        let mut done = acceptance_criteria.to_vec();
        done.push("The task must finish with either a meaningful code change plus verification, or a clear failure reason.".to_string());
        if task_type == "feature-implementation" || task_type == "refactoring" {
            done.push("At least one meaningful edit tool must succeed before the task can be considered complete.".to_string());
        }
        done
    }

    fn build_spec_summary(&self, request: &str, task_type: &str, acceptance_criteria: &[String]) -> String {
        format!(
            "Task type: {}. User request: {}. Expected outcome: {}",
            task_type,
            request.trim(),
            acceptance_criteria.first().cloned().unwrap_or_else(|| "Deliver the requested change.".to_string())
        )
    }

    fn build_execution_strategy(&self, task_type: &str) -> String {
        match task_type {
            "feature-implementation" => "Spec-first: define acceptance criteria, confirm target files, implement the smallest valuable feature slice, then verify.".to_string(),
            "bug-fix" => "Repro-first: confirm the failure shape, isolate the source, patch narrowly, then verify.".to_string(),
            "refactoring" => "Safety-first: identify the refactor seam, preserve behavior, then validate.".to_string(),
            "analysis" => "Investigation-first: gather focused evidence, synthesize findings, and avoid unnecessary edits.".to_string(),
            _ => "Plan before acting: understand intent, make the smallest safe change, then verify.".to_string(),
        }
    }

    fn plan_bug_fix(&self, acceptance_criteria: &[String]) -> Vec<PlanTask> {
        vec![
            PlanTask {
                id: "spec-bug".to_string(),
                description: "Capture the bug scope, reproduction clues, and acceptance criteria".to_string(),
                task_type: "spec".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 20,
                owner_agent: "product-manager".to_string(),
                deliverable: "Bug-fix spec brief".to_string(),
                acceptance_criteria: acceptance_criteria.to_vec(),
                requires_write: false,
            },
            PlanTask {
                id: "investigate-bug".to_string(),
                description: "Investigate the most likely files and isolate the defect".to_string(),
                task_type: "analysis".to_string(),
                priority: 2,
                dependencies: vec!["spec-bug".to_string()],
                estimated_duration: 25,
                owner_agent: "context-gatherer".to_string(),
                deliverable: "Focused investigation summary".to_string(),
                acceptance_criteria: vec!["A likely source file or code path is identified.".to_string()],
                requires_write: false,
            },
            PlanTask {
                id: "implement-fix".to_string(),
                description: "Implement the smallest safe fix".to_string(),
                task_type: "edit".to_string(),
                priority: 3,
                dependencies: vec!["investigate-bug".to_string()],
                estimated_duration: 35,
                owner_agent: "general-task-execution".to_string(),
                deliverable: "Bug fix in code".to_string(),
                acceptance_criteria: vec!["The broken behavior is addressed in code.".to_string()],
                requires_write: true,
            },
            PlanTask {
                id: "verify-fix".to_string(),
                description: "Run targeted verification after the fix".to_string(),
                task_type: "command".to_string(),
                priority: 4,
                dependencies: vec!["implement-fix".to_string()],
                estimated_duration: 20,
                owner_agent: "test-engineer".to_string(),
                deliverable: "Verification result".to_string(),
                acceptance_criteria: vec!["The fix is validated with a narrow verification step.".to_string()],
                requires_write: false,
            },
        ]
    }

    fn plan_feature_implementation(&self, acceptance_criteria: &[String]) -> Vec<PlanTask> {
        vec![
            PlanTask {
                id: "define-spec".to_string(),
                description: "Define the feature spec, assumptions, and user-visible acceptance criteria".to_string(),
                task_type: "spec".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 20,
                owner_agent: "product-manager".to_string(),
                deliverable: "Feature spec brief".to_string(),
                acceptance_criteria: acceptance_criteria.to_vec(),
                requires_write: false,
            },
            PlanTask {
                id: "design-approach".to_string(),
                description: "Choose the smallest architecture and UI approach that satisfies the spec".to_string(),
                task_type: "design".to_string(),
                priority: 2,
                dependencies: vec!["define-spec".to_string()],
                estimated_duration: 20,
                owner_agent: "architect".to_string(),
                deliverable: "Implementation approach".to_string(),
                acceptance_criteria: vec!["The implementation path matches the existing codebase shape.".to_string()],
                requires_write: false,
            },
            PlanTask {
                id: "locate-target-files".to_string(),
                description: "Identify the files that should be changed to implement the chosen feature slice".to_string(),
                task_type: "analysis".to_string(),
                priority: 3,
                dependencies: vec!["design-approach".to_string()],
                estimated_duration: 20,
                owner_agent: "context-gatherer".to_string(),
                deliverable: "Target file shortlist".to_string(),
                acceptance_criteria: vec!["A likely implementation file is identified before deep reads.".to_string()],
                requires_write: false,
            },
            PlanTask {
                id: "implement-feature".to_string(),
                description: "Implement the feature according to the spec and chosen slice".to_string(),
                task_type: "edit".to_string(),
                priority: 4,
                dependencies: vec!["locate-target-files".to_string()],
                estimated_duration: 60,
                owner_agent: "general-task-execution".to_string(),
                deliverable: "Feature code changes".to_string(),
                acceptance_criteria: vec!["The codebase contains the requested feature change, not just exploration.".to_string()],
                requires_write: true,
            },
            PlanTask {
                id: "verify-feature".to_string(),
                description: "Verify the implementation and check it against the acceptance criteria".to_string(),
                task_type: "command".to_string(),
                priority: 5,
                dependencies: vec!["implement-feature".to_string()],
                estimated_duration: 25,
                owner_agent: "test-engineer".to_string(),
                deliverable: "Verification output".to_string(),
                acceptance_criteria: vec!["Build or test verification passes after the edit.".to_string()],
                requires_write: false,
            },
            PlanTask {
                id: "review-scope".to_string(),
                description: "Review the result against the original request and edge cases".to_string(),
                task_type: "review".to_string(),
                priority: 6,
                dependencies: vec!["verify-feature".to_string()],
                estimated_duration: 15,
                owner_agent: "code-reviewer".to_string(),
                deliverable: "Scope review summary".to_string(),
                acceptance_criteria: vec!["The delivered feature still matches the user's request.".to_string()],
                requires_write: false,
            },
        ]
    }

    fn plan_refactoring(&self, acceptance_criteria: &[String]) -> Vec<PlanTask> {
        vec![
            PlanTask {
                id: "define-refactor-goal".to_string(),
                description: "Define what should improve and what behavior must remain unchanged".to_string(),
                task_type: "spec".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 20,
                owner_agent: "product-manager".to_string(),
                deliverable: "Refactor spec brief".to_string(),
                acceptance_criteria: acceptance_criteria.to_vec(),
                requires_write: false,
            },
            PlanTask {
                id: "design-refactor".to_string(),
                description: "Design the refactor seam and target structure".to_string(),
                task_type: "design".to_string(),
                priority: 2,
                dependencies: vec!["define-refactor-goal".to_string()],
                estimated_duration: 25,
                owner_agent: "architect".to_string(),
                deliverable: "Refactor design".to_string(),
                acceptance_criteria: vec!["The refactor plan minimizes behavior change risk.".to_string()],
                requires_write: false,
            },
            PlanTask {
                id: "apply-refactor".to_string(),
                description: "Apply the smallest safe refactor".to_string(),
                task_type: "edit".to_string(),
                priority: 3,
                dependencies: vec!["design-refactor".to_string()],
                estimated_duration: 45,
                owner_agent: "general-task-execution".to_string(),
                deliverable: "Refactored code".to_string(),
                acceptance_criteria: vec!["The code structure is improved while preserving behavior.".to_string()],
                requires_write: true,
            },
            PlanTask {
                id: "verify-refactor".to_string(),
                description: "Run verification to ensure behavior is preserved".to_string(),
                task_type: "command".to_string(),
                priority: 4,
                dependencies: vec!["apply-refactor".to_string()],
                estimated_duration: 20,
                owner_agent: "test-engineer".to_string(),
                deliverable: "Verification result".to_string(),
                acceptance_criteria: vec!["Verification passes after the refactor.".to_string()],
                requires_write: false,
            },
        ]
    }

    fn plan_analysis(&self, acceptance_criteria: &[String]) -> Vec<PlanTask> {
        vec![
            PlanTask {
                id: "define-analysis-scope".to_string(),
                description: "Define the analysis objective and expected output".to_string(),
                task_type: "spec".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 15,
                owner_agent: "product-manager".to_string(),
                deliverable: "Analysis brief".to_string(),
                acceptance_criteria: acceptance_criteria.to_vec(),
                requires_write: false,
            },
            PlanTask {
                id: "gather-analysis-context".to_string(),
                description: "Gather focused codebase evidence for the analysis".to_string(),
                task_type: "analysis".to_string(),
                priority: 2,
                dependencies: vec!["define-analysis-scope".to_string()],
                estimated_duration: 25,
                owner_agent: "context-gatherer".to_string(),
                deliverable: "Evidence set".to_string(),
                acceptance_criteria: vec!["The findings are grounded in repository evidence.".to_string()],
                requires_write: false,
            },
            PlanTask {
                id: "review-analysis".to_string(),
                description: "Review and summarize the findings".to_string(),
                task_type: "review".to_string(),
                priority: 3,
                dependencies: vec!["gather-analysis-context".to_string()],
                estimated_duration: 15,
                owner_agent: "code-reviewer".to_string(),
                deliverable: "Analysis summary".to_string(),
                acceptance_criteria: vec!["The output is actionable and tied to the original question.".to_string()],
                requires_write: false,
            },
        ]
    }

    fn plan_generic_task(&self, acceptance_criteria: &[String]) -> Vec<PlanTask> {
        vec![
            PlanTask {
                id: "define-goal".to_string(),
                description: "Define the goal, assumptions, and success criteria".to_string(),
                task_type: "spec".to_string(),
                priority: 1,
                dependencies: vec![],
                estimated_duration: 15,
                owner_agent: "product-manager".to_string(),
                deliverable: "Task brief".to_string(),
                acceptance_criteria: acceptance_criteria.to_vec(),
                requires_write: false,
            },
            PlanTask {
                id: "execute-goal".to_string(),
                description: "Execute the highest-value next step toward the goal".to_string(),
                task_type: "edit".to_string(),
                priority: 2,
                dependencies: vec!["define-goal".to_string()],
                estimated_duration: 40,
                owner_agent: "general-task-execution".to_string(),
                deliverable: "Task result".to_string(),
                acceptance_criteria: vec!["The task moves beyond analysis into concrete progress.".to_string()],
                requires_write: true,
            },
            PlanTask {
                id: "verify-goal".to_string(),
                description: "Verify the result against the success criteria".to_string(),
                task_type: "review".to_string(),
                priority: 3,
                dependencies: vec!["execute-goal".to_string()],
                estimated_duration: 15,
                owner_agent: "code-reviewer".to_string(),
                deliverable: "Verification summary".to_string(),
                acceptance_criteria: vec!["The outcome matches the stated goal.".to_string()],
                requires_write: false,
            },
        ]
    }

    fn optimize_parallel_execution(&self, tasks: &[PlanTask]) -> Vec<Vec<PlanTask>> {
        let mut groups: Vec<Vec<PlanTask>> = Vec::new();
        let mut completed = std::collections::HashSet::new();

        while completed.len() < tasks.len() {
            let mut current_group = Vec::new();

            for task in tasks {
                if completed.contains(&task.id) {
                    continue;
                }
                if task.dependencies.iter().all(|dep| completed.contains(dep)) {
                    current_group.push(task.clone());
                }
            }

            if current_group.is_empty() {
                break;
            }

            for task in &current_group {
                completed.insert(task.id.clone());
            }

            groups.push(current_group);
        }

        groups
    }

    fn assess_risk(&self, tasks: &[PlanTask]) -> String {
        let edit_tasks = tasks.iter().filter(|t| t.requires_write).count();
        let total_tasks = tasks.len().max(1);
        let ratio = edit_tasks as f32 / total_tasks as f32;

        if ratio > 0.5 {
            "high".to_string()
        } else if ratio > 0.25 {
            "medium".to_string()
        } else {
            "low".to_string()
        }
    }

    fn generate_fallback_strategies(&self, tasks: &[PlanTask]) -> Vec<String> {
        let mut strategies = Vec::new();

        if tasks.iter().any(|t| t.requires_write) {
            strategies.push("If implementation is ambiguous, deliver the smallest safe slice that still satisfies the acceptance criteria.".to_string());
        }

        if tasks.iter().any(|t| t.task_type == "command") {
            strategies.push("If full verification is expensive, prefer the narrowest build/test check that proves the edited path still works.".to_string());
        }

        strategies.push("If discovery expands, revisit the spec and acceptance criteria before reading more files.".to_string());
        strategies
    }

    #[allow(dead_code)]
    pub fn get_plan_history(&self) -> Vec<ExecutionPlan> {
        self.plan_history.clone()
    }
}

impl Default for PlanningSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
pub fn create_execution_plan(
    user_request: String,
    workspace_path: Option<String>,
) -> Result<ExecutionPlan> {
    let mut planner = PlanningSystem::new();
    Ok(planner.create_plan(&user_request, &workspace_path))
}
