use serde::{Deserialize, Serialize};

/// Peer node telemetry metrics used in gossip state sync and load-aware task routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeerMetrics {
    pub node_id: String,
    pub address: String,
    pub public_key: String,
    pub cpu_load_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_usage_percent: f64,
    pub task_queue_depth: usize,
    pub latency_ms: u64,
    pub is_overloaded: bool,
    pub status: String,
    pub last_seen: u64,
    pub sequence_number: u64,
}

impl PeerMetrics {
    pub fn new(node_id: String, address: String, public_key: String) -> Self {
        Self {
            node_id,
            address,
            public_key,
            cpu_load_percent: 0.0,
            memory_used_bytes: 0,
            memory_total_bytes: 0,
            memory_usage_percent: 0.0,
            task_queue_depth: 0,
            latency_ms: 0,
            is_overloaded: false,
            status: "Active".to_string(),
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            sequence_number: 1,
        }
    }

    /// Calculate routing score (lower is better: lower latency, CPU load, queue depth).
    pub fn score(&self) -> f64 {
        (self.latency_ms as f64) * 0.4
            + self.cpu_load_percent * 0.3
            + (self.task_queue_depth as f64) * 10.0
            + self.memory_usage_percent * 0.3
    }
}
