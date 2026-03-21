use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize, Clone)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError {
            code: "IO_ERROR".to_string(),
            message: err.to_string(),
            details: None,
        }
    }
}

impl From<String> for ApiError {
    fn from(msg: String) -> Self {
        ApiError {
            code: "ERROR".to_string(),
            message: msg,
            details: None,
        }
    }
}

impl From<&str> for ApiError {
    fn from(msg: &str) -> Self {
        ApiError {
            code: "ERROR".to_string(),
            message: msg.to_string(),
            details: None,
        }
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;
