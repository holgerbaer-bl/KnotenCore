# KnotenCore Security Policy & Formal Auth-Coverage Matrix (`v2.24.23`)

## 1. Supported Versions

KnotenCore maintains strict security support for the current release stream.

| Version Stream | Supported          | Status                                                      |
| -------------- | ------------------ | ----------------------------------------------------------- |
| `v2.24.x`      | :white_check_mark: | Active release stream (Zero-Trust Ed25519 & Dual-Engine)    |
| `< v2.24`      | :x:                | Deprecated / Unsupported                                    |

---

## 2. Architecture Context & Security Model

KnotenCore is engineered with a **Sandboxed Guest vs. Cryptographic Host** separation:

1. **Hermetic Guest Sandboxing**:
   - Guest execution runs inside isolated VM instances (`VMIsolate`) driven purely by deterministic JSON-AST bytecode.
   - Guest `.nod` and `.knoten` scripts have zero raw network access.
   - All file I/O operations are sandboxed through canonicalized paths (`dunce::canonicalize`), actively rejecting symlinks, directory traversal (`..`), and Windows UNC escape attempts.
   - Foreign Function Interface (FFI) bindings enforce strict whitelisting. No arbitrary host binaries can be invoked.

2. **Host Orchestration & P2P Mesh Security**:
   - Host nodes communicate over JSON-RPC 2.0 (TCP line-delimited and WebSocket RFC 6455).
   - In **Zero-Trust Mode**, all protected RPC endpoints strictly require canonical Ed25519 cryptographic signatures (`ring::signature::ED25519`).
   - Replay protection enforces strict time windows (30s for Ed25519 envelopes, 60s for HMAC tokens, with maximum clock drift of 300s) and non-repeating cryptographic nonces via `NonceCache`.
   - Anti-downgrade invariants strictly prevent client-directed downgrades to legacy shared-secret HMAC or unauthenticated payloads when Zero-Trust is active.
   - Revoked peer public keys are tracked dynamically and persisted to `revoked_keys.json`, immediately terminating connections and dropping gossip from compromised identities.

3. **Dual-Engine Divergence Quarantine Protocol**:
   - AST execution can be evaluated simultaneously on both the reference Tree-Walker evaluator and the AOT Stack-VM via `knc_eval_dual`.
   - Any semantic divergence (return value mismatch, observable heap mutation mismatch, or divergent fault categorization) triggers error code `-32020` (`ERR_ENGINE_DISCREPANCY`) and immediate quarantine containment, preventing unverified state propagation to CRDT storage.

---

## 3. Formal Auth-Coverage Matrix Snapshot ("State of Auth")

An exhaustive static audit verifies that every single endpoint in `REGISTERED_METHODS` is explicitly accounted for against `is_method_public()`.

- **Total Registered Endpoints**: 36
- **Protected / Auth-Gated Endpoints**: 34
- **Public Endpoints**: 2 (`knc_meaning_of_life`, `sys.meaning_of_life`)

| Endpoint Name | Transport Protocols | Auth Enforcement | Replay Protection Window | Zero-Trust Anti-Downgrade | Category / Module | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `knc_compile` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | VM Compilation | Protected |
| `knc_execute` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | VM Execution | Protected |
| `knc_yield_resume` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | VM Execution | Protected |
| `knc_inspect_state` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | VM Inspection | Protected |
| `knc_agent_handshake` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Agent Session | Protected |
| `knc_agent_snapshot` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Agent State | Protected |
| `knc_agent_restore` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Agent State | Protected |
| `knc_mesh_discover` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | P2P Mesh | Protected |
| `knc_mesh_peers` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | P2P Mesh | Protected |
| `knc_mesh_gossip` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Gossip Protocol | Protected |
| `knc_agent_teleport` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Agent Migration | Protected |
| `knc_task_submit` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Task Orchestration | Protected |
| `knc_task_status` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Task Orchestration | Protected |
| `knc_task_cancel` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Task Orchestration | Protected |
| `knc_task_steal` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Task Stealing | Protected |
| `knc_task_complete` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Task Offloading | **Protected (Verified)** |
| `knc_mesh_ping` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Mesh Liveness | Protected |
| `knc_mesh_metrics` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Mesh Telemetry | Protected |
| `knc_store_put` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | CRDT Distributed KV | Protected |
| `knc_store_get` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | CRDT Distributed KV | Protected |
| `knc_store_sync` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Anti-Entropy Sync | Protected |
| `knc_store_digest` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | State Digest SHA-256 | **Protected (Verified)** |
| `knc_store_diff` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Differential Sync | **Protected (Verified)** |
| `knc_swarm_elect` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Raft Governance | Protected |
| `knc_swarm_roles` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Raft Governance | Protected |
| `knc_swarm_quorum` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Raft Governance | Protected |
| `knc_swarm_request_vote` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Raft Consensus | **Protected (Verified)** |
| `knc_swarm_heartbeat` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Raft Governance | Protected |
| `knc_mesh_verify_peer` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Identity Verification | Protected |
| `knc_mesh_rotate_key` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Key Lifecycle | Protected |
| `knc_mesh_revoke_peer` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Revocation Gate | Protected |
| `knc_isolate_reload` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Isolate Sandboxing | Protected |
| `knc_eval_isolate` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Sandboxed Eval | Protected |
| `knc_eval_dual` | TCP, WebSocket | Ed25519 / HMAC | 30s ZT / 60s HMAC | Strict Rejection | Dual-Engine Quarantine | **Protected (Verified)** |
| `knc_meaning_of_life` | TCP, WebSocket | None (Public) | None | N/A | Deterministic Protocol | Public |
| `sys.meaning_of_life` | TCP, WebSocket | None (Public) | None | N/A | Protocol Alias | Public |

### Verification of Newest Endpoints:
1. **`knc_eval_dual`**: Fully auth-gated via `check_mesh_auth`. Enforces divergence quarantine protocol `-32020` on semantic discrepancy.
2. **`knc_store_diff`**: Fully auth-gated via `check_mesh_auth`. Computes delta synchronization sets bounded by `since_timestamp` and `limit`.
3. **`knc_store_digest`**: Fully auth-gated via `check_mesh_auth`. Computes deterministic SHA-256 state digests over active CRDT storage entries.
4. **`knc_task_complete`**: Fully auth-gated via `check_mesh_auth`. Validates worker-signed `SignedTaskResult` with peer revocation filtering.
5. **`knc_swarm_request_vote`**: Fully auth-gated via `check_mesh_auth`. Validates candidate election term and grants single-vote invariant per term.

---

## 4. Canonical Self-Certifying Node Identity & Legacy-HMAC Isolation (`v2.24.23`)

KnotenCore establishes self-certifying, deterministic node identity derivation for all Ed25519 mesh peers, superseding First-Seen Key Pinning (TOFU) and strictly isolating legacy authentication domains:

1. **Deterministic Canonical Identity Derivation**:
   - For all Ed25519 peers, `node_id` is deterministically computed from the complete 32-byte Ed25519 public key: `knc-<64_hex_chars>`.
   - Any signed envelope, heartbeat, or peer registration asserting a divergent `sender_node_id`, `leader_id`, or `peer_id` is rejected immediately with `-32001` (`ERR_UNAUTHORIZED`), completely eliminating front-running and node identity squatting.
2. **Strict Legacy-HMAC Domain Isolation**:
   - In mixed-mode clusters, requests authenticated via pre-shared HMAC secret tokens or signatures are strictly forbidden from claiming canonical self-certifying identities.
   - Any Legacy-HMAC request claiming an identity starting with the `knc-` prefix is unconditionally rejected with `-32001` (`ERR_UNAUTHORIZED`).
3. **Ephemeral In-Memory Peer State**:
   - Verified peer public keys are held exclusively in ephemeral in-memory state (`verified_peer_keys: Mutex<HashMap<String, String>>`) and are not persisted to unauthenticated disk caches, eliminating disk-based poisoning vectors across restarts.

---

## 5. Formal L4/L7 Zero-Trust Architecture Invariant

**TCP connectivity provides zero trust. Network reachability implies no privilege; trust is established exclusively via cryptographically signed and authenticated protocol envelopes.**

### Transport & Dispatch Audit
1. **L4 (Network / Transport Layer)**:
   - Establishing a TCP connection or completing a WebSocket handshake grants zero execution privileges.
   - Raw network packets or unauthenticated JSON-RPC requests are strictly denied execution on all protected endpoints, with access restricted solely to whitelisted public introspection endpoints (`knc_meaning_of_life`, `sys.meaning_of_life`).
2. **L7 (Application / Envelope Layer)**:
   - Every protected RPC endpoint verifies the Ed25519 signature of the caller over the canonical serialized payload, monotonic nonce, and timestamp window (30s).
   - **Deterministic Identity Binding**: A peer's declared `sender_node_id` is strictly and immutably bound 1-to-1 with its Ed25519 `public_key`. Envelopes with mismatched, spoofed, altered, or foreign node identities are rejected immediately with `ERR_UNAUTHORIZED` (`-32001`).
   - **Fail-Safe Poisoning Resilience**: Critical internal security locks (`verified_peer_keys`, `revoked_peer_keys`, `zero_trust_mode`, and `SwarmGovernance`) enforce fail-closed and fail-safe semantics. In the event of thread panics poisoning locks, state mutations are unconditionally rejected with `InternalSecurityError` and access defaults to fail-closed (`is_zero_trust() == true`, `is_peer_key_revoked() == true`, `role() == NodeRole::Observer`).

---

## 6. Reporting a Vulnerability

**Do NOT report security vulnerabilities in public GitHub issues.**

If you discover a potential vulnerability:
1. Open a **GitHub Security Advisory** privately on this repository.
2. Provide full reproduction steps (including payload, configuration, and environment details).
3. We acknowledge reports within 48 hours and coordinate fixes prior to public disclosure.
