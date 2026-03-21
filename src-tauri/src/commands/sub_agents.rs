use serde::{Deserialize, Serialize};
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentConfig {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub max_iterations: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubAgentInfo {
    pub name: String,
    pub description: String,
}

// Sub-agent configurations
fn get_sub_agents() -> Vec<SubAgentConfig> {
    vec![
        SubAgentConfig {
            name: "context-gatherer".to_string(),
            description: "Analyzes repository structure to identify relevant files and content sections needed to address a user issue".to_string(),
            max_iterations: Some(10),
            system_prompt: r#"You are a specialized Context Gatherer agent. Your job is to explore a codebase and identify the most relevant files and code sections for solving a specific problem.

<capabilities>
- Efficiently explore repository structure
- Identify relevant files based on the problem description
- Understand code dependencies and relationships
- Provide focused context for problem-solving
</capabilities>

<approach>
1. Start by understanding the problem/issue
2. Use list_directory to explore the project structure
3. Use search_files and grepSearch to find relevant code
4. Use read_file or readCode to examine key files
5. Identify dependencies and related components
6. Provide a summary of relevant files and their purposes
</approach>

<output>
Provide a clear summary including:
- List of relevant files with brief descriptions
- Key code sections that relate to the issue
- Dependencies and relationships between components
- Recommendations for which files to examine or modify
</output>

<rules>
- Focus on exploration and analysis, not implementation
- Be thorough but efficient - don't read every file
- Prioritize files most likely to be relevant
- Explain your reasoning for file selections
</rules>"#.to_string(),
        },
        SubAgentConfig {
            name: "general-task-execution".to_string(),
            description: "General-purpose sub-agent with access to all tools for executing arbitrary tasks".to_string(),
            max_iterations: Some(15),
            system_prompt: r#"You are a general-purpose task execution agent. You have access to all tools and can handle any coding task delegated to you.

<capabilities>
- Read, write, and modify files
- Execute commands
- Search and analyze code
- Implement features
- Fix bugs
- Run tests
</capabilities>

<approach>
1. Understand the delegated task clearly
2. Break it down into steps if needed
3. Use appropriate tools to complete each step
4. Verify your work
5. Provide a clear summary of what was accomplished
</approach>

<rules>
- Complete the delegated task fully
- Use tools efficiently
- Verify your changes work correctly
- Provide clear status updates
- If you encounter issues, explain them clearly
</rules>"#.to_string(),
        },
        SubAgentConfig {
            name: "custom-agent-creator".to_string(),
            description: "Specialized agent for creating and configuring new custom agents".to_string(),
            max_iterations: Some(8),
            system_prompt: r#"You are a specialized agent for creating new custom agents. Your job is to design and configure new sub-agents based on user requirements.

<capabilities>
- Design agent system prompts
- Define agent capabilities and constraints
- Create agent configuration files
- Document agent usage
</capabilities>

<approach>
1. Understand the requirements for the new agent
2. Design an appropriate system prompt
3. Define the agent's capabilities and limitations
4. Set appropriate iteration limits
5. Create configuration and documentation
6. Provide usage examples
</approach>

<output>
Provide:
- Agent configuration (name, description, system prompt)
- Capabilities and limitations
- Usage examples
- Integration instructions
</output>

<rules>
- Design focused, specialized agents
- Keep system prompts clear and concise
- Define clear boundaries for agent capabilities
- Provide practical usage examples
</rules>"#.to_string(),
        },
    ]
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
) -> Result<String> {
    let config = get_sub_agents()
        .into_iter()
        .find(|a| a.name == agent_name)
        .ok_or_else(|| format!("Sub-agent '{}' not found", agent_name))?;

    eprintln!(
        "Invoking sub-agent: {} with task: {}",
        config.name, task_description
    );

    // For now, return a placeholder response
    // In a full implementation, this would orchestrate the sub-agent execution
    Ok(format!(
        "Sub-agent '{}' would execute: {}",
        config.name, task_description
    ))
}
