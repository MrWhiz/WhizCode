use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::Result;

#[derive(Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub changes: Vec<GitChange>,
}

#[derive(Serialize, Deserialize)]
pub struct GitChange {
    pub file: String,
    pub status: String,
    pub staged: bool,
}

#[derive(Serialize, Deserialize)]
pub struct GitCommitResult {
    pub success: bool,
    pub output: String,
}

#[derive(Serialize, Deserialize)]
pub struct ReviewFinding {
    pub file: String,
    pub severity: String,
    pub line: u32,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ReviewReport {
    pub branch: String,
    pub files_reviewed: usize,
    pub findings: Vec<ReviewFinding>,
}

fn git_command(workspace_path: &PathBuf) -> tokio::process::Command {
    let mut command = tokio::process::Command::new("git");
    command.current_dir(workspace_path);
    command
}

fn parse_git_status(output: &str) -> GitStatus {
    let mut branch = "HEAD".to_string();
    let mut changes = Vec::new();

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            branch = rest
                .split("...")
                .next()
                .unwrap_or(rest)
                .trim()
                .to_string();
            continue;
        }

        if line.len() < 4 {
            continue;
        }

        let index_status = line.chars().next().unwrap_or(' ');
        let worktree_status = line.chars().nth(1).unwrap_or(' ');
        let file = line[3..].trim().to_string();

        let status = if worktree_status != ' ' {
            worktree_status
        } else if index_status != ' ' {
            index_status
        } else {
            '?'
        };

        changes.push(GitChange {
            file,
            status: status.to_string(),
            staged: index_status != ' ' && index_status != '?',
        });
    }

    GitStatus { branch, changes }
}

fn detect_language(file: &str) -> Option<String> {
    let lower = file.to_lowercase();
    if lower.ends_with(".ts") || lower.ends_with(".tsx") {
        Some("typescript".to_string())
    } else if lower.ends_with(".js") || lower.ends_with(".jsx") {
        Some("javascript".to_string())
    } else if lower.ends_with(".json") {
        Some("json".to_string())
    } else if lower.ends_with(".py") {
        Some("python".to_string())
    } else {
        None
    }
}

#[tauri::command]
pub async fn git_status(path: String) -> Result<GitStatus> {
    let workspace_path = PathBuf::from(&path);

    if !workspace_path.exists() || !workspace_path.is_dir() {
        return Err("Workspace path is invalid".into());
    }

    let branch_output = git_command(&workspace_path)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .await?;

    if !branch_output.status.success() {
        return Err("Not a git repository".into());
    }

    let output = git_command(&workspace_path)
        .arg("status")
        .arg("--short")
        .arg("--branch")
        .output()
        .await?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string().into());
    }

    Ok(parse_git_status(&String::from_utf8_lossy(&output.stdout)))
}

#[tauri::command]
pub async fn git_stage(path: String, file: String) -> Result<GitStatus> {
    let workspace_path = PathBuf::from(&path);
    let output = git_command(&workspace_path)
        .arg("add")
        .arg("--")
        .arg(&file)
        .output()
        .await?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string().into());
    }

    git_status(path).await
}

#[tauri::command]
pub async fn git_commit(path: String, message: String) -> Result<GitCommitResult> {
    let workspace_path = PathBuf::from(&path);

    let output = git_command(&workspace_path)
        .arg("commit")
        .arg("-m")
        .arg(&message)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let combined = if stdout.is_empty() {
        stderr.clone()
    } else if stderr.is_empty() {
        stdout.clone()
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    if !output.status.success() {
        return Err(combined.into());
    }

    Ok(GitCommitResult {
        success: true,
        output: combined,
    })
}

#[tauri::command]
pub async fn git_review(path: String) -> Result<ReviewReport> {
    let status = git_status(path.clone()).await?;
    let workspace_path = PathBuf::from(&path);
    let diagnostics = crate::commands::diagnostics_service::DiagnosticsService::new();
    let mut findings = Vec::new();
    let mut files_reviewed = 0usize;

    for change in &status.changes {
        let Some(language) = detect_language(&change.file) else {
            continue;
        };

        let file_path = workspace_path.join(&change.file);
        if !file_path.exists() {
            continue;
        }

        let content = tokio::fs::read_to_string(&file_path).await?;
        let report = diagnostics.check_file(change.file.clone(), content, language)?;
        files_reviewed += 1;

        for diagnostic in report.diagnostics {
            if diagnostic.severity == "info" {
                continue;
            }

            findings.push(ReviewFinding {
                file: diagnostic.file_path,
                severity: diagnostic.severity,
                line: diagnostic.line_number,
                message: diagnostic.message,
                suggestion: diagnostic.suggestion,
            });
        }
    }

    findings.sort_by(|a, b| {
        severity_rank(&a.severity)
            .cmp(&severity_rank(&b.severity))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });

    Ok(ReviewReport {
        branch: status.branch,
        files_reviewed,
        findings,
    })
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "error" => 0,
        "warning" => 1,
        _ => 2,
    }
}
