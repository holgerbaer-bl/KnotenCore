use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::crypto_ed25519::{Ed25519KeyPair, Ed25519PublicKey};

/// Signed transport frame for epidemic P2P mesh gossip propagation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipFrame {
    pub sender_node_id: String,
    pub sender_public_key: String,
    pub sequence_number: u64,
    pub timestamp: u64,
    pub payload: Value,
    pub signature: String,
}

impl GossipFrame {
    /// Formats canonical byte representation for signature computation and verification.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let payload_str = serde_json::to_string(&self.payload).unwrap_or_default();
        format!(
            "{}:{}:{}:{}",
            self.sender_node_id, self.sequence_number, self.timestamp, payload_str
        )
        .into_bytes()
    }
}

/// Tracks monotonic sequence numbers per peer node for anti-replay validation.
pub struct AntiReplayTracker {
    last_sequences: Mutex<HashMap<String, u64>>,
}

impl Default for AntiReplayTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl AntiReplayTracker {
    pub fn new() -> Self {
        Self {
            last_sequences: Mutex::new(HashMap::new()),
        }
    }

    /// Verifies and updates the monotonic sequence number for a sender. Returns false if replayed.
    pub fn validate_and_update(&self, sender_node_id: &str, sequence_number: u64) -> bool {
        let mut map = self
            .last_sequences
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(&last_seq) = map.get(sender_node_id)
            && sequence_number <= last_seq
        {
            return false;
        }
        map.insert(sender_node_id.to_string(), sequence_number);
        true
    }
}

/// Constructs and signs a GossipFrame using sender's Ed25519 keypair.
pub fn create_signed_gossip_frame(
    keypair: &Ed25519KeyPair,
    sender_node_id: impl Into<String>,
    sequence_number: u64,
    timestamp: u64,
    payload: Value,
) -> GossipFrame {
    let sender_node_id = sender_node_id.into();
    let sender_public_key = keypair.public_key_hex();
    let mut frame = GossipFrame {
        sender_node_id,
        sender_public_key,
        sequence_number,
        timestamp,
        payload,
        signature: String::new(),
    };
    let msg = frame.canonical_bytes();
    frame.signature = keypair.sign_hex(&msg);
    frame
}

/// Verifies Ed25519 cryptographic signature and anti-replay sequence number of a GossipFrame.
pub fn verify_gossip_frame(
    frame: &GossipFrame,
    replay_tracker: Option<&AntiReplayTracker>,
) -> Result<(), String> {
    if frame.sender_node_id.is_empty() || frame.sender_public_key.is_empty() {
        return Err("Missing sender node ID or public key in gossip frame".to_string());
    }

    let pubkey = Ed25519PublicKey::from_hex(&frame.sender_public_key)
        .map_err(|e| format!("Invalid gossip frame public key: {}", e))?;

    let msg = frame.canonical_bytes();
    if !pubkey.verify_hex(&msg, &frame.signature) {
        return Err("Cryptographic signature verification failed for gossip frame".to_string());
    }

    if let Some(tracker) = replay_tracker
        && !tracker.validate_and_update(&frame.sender_node_id, frame.sequence_number)
    {
        return Err("Replayed or out-of-order sequence number in gossip frame".to_string());
    }

    Ok(())
}
