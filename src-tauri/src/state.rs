use std::collections::HashMap;
use std::path::PathBuf;
use tauri::AppHandle;
use uuid::Uuid;

#[derive(Clone)]
pub struct TerminalSession {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub shell_type: String,
    #[allow(dead_code)]
    pub cwd: PathBuf,
    #[allow(dead_code)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct AppState {
    pub workspace_path: Option<PathBuf>,
    pub terminals: HashMap<String, TerminalSession>,
    pub app_handle: Option<AppHandle>,
}

#[allow(dead_code)]
impl AppState {
    pub fn new() -> Self {
        AppState {
            workspace_path: None,
            terminals: HashMap::new(),
            app_handle: None,
        }
    }

    pub fn set_workspace(&mut self, path: PathBuf) {
        self.workspace_path = Some(path);
    }

    pub fn get_workspace(&self) -> Option<&PathBuf> {
        self.workspace_path.as_ref()
    }

    pub fn create_terminal(&mut self, shell_type: String, cwd: PathBuf) -> String {
        let id = Uuid::new_v4().to_string();
        self.terminals.insert(
            id.clone(),
            TerminalSession {
                id: id.clone(),
                shell_type,
                cwd,
                created_at: chrono::Utc::now(),
            },
        );
        id
    }

    pub fn remove_terminal(&mut self, id: &str) -> Option<TerminalSession> {
        self.terminals.remove(id)
    }

    #[allow(dead_code)]
    pub fn get_terminal(&self, id: &str) -> Option<&TerminalSession> {
        self.terminals.get(id)
    }
}
