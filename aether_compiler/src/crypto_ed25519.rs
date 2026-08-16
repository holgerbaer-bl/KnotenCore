/// Zero-Trust Mesh — Ed25519 Cryptographic Module backed by ring.

use ring::digest::{SHA512, digest};
use ring::rand::SystemRandom;
use ring::signature::{ED25519, Ed25519KeyPair as RingKeyPair, KeyPair, UnparsedPublicKey};

pub fn sha512_digest(data: &[u8]) -> [u8; 64] {
    let d = digest(&SHA512, data);
    let mut arr = [0u8; 64];
    arr.copy_from_slice(d.as_ref());
    arr
}

#[derive(Debug)]
pub struct Ed25519KeyPair {
    pkcs8_bytes: Vec<u8>,
    public_bytes: [u8; 32],
}

impl Clone for Ed25519KeyPair {
    fn clone(&self) -> Self {
        Self {
            pkcs8_bytes: self.pkcs8_bytes.clone(),
            public_bytes: self.public_bytes,
        }
    }
}

impl Ed25519KeyPair {
    /// Generates an in-memory Ed25519 keypair securely. Private keys are never stored on disk.
    pub fn generate() -> Self {
        let rng = SystemRandom::new();
        let pkcs8_doc =
            RingKeyPair::generate_pkcs8(&rng).expect("Failed to generate Ed25519 keypair");
        let ring_pair = RingKeyPair::from_pkcs8(pkcs8_doc.as_ref())
            .expect("Failed to parse generated Ed25519 keypair");

        let mut pub_arr = [0u8; 32];
        pub_arr.copy_from_slice(ring_pair.public_key().as_ref());

        Self {
            pkcs8_bytes: pkcs8_doc.as_ref().to_vec(),
            public_bytes: pub_arr,
        }
    }

    pub fn public_key(&self) -> Ed25519PublicKey {
        Ed25519PublicKey {
            bytes: self.public_bytes,
        }
    }

    pub fn public_key_hex(&self) -> String {
        hex_encode(&self.public_bytes)
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let ring_pair = RingKeyPair::from_pkcs8(&self.pkcs8_bytes).expect("Valid PKCS#8 keypair");
        let sig = ring_pair.sign(message);
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(sig.as_ref());
        sig_arr
    }

    pub fn sign_hex(&self, message: &[u8]) -> String {
        hex_encode(&self.sign(message))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ed25519PublicKey {
    pub bytes: [u8; 32],
}

impl Ed25519PublicKey {
    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        let bytes = hex_decode(hex_str)?;
        if bytes.len() != 32 {
            return Err("Invalid public key length (must be 32 bytes)".to_string());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self { bytes: arr })
    }

    pub fn to_hex(&self) -> String {
        hex_encode(&self.bytes)
    }

    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        let peer_pk = UnparsedPublicKey::new(&ED25519, &self.bytes);
        peer_pk.verify(message, signature).is_ok()
    }

    pub fn verify_hex(&self, message: &[u8], signature_hex: &str) -> bool {
        let sig_bytes = match hex_decode(signature_hex) {
            Ok(b) if b.len() == 64 => b,
            _ => return false,
        };
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&sig_bytes);
        self.verify(message, &arr)
    }
}

// ── Hex Helpers ─────────────────────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("Odd hex length".to_string());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "Invalid hex character".to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha512_digest() {
        let digest = sha512_digest(b"abc");
        let hex = hex_encode(&digest);
        assert_eq!(
            hex,
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn test_ed25519_sign_and_verify() {
        let keypair = Ed25519KeyPair::generate();
        let pubkey = keypair.public_key();
        let msg = b"Zero-Trust Mesh Envelope Payload";

        let sig = keypair.sign(msg);
        assert!(pubkey.verify(msg, &sig));

        let mut tampered_msg = msg.to_vec();
        tampered_msg[0] ^= 1;
        assert!(!pubkey.verify(&tampered_msg, &sig));

        let mut tampered_sig = sig;
        tampered_sig[0] ^= 1;
        assert!(!pubkey.verify(msg, &tampered_sig));
    }
}
