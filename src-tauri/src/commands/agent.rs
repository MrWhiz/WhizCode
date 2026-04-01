use crate::error::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::oneshot;
use parking_lot::Mutex as PlMutex;
use std::sync::Mutex as StdMutex;

// Global permission channel to wait for user input
lazy_static::lazy_static! {
    pub static ref AGENT_CANCEL_TOKEN: Arc<PlMutex<bool>> = Arc::new(PlMutex::new(false));
    pub static ref PERMISSION_TX: StdMutex<HashMap<String, oneshot::Sender<bool>>> = StdMutex::new(HashMap::new());
    pub static ref ASK_USER_TX: StdMutex<HashMap<String, oneshot::Sender<String>>> = StdMutex::new(HashMap::new());
}

#[tauri::command]
pub async fn agent_stop() -> Result<()> {
    let mut cancel = AGENT_CANCEL_TOKEN.lock();
    *cancel = true;
    eprintln!("Agent stop requested");
    Ok(())
}

#[tauri::command]
pub async fn agent_reset() -> Result<()> {
    let mut cancel = AGENT_CANCEL_TOKEN.lock();
    *cancel = false;
    eprintln!("Agent reset");
    Ok(())
}

pub fn is_agent_cancelled() -> bool {
    *AGENT_CANCEL_TOKEN.lock()
}

#[tauri::command]
pub async fn agent_permission_response(approved: bool, request_id: Option<String>) -> Result<()> {
    let tx = {
        let mut pending = PERMISSION_TX.lock().unwrap();
        if let Some(request_id) = request_id {
            pending.remove(&request_id)
        } else if pending.len() == 1 {
            let key = pending.keys().next().cloned().unwrap_or_default();
            pending.remove(&key)
        } else {
            None
        }
    };

    if let Some(tx) = tx {
        let _ = tx.send(approved);
        eprintln!("Permission response received: approved={}", approved);
    } else {
        eprintln!("Received permission response but no matching request was waiting for it!");
    }
    Ok(())
}

#[tauri::command]
pub async fn agent_ask_user_response(response: String, request_id: Option<String>) -> Result<()> {
    let tx = {
        let mut pending = ASK_USER_TX.lock().unwrap();
        if let Some(request_id) = request_id {
            pending.remove(&request_id)
        } else if pending.len() == 1 {
            let key = pending.keys().next().cloned().unwrap_or_default();
            pending.remove(&key)
        } else {
            None
        }
    };

    if let Some(tx) = tx {
        let _ = tx.send(response);
        eprintln!("Ask-user response received");
    } else {
        eprintln!("Received ask-user response but no matching request was waiting for it!");
    }
    Ok(())
}
