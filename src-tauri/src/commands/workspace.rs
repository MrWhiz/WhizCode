use tauri::State;
use std::sync::Arc;
use parking_lot::RwLock;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::AppState;
use crate::error::Result;
use crate::commands::problem_identifier::TaskWorkingState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshotSymbol {
    pub name: String,
    pub symbol_type: String,
    pub file_path: String,
    pub line_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshotRelationship {
    pub from_symbol: String,
    pub to_symbol: String,
    pub relationship_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceContextSnapshot {
    pub workspace_path: String,
    pub updated_at: u64,
    pub key_files: Vec<String>,
    pub symbols: Vec<WorkspaceSnapshotSymbol>,
    pub relationships: Vec<WorkspaceSnapshotRelationship>,
    pub recent_edits: Vec<String>,
    pub recent_investigations: Vec<String>,
    pub summary: String,
}

impl WorkspaceContextSnapshot {
    pub fn to_prompt_block(&self) -> String {
        let mut block = String::new();
        block.push_str("<workspace_context_snapshot>\n");
        block.push_str(&format!("workspace_path: {}\n", self.workspace_path));
        block.push_str(&format!("updated_at: {}\n", self.updated_at));
        block.push_str(&format!("summary: {}\n", self.summary));

        if !self.key_files.is_empty() {
            block.push_str("key_files:\n");
            for file in self.key_files.iter().take(8) {
                block.push_str(&format!("- {}\n", file));
            }
        }

        if !self.symbols.is_empty() {
            block.push_str("symbols:\n");
            for symbol in self.symbols.iter().take(12) {
                block.push_str(&format!(
                    "- {} ({}) @ {}:{}\n",
                    symbol.name, symbol.symbol_type, symbol.file_path, symbol.line_number
                ));
            }
        }

        if !self.relationships.is_empty() {
            block.push_str("relationships:\n");
            for relationship in self.relationships.iter().take(12) {
                block.push_str(&format!(
                    "- {} -> {} [{}]\n",
                    relationship.from_symbol, relationship.to_symbol, relationship.relationship_type
                ));
            }
        }

        if !self.recent_edits.is_empty() {
            block.push_str("recent_edits:\n");
            for edit in self.recent_edits.iter().take(8) {
                block.push_str(&format!("- {}\n", edit));
            }
        }

        if !self.recent_investigations.is_empty() {
            block.push_str("recent_investigations:\n");
            for investigation in self.recent_investigations.iter().take(5) {
                block.push_str(&format!("- {}\n", investigation));
            }
        }

        block.push_str("</workspace_context_snapshot>\n");
        block
    }
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub path: String,
}

pub fn get_context_snapshot_path(workspace_path: &str) -> String {
    PathBuf::from(workspace_path)
        .join(".whizcode")
        .join("context_snapshot.json")
        .to_string_lossy()
        .to_string()
}

pub fn load_workspace_context_snapshot(
    workspace_path: &str,
) -> std::result::Result<Option<WorkspaceContextSnapshot>, String> {
    let snapshot_path = get_context_snapshot_path(workspace_path);
    if !PathBuf::from(&snapshot_path).exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&snapshot_path)
        .map_err(|e| format!("Failed to read workspace context snapshot: {}", e))?;

    serde_json::from_str(&content)
        .map(Some)
        .map_err(|e| format!("Failed to parse workspace context snapshot: {}", e))
}

pub fn save_workspace_context_snapshot(
    workspace_path: &str,
    snapshot: &WorkspaceContextSnapshot,
) -> std::result::Result<(), String> {
    let whizcode_dir = PathBuf::from(workspace_path).join(".whizcode");
    fs::create_dir_all(&whizcode_dir)
        .map_err(|e| format!("Failed to create .whizcode directory: {}", e))?;

    let snapshot_path = get_context_snapshot_path(workspace_path);
    let content = serde_json::to_string_pretty(snapshot)
        .map_err(|e| format!("Failed to serialize workspace context snapshot: {}", e))?;
    fs::write(&snapshot_path, content)
        .map_err(|e| format!("Failed to write workspace context snapshot: {}", e))?;

    eprintln!("[Workspace] Saved context snapshot to {}", snapshot_path);
    Ok(())
}

pub fn build_workspace_context_snapshot(
    workspace_path: &str,
    key_files: Vec<String>,
    symbols: Vec<crate::commands::code_intelligence::CodeSymbol>,
    relationships: Vec<crate::commands::code_intelligence::CodeRelationship>,
    task_state: Option<&TaskWorkingState>,
) -> WorkspaceContextSnapshot {
    let recent_edits = task_state
        .map(|state| {
            state
                .completed_checks
                .iter()
                .rev()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut recent_investigations = Vec::new();
    if let Some(state) = task_state {
        if let Some(summary) = &state.research_summary {
            let trimmed = summary.trim();
            if !trimmed.is_empty() {
                recent_investigations.push(trimmed.chars().take(1200).collect());
            }
        }
        if !state.blockers.is_empty() {
            recent_investigations.extend(state.blockers.iter().rev().take(5).cloned());
        }
    }

    let snapshot_symbols = symbols
        .into_iter()
        .take(30)
        .map(|symbol| WorkspaceSnapshotSymbol {
            name: symbol.name,
            symbol_type: symbol.symbol_type,
            file_path: symbol.file_path,
            line_number: symbol.line_number,
        })
        .collect::<Vec<_>>();

    let snapshot_relationships = relationships
        .into_iter()
        .take(40)
        .map(|relationship| WorkspaceSnapshotRelationship {
            from_symbol: relationship.from_symbol,
            to_symbol: relationship.to_symbol,
            relationship_type: relationship.relationship_type,
        })
        .collect::<Vec<_>>();

    let task_hint = task_state
        .map(|state| format!("Task kind: {}; Goal: {}", state.task_kind, state.current_goal))
        .unwrap_or_else(|| "No active task state.".to_string());

    let file_hint = if key_files.is_empty() {
        "No key files cached yet.".to_string()
    } else {
        format!("Key files: {}", key_files.iter().take(8).cloned().collect::<Vec<_>>().join(", "))
    };

    let symbol_hint = if snapshot_symbols.is_empty() {
        "No symbols cached yet.".to_string()
    } else {
        format!(
            "{} symbols cached; top entries: {}",
            snapshot_symbols.len(),
            snapshot_symbols
                .iter()
                .take(8)
                .map(|symbol| format!("{} ({})", symbol.name, symbol.file_path))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    WorkspaceContextSnapshot {
        workspace_path: workspace_path.to_string(),
        updated_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        key_files,
        symbols: snapshot_symbols,
        relationships: snapshot_relationships,
        recent_edits,
        recent_investigations,
        summary: format!("{}\n{}\n{}", task_hint, file_hint, symbol_hint),
    }
}

#[tauri::command]
pub async fn set_workspace(
    path: String,
    state: State<'_, Arc<RwLock<AppState>>>,
    vector_state: State<'_, Arc<std::sync::Mutex<crate::commands::vector_search::VectorSearchSystem>>>,
    history_state: State<'_, Arc<std::sync::Mutex<crate::commands::history::HistoryService>>>,
) -> Result<()> {
    let workspace_path = PathBuf::from(&path);
    
    if !workspace_path.exists() {
        return Err("Workspace path does not exist".into());
    }
    
    if !workspace_path.is_dir() {
        return Err("Workspace path is not a directory".into());
    }
    
    // Reinitialize vector search system with the new workspace root
    // so the DB lives inside the workspace's .whizcode folder
    if let Ok(new_system) = crate::commands::vector_search::VectorSearchSystem::new(&path) {
        let mut vs = vector_state.lock().unwrap();
        *vs = new_system;
    }

    {
        let mut history_service = history_state.lock().unwrap();
        history_service.set_workspace_path(&path)?;
    }

    let mut app_state = state.write();
    app_state.set_workspace(workspace_path);
    
    Ok(())
}

#[tauri::command]
pub async fn get_workspace(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<Option<WorkspaceInfo>> {
    let app_state = state.read();
    
    Ok(app_state.get_workspace().map(|p| WorkspaceInfo {
        path: p.to_string_lossy().to_string(),
    }))
}
