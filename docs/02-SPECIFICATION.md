# 0k-Sync - Technical Specification

**Version:** 2.3.0
**Date:** 2026-02-03
**Author:** James (LTIS Investments AB)
**Audience:** Implementers, Developers

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture](#2-architecture)
3. [Protocol Stack](#3-protocol-stack)
4. [Security Model](#4-security-model)
5. [Message Specification](#5-message-specification)
6. [Client Library](#6-client-library)
7. [Relay Server](#7-relay-server)
8. [Pairing Flow](#8-pairing-flow)
9. [Framework Integration](#9-framework-integration-tauri-example)
10. [Mobile Lifecycle Considerations](#10-mobile-lifecycle-considerations)
11. [Tier-Specific Configuration](#11-tier-specific-configuration)
12. [Error Handling](#12-error-handling)
13. [Configuration Reference](#13-configuration-reference)
14. [Best Practices](#14-best-practices)
15. [Device Revocation](#15-device-revocation)
16. [Push Notification Integration](#16-push-notification-integration)
17. [Large Content Transfer Protocol](#17-large-content-transfer-protocol)
- [Appendix A: Crate Dependencies](#appendix-a-crate-dependencies)

---

## 1. Overview

### 1.1 Purpose

0k-Sync provides secure, zero-knowledge synchronization between multiple instances of any local-first application across devices (desktop, mobile, web).

### 1.2 Design Principles

| Principle | Meaning |
|-----------|---------|
| **Zero Knowledge** | Relay cannot decrypt user data |
| **Local-First** | Apps work offline; sync is opportunistic |
| **Pass-Through Only** | Relay routes messages, does not store data long-term |
| **Client Constant** | Same client library for all relay tiers |
| **Open Standards** | 100% open source dependencies |

### 1.3 Non-Goals

- Real-time collaborative editing (CRDTs are app responsibility)
- File sync (use Syncthing; this is for app state)
- User accounts (zero-knowledge, QR pairing only)
- Background sync on mobile (OS limitations)
- Data storage (relay is ephemeral)

---

## 2. Architecture

### 2.1 Component Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Local-First Application                           │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                         Application Code                            │ │
│  │  • Business logic, UI, local database                              │ │
│  └───────────────────────────────┬────────────────────────────────────┘ │
│                                  │ push(blob) / pull()                  │
│  ┌───────────────────────────────▼────────────────────────────────────┐ │
│  │               Framework Integration (optional)                      │ │
│  │  • Platform-specific bindings (Tauri, Electron, React Native...)   │ │
│  │  • Event emission to frontend                                       │ │
│  └───────────────────────────────┬────────────────────────────────────┘ │
│                                  │                                      │
│  ┌───────────────────────────────▼────────────────────────────────────┐ │
│  │                         sync-client                                 │ │
│  │  • E2E encryption (Group Key)                                      │ │
│  │  • Connection management                                            │ │
│  │  • Cursor tracking                                                  │ │
│  │  • Pairing flow                                                     │ │
│  └───────────────────────────────┬────────────────────────────────────┘ │
└──────────────────────────────────┼──────────────────────────────────────┘
                                   │
                                   ▼
                    ┌──────────────────────────┐
                    │     Relay (any tier)     │
                    │  • Route encrypted blobs │
                    │  • Temporary buffer      │
                    │  • Delete on ACK         │
                    └──────────────────────────┘
```

### 2.2 Component Responsibilities

| Component | Responsibility |
|-----------|----------------|
| **Application** | Business logic, CRDT merge, conflict resolution, UI |
| **Framework Integration** | Platform bindings (optional), state management, events |
| **sync-client** | E2E encryption, connection, cursor tracking, pairing |
| **sync-types** | Wire format, message definitions, shared types |
| **sync-core** | Pure logic (state machine, buffer), no I/O |
| **sync-relay** | Connection management, message routing, temp buffer |

### 2.3 Data Flow

```
                    PUSH FLOW
                    ─────────
App Data ──► Serialize ──► Encrypt (Group Key) ──► Envelope ──►
         ──► Transport Encrypt (Noise) ──► iroh (QUIC) ──► Peer/Relay ──►
         ──► Forward to online peers ──► Buffer for offline peers

                    PULL FLOW
                    ─────────
Peer/Relay ──► iroh (QUIC) ──► Transport Decrypt (Noise) ──► Envelope ──►
           ──► Decrypt (Group Key) ──► Deserialize ──► App Data
```

---

## 3. Protocol Stack

### 3.1 Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 4: Application Sync Logic                            │
│  • App-specific merge strategies (CRDTs, LWW, etc.)         │
│  • Schema definitions                                       │
│  • UI sync status                                           │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: Content Transfer (via iroh-blobs)                 │
│  • ContentReference metadata via sync protocol              │
│  • Encrypt-then-hash for large blobs (photos, docs, audio)  │
│  • iroh-blobs for verified streaming transfer               │
│  • Thumbnail generation & progressive loading               │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: Sync Protocol (0k-Sync)                           │
│  • Cursor-ordered encrypted blobs (PUSH, PULL, NOTIFY)      │
│  • Device pairing / revocation                              │
│  • Sync groups with HELLO/WELCOME handshake                 │
│  • TTL-based relay buffering                                │
│  • E2E: XChaCha20-Poly1305 + Argon2id                       │
│  • Envelope routing (group_id, sender_id, cursor)           │
├─────────────────────────────────────────────────────────────┤
│  Layer 1: Hybrid Transport Security                         │
│  • Noise XX handshake via clatter                           │
│  • Curve25519 + ML-KEM-768 hybrid KEM                       │
│  • Post-quantum compliance from day one                     │
│  • Per-session keys, forward secrecy                        │
├─────────────────────────────────────────────────────────────┤
│  Layer 0: iroh Transport                                    │
│  • QUIC via Quinn (authenticated, encrypted)                │
│  • Hole punching + relay fallback                           │
│  • Discovery: DNS + mDNS + optional DHT                     │
│  • Connection migration (WiFi ↔ cellular)                   │
│  • ALPN routing: /private-sync/1, /iroh-bytes/4             │
└─────────────────────────────────────────────────────────────┘
```

> **Amendment (2026-02-02):** Layer structure updated per iroh-deep-dive-report.md recommendations. Added Layer 3 (Content Transfer) for large file handling via iroh-blobs. Layer 0 now explicitly includes mDNS local discovery and ALPN routing.

### 3.2 Serialization

**Format:** MessagePack (rmp-serde)

**Rationale:**
- Binary format (smaller than JSON)
- Schema-less (flexible evolution)
- Well-supported in Rust
- Cross-platform compatibility

---

## 4. Security Model

### 4.1 Key Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│                    Key Derivation                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  User Passphrase (or random 256-bit secret)                │
│           │                                                 │
│           ▼  Argon2id (device-adaptive, see below)         │
│                                                             │
│  Group Secret (256-bit)                                    │
│           │                                                 │
│           ▼  HKDF-SHA256                                   │
│     ┌─────┼─────────────┐                                  │
│     │     │             │                                  │
│     ▼     ▼             ▼                                  │
│  Sync   Auth      Content Key                              │
│  Key    Key       (for large files)                        │
│  (256b) (256b)    (256b per blob)                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Content Key Derivation (for large file transfer via iroh-blobs):**

Large content (photos, documents, audio) uses a separate key derived per-blob to allow independent lifecycle management:

```
content_key = HKDF-SHA256(
    ikm = GroupSecret,
    salt = "0k-sync-content-v1",
    info = blob_id || "content-encryption"
) → 32 bytes for XChaCha20-Poly1305
```

This allows:
- Same key for all devices in the group (they share GroupSecret)
- Independent rotation from sync blob encryption key
- Per-blob key isolation (compromise of one content key doesn't affect others)

**Device-Adaptive Argon2id Parameters:**

OWASP minimum (19 MiB, 2 iterations) performs well on modern devices but hits 800ms+ on low-end mobile. Use device-adaptive parameters:

| Device Class | Detection Signal | Memory | Iterations | Target Time |
|--------------|------------------|--------|------------|-------------|
| Low-end mobile | RAM < 2GB | 12 MiB | 3 | 300-500ms |
| Mid-range mobile | RAM 2-4GB | 19 MiB | 2 | 200-400ms |
| High-end mobile | RAM > 4GB | 46 MiB | 1 | 200-400ms |
| Desktop | Always | 64 MiB | 3 | 200-500ms |

**iOS Constraint:** AutoFill extension processes have ~55 MiB usable memory. Configurations above 46 MiB fail intermittently.

### 4.2 Noise Protocol Configuration

**Handshake Pattern:** XX (mutual authentication)

```
XX:
  → e
  ← e, ee, s, es
  → s, se
```

**Why XX:**
- Both parties prove identity
- Neither needs pre-shared keys
- Forward secrecy from message 2

**Cryptographic Primitives (Hybrid Post-Quantum):**

| Function | Algorithm | Crate |
|----------|-----------|-------|
| Key Exchange | X25519 + ML-KEM-768 | clatter **v2.1+** |
| Cipher | ChaChaPoly | clatter **v2.1+** |
| Hash | BLAKE2s | clatter **v2.1+** |

> ⚠️ **Hybrid Handshake:** Uses `noise_hybrid_XX` pattern with ML-KEM-768 (NIST Level 3) for quantum resistance. The clatter crate provides verified hybrid Noise protocol implementation.

### 4.3 Device Identity

- Each device generates Curve25519 keypair on first launch
- Public key = Device ID (32 bytes, base64 for display)
- Private key stored in OS keychain (via platform-specific secure storage)

### 4.4 Threat Model

| Threat | Mitigation |
|--------|------------|
| Relay reads data | E2E encryption; relay sees only ciphertext |
| MITM attack | Noise mutual auth + TLS |
| Replay attack | Nonces + monotonic cursors |
| Device compromise | Per-device keys; rotate Group Key to revoke |
| Relay compromise | No plaintext stored; temporary buffer only |
| Traffic analysis | Optional PADME padding (future) |

### 4.5 Trust Assumptions

1. User's devices are not compromised
2. Noise Protocol cryptography is sound
3. Argon2id parameters are sufficient
4. Relay is honest-but-curious

---

## 5. Message Specification

### 5.1 Envelope Format

```rust
#[derive(Serialize, Deserialize)]
pub struct Envelope {
    /// Protocol version (currently 1)
    pub version: u8,

    /// Message type (see MessageType enum)
    pub msg_type: u8,

    /// Sender's device public key
    pub sender_id: [u8; 32],

    /// Sync group identifier
    pub group_id: [u8; 32],

    /// Monotonic sequence number (assigned by relay)
    pub cursor: u64,

    /// Wall clock timestamp (informational only)
    pub timestamp: u64,

    /// Unique nonce for this message
    pub nonce: [u8; 24],

    /// E2E encrypted payload
    pub payload: Vec<u8>,
}
```

### 5.2 Message Types

| Type | Value | Direction | Purpose |
|------|-------|-----------|---------|
| `HELLO` | 0x01 | Client → Relay | Initial connection, declare group |
| `WELCOME` | 0x02 | Relay → Client | Connection accepted, relay info |
| `PUSH` | 0x10 | Client → Relay | Upload encrypted blob |
| `PUSH_ACK` | 0x11 | Relay → Client | Blob received, cursor assigned |
| `PULL` | 0x20 | Client → Relay | Request blobs after cursor |
| `PULL_RESPONSE` | 0x21 | Relay → Client | Deliver requested blobs |
| `PRESENCE` | 0x30 | Client → Relay | Heartbeat, online status |
| `NOTIFY` | 0x31 | Relay → Client | New blob available |
| `DELETE` | 0x40 | Client → Relay | Remove blob (after all ACK) |
| `REVOKE_DEVICE` | 0x50 | Client → Relay | Remove device from sync group |
| `DEVICE_REVOKED` | 0x51 | Relay → Client | Notify of device revocation |
| `REGISTER_PUSH` | 0x60 | Client → Relay | Register push notification token |
| `UNREGISTER_PUSH` | 0x61 | Client → Relay | Unregister push token |
| `CONTENT_REF` | 0x70 | Client → Relay | Large content reference (iroh-blobs) |
| `CONTENT_ACK` | 0x71 | Client → Relay | Acknowledge content transfer complete |
| `ERROR` | 0xFF | Either | Error with code and message |

### 5.3 Message Structures

#### HELLO
```rust
pub struct Hello {
    pub version: u8,
    pub device_name: String,      // Human readable
    pub group_id: [u8; 32],
    pub last_cursor: u64,         // 0 if first sync
}
```

#### WELCOME
```rust
pub struct Welcome {
    pub version: u8,
    pub relay_id: String,
    pub max_cursor: u64,          // Highest cursor for group
    pub pending_count: u32,       // Blobs waiting for this device
}
```

#### PUSH
```rust
pub struct Push {
    pub blob_id: [u8; 16],        // Client-generated UUID
    pub payload: Vec<u8>,         // E2E encrypted
    pub ttl: u32,                 // Seconds until auto-delete (0 = default)
}
```

#### PUSH_ACK
```rust
pub struct PushAck {
    pub blob_id: [u8; 16],
    pub cursor: u64,              // Assigned cursor
    pub timestamp: u64,           // Server timestamp
}
```

#### PULL
```rust
pub struct Pull {
    pub after_cursor: u64,        // Return blobs with cursor > this
    pub limit: u32,               // Max blobs (default 100)
}
```

#### PULL_RESPONSE
```rust
pub struct PullResponse {
    pub blobs: Vec<BlobEntry>,
    pub has_more: bool,
    pub max_cursor: u64,
}

pub struct BlobEntry {
    pub blob_id: [u8; 16],
    pub cursor: u64,
    pub sender_id: [u8; 32],
    pub payload: Vec<u8>,
    pub timestamp: u64,
}
```

#### NOTIFY
```rust
pub struct Notify {
    pub blob_id: [u8; 16],
    pub cursor: u64,
    pub sender_id: [u8; 32],
    pub timestamp: u64,
    pub size: u32,
}
```

#### ERROR
```rust
pub struct Error {
    pub code: u32,
    pub message: String,
}
```

#### CONTENT_REF (Large Content Transfer)
```rust
/// Reference to large content stored via iroh-blobs
/// The actual encrypted content is transferred P2P via iroh-blobs,
/// only this small metadata blob goes through the sync relay.
pub struct ContentRef {
    /// Client-generated UUID (our protocol's ID)
    pub blob_id: [u8; 16],

    /// BLAKE3 hash of CIPHERTEXT (iroh-blobs content address)
    /// This is the hash of encrypted bytes, not plaintext
    pub content_hash: [u8; 32],

    /// XChaCha20-Poly1305 nonce used for encryption
    pub encryption_nonce: [u8; 24],

    /// Original plaintext size in bytes
    pub content_size: u64,

    /// Ciphertext size in bytes
    pub encrypted_size: u64,

    /// MIME type ("image/jpeg", "audio/opus", "application/pdf")
    pub mime_type: String,

    /// Optional thumbnail hash (also encrypted, for preview)
    pub thumbnail_hash: Option<[u8; 32]>,

    /// Thumbnail nonce (if thumbnail present)
    pub thumbnail_nonce: Option<[u8; 24]>,
}
```

#### CONTENT_ACK
```rust
/// Acknowledge successful content transfer
pub struct ContentAck {
    /// Reference to the content that was transferred
    pub blob_id: [u8; 16],

    /// Hash that was successfully received and verified
    pub content_hash: [u8; 32],

    /// Timestamp of successful transfer
    pub timestamp: u64,
}
```

> **Note:** Content transfers bypass the sync relay entirely. The relay only sees the small ContentRef metadata blob. Actual encrypted content is transferred device-to-device via iroh-blobs (QUIC/P2P). See Section 17: Large Content Transfer Protocol.

### 5.4 Cursor vs Timestamp

**Why cursors instead of timestamps?**

| Problem with Timestamps | Solution with Cursors |
|------------------------|----------------------|
| Device clocks drift | Relay assigns monotonic cursor |
| No guaranteed ordering | Cursor guarantees: "after 500" = all > 500 |
| Gap detection impossible | No gaps: cursor 501 follows 500 |
| Mobile clocks unreliable | Cursor independent of device clock |

Timestamps are kept for logging/debugging only, never for ordering.

---

## 6. Client Library

### 6.1 Public API

```rust
pub struct SyncClient {
    // Internal: connection, state, crypto
}

impl SyncClient {
    /// Create new client
    pub async fn new(config: SyncConfig) -> Result<Self>;

    /// Connect to relay
    pub async fn connect(&mut self) -> Result<ConnectionInfo>;

    /// Disconnect gracefully
    pub async fn disconnect(&mut self) -> Result<()>;

    /// Push encrypted blob
    pub async fn push(&self, data: &[u8]) -> Result<PushResult>;

    /// Pull blobs after cursor
    pub async fn pull(&self, after_cursor: u64) -> Result<PullResult>;

    /// Subscribe to events
    pub fn subscribe(&self) -> Receiver<SyncEvent>;

    /// Current status
    pub fn status(&self) -> ConnectionStatus;

    /// Last synced cursor (persisted)
    pub fn last_cursor(&self) -> u64;

    /// Device public key
    pub fn device_id(&self) -> DeviceId;
}
```

### 6.2 Configuration

```rust
pub struct SyncConfig {
    /// Relay backend selection
    pub backend: RelayBackend,

    /// Device keypair (generated or loaded)
    pub device_keypair: Option<Keypair>,

    /// Group key (from pairing)
    pub group_key: Option<GroupKey>,

    /// Auto-reconnect on disconnect
    pub auto_reconnect: bool,

    /// Reconnect backoff (initial)
    pub reconnect_delay: Duration,

    /// Max reconnect backoff
    pub max_reconnect_delay: Duration,
}

pub enum RelayBackend {
    /// Tier 1: iroh public network (P2P + iroh-relay fallback)
    /// No custom infrastructure needed.
    Iroh,

    /// Tiers 2-3: Self-hosted sync-relay (iroh Endpoint)
    /// Connect to a known sync-relay by NodeId.
    SyncRelay {
        node_id: iroh::NodeId,
        relay_url: Option<Url>,  // Optional custom iroh-relay for this deployment
    },

    /// Tiers 4-5: Managed Cloud
    /// API key resolves to a sync-relay NodeId via discovery service.
    ManagedCloud {
        api_key: String,
    },

    /// Tier 6: Enterprise
    /// Dedicated sync-relay with enterprise authentication.
    Enterprise {
        node_id: iroh::NodeId,
        auth: EnterpriseAuth,
    },
}
```

### 6.3 Events

```rust
pub enum SyncEvent {
    /// Connected to relay
    Connected { info: ConnectionInfo },

    /// Disconnected from relay
    Disconnected { reason: String },

    /// New blob available (NOTIFY received)
    BlobAvailable { id: BlobId, cursor: u64, sender: DeviceId },

    /// Blob successfully pushed
    BlobPushed { id: BlobId, cursor: u64 },

    /// Error occurred
    Error { code: u32, message: String },

    /// Connection status changed
    StatusChanged { status: ConnectionStatus },
}

pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32 },
}
```

### 6.4 Encryption Flow

**Encrypt (before push):**
```
plaintext
    → serialize (MessagePack)
    → encrypt (XChaCha20-Poly1305 with Group Key + random 192-bit nonce)
    → wrap in Envelope
    → encrypt (Noise session key)
    → send
```

**Decrypt (after pull):**
```
receive
    → decrypt (Noise session key)
    → unwrap Envelope
    → decrypt (XChaCha20-Poly1305 with Group Key + nonce from envelope)
    → deserialize (MessagePack)
    → plaintext
```

> **Why XChaCha20 (not standard ChaCha20)?** Standard ChaCha20-Poly1305 uses 96-bit nonces with a safe threshold of ~4.3 billion messages. XChaCha20 uses 192-bit nonces, making random nonce generation safe without cross-device coordination (safe threshold: 2^80).

---

## 7. Relay Server

### 7.1 Responsibilities

| Do | Don't |
|----|-------|
| Accept iroh connections (QUIC) | Store data long-term |
| Noise handshake | Decrypt payloads |
| Route messages by group_id | Know what's in blobs |
| Buffer for offline devices (temp) | Require accounts |
| Assign monotonic cursors | Process application logic |
| Clean up expired blobs | |

### 7.2 Storage Schema (Temporary Buffer)

```sql
-- SQLite with WAL mode for concurrent access
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA busy_timeout=5000;

-- Cursor sequence per group
CREATE TABLE group_cursors (
    group_id BLOB PRIMARY KEY,
    next_cursor INTEGER NOT NULL DEFAULT 1
);

-- Temporary blob buffer
CREATE TABLE blobs (
    blob_id BLOB PRIMARY KEY,
    group_id BLOB NOT NULL,
    cursor INTEGER NOT NULL,
    sender_id BLOB NOT NULL,
    payload BLOB NOT NULL,         -- Still encrypted
    timestamp INTEGER,
    expires_at INTEGER NOT NULL,   -- Auto-delete time
    created_at INTEGER,
    UNIQUE(group_id, cursor)
);

-- Track delivery status
CREATE TABLE deliveries (
    blob_id BLOB,
    device_id BLOB,
    delivered_at INTEGER,
    PRIMARY KEY (blob_id, device_id)
);

-- Indexes
CREATE INDEX idx_blobs_group_cursor ON blobs(group_id, cursor);
CREATE INDEX idx_blobs_expires ON blobs(expires_at);
```

### 7.3 Relay Behavior

**On Connection (HELLO):**
1. Complete Noise handshake
2. Validate HELLO message
3. Register device for group
4. Send WELCOME with max_cursor, pending_count
5. Send NOTIFY for each pending blob

**On PUSH:**
1. Begin transaction
2. Increment cursor atomically:
   ```sql
   INSERT INTO group_cursors (group_id, next_cursor) VALUES (?, 1)
   ON CONFLICT(group_id) DO UPDATE SET next_cursor = next_cursor + 1
   RETURNING next_cursor - 1 AS assigned_cursor;
   ```
3. Store blob with cursor and TTL
4. Commit
5. Send PUSH_ACK to sender
6. Send NOTIFY to all other online devices in group

**On PULL:**
1. Query blobs where cursor > after_cursor
2. Order by cursor ASC
3. Limit results
4. Return blobs with has_more flag
5. Mark as delivered for this device

**On DELETE:**
1. Check if all devices have ACKed
2. If yes, delete blob
3. If no, mark pending delete

**Cleanup (hourly):**
1. Delete expired blobs (past TTL)
2. Delete blobs where all devices ACKed
3. Run PRAGMA incremental_vacuum

### 7.4 Rate Limits

| Resource | Limit | Window |
|----------|-------|--------|
| Connections per IP | 10 | Concurrent |
| Messages per device | 100 | Per minute |
| Blob size | 1 MB | Per blob |
| Buffer per group | 100 MB | Total |
| Default TTL | 7 days | Per blob |

### 7.5 Health Endpoints

**GET /health**
```json
{
  "status": "ok",
  "version": "1.0.0",
  "connections": 42,
  "groups": 15,
  "blobs_buffered": 127,
  "uptime_seconds": 86400
}
```

**GET /metrics** (Prometheus format)
```
sync_relay_connections_total 1567
sync_relay_connections_active 42
sync_relay_blobs_buffered 127
sync_relay_blobs_delivered 45892
sync_relay_bytes_transferred 157286400
```

---

## 8. Pairing Flow

### 8.1 Overview

**Zero accounts. QR or short code pairing.**

### 8.2 Create Sync Group (First Device)

```
1. User taps "Enable Sync"
2. Generate:
   - group_id: random 32 bytes
   - group_secret: random 32 bytes
   - device_keypair (if not exists)
3. Create invite payload:
   {
     relay_node: "sync-relay-node-id",  // NodeId of sync-relay, or omitted for Tier 1
     group_id: base64(group_id),
     group_secret: base64(group_secret),
     created_by: base64(device_pubkey),
     expires: unix_timestamp + 600  // 10 minutes
   }
4. Display QR code or short code
```

### 8.3 Join Sync Group (Second Device)

```
1. User taps "Join Sync"
2. Scan QR or enter short code
3. Decode invite payload
4. Store in keychain:
   - group_id
   - group_secret (→ derive Group Key via Argon2id)
   - relay NodeId (if present — Tier 1 uses iroh public network, no relay NodeId needed)
5. Connect to relay
6. Sync begins
```

### 8.4 QR Code Format

```
URL: your-app://sync?invite=BASE64_PAYLOAD

Payload (before base64):
{
  "v": 1,
  "r": "sync-relay-node-id-or-discovery-url",
  "g": "base64(group_id)",
  "s": "base64(group_secret)",
  "c": "base64(creator_pubkey)",
  "e": 1705000000
}

Note: For Tier 1 (iroh public network), the `r` field is omitted — peers discover each other via the public iroh relay network. For Tiers 2-6, `r` contains the sync-relay's NodeId or an HTTPS discovery URL (e.g., `https://sync.example.com/.well-known/iroh`) that resolves to a NodeId.
```

Note: Replace `your-app://` with your application's custom URL scheme.

### 8.5 Short Code Format (Alternative)

16-character alphanumeric: `XXXX-XXXX-XXXX-XXXX`

Split:
- First 8 chars: lookup_key (sent to relay)
- Last 8 chars: decrypt_key (never sent)

Flow:
1. Creator encrypts payload with decrypt_key
2. Creator POSTs `{lookup_key, encrypted_payload}` to relay
3. Joiner GETs `/invite/{lookup_key}` from relay
4. Relay returns encrypted_payload and deletes it
5. Joiner decrypts with decrypt_key

**Relay never sees decrypt_key or plaintext invite.**

### 8.6 Invite Security

| Property | Mechanism |
|----------|-----------|
| Time-limited | 10 minute expiry |
| Single-use | Deleted on first claim |
| Encrypted | Relay can't read (for short codes) |
| Revocable | Creator can cancel |

---

## 9. Framework Integration (Tauri Example)

### 9.1 Rust Side

```rust
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime, State,
};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("sync")
        .invoke_handler(tauri::generate_handler![
            sync_enable,
            sync_disable,
            sync_create_invite,
            sync_join_invite,
            sync_push,
            sync_pull,
            sync_status,
            sync_last_cursor,
        ])
        .setup(|app, api| {
            // Initialize sync state
            app.manage(SyncState::new(api.config()));
            Ok(())
        })
        .build()
}

#[tauri::command]
async fn sync_enable(state: State<'_, SyncState>) -> Result<(), String>;

#[tauri::command]
async fn sync_create_invite(state: State<'_, SyncState>) -> Result<Invite, String>;

#[tauri::command]
async fn sync_join_invite(
    state: State<'_, SyncState>,
    invite: String,
) -> Result<(), String>;

#[tauri::command]
async fn sync_push(
    state: State<'_, SyncState>,
    data: Vec<u8>,
) -> Result<PushResult, String>;

#[tauri::command]
async fn sync_pull(
    state: State<'_, SyncState>,
    after_cursor: Option<u64>,
) -> Result<PullResult, String>;

#[tauri::command]
fn sync_status(state: State<'_, SyncState>) -> ConnectionStatus;

#[tauri::command]
fn sync_last_cursor(state: State<'_, SyncState>) -> u64;
```

### 9.2 JavaScript API

```typescript
// @0k-sync/tauri-plugin

export interface SyncPlugin {
  enable(): Promise<void>;
  disable(): Promise<void>;
  createInvite(): Promise<Invite>;
  joinInvite(invite: string): Promise<void>;
  push(data: Uint8Array): Promise<PushResult>;
  pull(afterCursor?: number): Promise<PullResult>;
  status(): ConnectionStatus;
  lastCursor(): number;

  // Event listeners
  on(event: 'connected', handler: (info: ConnectionInfo) => void): void;
  on(event: 'disconnected', handler: (reason: string) => void): void;
  on(event: 'blob-available', handler: (blob: BlobInfo) => void): void;
  on(event: 'error', handler: (error: SyncError) => void): void;
}

export interface Invite {
  qrCode: string;      // Data URL for QR image
  shortCode: string;   // XXXX-XXXX-XXXX-XXXX
  expiresAt: number;   // Unix timestamp
}

export interface PushResult {
  blobId: string;
  cursor: number;
}

export interface PullResult {
  blobs: Blob[];
  hasMore: boolean;
  maxCursor: number;
}

export interface Blob {
  id: string;
  cursor: number;
  sender: string;
  data: Uint8Array;
  timestamp: number;
}
```

### 9.3 Usage Example

```typescript
import { sync } from '@0k-sync/tauri-plugin';

// Initialize sync
await sync.enable();

// Create invite for new device
const invite = await sync.createInvite();
showQRCode(invite.qrCode);

// Or join existing sync group
await sync.joinInvite('XXXX-XXXX-XXXX-XXXX');

// Push data
const blob = new TextEncoder().encode(JSON.stringify(myData));
const result = await sync.push(blob);
console.log(`Synced at cursor ${result.cursor}`);

// Pull data
const { blobs, hasMore } = await sync.pull(lastKnownCursor);
for (const blob of blobs) {
  const data = JSON.parse(new TextDecoder().decode(blob.data));
  await mergeIntoLocalDb(data);
}

// Listen for real-time updates
sync.on('blob-available', async (info) => {
  const { blobs } = await sync.pull(info.cursor - 1);
  // Process new blob
});
```

---

## 10. Mobile Lifecycle Considerations

### 10.1 The "Mobile Exit" Problem

> ⚠️ **Critical Reality:** On iOS and modern Android, background network connections (including QUIC/iroh) are killed within ~30 seconds of the app being backgrounded. You cannot rely on persistent connections for sync.

**iOS `applicationWillTerminate`** does NOT guarantee enough execution time to:
1. Establish an iroh connection
2. Perform a Noise handshake
3. Upload a blob

The OS watchdog will kill your app before completing.

**Design Assumption:** Data generated while offline or just before closing **will not sync until the next app launch**. The UI must reflect this.

### 10.2 Sync Status Indicators

The UI **must** show sync state to avoid user confusion:

| Indicator | Meaning | User Action |
|-----------|---------|-------------|
| ☁️✓ | Synced to relay | None needed |
| ☁️⏳ | Pending sync (will sync on next launch) | None; automatic |
| ☁️✗ | Sync failed (will retry) | Check connection |
| 📴 | Offline (changes saved locally) | Restore connection |

### 10.3 Stranded Commits

**Definition:** Local changes that exist only on the device and haven't synced to the relay.

**Rules:**
1. **Never block UI** trying to sync on exit — you will annoy users or trigger OS watchdog
2. **Assume stranded commits** — local changes may not sync until next launch
3. **Local-first always** — save to local DB first, sync is opportunistic

### 10.4 Optimistic Local Updates Pattern

The UI should **never wait** for sync to complete before showing changes:

```rust
// In your local-first app
async fn save_transaction(tx: Transaction, sync: &SyncClient, db: &LocalDb) {
    // 1. Save locally FIRST (instant, always succeeds)
    db.insert(&tx).await?;

    // 2. Update UI immediately (optimistic)
    emit_to_frontend("transaction_saved", &tx);

    // 3. Queue for sync (fire-and-forget)
    let blob = serialize(&tx);
    match sync.push(&blob).await {
        Ok(result) => {
            // Mark as synced in local DB
            db.mark_synced(tx.id, result.cursor).await?;
            emit_to_frontend("sync_status", SyncStatus::Synced);
        }
        Err(_) => {
            // Will retry on next app launch
            emit_to_frontend("sync_status", SyncStatus::Pending);
        }
    }
}
```

**Key Principle:** The local database is the source of truth. Sync is opportunistic. Users should never lose data because sync failed.

### 10.5 Sync Trigger Points

Sync should be triggered on these application lifecycle events:

```rust
// Pseudocode - adapt to your framework (Tauri, Electron, React Native, etc.)

// On app startup
async fn on_app_start(sync_client: &SyncClient) {
    // Initial sync on app start
    sync_client.connect_and_pull().await;
}

// On app resumed (came to foreground)
async fn on_app_resume(sync_client: &SyncClient) {
    // App came to foreground - pull new data, push pending
    sync_client.sync_pending_then_pull().await;
}

// On app closing
async fn on_app_close(sync_client: &SyncClient) {
    // App closing - DO NOT block waiting for sync!
    // Just mark pending items; they'll sync next launch
    sync_client.mark_pending_for_next_launch();

    // Fire-and-forget attempt (may not complete)
    let _ = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        sync_client.quick_flush()
    ).await;
}
```

### 10.6 Sync Strategy Summary

| Event | Action | Blocking? |
|-------|--------|-----------|
| App launch | Connect + push pending + pull new | No (background) |
| App resume (foreground) | Push pending + pull new | No (background) |
| App pause (background) | Mark state, fire-and-forget flush | No (500ms max) |
| User action (save, etc.) | Save local, queue for sync, attempt push | No |
| Manual "Sync Now" button | Full sync cycle | Yes (with spinner) |

### 10.7 What We Don't Support (Yet)

| Feature | Reason | Future Possibility |
|---------|--------|-------------------|
| Background sync | iOS/Android kill background connections | Push notifications |
| Push notifications for new data | Requires APNS/FCM integration | Optional plugin |
| Always-on sync | Not possible on mobile without OS support | None |
| Guaranteed sync on exit | OS doesn't allow it | None |

---

## 11. Tier-Specific Configuration

### 11.1 Tier 1: Vibe Coder (iroh)

```rust
SyncConfig {
    backend: RelayBackend::Iroh,
    ..Default::default()
}
```

Uses iroh public network. No relay infrastructure needed.

> **iroh Version Strategy:**
> - Using **iroh 0.96** (pre-1.0, requires cargo patch for curve25519-dalek)
> - iroh-blobs 0.98 for large content transfer
> - Self-hosted infrastructure available via iroh-relay and iroh-dns-server

### 11.2 Tier 2: Home Developer

```rust
SyncConfig {
    backend: RelayBackend::SyncRelay {
        node_id: "your-sync-relay-node-id".parse()?,
        relay_url: None,  // Uses default iroh-relay, or set custom
    },
    ..Default::default()
}
```

Self-hosted Docker container. Discovered via mDNS on LAN or DNS for remote. Cloudflare Tunnel can proxy the QUIC connection.

### 11.3 Tier 3: Vercel-style

```rust
SyncConfig {
    backend: RelayBackend::SyncRelay {
        node_id: "fly-relay-node-id".parse()?,
        relay_url: Some("https://relay.fly.dev".parse()?),
    },
    ..Default::default()
}
```

Container deployed to Fly.io. NodeId published via DNS TXT record at a known domain.

### 11.4 Tier 4-5: Managed Cloud

```rust
SyncConfig {
    backend: RelayBackend::ManagedCloud {
        api_key: "cn_live_xxxx".into(),
    },
    ..Default::default()
}
```

API key authenticates to CrabNebula managed infrastructure. Discovery service resolves API key to a sync-relay NodeId.

### 11.5 Tier 6: Enterprise

```rust
SyncConfig {
    backend: RelayBackend::Enterprise {
        node_id: "enterprise-relay-node-id".parse()?,
        auth: EnterpriseAuth::Oidc {
            issuer: "https://auth.corp.com".into(),
            client_id: "sync-client".into(),
        },
    },
    ..Default::default()
}
```

Customer-deployed with enterprise auth integration. Dedicated sync-relay identified by NodeId.

---

## 12. Error Handling

### 12.1 Error Codes

| Code | Name | Description |
|------|------|-------------|
| 1000 | `INVALID_MESSAGE` | Malformed message |
| 1001 | `UNKNOWN_GROUP` | Group ID not found |
| 1002 | `UNAUTHORIZED` | Auth failed |
| 1003 | `DEVICE_REVOKED` | Device has been revoked from group |
| 1004 | `NOT_BLOB_OWNER` | Only blob creator can force delete |
| 2000 | `RATE_LIMITED` | Too many requests |
| 2001 | `BLOB_TOO_LARGE` | Exceeds 1 MB limit |
| 2002 | `GROUP_QUOTA_EXCEEDED` | Group storage full |
| 2003 | `INVALID_PUSH_TOKEN` | Push token format invalid |
| 3000 | `RELAY_OVERLOADED` | Server at capacity |
| 3001 | `RELAY_SHUTTING_DOWN` | Graceful shutdown |

### 12.2 Reconnection Strategy

> ⚠️ **Thundering Herd Mitigation Required:** After relay restart, all clients reconnect simultaneously, potentially crashing the database or exhausting connection limits. Clients MUST implement jittered backoff.

```
Attempt 1: Wait 1s + jitter
Attempt 2: Wait 2s + jitter
Attempt 3: Wait 4s + jitter
Attempt 4: Wait 8s + jitter
Attempt 5: Wait 16s + jitter
Attempt 6+: Wait 30s (max) + jitter

Jitter: 0-5000ms random (not ±20%)
Reset: On successful connection
```

**Implementation:**
```rust
async fn reconnect_with_backoff(attempt: u32) {
    let base_delay = Duration::from_millis(1000 * 2u64.pow(attempt.min(5)));
    let jitter = Duration::from_millis(rand::thread_rng().gen_range(0..5000));
    let delay = (base_delay + jitter).min(Duration::from_secs(30));
    tokio::time::sleep(delay).await;
}
```

---

## 13. Configuration Reference

### 13.1 Relay Configuration (relay.toml)

```toml
[server]
bind = "127.0.0.1:8080"
max_connections = 1000

[storage]
database = "/data/relay.db"
max_blob_size = 1048576        # 1 MB
max_group_storage = 104857600  # 100 MB
default_ttl = 604800           # 7 days

[cleanup]
interval = 3600                # 1 hour
vacuum_on_cleanup = true

[limits]
messages_per_minute = 100
connections_per_ip = 10
```

### 13.2 Client Configuration

```toml
[sync]
backend = "sync-relay"
relay_node_id = "your-sync-relay-node-id"
# or for DNS-based discovery:
# relay_discovery = "https://sync.example.com/.well-known/iroh"
auto_reconnect = true
reconnect_delay_ms = 1000
max_reconnect_delay_ms = 30000
```

### 13.3 Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `SYNC_RELAY_NODE_ID` | Override relay NodeId | Config file |
| `SYNC_API_KEY` | Managed Cloud API key | None |
| `SYNC_LOG_LEVEL` | Logging verbosity | `info` |
| `SYNC_DEVICE_NAME` | Human-readable name | Hostname |

---

## 14. Best Practices

### 14.1 Blob Size Strategy

> ⚠️ **The 1MB Blob Trap:** While sync-relay limits blobs to 1MB (appropriate for state deltas), developers will inevitably try to sync images, videos, and large files. This section provides guidance.

**What Belongs in Sync Blobs:**

| ✅ Sync via Relay | ❌ Store Elsewhere |
|-------------------|-------------------|
| Transaction records | Images/photos |
| User preferences | Videos |
| App state deltas | PDF documents |
| Small JSON (<100KB) | Large binary files |
| CRDT operations | User-generated media |

**Large Asset Strategy:**

```
┌─────────────────────────────────────────────────────────────────┐
│  Recommended Pattern: Metadata via Sync, Assets via Object Store│
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. User adds image to app                                      │
│  2. App uploads image to S3/R2/Supabase Storage                │
│  3. App creates metadata record:                                │
│     {                                                           │
│       "id": "uuid",                                             │
│       "type": "image",                                          │
│       "storage_url": "https://r2.example.com/images/abc.jpg",  │
│       "mime_type": "image/jpeg",                                │
│       "size_bytes": 2457600,                                    │
│       "created_at": 1705000000                                  │
│     }                                                           │
│  4. App syncs metadata via relay (< 1KB)                       │
│  5. Other devices pull metadata, fetch image from storage URL  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Implementation Example:**

```rust
// ❌ WRONG: Syncing large files directly
async fn save_photo_wrong(photo_bytes: &[u8], sync: &SyncClient) {
    // This will fail if photo > 1MB
    sync.push(photo_bytes).await?; // ERROR: BLOB_TOO_LARGE
}

// ✅ CORRECT: Sync metadata, store asset externally
async fn save_photo_correct(
    photo_bytes: &[u8],
    storage: &ObjectStorage,
    sync: &SyncClient,
) -> Result<PhotoRecord> {
    // 1. Upload to object storage
    let storage_url = storage.upload("photos", photo_bytes).await?;

    // 2. Create metadata record
    let record = PhotoRecord {
        id: Uuid::new_v4(),
        storage_url,
        mime_type: "image/jpeg".into(),
        size_bytes: photo_bytes.len(),
        created_at: now(),
    };

    // 3. Sync only the metadata (< 1KB)
    let metadata_blob = serde_json::to_vec(&record)?;
    sync.push(&metadata_blob).await?;

    Ok(record)
}
```

**Recommended Object Storage Options:**

| Provider | Best For | Pricing Model |
|----------|----------|---------------|
| Cloudflare R2 | Cost-effective, no egress | Pay per storage |
| Supabase Storage | Integrated with Supabase | Generous free tier |
| AWS S3 | Enterprise, existing AWS | Pay per everything |
| Self-hosted MinIO | Full control | Infrastructure cost |

---

## 15. Device Revocation

### 15.1 The Lost Device Problem

**Scenario:** A user loses their phone. The phone never comes back online to ACK pending blobs. Under normal operation, those blobs remain on the relay until the 7-day TTL expires.

**Problems:**
1. Storage waste on relay
2. Blobs stuck in "pending delivery" state
3. No way to remove the lost device from the sync group

### 15.2 Device Revocation Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    Device Revocation Flow                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. User realizes device is lost                                │
│  2. User opens Settings → Sync → Devices                        │
│  3. User selects lost device → "Remove Device"                  │
│  4. App sends REVOKE_DEVICE message to relay                    │
│  5. Relay:                                                       │
│     a. Marks device as revoked                                  │
│     b. Clears pending ACKs for that device                      │
│     c. Deletes blobs that were only pending for that device     │
│     d. Notifies other devices of revocation                     │
│  6. (Optional) Rotate Group Key for paranoid security           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 15.3 Message Specification

#### REVOKE_DEVICE
```rust
pub struct RevokeDevice {
    /// Device to revoke (public key)
    pub device_id: [u8; 32],

    /// Reason (for audit log)
    pub reason: RevokeReason,

    /// Also clear pending blobs for this device
    pub clear_pending: bool,
}

pub enum RevokeReason {
    Lost,           // Device lost/stolen
    Decommissioned, // User retired device
    Compromised,    // Security concern
}
```

#### DEVICE_REVOKED (notification to other devices)
```rust
pub struct DeviceRevoked {
    /// Revoked device
    pub device_id: [u8; 32],

    /// Who revoked it
    pub revoked_by: [u8; 32],

    /// When
    pub timestamp: u64,

    /// Reason
    pub reason: RevokeReason,
}
```

### 15.4 Force Delete

For blobs that are stuck due to offline devices (not just revoked), provide a force delete option:

```rust
pub struct Delete {
    pub blob_id: [u8; 16],

    /// Normal delete: wait for all ACKs
    /// Force delete: delete immediately, clear pending ACKs
    pub force: bool,
}
```

**Force Delete Rules:**
- Only the blob creator can force delete
- Force delete is audited (logged)
- Other devices receive DELETE_NOTIFICATION so they can remove local copies

### 15.5 Relay Behavior Changes

**On REVOKE_DEVICE:**
```sql
-- 1. Mark device as revoked
INSERT INTO revoked_devices (device_id, group_id, revoked_at, reason)
VALUES (?, ?, ?, ?);

-- 2. Clear pending deliveries for revoked device
DELETE FROM deliveries
WHERE device_id = ? AND delivered_at IS NULL;

-- 3. Delete blobs that only had this device pending
DELETE FROM blobs
WHERE blob_id IN (
    SELECT b.blob_id FROM blobs b
    LEFT JOIN deliveries d ON b.blob_id = d.blob_id
    WHERE d.blob_id IS NULL  -- No remaining pending deliveries
);
```

**On Connection (HELLO):**
```rust
// Reject connections from revoked devices
if is_device_revoked(&hello.device_id, &hello.group_id) {
    return Err(Error::DeviceRevoked);
}
```

### 15.6 Client API Additions

```rust
impl SyncClient {
    /// List all devices in the sync group
    pub async fn list_devices(&self) -> Result<Vec<DeviceInfo>>;

    /// Revoke a device (remove from sync group)
    pub async fn revoke_device(
        &self,
        device_id: DeviceId,
        reason: RevokeReason
    ) -> Result<()>;

    /// Force delete a blob (skip waiting for ACKs)
    pub async fn force_delete(&self, blob_id: BlobId) -> Result<()>;
}

pub struct DeviceInfo {
    pub device_id: DeviceId,
    pub device_name: String,
    pub last_seen: u64,
    pub pending_blobs: u32,
    pub is_online: bool,
}
```

### 15.7 Security Considerations

| Concern | Mitigation |
|---------|------------|
| Unauthorized revocation | Only devices in group can revoke |
| Revocation race condition | Atomic operation, idempotent |
| Revoked device reconnects | Checked on every HELLO |
| Key rotation after compromise | Optional but recommended |

**Optional Key Rotation:**

If a device is revoked due to compromise, the group key should be rotated:

1. Generate new group_secret
2. Distribute to remaining devices via existing E2E channel
3. Old group key becomes invalid
4. Compromised device cannot decrypt new blobs

---

## 16. Push Notification Integration

> ⚠️ **Critical for Mobile:** Push notifications are not optional for production mobile apps. Without them, users must manually open the app to receive synced data.

### 16.1 Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                 Push Notification Flow                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Device A (sender)           Relay              Device B (mobile)│
│       │                        │                      │          │
│       │── PUSH (blob) ────────►│                      │          │
│       │                        │                      │          │
│       │                        │── NOTIFY ───────────►│ (if online)
│       │                        │                      │          │
│       │                        │── Push Notification ─►│ (if offline)
│       │                        │   via APNS/FCM       │          │
│       │                        │                      │          │
│       │                        │◄───── App Wakes ─────│          │
│       │                        │◄───── PULL ──────────│          │
│       │                        │                      │          │
└─────────────────────────────────────────────────────────────────┘
```

### 16.2 Client API (Day 1 Requirements)

The sync-client MUST expose push token hooks from Day 1, even if backend integration comes later:

```rust
// sync-client API

impl SyncClient {
    pub async fn register_push_token(
        &self,
        token: String,
        platform: PushPlatform,
    ) -> Result<()>;

    pub async fn unregister_push_token(&self) -> Result<()>;
}

pub enum PushPlatform {
    Apns,      // Apple Push Notification Service
    Fcm,       // Firebase Cloud Messaging
    Web,       // Web Push (future)
}
```

```typescript
// JavaScript/TypeScript bindings (for JS-based frameworks)
export interface SyncClient {
    // ... existing methods ...

    // Push notification integration (Day 1)
    registerPushToken(token: string, platform: 'apns' | 'fcm'): Promise<void>;
    unregisterPushToken(): Promise<void>;

    // Event for handling push-triggered wake
    on(event: 'push-wake', handler: () => void): void;
}
```

### 16.3 Message Specification

#### REGISTER_PUSH
```rust
pub struct RegisterPush {
    /// Platform-specific push token
    pub token: String,

    /// Platform identifier
    pub platform: PushPlatform,

    /// App bundle ID (for APNS)
    pub app_id: Option<String>,
}
```

#### UNREGISTER_PUSH
```rust
pub struct UnregisterPush {
    /// Reason for unregistering
    pub reason: UnregisterReason,
}

pub enum UnregisterReason {
    UserDisabled,
    TokenExpired,
    AppUninstalled,
}
```

### 16.4 Relay Push Behavior

**On PUSH (when recipient offline):**
```rust
async fn handle_push_for_offline_device(
    device: &Device,
    blob: &Blob,
    push_service: &PushService,
) {
    if let Some(push_token) = device.push_token {
        // Send silent push notification
        let notification = PushNotification {
            token: push_token,
            platform: device.push_platform,
            payload: PushPayload::SilentSync {
                group_id: blob.group_id,
                cursor: blob.cursor,
            },
            // Silent push - no user-visible notification
            content_available: true,
            alert: None,
        };

        push_service.send(notification).await?;
    }
}
```

### 16.5 iOS/Android Integration

**iOS (APNS):**
```swift
// In your iOS app delegate
func application(_ application: UIApplication,
                 didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data) {
    let token = deviceToken.hexString
    // Pass to your sync client (adapt to your framework's native bridge)
    SyncClient.shared.registerPushToken(token: token, platform: .apns)
}

func application(_ application: UIApplication,
                 didReceiveRemoteNotification userInfo: [AnyHashable: Any],
                 fetchCompletionHandler: @escaping (UIBackgroundFetchResult) -> Void) {
    // Silent push received - trigger sync
    SyncClient.shared.handlePushWake()
    fetchCompletionHandler(.newData)
}
```

**Android (FCM):**
```kotlin
// In your Firebase messaging service
class SyncFirebaseService : FirebaseMessagingService() {
    override fun onNewToken(token: String) {
        // Pass to your sync client (adapt to your framework's native bridge)
        SyncClient.getInstance().registerPushToken(token, PushPlatform.FCM)
    }

    override fun onMessageReceived(message: RemoteMessage) {
        if (message.data["type"] == "sync") {
            // Trigger sync in background
            SyncClient.getInstance().handlePushWake()
        }
    }
}
```

### 16.6 Implementation Phases

| Phase | Scope | Dependency |
|-------|-------|------------|
| **Day 1** | Client API hooks (register/unregister) | None |
| **Day 1** | Message types (REGISTER_PUSH, etc.) | sync-types |
| **Phase 5** | Relay stores push tokens | sync-relay |
| **Phase 5** | Relay sends to APNS/FCM | Push service integration |
| **Future** | Web Push support | Service workers |

**Key Point:** The client-side hooks MUST exist from Day 1 so apps can register tokens. The relay-side push sending can come later, but the API contract must be stable.

---

## 17. Large Content Transfer Protocol

> **Amendment (2026-02-02):** Added per iroh-deep-dive-report.md recommendations.

### 17.1 The Problem

The sync protocol is optimized for small encrypted blobs (app state, JSON entries, ~100 KB sweet spot). When apps need to sync photos (2-10 MB), voice memos (1-5 MB), document scans (0.5-3 MB), or PDFs, pushing megabytes through a relay designed for kilobytes is inefficient.

**Affected use cases by app type:**

| App Type | Content | Size Range | Frequency |
|----------|---------|------------|-----------|
| Journal apps | Photos, voice memos | 1-15 MB | High (daily) |
| Health apps | Meal photos, progress pics, scans | 1-10 MB | High |
| Finance apps | Receipt scans, invoice PDFs | 0.5-5 MB | Medium |
| Note apps | Article images, PDF attachments | 0.5-20 MB | Medium |
| Contact apps | Contact photos | 50-500 KB | Low (small enough for sync relay) |
| Password managers | None | — | None |

### 17.2 Architecture: Encrypt-Then-Hash with iroh-blobs

Large content bypasses the sync relay entirely. The relay handles only small metadata; actual content transfers device-to-device via iroh-blobs.

**Flow:**

```
┌─────────────────────────────────────────────────────────────────┐
│                    Content Transfer Flow                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  SENDER DEVICE:                                                  │
│  1. App creates content (photo, document, etc.)                  │
│  2. Derive content_key from GroupSecret via HKDF                │
│  3. Encrypt content with XChaCha20-Poly1305 → ciphertext        │
│  4. Hash ciphertext with BLAKE3 → content_hash                  │
│  5. Store ciphertext in local iroh-blobs FsStore                │
│  6. Create ContentRef message with content_hash                 │
│  7. Send ContentRef through normal sync protocol (small blob)   │
│                                                                  │
│  RELAY:                                                          │
│  - Sees only the small ContentRef (< 1 KB)                      │
│  - Never handles the actual large content                        │
│  - Assigns cursor, notifies other devices                        │
│                                                                  │
│  RECEIVER DEVICE:                                                │
│  1. Receives ContentRef through normal sync pull                │
│  2. Requests blob from sender via iroh-blobs (P2P or relay)     │
│  3. iroh-blobs verifies chunks during streaming (Bao)           │
│  4. Derive content_key from GroupSecret                         │
│  5. Decrypt ciphertext → plaintext                               │
│  6. Store in app's content store                                 │
│  7. Send CONTENT_ACK to confirm transfer                         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Key properties:**
- **iroh-blobs sees only ciphertext** — BLAKE3 hash is of encrypted bytes
- **Resumable** — iroh-blobs resumes from last verified chunk if interrupted
- **Verified** — every 16 KiB chunk integrity-checked during streaming
- **No relay load** — large files travel P2P, relay handles only metadata

### 17.3 ContentReference Message

See Section 5.3 for the full struct definition. Key fields:

| Field | Purpose |
|-------|---------|
| `content_hash` | BLAKE3 hash of ciphertext (iroh-blobs address) |
| `encryption_nonce` | XChaCha20-Poly1305 nonce for decryption |
| `content_size` | Original size (for UI progress) |
| `mime_type` | Content type for app handling |
| `thumbnail_hash` | Optional preview (encrypted, much smaller) |

### 17.4 Content Key Derivation

See Section 4.1 for the HKDF derivation. Same key for all devices in the group.

### 17.5 Garbage Collection

| Device | GC Strategy |
|--------|-------------|
| **Sender** | Keeps blob tagged until all devices ACK |
| **Receiver** | Drops tag after successful download + local storage |
| **Orphan protection** | Sender keeps tag for grace period (30 days) if ContentRef deleted before all fetches |

### 17.6 Mobile Considerations

**Thumbnail-first strategy:**
1. Show "Photo attached" immediately (from ContentRef)
2. Show thumbnail if `thumbnail_hash` present (tiny iroh-blobs fetch)
3. Download full resolution on demand or WiFi

**Background transfer:**
- Queue large downloads for WiFi / charging
- iroh-blobs handles resumption natively
- iOS/Android background fetch can trigger thumbnail downloads

**Storage quota:**
- Each app manages its own content storage budget
- Old content can be evicted and re-fetched (ContentRef persists in sync log)

### 17.7 Offline / Asynchronous Transfer

| Scenario | Solution |
|----------|----------|
| Same LAN | mDNS direct discovery → LAN-speed transfer |
| Different networks, both online | iroh relay forwards encrypted blobs |
| Asynchronous (rarely online together) | Pending downloads queue, resume on next app foreground |
| Future enhancement | Relay-hosted blob cache with TTL (deployment optimization) |

### 17.8 Discovery Configuration

```rust
// Enable mDNS for same-LAN direct transfer (recommended)
let discovery = ConcurrentDiscovery::new()
    .add(MdnsDiscovery::new())        // LAN discovery
    .add(DnsDiscovery::new(dns_url))  // Our DNS server
    .add(DhtDiscovery::new());        // Fallback (optional)

let endpoint = Endpoint::builder()
    .discovery(discovery)
    .relay_mode(RelayMode::Custom(our_relay_map))  // Our infrastructure only
    .build()
    .await?;
```

### 17.9 Self-Hosted Infrastructure

For production deployments, run your own iroh infrastructure:

| Component | Purpose | Deployment |
|-----------|---------|------------|
| `iroh-relay` | Encrypted datagram relay when P2P fails | Docker, any VPS |
| `iroh-dns-server` | Endpoint discovery by ID | Docker, your domain |
| mDNS | LAN discovery | Zero infrastructure |
| DHT | Decentralized fallback | Zero infrastructure |

**Configuration:**
```rust
// Disable n0 default relays, use only our infrastructure
let relay_map = RelayMap::from_url("https://relay.yourdomain.com".parse()?);
```

This ensures zero runtime dependency on any third party.

---

## Appendix A: Crate Dependencies

### sync-types
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
rmp-serde = "1"
uuid = { version = "1", features = ["v4", "serde"] }
```

### sync-core
```toml
[dependencies]
sync-types = { path = "../sync-types" }
```

### sync-client
```toml
[dependencies]
sync-types = { path = "../sync-types" }
sync-core = { path = "../sync-core" }
sync-content = { path = "../sync-content" }  # Large content transfer
tokio = { version = "1", features = ["rt", "sync", "time"] }
clatter = "2.2"                  # Hybrid Noise protocol (ML-KEM-768 + X25519)
iroh = "0.96"                    # Endpoint, connections, discovery (all tiers) - requires cargo patch
argon2 = "0.5"
chacha20poly1305 = "0.10"        # Supports XChaCha20
rand = "0.8"
thiserror = "1"
tracing = "0.1"
```

### sync-content
```toml
[dependencies]
sync-types = { path = "../sync-types" }
iroh-blobs = "0.98"             # Content-addressed storage with BLAKE3/Bao
iroh = "0.96"                   # Endpoint for transfers - requires cargo patch
chacha20poly1305 = "0.10"       # XChaCha20-Poly1305 for content encryption
blake3 = "1"                    # Hashing ciphertext for content address
hkdf = "0.12"                   # Content key derivation from GroupSecret
sha2 = "0.10"                   # HKDF-SHA256
tokio = { version = "1", features = ["rt", "sync", "fs"] }
thiserror = "1"
tracing = "0.1"
```

### sync-relay
```toml
[dependencies]
sync-types = { path = "../sync-types" }
tokio = { version = "1", features = ["full"] }
iroh = "0.96"                    # Endpoint for accepting client connections (QUIC) - requires cargo patch
clatter = "2.2"                  # Hybrid Noise protocol (ML-KEM-768 + X25519)
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio"] }
axum = "0.7"                     # Health/metrics HTTP endpoints only
tower = "0.4"
tracing = "0.1"
tracing-subscriber = "0.3"
config = "0.14"
```

### Framework Integration Example: Tauri Plugin
```toml
# tauri-plugin-sync (optional - example integration)
[dependencies]
sync-client = { path = "../sync-client" }
tauri = "2"
tauri-plugin = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

---

## Changelog

**v2.3.0 (2026-02-03):** Removed WebSocket transport from all tiers. All connections now use iroh QUIC. sync-relay (Phase 6) redesigned as iroh Endpoint instead of WebSocket server. Removed tokio-tungstenite dependency. Updated RelayBackend enum to use NodeId addressing. Fixed ManagedCloud enum variant (space in name). Updated data flow diagram, tier configs, pairing format, mobile lifecycle section, CLI config, and dependency lists.

**v2.2.0 (2026-02-02):** Layer structure updated per iroh-deep-dive-report.md. Added Layer 3 (Content Transfer) for large file handling via iroh-blobs. Added Section 17 (Large Content Transfer Protocol).

---

*Document: 02-SPECIFICATION.md | Version: 2.3.0 | Date: 2026-02-03*
