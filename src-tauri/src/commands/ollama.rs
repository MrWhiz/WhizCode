use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use crate::error::Result;

#[derive(Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub healthy: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn ollama_health_check() -> Result<HealthCheckResponse> {
    match reqwest::Client::new()
        .get("http://localhost:11434/api/tags")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                Ok(HealthCheckResponse { healthy: true, error: None })
            } else {
                Ok(HealthCheckResponse { healthy: false, error: Some("Ollama server returned error".to_string()) })
            }
        }
        Err(e) => Ok(HealthCheckResponse {
            healthy: false,
            error: Some(format!("Failed to connect to Ollama: {}", e)),
        }),
    }
}

#[tauri::command]
pub async fn ollama_get_models() -> Result<Vec<String>> {
    match reqwest::Client::new()
        .get("http://localhost:11434/api/tags")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(response) => {
            if let Ok(data) = response.json::<serde_json::Value>().await {
                if let Some(models) = data.get("models").and_then(|m| m.as_array()) {
                    let model_names: Vec<String> = models
                        .iter()
                        .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                        .collect();
                    return Ok(model_names);
                }
            }
            Ok(vec![])
        }
        Err(_) => Ok(vec![]),
    }
}

#[tauri::command]
pub async fn ollama_pull_model(app: AppHandle, model_name: String) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    let mut child = Command::new("ollama")
        .args(["pull", &model_name])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| crate::error::ApiError::from(format!("Failed to run ollama pull: {}", e)))?;

    if let Some(stdout) = child.stdout.take() {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = app.emit("ollama:pull_progress", serde_json::json!({
                "model": model_name,
                "status": line,
                "completed": null,
                "total": null,
            }));
        }
    }

    let status = child.wait().await
        .map_err(|e| crate::error::ApiError::from(format!("ollama pull failed: {}", e)))?;

    let final_status = if status.success() { "done" } else { "error" };
    let _ = app.emit("ollama:pull_progress", serde_json::json!({
        "model": model_name,
        "status": final_status,
        "completed": null,
        "total": null,
    }));

    Ok(())
}
