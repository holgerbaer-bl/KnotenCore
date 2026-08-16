use serde_json::Value;

use crate::crypto_ed25519::Ed25519PublicKey;
use crate::rpc::types::{
    validate_param_string_len, MAX_CLOCK_DRIFT_SECS, MAX_REPLAY_WINDOW_SECS,
    MAX_ZERO_TRUST_WINDOW_SECS,
};

/// Constant-time byte array equality check resisting timing side-channel attacks.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Compute HMAC-SHA256 signature encoded as lowercase hex.
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> String {
    use ring::hmac;
    let s_key = hmac::Key::new(hmac::HMAC_SHA256, key);
    let signature = hmac::sign(&s_key, data);
    let bytes = signature.as_ref();
    let mut hex = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        use std::fmt::Write;
        let _ = write!(hex, "{:02x}", b);
    }
    hex
}

impl super::RpcServer {
    /// Validates mesh authentication token or Ed25519 zero-trust signature.
    pub fn check_mesh_auth(&self, params: &Value) -> Result<(), String> {
        let is_zt = self.is_zero_trust()
            || params
                .get("zero_trust")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || params.get("zero_trust_envelope").is_some();

        let envelope = params.get("zero_trust_envelope");
        let pubkey_str = envelope
            .and_then(|e| e.get("public_key"))
            .or_else(|| params.get("public_key"))
            .and_then(|v| v.as_str());
        let sig_str = envelope
            .and_then(|e| e.get("signature").or_else(|| e.get("ed25519_signature")))
            .or_else(|| params.get("signature"))
            .or_else(|| params.get("ed25519_signature"))
            .and_then(|v| v.as_str());

        if is_zt && (pubkey_str.is_none() || sig_str.is_none()) {
            return Err(
                "Unauthorized: Unsigned payload or legacy HMAC token rejected in zero-trust mode"
                    .to_string(),
            );
        }

        if let (Some(pubkey_hex), Some(sig_hex)) = (pubkey_str, sig_str) {
            let normalized_pubkey = pubkey_hex.trim().to_lowercase();
            if self.is_peer_key_revoked(&normalized_pubkey) {
                return Err("Unauthorized: Peer public key has been revoked".to_string());
            }

            let ts = envelope
                .and_then(|e| e.get("timestamp"))
                .or_else(|| params.get("timestamp"))
                .and_then(|v| v.as_u64());

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if let Some(ts_val) = ts {
                let ts_secs = if ts_val > 10_000_000_000 {
                    ts_val / 1000
                } else {
                    ts_val
                };
                let diff = now.abs_diff(ts_secs);
                if diff > MAX_ZERO_TRUST_WINDOW_SECS {
                    return Err(
                        "Unauthorized: Request timestamp expired or invalid (replay protection window: 30s)"
                            .to_string(),
                    );
                }
            } else {
                return Err("Unauthorized: Missing timestamp in zero-trust envelope".to_string());
            }

            let nonce_str = envelope
                .and_then(|e| e.get("nonce"))
                .or_else(|| params.get("nonce"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            if nonce_str.is_empty() {
                return Err("Unauthorized: Missing nonce in zero-trust envelope".to_string());
            }

            let ts_val = ts.unwrap_or(0);
            let ts_secs = if ts_val > 10_000_000_000 {
                ts_val / 1000
            } else {
                ts_val
            };

            let mut nonce_cache = self.used_nonces.lock().unwrap_or_else(|e| e.into_inner());
            let nonce_entry = format!("{}:{}", normalized_pubkey, nonce_str);
            if !nonce_cache.insert(nonce_entry, ts_secs) {
                return Err("Unauthorized: Replayed nonce detected".to_string());
            }

            let sender = envelope
                .and_then(|e| e.get("sender_node_id"))
                .or_else(|| params.get("sender_node_id"))
                .or_else(|| params.get("node_id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let msg = format!("{}:{}:{}", ts_secs, nonce_str, sender);

            let pubkey = Ed25519PublicKey::from_hex(pubkey_hex)
                .map_err(|e| format!("Unauthorized: Bad public key hex: {}", e))?;

            if !pubkey.verify_hex(msg.as_bytes(), sig_hex) {
                return Err(
                    "Unauthorized: Invalid Ed25519 signature in zero-trust envelope".to_string(),
                );
            }

            if !sender.is_empty() {
                let mut verified_keys = self
                    .verified_peer_keys
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                verified_keys.insert(sender.to_string(), normalized_pubkey);
            }
            return Ok(());
        }

        if let Some(expected_token) = &self.mesh_auth_token {
            if let Some(sig) = params
                .get("mesh_auth_signature")
                .or_else(|| params.get("signature"))
                .and_then(|v| v.as_str())
            {
                if let Some(ts) = params.get("timestamp").and_then(|v| v.as_u64()) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let ts_secs = if ts > 10_000_000_000 { ts / 1000 } else { ts };
                    if now > ts_secs && (now - ts_secs) > MAX_REPLAY_WINDOW_SECS {
                        return Err(
                            "Unauthorized: Request timestamp expired (replay attack)".to_string()
                        );
                    }
                    if ts_secs > now.saturating_add(MAX_CLOCK_DRIFT_SECS) {
                        return Err("Unauthorized: Request timestamp in the future".to_string());
                    }
                }

                let timestamp_or_nonce = params
                    .get("timestamp")
                    .map(|v| v.to_string())
                    .or_else(|| {
                        params
                            .get("nonce")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_default();
                let sender = params
                    .get("sender_node_id")
                    .or_else(|| params.get("node_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let nonce_str = params
                    .get("nonce")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&timestamp_or_nonce);

                if !nonce_str.is_empty() {
                    validate_param_string_len(nonce_str)?;
                    let ts_secs = params
                        .get("timestamp")
                        .and_then(|v| v.as_u64())
                        .map(|ts| if ts > 10_000_000_000 { ts / 1000 } else { ts })
                        .unwrap_or_else(|| {
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                        });
                    let mut nonce_cache =
                        self.used_nonces.lock().unwrap_or_else(|e| e.into_inner());
                    let nonce_entry = format!("hmac:{}:{}", expected_token, nonce_str);
                    if !nonce_cache.insert(nonce_entry, ts_secs) {
                        return Err("Unauthorized: Replayed nonce detected".to_string());
                    }
                }

                let message = format!("{}:{}", timestamp_or_nonce, sender);
                let expected_sig = hmac_sha256(expected_token.as_bytes(), message.as_bytes());

                if constant_time_eq(sig.as_bytes(), expected_sig.as_bytes()) {
                    return Ok(());
                }
                return Err("Unauthorized: Invalid mesh_auth_signature".to_string());
            }

            let token = params
                .get("mesh_auth_token")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !constant_time_eq(token.as_bytes(), expected_token.as_bytes()) {
                return Err("Unauthorized: Invalid or missing mesh_auth_token".to_string());
            }
        }
        Ok(())
    }
}
