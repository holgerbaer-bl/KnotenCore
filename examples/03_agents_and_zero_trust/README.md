# Zero-Trust Host vs. Guest Architecture & Agent Orchestration Guide

This guide details the architectural security boundary between **Guest Sandboxed Isolates** (`.knoten` scripts) and the **Host Orchestrator Layer** (Rust / JSON-RPC / CLI).

---

## 1. Architecture Overview: Guest vs. Host Separation

In KnotenCore's Zero-Trust Mesh Architecture, execution is strictly partitioned into two layers:

```
┌────────────────────────────────────────────────────────────────────────────────┐
│                           HOST / ORCHESTRATOR LAYER                            │
│  (Rust Engine, RPC Server, Network Sockets, Ed25519 Signing, Peer Telemetry)   │
└───────────────────────┬────────────────────────────────┬───────────────────────┘
                        │                                │
        1. Submits Task │ (Signed JSON-RPC Payload)      │ 4. Returns Verified
                        ▼                                │    SignedTaskResult
┌────────────────────────────────────────────────────────┴───────────────────────┐
│                           GUEST SANDBOX ISOLATE                                │
│  (Pure KnotenCore AST Compute, Zero Network/Sockets, Hermetic Memory Heap)     │
└────────────────────────────────────────────────────────────────────────────────┘
```

### Guest Sandboxed Isolate (`.knoten` DSL Scripts)
- **Zero I/O & Network Access**: Guest `.knoten` scripts have **zero network, socket, or host filesystem access** intrinsics by design.
- **Hermetic Memory & Gas Limits**: Executes inside an isolated VM instance governed by strict instruction quotas and memory caps.
- **Pure AST Compute**: Receives JSON-AST payloads, performs compute operations, and returns evaluation outputs to the host.

### Host / Orchestrator Layer (Rust / JSON-RPC / CLI)
- **Identity & Cryptography**: Owns Ed25519 keypairs, signs task payload envelopes, and verifies incoming worker result signatures.
- **Network Transport & Peer Selection**: Manages P2P epidemic gossip discovery, inspects peer load metrics (CPU, RAM, queue depth, latency), and routes tasks over authenticated TCP/WebSocket RPC transports.
- **Lifecycle Management**: Submits tasks via `knc_task_submit`, handles work-stealing, and processes `SignedTaskResult` completion notifications.

---

## 2. Real CLI & JSON-RPC Orchestration

### Constructing a Signed `knc_task_submit` JSON-RPC Request

To delegate a compute task to a mesh peer, the host constructs a canonical payload string `task_id:timestamp:ast_json_str` and signs it using Ed25519.

#### Example JSON-RPC Request Payload (`knc_task_submit`):
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "knc_task_submit",
  "params": {
    "task_id": "task-offload-101",
    "ast": [
      "Block", [
        ["Assign", "x", ["IntLiteral", 40]],
        ["Assign", "y", ["IntLiteral", 2]],
        ["BinaryOp", "+", ["Identifier", "x"], ["Identifier", "y"]]
      ]
    ],
    "sender_public_key": "7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a",
    "timestamp": 1776600000,
    "signature": "3a2b1c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d"
  }
}
```

### Submitting via `curl` (HTTP / JSON-RPC Transport):
```bash
curl -X POST http://127.0.0.1:8080/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "knc_task_submit",
    "params": {
      "task_id": "task-offload-101",
      "ast": ["Block", [["BinaryOp", "+", ["IntLiteral", 40], ["IntLiteral", 2]]]],
      "sender_public_key": "7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a",
      "timestamp": 1776600000,
      "signature": "3a2b1c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d"
    }
  }'
```

### Polling / Receiving `SignedTaskResult`:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": "completed",
    "task_id": "task-offload-101",
    "signed_result": {
      "task_id": "task-offload-101",
      "worker_node_id": "node-worker-alpha-02",
      "worker_public_key": "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2",
      "result": 42,
      "timestamp": 1776600005,
      "worker_signature": "9f8e7d6c5b4a3f2e1d0c9b8a7f6e5d4c3b2a1f0e9d8c7b6a5f4e3d2c1b0a9f8e7d6c5b4a3f2e1d0c9b8a7f6e5d4c3b2a1f0e9d8c7b6a5f4e3d2c1b0a9f8e7d6c"
    }
  }
}
```

---

## 3. Host Rust Implementation Reference

Below is a complete, production-grade Host Orchestration snippet using KnotenCore's native cryptographic module (`aether_compiler::crypto_ed25519` backed by `ring`).

```rust
use aether_compiler::crypto_ed25519::{Ed25519KeyPair, Ed25519PublicKey};
use aether_compiler::rpc::handlers::tasks::{SignedTaskResult, create_signed_task_result};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== KnotenCore Zero-Trust Host Orchestrator ===");

    // 1. Generate Host & Worker Keypairs using native ring-backed Ed25519 module
    let host_keypair = Ed25519KeyPair::generate();
    let worker_keypair = Ed25519KeyPair::generate();

    println!("Host Public Key:   {}", host_keypair.public_key_hex());
    println!("Worker Public Key: {}", worker_keypair.public_key_hex());

    // 2. Prepare Task AST and Canonical Message Bytes
    let task_id = "task-offload-101".to_string();
    let timestamp = 1776600000u64;
    let ast_payload = json!(["Block", [["BinaryOp", "+", ["IntLiteral", 40], ["IntLiteral", 2]]]]);

    let canonical_msg = format!("{}:{}:{}", task_id, timestamp, ast_payload.to_string());
    let host_signature = host_keypair.sign_hex(canonical_msg.as_bytes());

    println!("Task Signature generated: {}", host_signature);

    // 3. Worker Execution Simulation & Result Construction
    let execution_output = json!(42);
    let worker_node_id = "node-worker-alpha-02".to_string();
    let signed_task_result: SignedTaskResult = create_signed_task_result(
        &worker_keypair,
        task_id.clone(),
        worker_node_id,
        execution_output,
        timestamp + 5,
    );

    // 4. Verify SignedTaskResult Anti-Replay Cryptographic Signature
    match signed_task_result.verify() {
        Ok(()) => println!("✅ SignedTaskResult cryptographically verified successfully!"),
        Err(e) => eprintln!("❌ Verification failed: {}", e),
    }

    Ok(())
}
```
