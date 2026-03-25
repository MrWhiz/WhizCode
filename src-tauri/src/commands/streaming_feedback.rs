use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingMetrics {
    pub total_time_ms: u32,
    pub phases_completed: u32,
}

pub struct StreamingFeedback {
    pub current_phase: Option<String>,
    pub phase_start_time: Option<u64>,
    pub metrics: StreamingMetrics,
}

impl StreamingFeedback {
    pub fn new() -> Self {
        Self {
            current_phase: None,
            phase_start_time: None,
            metrics: StreamingMetrics {
                total_time_ms: 0,
                phases_completed: 0,
            },
        }
    }

    pub fn start_phase(&mut self, phase_name: &str) {
        self.current_phase = Some(phase_name.to_string());
        self.phase_start_time = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        );
    }

    pub fn complete_phase(&mut self, _phase_name: &str) {
        if let Some(start_time) = self.phase_start_time {
            let end_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            self.metrics.total_time_ms = (end_time - start_time) as u32;
            self.metrics.phases_completed += 1;
        }
        self.current_phase = None;
        self.phase_start_time = None;
    }

    pub fn get_metrics(&self) -> StreamingMetrics {
        self.metrics.clone()
    }
}
