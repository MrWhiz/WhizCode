use serde::{Deserialize, Serialize};
use crate::error::Result;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CustomTool {
    pub name: String,
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct CustomToolRegistry {
    tools: HashMap<String, CustomTool>,
}

impl CustomToolRegistry {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn register(&mut self, tool: CustomTool) -> Result<()> {
        if self.tools.contains_key(&tool.name) {
            return Err(format!("Tool {} already registered", tool.name).into());
        }
        self.tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn unregister(&mut self, name: &str) -> Result<()> {
        if self.tools.remove(name).is_none() {
            return Err(format!("Tool {} not found", name).into());
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&CustomTool> {
        self.tools.get(name)
    }

    #[allow(dead_code)]
    pub fn list(&self) -> Vec<CustomTool> {
        self.tools.values().cloned().collect()
    }

    #[allow(dead_code)]
    pub fn list_enabled(&self) -> Vec<CustomTool> {
        self.tools.values()
            .filter(|t| t.enabled)
            .cloned()
            .collect()
    }

    #[allow(dead_code)]
    pub fn enable(&mut self, name: &str) -> Result<()> {
        if let Some(tool) = self.tools.get_mut(name) {
            tool.enabled = true;
            Ok(())
        } else {
            Err(format!("Tool {} not found", name).into())
        }
    }

    #[allow(dead_code)]
    pub fn disable(&mut self, name: &str) -> Result<()> {
        if let Some(tool) = self.tools.get_mut(name) {
            tool.enabled = false;
            Ok(())
        } else {
            Err(format!("Tool {} not found", name).into())
        }
    }

    #[allow(dead_code)]
    pub async fn execute(&self, name: &str, args: Vec<String>) -> Result<String> {
        let tool = self.get(name)
            .ok_or(format!("Tool {} not found", name))?;
        
        if !tool.enabled {
            return Err(format!("Tool {} is disabled", name).into());
        }

        let mut cmd = tokio::process::Command::new(&tool.command);
        
        for arg in &tool.args {
            cmd.arg(arg);
        }
        
        for arg in args {
            cmd.arg(arg);
        }

        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(format!("Output:\n{}\nErrors:\n{}", stdout, stderr))
    }
}

#[tauri::command]
pub async fn register_custom_tool(tool: CustomTool) -> Result<()> {
    // This would need to be called with shared state
    Ok(())
}

#[tauri::command]
pub async fn unregister_custom_tool(name: String) -> Result<()> {
    // This would need to be called with shared state
    Ok(())
}

#[tauri::command]
pub async fn list_custom_tools() -> Result<Vec<CustomTool>> {
    // This would need to be called with shared state
    Ok(Vec::new())
}

#[tauri::command]
pub async fn execute_custom_tool(name: String, args: Vec<String>) -> Result<String> {
    // This would need to be called with shared state
    Ok(String::new())
}
