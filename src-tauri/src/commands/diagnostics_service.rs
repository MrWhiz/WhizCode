use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub id: String,
    pub file_path: String,
    pub line_number: u32,
    pub column: u32,
    pub severity: String, // 'error' | 'warning' | 'info'
    pub message: String,
    pub code: Option<String>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub file_path: String,
    pub diagnostics: Vec<Diagnostic>,
    pub checked_at: u64,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsStats {
    pub total_files_checked: usize,
    pub total_diagnostics: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub last_check: u64,
}

#[allow(dead_code)]
pub struct DiagnosticsService {
    reports: Arc<Mutex<HashMap<String, DiagnosticReport>>>,
    history: Arc<Mutex<Vec<DiagnosticReport>>>,
}

#[allow(dead_code)]
impl DiagnosticsService {
    pub fn new() -> Self {
        Self {
            reports: Arc::new(Mutex::new(HashMap::new())),
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn check_file(&self, file_path: String, content: String, language: String) -> Result<DiagnosticReport> {
        let mut diagnostics = vec![];

        // Basic syntax checking based on language
        match language.as_str() {
            "javascript" | "typescript" => {
                diagnostics.extend(self.check_javascript_typescript(&content, &file_path));
            }
            "json" => {
                diagnostics.extend(self.check_json(&content, &file_path));
            }
            "python" => {
                diagnostics.extend(self.check_python(&content, &file_path));
            }
            _ => {}
        }

        let report = DiagnosticReport {
            file_path: file_path.clone(),
            diagnostics,
            checked_at: Self::current_timestamp(),
            language,
        };

        let mut reports = self.reports.lock().unwrap();
        reports.insert(file_path, report.clone());

        let mut history = self.history.lock().unwrap();
        history.push(report.clone());

        Ok(report)
    }

    fn check_javascript_typescript(&self, content: &str, file_path: &str) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_number = (line_num + 1) as u32;

            // Check for console.log
            if line.contains("console.log") {
                diagnostics.push(Diagnostic {
                    id: format!("{}:{}", file_path, line_number),
                    file_path: file_path.to_string(),
                    line_number,
                    column: line.find("console.log").unwrap_or(0) as u32,
                    severity: "warning".to_string(),
                    message: "Unexpected console statement".to_string(),
                    code: Some("no-console".to_string()),
                    suggestion: Some("Remove console.log or use a logger".to_string()),
                });
            }

            // Check for var usage
            if line.trim().starts_with("var ") {
                diagnostics.push(Diagnostic {
                    id: format!("{}:{}", file_path, line_number),
                    file_path: file_path.to_string(),
                    line_number,
                    column: line.find("var").unwrap_or(0) as u32,
                    severity: "warning".to_string(),
                    message: "Unexpected var, use let or const instead".to_string(),
                    code: Some("no-var".to_string()),
                    suggestion: Some("Replace var with let or const".to_string()),
                });
            }

            // Check for trailing whitespace
            if line.ends_with(' ') || line.ends_with('\t') {
                diagnostics.push(Diagnostic {
                    id: format!("{}:{}", file_path, line_number),
                    file_path: file_path.to_string(),
                    line_number,
                    column: line.len() as u32,
                    severity: "info".to_string(),
                    message: "Trailing whitespace".to_string(),
                    code: Some("no-trailing-spaces".to_string()),
                    suggestion: Some("Remove trailing whitespace".to_string()),
                });
            }
        }

        diagnostics
    }

    fn check_json(&self, content: &str, file_path: &str) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Try to parse JSON
        match serde_json::from_str::<serde_json::Value>(content) {
            Err(e) => {
                diagnostics.push(Diagnostic {
                    id: format!("{}:1", file_path),
                    file_path: file_path.to_string(),
                    line_number: 1,
                    column: 1,
                    severity: "error".to_string(),
                    message: format!("Invalid JSON: {}", e),
                    code: Some("json-parse-error".to_string()),
                    suggestion: Some("Check JSON syntax".to_string()),
                });
            }
            Ok(_) => {
                // Valid JSON
            }
        }

        diagnostics
    }

    fn check_python(&self, content: &str, file_path: &str) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line_number = (line_num + 1) as u32;

            // Check for print statements
            if line.contains("print(") {
                diagnostics.push(Diagnostic {
                    id: format!("{}:{}", file_path, line_number),
                    file_path: file_path.to_string(),
                    line_number,
                    column: line.find("print").unwrap_or(0) as u32,
                    severity: "info".to_string(),
                    message: "Print statement found".to_string(),
                    code: Some("print-statement".to_string()),
                    suggestion: Some("Consider using logging instead".to_string()),
                });
            }

            // Check for trailing whitespace
            if line.ends_with(' ') || line.ends_with('\t') {
                diagnostics.push(Diagnostic {
                    id: format!("{}:{}", file_path, line_number),
                    file_path: file_path.to_string(),
                    line_number,
                    column: line.len() as u32,
                    severity: "info".to_string(),
                    message: "Trailing whitespace".to_string(),
                    code: Some("trailing-whitespace".to_string()),
                    suggestion: Some("Remove trailing whitespace".to_string()),
                });
            }
        }

        diagnostics
    }

    pub fn get_report(&self, file_path: &str) -> Option<DiagnosticReport> {
        let reports = self.reports.lock().unwrap();
        reports.get(file_path).cloned()
    }

    pub fn get_all_reports(&self) -> Vec<DiagnosticReport> {
        let reports = self.reports.lock().unwrap();
        reports.values().cloned().collect()
    }

    pub fn get_stats(&self) -> DiagnosticsStats {
        let reports = self.reports.lock().unwrap();

        let total_files_checked = reports.len();
        let mut total_diagnostics = 0;
        let mut errors = 0;
        let mut warnings = 0;
        let mut infos = 0;

        for report in reports.values() {
            for diag in &report.diagnostics {
                total_diagnostics += 1;
                match diag.severity.as_str() {
                    "error" => errors += 1,
                    "warning" => warnings += 1,
                    "info" => infos += 1,
                    _ => {}
                }
            }
        }

        DiagnosticsStats {
            total_files_checked,
            total_diagnostics,
            errors,
            warnings,
            infos,
            last_check: Self::current_timestamp(),
        }
    }

    pub fn clear_reports(&self) -> Result<()> {
        let mut reports = self.reports.lock().unwrap();
        reports.clear();
        Ok(())
    }

    pub fn get_history(&self) -> Vec<DiagnosticReport> {
        let history = self.history.lock().unwrap();
        history.clone()
    }
}

#[tauri::command]
pub async fn diagnostics_check_file(
    file_path: String,
    content: String,
    language: String,
    state: tauri::State<'_, Arc<std::sync::Mutex<DiagnosticsService>>>,
) -> Result<DiagnosticReport> {
    let service = state.lock().unwrap();
    service.check_file(file_path, content, language)
}

#[tauri::command]
pub async fn diagnostics_get_report(file_path: String) -> Result<Option<DiagnosticReport>> {
    eprintln!("Getting diagnostics report for: {}", file_path);
    Ok(None)
}

#[tauri::command]
pub async fn diagnostics_get_all_reports() -> Result<Vec<DiagnosticReport>> {
    eprintln!("Getting all diagnostics reports");
    Ok(vec![])
}

#[tauri::command]
pub async fn diagnostics_get_stats() -> Result<DiagnosticsStats> {
    eprintln!("Getting diagnostics statistics");
    Ok(DiagnosticsStats {
        total_files_checked: 0,
        total_diagnostics: 0,
        errors: 0,
        warnings: 0,
        infos: 0,
        last_check: 0,
    })
}

#[tauri::command]
pub async fn diagnostics_clear_reports() -> Result<()> {
    eprintln!("Clearing diagnostics reports");
    Ok(())
}

#[tauri::command]
pub async fn diagnostics_get_history() -> Result<Vec<DiagnosticReport>> {
    eprintln!("Getting diagnostics history");
    Ok(vec![])
}
