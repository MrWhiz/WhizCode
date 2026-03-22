use serde::{Deserialize, Serialize};
use crate::error::Result;
use crate::commands::prompts;
use std::sync::Arc;
use parking_lot::Mutex;

// Re-export SubAgentConfig from prompts module
pub use prompts::SubAgentConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubAgentInfo {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubAgentExecution {
    pub agent_name: String,
    pub task: String,
    pub status: String,
    pub result: String,
    pub iterations: u32,
    pub tools_used: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubAgentResult {
    pub success: bool,
    pub response: String,
    pub iterations: u32,
    pub tools_used: Vec<String>,
    pub error: Option<String>,
}

pub struct SubAgentExecutor {
    executions: Arc<Mutex<Vec<SubAgentExecution>>>,
    max_iterations: u32,
}

impl SubAgentExecutor {
    pub fn new() -> Self {
        Self {
            executions: Arc::new(Mutex::new(Vec::new())),
            max_iterations: 10,
        }
    }

    pub async fn execute_sub_agent(
        &self,
        agent_name: String,
        task: String,
        _workspace_path: Option<String>,
    ) -> Result<SubAgentResult> {
        eprintln!("[SUB_AGENT] Executing sub-agent: {}", agent_name);
        eprintln!("[SUB_AGENT] Task: {}", task);

        let config = get_sub_agents()
            .into_iter()
            .find(|a| a.name == agent_name)
            .ok_or_else(|| format!("Sub-agent '{}' not found", agent_name))?;

        // ── PHASE 3A: INITIALIZE SUB-AGENT ────────────────────────────
        eprintln!("[SUB_AGENT] Initializing sub-agent with system prompt");
        let mut messages = vec![
            ("system".to_string(), config.system_prompt.clone()),
            ("user".to_string(), task.clone()),
        ];

        let mut iterations = 0u32;
        let mut tools_used = Vec::new();
        let mut final_response = String::new();

        // ── PHASE 3B: RUN SUB-AGENT LOOP ──────────────────────────────
        while iterations < self.max_iterations {
            iterations += 1;
            eprintln!("[SUB_AGENT] Iteration {}/{}", iterations, self.max_iterations);

            // Call LLM
            let response = self.call_llm(&messages, &agent_name).await?;
            final_response = response.clone();

            // Parse tool calls
            let tool_calls = extract_tool_calls(&response);

            if tool_calls.is_empty() {
                eprintln!("[SUB_AGENT] No tool calls, sub-agent is done");
                break;
            }

            // ── PHASE 3C: EXECUTE TOOLS ───────────────────────────────
            let mut turn_results = Vec::new();
            for tool_call in &tool_calls {
                eprintln!("[SUB_AGENT] Executing tool: {}", tool_call);
                tools_used.push(tool_call.clone());

                // Execute tool (simplified for now)
                let result = format!("Tool '{}' executed successfully", tool_call);
                turn_results.push(result);
            }

            // ── PHASE 3D: AGGREGATE RESULTS ────────────────────────────
            messages.push(("assistant".to_string(), response));
            messages.push(("user".to_string(), turn_results.join("\n\n")));
        }

        // ── PHASE 3E: RECORD EXECUTION ─────────────────────────────────
        let execution = SubAgentExecution {
            agent_name: agent_name.clone(),
            task,
            status: "completed".to_string(),
            result: final_response.clone(),
            iterations,
            tools_used: tools_used.clone(),
        };

        let mut executions = self.executions.lock();
        executions.push(execution);
        drop(executions);

        eprintln!("[SUB_AGENT] Sub-agent execution complete");

        Ok(SubAgentResult {
            success: true,
            response: final_response,
            iterations,
            tools_used,
            error: None,
        })
    }

    async fn call_llm(&self, messages: &[(String, String)], agent_name: &str) -> Result<String> {
        let mut prompt = String::new();

        for (role, content) in messages {
            if role == "system" {
                prompt.push_str(&format!("{}\n\n", content));
            }
        }

        for (role, content) in messages {
            if role != "system" {
                prompt.push_str(&format!("[{}]\n{}\n\n", role.to_uppercase(), content));
            }
        }

        eprintln!("[SUB_AGENT] Calling LLM for {}", agent_name);

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "model": "llama2",
            "prompt": prompt,
            "stream": false,
            "temperature": 0.1,
        });

        match client
            .post("http://localhost:11434/api/generate")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(300))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(data) = response.json::<serde_json::Value>().await {
                    if let Some(response_text) = data.get("response").and_then(|r| r.as_str()) {
                        return Ok(response_text.to_string());
                    }
                }
                Err("Failed to parse LLM response".into())
            }
            Err(e) => Err(format!("Failed to connect to LLM: {}", e).into()),
        }
    }

    #[allow(dead_code)]
    pub fn get_execution_history(&self) -> Vec<SubAgentExecution> {
        self.executions.lock().clone()
    }

    #[allow(dead_code)]
    pub fn clear_history(&self) {
        self.executions.lock().clear();
    }
}

// Sub-agent configurations
fn get_sub_agents() -> Vec<SubAgentConfig> {
    prompts::get_sub_agents()
}

#[tauri::command]
pub async fn list_sub_agents() -> Result<Vec<SubAgentInfo>> {
    let agents = get_sub_agents();
    Ok(agents
        .into_iter()
        .map(|a| SubAgentInfo {
            name: a.name,
            description: a.description,
        })
        .collect())
}

#[tauri::command]
pub async fn get_sub_agent_config(agent_name: String) -> Result<Option<SubAgentConfig>> {
    let agents = get_sub_agents();
    Ok(agents.into_iter().find(|a| a.name == agent_name))
}

#[tauri::command]
pub async fn invoke_sub_agent(
    agent_name: String,
    task_description: String,
) -> Result<SubAgentResult> {
    eprintln!("[SUB_AGENT] Invoking sub-agent: {}", agent_name);

    let executor = SubAgentExecutor::new();
    executor
        .execute_sub_agent(agent_name, task_description, None)
        .await
}

fn extract_tool_calls(response: &str) -> Vec<String> {
    let mut tools = Vec::new();

    // Simple extraction - look for tool mentions
    if response.contains("read_file") {
        tools.push("read_file".to_string());
    }
    if response.contains("write_file") {
        tools.push("write_file".to_string());
    }
    if response.contains("run_command") {
        tools.push("run_command".to_string());
    }

    tools
}
