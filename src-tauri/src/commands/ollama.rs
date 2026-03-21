use serde::{Deserialize, Serialize};
use crate::error::Result;

#[derive(Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub healthy: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn ollama_health_check() -> Result<HealthCheckResponse> {
    // Try to connect to Ollama on default port
    match reqwest::Client::new()
        .get("http://localhost:11434/api/tags")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                Ok(HealthCheckResponse {
                    healthy: true,
                    error: None,
                })
            } else {
                Ok(HealthCheckResponse {
                    healthy: false,
                    error: Some("Ollama server returned error".to_string()),
                })
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
    // Try to get models from Ollama
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
