use serde::{Deserialize, Serialize};
use crate::error::Result;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditFileArgs {
    pub path: String,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitOperationArgs {
    pub operation: String,
    pub path: Option<String>,
    pub message: Option<String>,
    pub branch: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NpmOperationArgs {
    pub operation: String,
    pub package: Option<String>,
    pub version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DockerOperationArgs {
    pub operation: String,
    pub container: Option<String>,
    pub image: Option<String>,
    pub args: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

pub struct AdvancedToolExecutor;

impl AdvancedToolExecutor {
    pub async fn edit_file(args: &EditFileArgs) -> Result<ToolResult> {
        let path = Path::new(&args.path);
        
        if !path.exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("File not found: {}", args.path)),
            });
        }

        let content = tokio::fs::read_to_string(path).await?;
        let lines: Vec<&str> = content.lines().collect();

        let start = args.start_line.unwrap_or(1) as usize;
        let end = args.end_line.unwrap_or(lines.len() as u32) as usize;

        let mut new_lines = Vec::new();
        
        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;
            if line_num >= start && line_num <= end {
                if line_num == start {
                    new_lines.push(args.content.clone());
                }
            } else {
                new_lines.push(line.to_string());
            }
        }

        let new_content = new_lines.join("\n");
        tokio::fs::write(path, &new_content).await?;

        Ok(ToolResult {
            success: true,
            output: format!("Successfully edited {} (lines {}-{})", args.path, start, end),
            error: None,
        })
    }

    pub async fn git_operation(args: &GitOperationArgs) -> Result<ToolResult> {
        let output = match args.operation.as_str() {
            "status" => {
                let output = tokio::process::Command::new("git")
                    .arg("status")
                    .arg("--porcelain")
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "add" => {
                let path = args.path.as_ref().ok_or("Missing path for git add")?;
                let output = tokio::process::Command::new("git")
                    .arg("add")
                    .arg(path)
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "commit" => {
                let message = args.message.as_ref().ok_or("Missing message for git commit")?;
                let output = tokio::process::Command::new("git")
                    .arg("commit")
                    .arg("-m")
                    .arg(message)
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "push" => {
                let output = tokio::process::Command::new("git")
                    .arg("push")
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "pull" => {
                let output = tokio::process::Command::new("git")
                    .arg("pull")
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "branch" => {
                let output = tokio::process::Command::new("git")
                    .arg("branch")
                    .arg("-a")
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "checkout" => {
                let branch = args.branch.as_ref().ok_or("Missing branch for git checkout")?;
                let output = tokio::process::Command::new("git")
                    .arg("checkout")
                    .arg(branch)
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "log" => {
                let output = tokio::process::Command::new("git")
                    .arg("log")
                    .arg("--oneline")
                    .arg("-10")
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            _ => return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown git operation: {}", args.operation)),
            }),
        };

        Ok(ToolResult {
            success: true,
            output,
            error: None,
        })
    }

    pub async fn npm_operation(args: &NpmOperationArgs) -> Result<ToolResult> {
        let output = match args.operation.as_str() {
            "install" => {
                let output = tokio::process::Command::new("npm")
                    .arg("install")
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "add" => {
                let package = args.package.as_ref().ok_or("Missing package for npm add")?;
                let mut cmd = tokio::process::Command::new("npm");
                cmd.arg("install").arg(package);
                
                if let Some(version) = &args.version {
                    cmd.arg(format!("@{}", version));
                }
                
                let output = cmd.output().await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "remove" => {
                let package = args.package.as_ref().ok_or("Missing package for npm remove")?;
                let output = tokio::process::Command::new("npm")
                    .arg("uninstall")
                    .arg(package)
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "list" => {
                let output = tokio::process::Command::new("npm")
                    .arg("list")
                    .arg("--depth=0")
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "update" => {
                let output = tokio::process::Command::new("npm")
                    .arg("update")
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "run" => {
                let script = args.package.as_ref().ok_or("Missing script for npm run")?;
                let output = tokio::process::Command::new("npm")
                    .arg("run")
                    .arg(script)
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            _ => return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown npm operation: {}", args.operation)),
            }),
        };

        Ok(ToolResult {
            success: true,
            output,
            error: None,
        })
    }

    pub async fn docker_operation(args: &DockerOperationArgs) -> Result<ToolResult> {
        let output = match args.operation.as_str() {
            "ps" => {
                let output = tokio::process::Command::new("docker")
                    .arg("ps")
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "images" => {
                let output = tokio::process::Command::new("docker")
                    .arg("images")
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "run" => {
                let image = args.image.as_ref().ok_or("Missing image for docker run")?;
                let mut cmd = tokio::process::Command::new("docker");
                cmd.arg("run").arg(image);
                
                if let Some(cmd_args) = &args.args {
                    for arg in cmd_args {
                        cmd.arg(arg);
                    }
                }
                
                let output = cmd.output().await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "stop" => {
                let container = args.container.as_ref().ok_or("Missing container for docker stop")?;
                let output = tokio::process::Command::new("docker")
                    .arg("stop")
                    .arg(container)
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "logs" => {
                let container = args.container.as_ref().ok_or("Missing container for docker logs")?;
                let output = tokio::process::Command::new("docker")
                    .arg("logs")
                    .arg(container)
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            "build" => {
                let image = args.image.as_ref().ok_or("Missing image name for docker build")?;
                let output = tokio::process::Command::new("docker")
                    .arg("build")
                    .arg("-t")
                    .arg(image)
                    .arg(".")
                    .output()
                    .await?;
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            _ => return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown docker operation: {}", args.operation)),
            }),
        };

        Ok(ToolResult {
            success: true,
            output,
            error: None,
        })
    }
}

#[tauri::command]
pub async fn execute_edit_file(args: EditFileArgs) -> Result<ToolResult> {
    AdvancedToolExecutor::edit_file(&args).await
}

#[tauri::command]
pub async fn execute_git_operation(args: GitOperationArgs) -> Result<ToolResult> {
    AdvancedToolExecutor::git_operation(&args).await
}

#[tauri::command]
pub async fn execute_npm_operation(args: NpmOperationArgs) -> Result<ToolResult> {
    AdvancedToolExecutor::npm_operation(&args).await
}

#[tauri::command]
pub async fn execute_docker_operation(args: DockerOperationArgs) -> Result<ToolResult> {
    AdvancedToolExecutor::docker_operation(&args).await
}
