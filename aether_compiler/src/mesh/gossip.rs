use std::collections::HashMap;
use std::sync::Mutex;

use super::telemetry::PeerMetrics;

/// Epidemic gossip propagation state and load-aware peer selection.
pub struct GossipState {
    peers: Mutex<HashMap<String, PeerMetrics>>,
}

impl Default for GossipState {
    fn default() -> Self {
        Self::new()
    }
}

impl GossipState {
    pub fn new() -> Self {
        Self {
            peers: Mutex::new(HashMap::new()),
        }
    }

    /// Epidemic propagation update for incoming peer telemetry metrics.
    /// Returns true if updated or newly inserted, false if older sequence number.
    pub fn update_peer_metrics(&self, mut incoming: PeerMetrics) -> bool {
        let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = peers.get_mut(&incoming.node_id) {
            if incoming.sequence_number <= existing.sequence_number {
                return false;
            }
            incoming.last_seen = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            incoming.status = "Active".to_string();
            *existing = incoming;
            true
        } else {
            incoming.last_seen = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            incoming.status = "Active".to_string();
            peers.insert(incoming.node_id.clone(), incoming);
            true
        }
    }

    /// Decay unresponsive peers based on time thresholds. Returns (active, stale, evicted).
    pub fn decay_unresponsive_peers(
        &self,
        current_time_secs: u64,
        stale_timeout_secs: u64,
        eviction_timeout_secs: u64,
    ) -> (usize, usize, usize) {
        let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        let mut active = 0;
        let mut stale = 0;
        let mut evicted = 0;

        for peer in peers.values_mut() {
            let elapsed = current_time_secs.saturating_sub(peer.last_seen);
            if elapsed >= eviction_timeout_secs {
                peer.status = "Evicted".to_string();
                evicted += 1;
            } else if elapsed >= stale_timeout_secs {
                peer.status = "Stale".to_string();
                stale += 1;
            } else {
                peer.status = "Active".to_string();
                active += 1;
            }
        }
        (active, stale, evicted)
    }

    /// Latency-weighted and load-aware peer selection strategy for AST task routing.
    /// Selects active, non-overloaded peer with the minimum composite routing score.
    pub fn select_optimal_peer(&self) -> Option<PeerMetrics> {
        let peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        peers
            .values()
            .filter(|p| p.status == "Active" && !p.is_overloaded)
            .min_by(|a, b| {
                a.score()
                    .partial_cmp(&b.score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    pub fn get_peer(&self, node_id: &str) -> Option<PeerMetrics> {
        let peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        peers.get(node_id).cloned()
    }

    pub fn list_peers(&self) -> Vec<PeerMetrics> {
        let peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        peers.values().cloned().collect()
    }
}
