use chrono::{Duration, Utc};
use reqwest::header::{HeaderMap, SET_COOKIE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::Result;

#[derive(Serialize, Deserialize)]
pub struct TokenStatus {
    pub has_token: bool,
    pub expires_in: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct TokenResponse {
    pub success: bool,
    pub token: Option<String>,
    pub expires_at: Option<u64>,
    pub error: Option<String>,
}

fn default_token_expiry() -> u64 {
    (Utc::now() + Duration::hours(24)).timestamp() as u64
}

fn extract_first_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for key in keys {
                if let Some(found) = map.get(*key) {
                    if let Some(text) = found.as_str().filter(|text| !text.trim().is_empty()) {
                        return Some(text.trim().to_string());
                    }

                    if let Some(text) = extract_first_string(found, keys) {
                        return Some(text);
                    }
                }
            }

            for nested in map.values() {
                if let Some(text) = extract_first_string(nested, keys) {
                    return Some(text);
                }
            }

            None
        }
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(text) = extract_first_string(item, keys) {
                    return Some(text);
                }
            }
            None
        }
        _ => None,
    }
}

fn extract_numeric_key(value: &serde_json::Value, key: &str) -> Option<u64> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(found) = map.get(key) {
                if let Some(number) = found.as_u64() {
                    return Some(number);
                }

                if let Some(text) = found.as_str() {
                    if let Ok(number) = text.trim().parse::<u64>() {
                        return Some(number);
                    }
                }
            }

            for nested in map.values() {
                if let Some(number) = extract_numeric_key(nested, key) {
                    return Some(number);
                }
            }

            None
        }
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(number) = extract_numeric_key(item, key) {
                    return Some(number);
                }
            }
            None
        }
        _ => None,
    }
}

fn extract_token_from_headers(headers: &HeaderMap) -> Option<String> {
    let direct_header_names = ["authorization", "x-access-token", "x-session-token", "x-auth-token"];

    for header_name in direct_header_names {
        if let Some(value) = headers.get(header_name) {
            if let Ok(text) = value.to_str() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    if header_name == "authorization" {
                        if let Some(token) = trimmed.strip_prefix("Bearer ").or_else(|| trimmed.strip_prefix("bearer ")) {
                            return Some(token.trim().to_string());
                        }
                    }
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    if let Some(cookie_value) = headers.get(SET_COOKIE) {
        if let Ok(text) = cookie_value.to_str() {
            let lower = text.to_lowercase();
            for cookie_name in ["token", "session", "session_token", "auth", "auth_token", "access_token"] {
                let needle = format!("{}=", cookie_name);
                if let Some(index) = lower.find(&needle) {
                    let remainder = &text[index + needle.len()..];
                    let token = remainder.split(';').next().unwrap_or("").trim();
                    if !token.is_empty() {
                        return Some(token.to_string());
                    }
                }
            }
        }
    }

    None
}

fn extract_token_from_payload(payload: &serde_json::Value) -> Option<String> {
    extract_first_string(
        payload,
        &[
            "token",
            "access_token",
            "session_token",
            "sessionToken",
            "bearer_token",
            "bearerToken",
            "id_token",
            "idToken",
        ],
    )
}

fn extract_expiry_from_payload(payload: &serde_json::Value) -> Option<u64> {
    if let Some(expiry) = extract_numeric_key(payload, "expires_at").or_else(|| extract_numeric_key(payload, "expiresAt")).or_else(|| extract_numeric_key(payload, "exp")) {
        return Some(expiry);
    }

    if let Some(expires_in) = extract_numeric_key(payload, "expires_in").or_else(|| extract_numeric_key(payload, "expiresIn")).or_else(|| extract_numeric_key(payload, "expires")) {
        if expires_in > 0 {
            return Some((Utc::now() + Duration::seconds(expires_in as i64)).timestamp() as u64);
        }
    }

    None
}

fn extract_token_from_response(body: &str, headers: &HeaderMap) -> Option<(String, Option<u64>)> {
    if let Some(token) = extract_token_from_headers(headers) {
        return Some((token, None));
    }

    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(token) = extract_token_from_payload(&payload) {
            return Some((token, extract_expiry_from_payload(&payload)));
        }
    }

    let trimmed = body.trim();
    if !trimmed.is_empty() && !trimmed.starts_with('<') {
        return Some((trimmed.to_string(), None));
    }

    None
}

async fn login_and_extract_token(
    client: &reqwest::Client,
    login_url: &str,
    username: &str,
    password: &str,
    as_json: bool,
) -> std::result::Result<(String, Option<u64>), String> {
    let mut request = client.post(login_url).timeout(std::time::Duration::from_secs(120));

    if as_json {
        request = request.json(&serde_json::json!({
            "username": username,
            "password": password,
        }));
    } else {
        request = request.form(&HashMap::from([
            ("username", username),
            ("password", password),
        ]));
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("Failed to reach Azure login endpoint: {}", error))?;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Azure login failed ({}): {}", status, body.trim()));
    }

    let token = extract_token_from_response(&body, &headers)
        .ok_or_else(|| "Azure login response did not contain a session token.".to_string())?;

    Ok(token)
}

#[tauri::command]
pub async fn azure_get_token_status() -> Result<TokenStatus> {
    Ok(TokenStatus {
        has_token: false,
        expires_in: None,
    })
}

#[tauri::command]
pub async fn azure_generate_token(
    login_url: String,
    username: String,
    password: String,
) -> Result<TokenResponse> {
    let login_url = login_url.trim();
    if login_url.is_empty() {
        return Ok(TokenResponse {
            success: false,
            token: None,
            expires_at: None,
            error: Some("Azure login URL is required.".to_string()),
        });
    }

    if username.trim().is_empty() || password.is_empty() {
        return Ok(TokenResponse {
            success: false,
            token: None,
            expires_at: None,
            error: Some("Azure username and password are required.".to_string()),
        });
    }

    let client = reqwest::Client::builder()
        .build()
        .map_err(|error| format!("Failed to create Azure login client: {}", error))?;

    let json_attempt = login_and_extract_token(&client, login_url, &username, &password, true).await;
    let (token, expires_at) = match json_attempt {
        Ok(result) => result,
        Err(json_error) => {
            match login_and_extract_token(&client, login_url, &username, &password, false).await {
                Ok(result) => result,
                Err(form_error) => {
                    return Ok(TokenResponse {
                        success: false,
                        token: None,
                        expires_at: None,
                        error: Some(format!("Azure session token generation failed. JSON login error: {}. Form login error: {}", json_error, form_error)),
                    });
                }
            }
        }
    };

    let expires_at = expires_at.unwrap_or_else(default_token_expiry);

    Ok(TokenResponse {
        success: true,
        token: Some(token),
        expires_at: Some(expires_at),
        error: None,
    })
}
