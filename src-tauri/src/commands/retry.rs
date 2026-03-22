#![allow(dead_code)]

use std::time::Duration;
use crate::error::Result;

/// Retry configuration with exponential backoff
#[derive(Clone, Debug)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 30000, // 30 seconds
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Calculate backoff duration for a given attempt number
    pub fn backoff_duration(&self, attempt: u32) -> Duration {
        let backoff_ms = (self.initial_backoff_ms as f64 * self.backoff_multiplier.powi(attempt as i32)) as u64;
        let capped = backoff_ms.min(self.max_backoff_ms);
        Duration::from_millis(capped)
    }
}

/// Executes a function with exponential backoff retry
pub async fn retry_with_backoff<F, Fut, T>(
    config: RetryConfig,
    mut f: F,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;

    loop {
        match f().await {
            Ok(result) => {
                if attempt > 0 {
                    eprintln!("[Retry] Success on attempt {}", attempt + 1);
                }
                return Ok(result);
            }
            Err(e) if attempt < config.max_retries => {
                let backoff = config.backoff_duration(attempt);
                eprintln!(
                    "[Retry] Attempt {} failed: {}. Retrying in {}ms...",
                    attempt + 1,
                    e,
                    backoff.as_millis()
                );
                tokio::time::sleep(backoff).await;
                attempt += 1;
            }
            Err(e) => {
                eprintln!("[Retry] Failed after {} attempts: {}", config.max_retries + 1, e);
                return Err(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_calculation() {
        let config = RetryConfig::default();
        
        assert_eq!(config.backoff_duration(0).as_millis(), 100);
        assert_eq!(config.backoff_duration(1).as_millis(), 200);
        assert_eq!(config.backoff_duration(2).as_millis(), 400);
        assert_eq!(config.backoff_duration(3).as_millis(), 800);
    }

    #[test]
    fn test_backoff_max_cap() {
        let config = RetryConfig {
            max_backoff_ms: 1000,
            ..Default::default()
        };
        
        // Should cap at max_backoff_ms
        assert!(config.backoff_duration(10).as_millis() <= 1000);
    }

    #[tokio::test]
    async fn test_retry_success_first_attempt() {
        let config = RetryConfig::default();
        let mut attempts = 0;

        let result = retry_with_backoff(config, || async {
            attempts += 1;
            Ok::<_, String>(42)
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 1);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let config = RetryConfig {
            max_retries: 3,
            initial_backoff_ms: 10,
            ..Default::default()
        };
        let mut attempts = 0;

        let result = retry_with_backoff(config, || async {
            attempts += 1;
            if attempts < 3 {
                Err::<i32, _>("temporary error".into())
            } else {
                Ok(42)
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn test_retry_max_attempts_exceeded() {
        let config = RetryConfig {
            max_retries: 2,
            initial_backoff_ms: 10,
            ..Default::default()
        };
        let mut attempts = 0;

        let result = retry_with_backoff(config, || async {
            attempts += 1;
            Err::<i32, _>("permanent error".into())
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts, 3); // max_retries + 1 initial attempt
    }
}
