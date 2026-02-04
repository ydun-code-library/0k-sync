# Sync Relay Specification

## E2E Encrypted Local-First Sync for Tauri Apps

**Version:** 0.3.0  
**Status:** Draft (Reviewed + Architecture Defined)  
**Author:** James (LTIS Investments AB)  
**Target:** CashTable, Regime Tracker, future Tauri apps  
**Last Updated:** January 2026  

---

## 1. Overview

### 1.1 Purpose

A lightweight, self-hosted relay server that enables secure synchronization between multiple instances of a Tauri application (desktop, mobile, web) without the relay ever seeing plaintext data.

### 1.2 Design Principles

1. **Zero Knowledge** — Relay cannot decrypt user data
2. **Open Standards** — Built entirely on open source protocols
3. **Local-First** — Apps work offline; sync is opportunistic
4. **Rust Native** — Server and client libraries in Rust
5. **Simple** — Relay is a dumb pipe; intelligence lives client-side
6. **Standalone Ecosystem** — Built as reusable infrastructure, not app feature (see **Section 15**)

### 1.3 Non-Goals

- Real-time collaborative editing (use Any-Sync directly for that)
- File sync (use Syncthing for that)
- User account management (handled by client apps)
- Background push notifications (see **Section 7.4** for mobile lifecycle details)
- Always-on sync on mobile (OS kills background connections)

---

## 2. Architecture

### 2.1 High-Level Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        THE BEAST (Home Server)                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                     Sync Relay Container                   │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │  Relay Service (Rust)                               │  │  │
│  │  │  - Accepts iroh connections (QUIC)                  │  │  │
│  │  │  - Noise protocol handshake                         │  │  │
│  │  │  - Routes encrypted blobs between devices           │  │  │
│  │  │  - Temporary blob storage (SQLite)                  │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│                    Cloudflare Tunnel                             │
└──────────────────────────────┼───────────────────────────────────┘
                               │
                         Public Internet
                               │
         ┌─────────────────────┼─────────────────────┐
         │                     │                     │
    ┌────┴────┐          ┌─────┴─────┐         ┌─────┴─────┐
    │  Phone  │          │  Desktop  │         │  Laptop   │
    │ Android │          │  Linux    │         │  Windows  │
    │ Tauri   │          │  Tauri    │         │  Tauri    │
    └─────────┘          └───────────┘         └───────────┘
```

### 2.2 Component Responsibilities

| Component | Responsibility |
|-----------|----------------|
| **Tauri Client** | Encryption, CRDT merge, conflict resolution, UI |
| **Sync Library** | Noise handshake, message framing, retry logic |
| **Relay Server** | Connection management, blob routing, temp storage |
| **Cloudflare Tunnel** | TLS termination, DDoS protection, public endpoint |

### 2.3 What the Relay Sees

```
┌─────────────────────────────────────────┐
│  Relay's View of Data                   │
├─────────────────────────────────────────┤
│  • Device public keys (Noise)           │
│  • Encrypted blob sizes                 │
│  • Timestamps                           │
│  • Which devices sync together          │
│                                         │
│  NEVER SEES:                            │
│  • Plaintext content                    │
│  • What app is syncing                  │
│  • Semantic meaning of data             │
└─────────────────────────────────────────┘
```

---

## 3. Protocol Stack

### 3.1 Layer Diagram

```
┌─────────────────────────────────────────┐
│  Layer 4: Application Messages          │  ← Your sync logic
│  (Push, Pull, Ack, Presence)            │
├─────────────────────────────────────────┤
│  Layer 3: Envelope                      │  ← Framing + routing
│  (Device ID, Timestamp, Blob)           │
├─────────────────────────────────────────┤
│  Layer 2: Noise Protocol                │  ← E2E encryption
│  (XX handshake pattern)                 │
├─────────────────────────────────────────┤
│  Layer 1: iroh (QUIC)                   │  ← Transport
│  (Binary frames)                        │
├─────────────────────────────────────────┤
│  Layer 0: TLS 1.3                       │  ← Cloudflare handles
│  (HTTPS)                                │
└─────────────────────────────────────────┘
```

### 3.2 Why This Stack

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Transport | iroh (QUIC) | NAT traversal, P2P + relay fallback |
| Encryption | Noise XX | Mutual auth, forward secrecy, battle-tested (WireGuard uses it) |
| Framing | Custom envelope | Minimal overhead, just what we need |
| Application | Custom messages | Tailored to sync use case |

---

## 4. Security Model

### 4.1 Noise Protocol Configuration

**Handshake Pattern:** XX (mutual authentication)

```
XX:
  → e
  ← e, ee, s, es
  → s, se
```

**Why XX:**
- Both parties prove identity
- Neither needs to know the other's key in advance
- Perfect forward secrecy from message 2 onwards

**Cryptographic Primitives:**

| Function | Algorithm |
|----------|-----------|
| DH | Curve25519 |
| Cipher | ChaChaPoly |
| Hash | BLAKE2s |

**Rust Crate:** `snow` (Noise Protocol implementation)

### 4.2 Identity Model

**Device Identity:**
- Each device generates a Curve25519 keypair on first launch
- Public key = Device ID (32 bytes, base64 encoded for display)
- Private key stored securely (OS keychain via Tauri plugin)

**Sync Group:**
- User creates a "sync group" identified by a shared secret
- Shared secret derived from user passphrase via Argon2id
- Devices in same sync group can decrypt each other's blobs

```
┌─────────────────────────────────────────┐
│  Key Hierarchy                          │
├─────────────────────────────────────────┤
│                                         │
│  User Passphrase                        │
│         │                               │
│         ▼ (Argon2id)                    │
│  Sync Group Key                         │
│         │                               │
│         ▼ (HKDF)                        │
│  ┌──────┴──────┐                        │
│  │             │                        │
│  ▼             ▼                        │
│ Encryption   Authentication             │
│ Key          Key                        │
│                                         │
└─────────────────────────────────────────┘
```

### 4.3 Threat Model

| Threat | Mitigation |
|--------|------------|
| Relay operator reads data | E2E encryption; relay only sees ciphertext |
| MITM attack | Noise mutual auth + TLS |
| Replay attack | Nonces + timestamps in envelope |
| Device compromise | Per-device keys; revocation via sync group rotation |
| Relay compromise | No plaintext stored; rotate relay if needed |
| Traffic analysis | All blobs padded to standard sizes (optional) |

### 4.4 Trust Assumptions

1. User's devices are not compromised
2. Noise Protocol cryptography is sound
3. Cloudflare is trusted for TLS termination (or self-host tunnel)
4. Relay server is honest-but-curious (won't actively attack, might try to read)

---

## 5. Message Specification

### 5.1 Envelope Format

All messages wrapped in an envelope (after Noise encryption):

```
┌─────────────────────────────────────────┐
│  Envelope (serialized via MessagePack)  │
├─────────────────────────────────────────┤
│  version: u8           (protocol ver)   │
│  msg_type: u8          (see 5.2)        │
│  sender_id: [u8; 32]   (device pubkey)  │
│  group_id: [u8; 32]    (sync group)     │
│  cursor: u64           (sequence num)   │  ← Assigned by relay, monotonic per group
│  timestamp: u64        (unix millis)    │  ← Wall clock, informational only
│  nonce: [u8; 24]       (unique)         │
│  payload: Vec<u8>      (encrypted blob) │
└─────────────────────────────────────────┘
```

**Why cursor instead of timestamp for ordering?**
- Wall clocks drift between devices (mobile clocks especially)
- Cursor is monotonically increasing, assigned by relay
- Guarantees no gaps: "give me everything after cursor 500"
- Timestamp kept for human-readable logging only

See **Section 6.3** for how the relay assigns cursors.

### 5.2 Message Types

| Type | Value | Direction | Purpose |
|------|-------|-----------|---------|
| `HELLO` | 0x01 | Client → Relay | Initial connection, declare group, report last cursor |
| `WELCOME` | 0x02 | Relay → Client | Connection accepted, relay info, current max cursor |
| `PUSH` | 0x10 | Client → Relay | Upload encrypted blob |
| `PUSH_ACK` | 0x11 | Relay → Client | Blob stored, assigned cursor |
| `PULL` | 0x20 | Client → Relay | Request blobs after cursor (not timestamp) |
| `PULL_RESPONSE` | 0x21 | Relay → Client | Deliver requested blobs with cursors |
| `PRESENCE` | 0x30 | Client → Relay | Heartbeat, online status |
| `NOTIFY` | 0x31 | Relay → Client | New blob available with cursor |
| `DELETE` | 0x40 | Client → Relay | Remove blob (after all devices ack) |
| `ERROR` | 0xFF | Either | Error with code and message |

### 5.3 Message Details

#### HELLO
```
{
  version: 1,
  device_name: "James's Phone",    // Human readable
  group_id: [u8; 32],              // Which sync group
  last_cursor: u64,                // Last cursor we have (0 if first sync)
}
```

#### WELCOME
```
{
  version: 1,
  relay_id: String,                // Relay identifier
  max_cursor: u64,                 // Current highest cursor for this group
  pending_count: u32,              // Blobs waiting for this device
}
```

#### PUSH
```
{
  blob_id: [u8; 16],               // Client-generated UUID
  payload: Vec<u8>,                // E2E encrypted, relay can't read
  ttl: u32,                        // Seconds until auto-delete (0 = forever)
}
```

#### PUSH_ACK
```
{
  blob_id: [u8; 16],               // Echoed from PUSH
  cursor: u64,                     // Assigned cursor (monotonic)
  timestamp: u64,                  // Server timestamp (informational)
}
```

#### PULL
```
{
  after_cursor: u64,               // Return blobs with cursor > this value
  limit: u32,                      // Max blobs to return (default 100)
}
```

**Note:** Using `after_cursor` instead of `since` timestamp guarantees no gaps regardless of clock drift. See **Section 5.1** for rationale.

#### PULL_RESPONSE
```
{
  blobs: Vec<{
    blob_id: [u8; 16],
    cursor: u64,                   // For client to track progress
    sender_id: [u8; 32],
    payload: Vec<u8>,
    timestamp: u64,                // Informational
  }>,
  has_more: bool,                  // True if more blobs available
  max_cursor: u64,                 // Highest cursor in this response
}
```

#### NOTIFY (server push when new blob arrives)
```
{
  blob_id: [u8; 16],
  cursor: u64,                     // So client knows if it's ahead/behind
  sender_id: [u8; 32],
  timestamp: u64,
  size: u32,
}
```

---

## 6. Relay Server Specification

### 6.1 Responsibilities

1. Accept iroh connections (QUIC)
2. Perform Noise XX handshake
3. Route messages between devices in same sync group
4. Store blobs temporarily for offline devices
5. Clean up expired blobs
6. Track device presence

### 6.2 Storage Schema (SQLite)

```sql
-- Performance pragmas (set on connection open)
PRAGMA journal_mode=WAL;              -- Concurrent readers + single writer
PRAGMA synchronous=NORMAL;            -- Balance durability/performance
PRAGMA auto_vacuum=INCREMENTAL;       -- Reclaim space from deleted blobs
PRAGMA busy_timeout=5000;             -- 5 second timeout on locks

-- Cursor sequence per group (monotonically increasing)
CREATE TABLE group_cursors (
    group_id BLOB PRIMARY KEY,        -- Which sync group
    next_cursor INTEGER NOT NULL DEFAULT 1  -- Next cursor to assign
);

-- Registered devices (public info only)
CREATE TABLE devices (
    device_id BLOB PRIMARY KEY,       -- 32-byte public key
    device_name TEXT,
    group_id BLOB NOT NULL,           -- Which sync group
    last_cursor INTEGER DEFAULT 0,    -- Last cursor this device synced to
    last_seen INTEGER,                -- Unix timestamp
    created_at INTEGER
);

-- Pending blobs awaiting delivery
CREATE TABLE blobs (
    blob_id BLOB PRIMARY KEY,         -- 16-byte UUID
    group_id BLOB NOT NULL,
    cursor INTEGER NOT NULL,          -- Assigned cursor (monotonic per group)
    sender_id BLOB NOT NULL,
    payload BLOB NOT NULL,            -- Encrypted, relay can't read
    timestamp INTEGER,                -- Wall clock (informational)
    expires_at INTEGER,               -- Auto-delete time
    created_at INTEGER,
    UNIQUE(group_id, cursor)          -- Cursor is unique within group
);

-- Track which devices have received which blobs
CREATE TABLE deliveries (
    blob_id BLOB,
    device_id BLOB,
    delivered_at INTEGER,
    PRIMARY KEY (blob_id, device_id)
);

-- Indexes for efficient queries
CREATE INDEX idx_blobs_group_cursor ON blobs(group_id, cursor);  -- For PULL queries
CREATE INDEX idx_blobs_expires ON blobs(expires_at);
CREATE INDEX idx_devices_group ON devices(group_id);
```

**Cursor Assignment:** When a PUSH arrives, the relay atomically increments `group_cursors.next_cursor` and assigns that value to the blob. This guarantees ordering without relying on wall clocks. See **Section 6.3** for the exact logic.

### 6.3 Relay Behavior

**On HELLO:**
1. Validate Noise handshake completed
2. Register/update device in database
3. Send WELCOME with:
   - Current `max_cursor` for the group
   - Count of pending blobs for this device
4. Check for pending blobs (cursor > device's `last_cursor`), send NOTIFY for each

**On PUSH:**
1. Begin transaction
2. Get and increment `group_cursors.next_cursor` atomically:
   ```sql
   INSERT INTO group_cursors (group_id, next_cursor) 
   VALUES (?, 1)
   ON CONFLICT(group_id) DO UPDATE SET next_cursor = next_cursor + 1
   RETURNING next_cursor - 1 AS assigned_cursor;
   ```
3. Store blob with assigned cursor
4. Commit transaction
5. Send PUSH_ACK to sender with `cursor` and `timestamp`
6. Send NOTIFY to all other online devices in group (include cursor)
7. Store for offline devices

**On PULL:**
1. Query blobs for group where `cursor > after_cursor`
2. Order by cursor ascending
3. Return up to limit blobs with their cursors
4. Set `has_more = true` if more blobs exist
5. Update device's `last_cursor` to `max_cursor` of returned blobs
6. Mark blobs as delivered for this device

**On DELETE:**
1. Check if all devices in group have received blob
2. If yes, delete from database
3. If no, mark as "pending delete"

**Cleanup Job (runs every hour):**
1. Delete expired blobs (past TTL)
2. Delete blobs marked "pending delete" where all devices acked
3. Remove devices not seen in 90 days
4. Run `PRAGMA incremental_vacuum` to reclaim space

### 6.4 Rate Limits

| Resource | Limit | Window |
|----------|-------|--------|
| Connections per IP | 10 | Concurrent |
| Messages per device | 100 | Per minute |
| Blob size | 1 MB | Per blob |
| Total storage per group | 100 MB | Rolling |
| Groups per relay | 1000 | Total |

### 6.5 Configuration

```toml
# relay.toml

[server]
bind = "127.0.0.1:8080"          # Cloudflare tunnel connects here
max_connections = 1000

[storage]
database = "/data/relay.db"
max_blob_size = 1048576          # 1 MB per blob
max_group_storage = 104857600    # 100 MB per sync group
max_db_size = 1073741824         # 1 GB total - reject PUSH if exceeded
default_ttl = 604800             # 7 days

[cleanup]
interval = 3600                  # Run every hour
device_expiry = 7776000          # 90 days
vacuum_on_cleanup = true         # Run incremental vacuum

[limits]
messages_per_minute = 100
connections_per_ip = 10
```

**Storage Safety:** The relay checks `max_db_size` before accepting any PUSH. If the database file exceeds this limit, PUSH requests return an ERROR with code `STORAGE_FULL`. This prevents disk-fill attacks.

---

## 7. Client Library Specification

### 7.1 Public API (Rust)

```rust
// What the Tauri app imports and uses

pub struct SyncClient { /* ... */ }

impl SyncClient {
    /// Create new client with device keypair
    pub fn new(config: SyncConfig) -> Result<Self>;
    
    /// Connect to relay server
    pub async fn connect(&mut self) -> Result<ConnectionInfo>;
    
    /// Disconnect gracefully
    pub async fn disconnect(&mut self) -> Result<()>;
    
    /// Push encrypted blob to sync group
    /// Returns assigned cursor for tracking
    pub async fn push(&self, data: &[u8]) -> Result<PushResult>;
    
    /// Pull all blobs after cursor (not timestamp!)
    /// Use cursor=0 for initial sync
    pub async fn pull(&self, after_cursor: u64) -> Result<PullResult>;
    
    /// Subscribe to real-time notifications
    pub fn subscribe(&self) -> Receiver<SyncEvent>;
    
    /// Get current connection status
    pub fn status(&self) -> ConnectionStatus;
    
    /// Get last synced cursor (persisted locally)
    pub fn last_cursor(&self) -> u64;
}

pub struct ConnectionInfo {
    pub relay_id: String,
    pub max_cursor: u64,       // Current highest cursor on relay
    pub pending_count: u32,    // Blobs waiting for us
}

pub struct PushResult {
    pub blob_id: BlobId,
    pub cursor: u64,           // Assigned cursor
}

pub struct PullResult {
    pub blobs: Vec<Blob>,
    pub max_cursor: u64,       // Highest cursor in response
    pub has_more: bool,        // More blobs available
}

pub struct Blob {
    pub id: BlobId,
    pub cursor: u64,           // For tracking sync progress
    pub sender: DeviceId,
    pub data: Vec<u8>,         // Decrypted payload
    pub timestamp: u64,        // Informational
}

pub enum SyncEvent {
    Connected { info: ConnectionInfo },
    Disconnected { reason: String },
    BlobAvailable { id: BlobId, cursor: u64, sender: DeviceId },
    Error { code: u32, message: String },
}

pub struct SyncConfig {
    pub relay_url: String,
    pub device_keypair: Keypair,
    pub group_key: GroupKey,
    pub auto_reconnect: bool,
}
```

**Cursor Tracking:** The client persists `last_cursor` locally. On reconnect, it calls `pull(last_cursor)` to get any missed blobs. This is reliable regardless of clock drift between devices. See **Section 5.1** for why cursors matter.

### 7.2 Encryption Layer (Client-Side)

**Before sending to relay:**
```
Plaintext (app data)
    │
    ▼ (Serialize with MessagePack)
Binary blob
    │
    ▼ (Encrypt with Sync Group Key + random nonce)
Encrypted blob
    │
    ▼ (Wrap in Envelope)
Envelope
    │
    ▼ (Encrypt with Noise session key)
Wire format → Send to relay
```

**On receiving from relay:**
```
Wire format
    │
    ▼ (Decrypt with Noise session key)
Envelope
    │
    ▼ (Extract payload)
Encrypted blob
    │
    ▼ (Decrypt with Sync Group Key + nonce from envelope)
Binary blob
    │
    ▼ (Deserialize with MessagePack)
Plaintext (app data)
```

### 7.3 Integration with Tauri

The sync library exposes Tauri commands:

```rust
#[tauri::command]
async fn sync_connect(state: State<SyncState>) -> Result<ConnectionInfo, String>;

#[tauri::command]
async fn sync_push(state: State<SyncState>, data: Vec<u8>) -> Result<PushResult, String>;

#[tauri::command]
async fn sync_pull(state: State<SyncState>, after_cursor: u64) -> Result<PullResult, String>;

#[tauri::command]
fn sync_status(state: State<SyncState>) -> ConnectionStatus;

#[tauri::command]
fn sync_last_cursor(state: State<SyncState>) -> u64;
```

Frontend (Svelte) calls these via `invoke()`:

```typescript
// Svelte component
import { invoke } from '@tauri-apps/api/core';

// Initial sync on app load
const info = await invoke('sync_connect');
const result = await invoke('sync_pull', { afterCursor: lastKnownCursor });

// Push new data
const pushResult = await invoke('sync_push', { data: encryptedBytes });
console.log(`Assigned cursor: ${pushResult.cursor}`);
```

---

### 7.4 Mobile Lifecycle Considerations

**Critical:** On iOS and modern Android, background network connections are killed within ~30 seconds of the app being backgrounded. You cannot rely on persistent connections for sync.

#### Sync Trigger Points

Sync should be triggered on these Tauri lifecycle events:

```rust
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Initial sync on app start
            let sync_client = app.state::<SyncState>();
            tauri::async_runtime::spawn(async move {
                sync_client.connect_and_pull().await;
            });
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::Focused(true) => {
                // App came to foreground - sync
                let sync_client = window.state::<SyncState>();
                tauri::async_runtime::spawn(async move {
                    sync_client.pull_since_last_cursor().await;
                });
            }
            WindowEvent::CloseRequested { .. } => {
                // App closing - flush pending pushes
                let sync_client = window.state::<SyncState>();
                tauri::async_runtime::block_on(async {
                    sync_client.flush_and_disconnect().await;
                });
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

#### Sync Strategy Summary

| Event | Action |
|-------|--------|
| App launch | Connect + full pull since `last_cursor` |
| App resume (foreground) | Pull since `last_cursor` |
| App pause (background) | Flush pending pushes, optionally disconnect |
| User action (save, etc.) | Push immediately |
| Manual "Sync Now" button | Full pull + push cycle |

#### What We DON'T Support (Yet)

- **Background sync**: iOS/Android kill background connections
- **Push notifications for new data**: Would require APNS/FCM integration
- **Always-on sync**: Not possible on mobile without OS support

See **Section 12.1** for future enhancement notes on push notification integration.

---

## 8. Deployment

### 8.1 Container Setup (The Beast)

```dockerfile
# Dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/sync-relay /usr/local/bin/
COPY relay.toml /etc/sync-relay/
EXPOSE 8080
CMD ["sync-relay", "--config", "/etc/sync-relay/relay.toml"]
```

```yaml
# docker-compose.yml
version: '3.8'
services:
  sync-relay:
    build: .
    restart: unless-stopped
    volumes:
      - relay-data:/data
    ports:
      - "127.0.0.1:8080:8080"
    environment:
      - RUST_LOG=info

volumes:
  relay-data:
```

### 8.2 Cloudflare Tunnel Integration

```yaml
# ~/.cloudflared/config.yml (add to existing)
ingress:
  - hostname: sync.yourdomain.com
    service: http://localhost:8080
  # ... other services
```

### 8.3 Monitoring

**Health endpoint:** `GET /health`
```json
{
  "status": "ok",
  "version": "0.1.0",
  "connections": 3,
  "groups": 1,
  "blobs_stored": 42,
  "uptime_seconds": 86400
}
```

**Metrics endpoint:** `GET /metrics` (Prometheus format)
```
sync_relay_connections_total 156
sync_relay_connections_active 3
sync_relay_blobs_stored 42
sync_relay_blobs_delivered 1337
sync_relay_bytes_transferred 15728640
```

---

## 9. Rust Dependencies

> **Note:** These dependencies are organized per-crate in a Cargo workspace. See **Section 15** for the complete project structure and crate organization.

### 9.1 Relay Server (`sync-relay`)

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
iroh = "1.0"                         # QUIC transport
snow = "0.9"                          # Noise Protocol
sqlx = { version = "0.8", default-features = false, features = ["sqlite", "runtime-tokio", "derive"] }
serde = { version = "1", features = ["derive"] }
rmp-serde = "1"                       # MessagePack
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
tracing = "0.1"
tracing-subscriber = "0.3"
config = "0.14"                       # Configuration
axum = "0.7"                          # Health/metrics endpoints
```

### 9.2 Client Library (`sync-client`)

```toml
[dependencies]
tokio = { version = "1", features = ["rt", "sync", "time"] }
iroh = "1.0"
snow = "0.9"
serde = { version = "1", features = ["derive"] }
rmp-serde = "1"
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
argon2 = "0.5"                        # Passphrase → Group Key
thiserror = "1"
```

### 9.3 Tauri Plugin

```toml
[dependencies]
tauri = "2"
tauri-plugin = "2"
sync-client = { path = "../sync-client" }  # The client library
keyring = "2"                         # OS keychain for private keys
```

---

## 10. Open Source Licenses

| Component | License | Notes |
|-----------|---------|-------|
| Noise Protocol | Public Domain | Specification is unencumbered |
| snow (Rust) | Apache-2.0 | Noise implementation |
| tokio | MIT | Async runtime |
| SQLite | Public Domain | Database |
| MessagePack | MIT | Serialization |
| Argon2 | Apache-2.0 / MIT | Key derivation |

**Your Code:** Recommend Apache-2.0 or MIT for maximum compatibility.

---

## 11. Security Considerations

### 11.1 What To Audit

1. Noise handshake implementation (rely on `snow` which is audited)
2. Key derivation from passphrase (Argon2id parameters)
3. Nonce generation (must be unique per message)
4. Blob encryption (ChaCha20-Poly1305)

### 11.2 What NOT To Build Yourself

- Cryptographic primitives (use `snow`, `ring`, `chacha20poly1305`)
- Random number generation (use `rand` with OsRng)
- Key derivation (use `argon2` crate)

### 11.3 Recommendations

1. Run relay on dedicated container, isolated from other services
2. Enable Cloudflare WAF rules
3. Rotate sync group keys periodically (app feature)
4. Implement device revocation (remove from group, rotate key)
5. Log connections but NEVER log blob contents

---

## 12. Future Considerations

### 12.1 Potential Enhancements

| Feature | Priority | Notes |
|---------|----------|-------|
| Multi-relay federation | Low | Redundancy, geographic distribution |
| Blob compression | Medium | LZ4 before encryption |
| Delta sync | Medium | Only send changes, not full state |
| WebRTC fallback | Low | P2P when both devices online |
| Any-Sync integration | Medium | Full CRDT support |
| **PADME padding** | Low | See 12.3 below |
| **Key rotation** | Medium | See 12.4 below |
| **Push notifications** | Medium | See 12.5 below |

### 12.2 CrabNebula Product Opportunity

This could become **"0k-Sync"**:
- Managed relay infrastructure
- Tauri plugin for easy integration
- Dashboard for monitoring
- Complements existing Cloud (distribution) product

### 12.3 Traffic Analysis Mitigation (PADME Padding)

**Current state:** Blob sizes leak information. A 16-byte blob (boolean change) vs 500-byte blob (paragraph) reveals activity type to the relay/ISP.

**Future enhancement:** Implement PADME (Padding to Avoid Meaningful Distinction in Envelopes):

```rust
fn pad_to_power_of_two(data: &[u8]) -> Vec<u8> {
    let target_size = data.len().next_power_of_two().max(256);
    let mut padded = data.to_vec();
    padded.resize(target_size, 0);
    padded
}
// 100 bytes → 256 bytes
// 500 bytes → 512 bytes
// 1000 bytes → 1024 bytes
```

**Trade-off:** Increases bandwidth usage by ~40% average, but makes all messages in a size class indistinguishable.

### 12.4 Forward Secrecy & Key Rotation

**Current state:** All blobs encrypted with static Group Key. If key is compromised, all past traffic (if recorded) can be decrypted.

**Future enhancement:** Periodic key rotation:

1. **Time-based rotation:** Every 30 days, generate new Group Key
2. **Event-based rotation:** Rotate when device is removed from group
3. **Rotation message:** Push "KEY_ROTATE" event to all devices, encrypted with OLD key, containing NEW key
4. **Grace period:** Accept blobs encrypted with previous key for 24 hours

**Advanced option:** Double Ratchet (like Signal) for per-message forward secrecy. Significantly more complex but provides break-in recovery.

### 12.5 Background Push Notifications (Mobile)

**Current state:** Sync only happens when app is in foreground (see **Section 7.4**). 

**Future enhancement:** Integrate native push to wake app:

```
┌─────────────────────────────────────────────────────────────┐
│  Push Notification Flow                                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. Device B pushes blob to relay                           │
│  2. Relay sends silent push via APNS/FCM to Device A        │
│     (payload: just "new data available", no content)        │
│  3. Device A wakes briefly, pulls new blob                  │
│  4. Device A shows local notification if relevant           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Challenges:**
- Requires Apple/Google developer accounts
- Push tokens must be registered with relay (adds state)
- Complicates self-hosted deployment
- Silent pushes have delivery limitations

**Recommendation:** Implement as optional plugin, not core functionality.

---

## 13. Frictionless Pairing (No Accounts)

### 13.1 Design Goal

**Zero accounts. Zero passwords. Just scan and sync.**

The user never creates an account on the relay. The relay doesn't know who they are. Devices pair directly using a one-time invite.

### 13.2 Pairing Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  DEVICE A (First Device - Creates Sync Group)                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. User taps "Enable Sync" in app settings                     │
│                                                                 │
│  2. App generates:                                              │
│     • Random Sync Group ID (32 bytes)                           │
│     • Random Group Secret (32 bytes)                            │
│     • Device A keypair (if not exists)                          │
│                                                                 │
│  3. App creates Invite Payload:                                 │
│     {                                                           │
│       relay: "sync.yourdomain.com",                             │
│       group_id: "base64...",                                    │
│       group_secret: "base64...",                                │
│       created_by: "Device A pubkey",                            │
│       expires: 1704067200                                       │
│     }                                                           │
│                                                                 │
│  4. App encrypts payload with temporary key                     │
│                                                                 │
│  5. App displays:                                               │
│     ┌─────────────────────────┐                                 │
│     │  ▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄  │                                 │
│     │  █ QR CODE          █  │  ← Scan with other device        │
│     │  █                  █  │                                  │
│     │  █▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄█  │                                  │
│     │                        │                                  │
│     │  Or enter code:        │                                  │
│     │  XXXX-XXXX-XXXX-XXXX   │  ← 16 chars, expires in 10 min   │
│     └─────────────────────────┘                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  DEVICE B (Second Device - Joins Sync Group)                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. User taps "Join Sync" in app settings                       │
│                                                                 │
│  2. App prompts: Scan QR or Enter Code                          │
│                                                                 │
│  3. App decodes invite payload                                  │
│                                                                 │
│  4. App stores:                                                 │
│     • Relay URL                                                 │
│     • Group ID                                                  │
│     • Group Secret (in OS keychain)                             │
│                                                                 │
│  5. App connects to relay, joins group                          │
│                                                                 │
│  6. Both devices now sync automatically                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 13.3 Invite Format Options

**Option A: QR Code (Recommended)**
```
Full payload encoded as QR, encrypted with embedded key
URL format: cashtable://sync?invite=BASE64_ENCRYPTED_PAYLOAD

Pros: One scan, all info transferred
Cons: Requires camera
```

**Option B: Short Code**
```
16-character alphanumeric: XXXX-XXXX-XXXX-XXXX
First 8 chars = lookup key (relay stores encrypted payload)
Last 8 chars = decryption key (never sent to relay)

Pros: Works without camera, easy to read aloud
Cons: Requires relay to temporarily store invite
```

**Option C: Link**
```
https://sync.yourdomain.com/join#SECRET_IN_FRAGMENT
Fragment (#...) never sent to server

Pros: Works in any browser, shareable
Cons: Less secure if link intercepted
```

### 13.4 Invite Security

| Property | How It's Achieved |
|----------|-------------------|
| **Time-limited** | Expires after 10 minutes |
| **Single-use** | Marked as used after first claim |
| **Encrypted** | Relay never sees group secret |
| **Revocable** | Creator can cancel before use |

### 13.5 Short Code Protocol (if using Option B)

```
Device A (Creator):
1. Generate group_id, group_secret
2. Generate random lookup_key (8 chars) and decrypt_key (8 chars)
3. Encrypt invite payload with decrypt_key
4. POST to relay: { lookup_key, encrypted_payload, expires }
5. Display to user: lookup_key + decrypt_key = "XXXX-XXXX-XXXX-XXXX"

Device B (Joiner):
1. User enters code "XXXX-XXXX-XXXX-XXXX"
2. Split into lookup_key and decrypt_key
3. GET from relay: /invite/{lookup_key}
4. Relay returns encrypted_payload (and deletes it immediately)
5. Decrypt with decrypt_key
6. Now has group_id, group_secret, relay URL
7. Connect and sync!

Relay sees: lookup_key, encrypted blob it can't read
Relay never sees: decrypt_key, group_secret, plaintext
```

#### Rate Limiting (Critical Security)

The invite endpoints MUST be rate-limited to prevent brute-force attacks on the 8-character lookup key:

| Endpoint | Limit | Window | Action on Exceed |
|----------|-------|--------|------------------|
| `POST /invite` | 5 | Per IP per minute | 429 Too Many Requests |
| `GET /invite/{key}` | 10 | Per IP per minute | 429 Too Many Requests |
| `GET /invite/{key}` (not found) | 3 | Per IP per minute | 429 + exponential backoff |

**Additional protections:**
- Invite auto-expires after 10 minutes
- Single-use: deleted immediately on first successful GET
- Failed lookups logged for abuse detection
- Consider CAPTCHA for invite creation in high-abuse scenarios

#### Why This Design (Not PAKE)

A full PAKE (Password Authenticated Key Exchange) like OPAQUE would be more cryptographically rigorous, but:
- Adds significant complexity
- Requires multiple round-trips
- The "split key" approach (lookup vs decrypt) provides similar security for this threat model
- Attacker who brute-forces lookup_key still can't decrypt without decrypt_key

### 13.6 Adding More Devices

Same flow. Any device in the sync group can generate an invite:

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│ Device A │     │ Device B │     │ Device C │
│ (origin) │     │ (joined) │     │ (new)    │
└────┬─────┘     └────┬─────┘     └────┬─────┘
     │                │                │
     │  Created group │                │
     │◄───────────────│                │
     │                │                │
     │                │  Can also      │
     │                │  create invite │
     │                │───────────────►│
     │                │                │
     │    All three now sync           │
     │◄───────────────┼───────────────►│
```

### 13.7 Removing Devices / Revoking Access

**Soft Revoke:**
- Remove device from local "known devices" list
- That device can still connect but won't receive pushes

**Hard Revoke (Key Rotation):**
1. Generate new group_secret
2. Push "key rotation" message to all OTHER devices
3. All devices except revoked one update their key
4. Revoked device can no longer decrypt new blobs
5. Optionally: re-encrypt and re-push recent data

### 13.8 User Experience Summary

| Action | Steps | Account Needed? |
|--------|-------|-----------------|
| **First device setup** | 1 tap | No |
| **Pair second device** | Scan QR or enter code | No |
| **Add more devices** | Same as above | No |
| **Sync data** | Automatic, background | No |
| **Remove device** | 1 tap in settings | No |

**Total accounts needed: ZERO**

The relay is just infrastructure. It doesn't know or care who's using it.

---

## 14. Relay Deployment Options

Users can choose where their sync relay runs. All options maintain E2E encryption — the relay never sees plaintext regardless of who operates it.

### 14.1 Option Comparison

| Option | Cost | Privacy | Reliability | Setup |
|--------|------|---------|-------------|-------|
| **Self-hosted** | ~$5/mo VPS or free (home server) | Maximum | You control it | Medium |
| **iroh Public Relays** | Free | Good (E2E) | Excellent | Easy |
| **Nostr Network** | Free | Good (E2E) | Variable | Easy |
| **0k-Sync** | TBD (potential product) | Enterprise-grade | Managed SLA | Easy |

### 14.2 Self-Hosted (Our Reference Implementation)

Run the relay on your own infrastructure.

**Pros:**
- Complete control
- No third-party trust
- Can add custom rules/limits
- Free if using home server

**Cons:**
- You maintain it
- Need domain + TLS (Cloudflare Tunnels makes this easy)

**Best for:** Privacy-focused users, developers, home lab enthusiasts

```
┌─────────────────────────────────────────────┐
│  Your Infrastructure                        │
│  ┌─────────────────┐                        │
│  │ sync-relay      │◄── Cloudflare Tunnel   │
│  │ (Rust binary)   │                        │
│  │ + SQLite        │                        │
│  └─────────────────┘                        │
│  The Beast / VPS / Raspberry Pi             │
└─────────────────────────────────────────────┘
```

### 14.3 iroh Public Relays (Recommended for MVP)

[iroh](https://github.com/n0-computer/iroh) by n0-computer provides free public relay infrastructure.

**What iroh provides:**
- `iroh-relay`: Public relay servers for NAT traversal
- `iroh-blobs`: Content-addressed blob transfer (BLAKE3)
- `iroh-gossip`: Pub/sub overlay networks
- `iroh-docs`: Multi-dimensional key-value sync with CRDT

**Architecture:**
```
┌─────────────┐         ┌─────────────┐
│  Device A   │         │  Device B   │
│  (Tauri)    │         │  (Tauri)    │
└──────┬──────┘         └──────┬──────┘
       │                       │
       │   QUIC + Noise XX     │
       ▼                       ▼
┌─────────────────────────────────────┐
│  iroh Public Relay Network          │
│  (operated by n0-computer)          │
│                                     │
│  • NAT hole-punching                │
│  • Fallback relay when direct fails │
│  • Free & open ecosystem            │
└─────────────────────────────────────┘
```

**Key features:**
- **Direct when possible**: iroh tries hole-punching first, relay is fallback
- **QUIC transport**: Modern, multiplexed, low-latency
- **Rust native**: Perfect fit for Tauri
- **Production-tested**: 200k+ concurrent connections, millions of devices

**Integration:**
```toml
# Cargo.toml
[dependencies]
iroh = "0.35"
iroh-blobs = "0.35"
iroh-gossip = "0.35"
```

```rust
use iroh::Endpoint;

// Connect to iroh's public relay network
let endpoint = Endpoint::builder()
    .discovery_n0()  // Use n0's discovery + relay
    .bind()
    .await?;
```

**Pros:**
- Zero infrastructure to maintain
- Battle-tested at scale
- Direct P2P when possible (faster)
- MIT licensed

**Cons:**
- Dependent on n0's infrastructure (can self-host relay)
- Learning curve for iroh APIs

**Best for:** Most users, especially during MVP phase

### 14.4 Nostr Relay Network

Leverage the existing [Nostr](https://nostr.com/) relay ecosystem.

**What is Nostr?**
- Decentralized protocol for publishing/subscribing to messages
- ~1000 public relays globally
- Uses Schnorr signatures (secp256k1)
- QUIC-based (iroh)

**How we'd use it:**
- Create a custom NIP (Nostr Implementation Possibility) for sync
- Publish encrypted blobs as Nostr events
- Subscribe to events by pubkey

```
┌─────────────┐         ┌─────────────┐
│  Device A   │         │  Device B   │
│  npub_xxx   │         │  npub_xxx   │
└──────┬──────┘         └──────┬──────┘
       │                       │
       │  Nostr Events (E2E)   │
       ▼                       ▼
┌─────────────────────────────────────┐
│  Nostr Relay Network                │
│                                     │
│  relay.damus.io                     │
│  nos.lol                            │
│  relay.nostr.band                   │
│  ... ~1000 relays worldwide         │
└─────────────────────────────────────┘
```

**Pros:**
- Massive existing infrastructure
- True decentralization (no single operator)
- Censorship-resistant
- Free public relays available
- Bitcoin/Lightning integration possible

**Cons:**
- Not designed for sync (social media focus)
- Event size limits (~64KB typical)
- May need paid relay for reliability
- Different crypto primitives (secp256k1 vs Curve25519)

**Best for:** Users already in Nostr/Bitcoin ecosystem, maximum decentralization

### 14.5 libp2p Network

Build on [libp2p](https://libp2p.io/), the networking stack behind IPFS.

**Features:**
- DHT for peer discovery (Kademlia)
- Multiple transports (TCP, QUIC, WebRTC)
- Noise Protocol for encryption
- Gossipsub for pub/sub

**Pros:**
- Most mature P2P stack
- IPFS ecosystem compatibility
- Multiple language implementations

**Cons:**
- Complex API
- Heavier dependency
- "Boil the ocean" approach (vs iroh's focused design)

**Best for:** Integration with IPFS/Filecoin ecosystem

### 14.6 0k-Sync (Future / Potential Product)

**Currently:** CrabNebula Cloud offers app distribution (CDN, updates)  
**Potential:** Add managed sync relay as a service

```
┌─────────────────────────────────────────────┐
│  CrabNebula Cloud                           │
│  ┌─────────────┐  ┌─────────────┐           │
│  │ Distribution│  │ Sync Relay  │ ◄── NEW   │
│  │ (CDN)       │  │ (Managed)   │           │
│  └─────────────┘  └─────────────┘           │
│                                             │
│  • Global edge locations                    │
│  • Dashboard + metrics                      │
│  • Enterprise SLA                           │
│  • First-party Tauri integration            │
└─────────────────────────────────────────────┘
```

**Why this makes sense for CrabNebula:**
- Natural extension of their Tauri tooling
- Fills obvious gap in ecosystem
- Recurring revenue opportunity
- Our open-source relay becomes the backend

**Pricing model (hypothetical):**
- Free tier: 100MB sync, 3 devices
- Pro: $5/mo, 10GB, unlimited devices
- Enterprise: Custom

**Best for:** Teams/companies wanting managed solution

### 14.7 Hybrid Approach (Recommended)

Use multiple backends with automatic fallback:

```rust
enum RelayBackend {
    SelfHosted(Url),      // Primary: your relay
    Iroh,                 // Fallback 1: iroh network
    Nostr(Vec<Url>),      // Fallback 2: nostr relays
}

struct SyncConfig {
    backends: Vec<RelayBackend>,
    prefer_direct: bool,  // Try P2P before relay
}
```

**User flow:**
1. First launch → Connect to iroh (zero config)
2. Power user → Add self-hosted relay as primary
3. Fallback → If primary fails, use iroh/nostr

This gives users the best of all worlds: works immediately, but can be customized.

### 14.8 Decision Matrix

| Use Case | Recommended Option |
|----------|-------------------|
| Just want it to work | iroh public relays |
| Privacy maximalist | Self-hosted |
| Already use Nostr | Nostr relays |
| Enterprise/team | 0k-Sync (when available) |
| Offline-first + occasional sync | iroh (direct P2P preferred) |
| Building for IPFS ecosystem | libp2p |

---

## 15. Project Structure (Standalone Ecosystem)

### 15.1 Strategic Decision: Infrastructure, Not Feature

**Decision:** Build the sync relay as a standalone, project-agnostic Cargo workspace — NOT embedded inside CashTable or any specific app.

**Rationale:**

| Argument | Embedded Approach | Standalone Approach |
|----------|-------------------|---------------------|
| **Code Coupling** | App-specific types leak into sync layer (e.g., `Transaction` struct) | Forces "Dumb Pipe" — sync only sees `Blob`, guaranteeing Zero Knowledge |
| **Testing** | Must launch full Tauri GUI to debug sync | CLI tool scripts scenarios in seconds |
| **Reusability** | Copy-paste nightmare for second app | `cargo add sync-client` in any project |
| **Open Source** | Awkward to extract later | Ready for public release from day one |

### 15.2 Repository Structure (Cargo Workspace)

```
tauri-secure-sync/
├── Cargo.toml                 # Workspace definition
│
├── sync-types/                # Shared types (used by ALL crates)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs             # Envelope, Message, DeviceId, GroupId, Cursor
│
├── sync-relay/                # The Server (runs on "The Beast")
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs            # Entry point
│   │   ├── server.rs          # iroh Endpoint + Noise handshake
│   │   ├── storage.rs         # SQLite operations
│   │   └── config.rs          # relay.toml parsing
│   └── Dockerfile
│
├── sync-client/               # The Library (apps import this)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs             # Public API: SyncClient, SyncConfig
│       ├── connection.rs      # iroh connection + reconnect logic
│       ├── crypto.rs          # Noise session, Group Key encryption
│       └── storage.rs         # Local cursor persistence
│
├── sync-cli/                  # Testing/Verification Tool
│   ├── Cargo.toml
│   └── src/
│       └── main.rs            # "sync-cli push 'hello'" / "sync-cli pair"
│
├── tauri-plugin-sync/         # Optional: Tauri plugin wrapper
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs             # Exposes sync-client as Tauri commands
│
└── README.md
```

### 15.3 Crate Responsibilities

| Crate | Purpose | Dependencies |
|-------|---------|--------------|
| `sync-types` | Wire format, message structs, crypto primitives | `serde`, `snow`, `uuid` |
| `sync-relay` | Server binary | `sync-types`, `tokio`, `sqlx`, `axum` |
| `sync-client` | Library for apps | `sync-types`, `tokio`, `argon2` |
| `sync-cli` | Test harness | `sync-client`, `clap`, `dialoguer` |
| `tauri-plugin-sync` | Tauri integration | `sync-client`, `tauri` |

### 15.4 The "Headless" Testing Advantage

With `sync-cli`, you can script complex scenarios without touching a GUI:

```bash
# Terminal 1: Start relay
cd sync-relay && cargo run

# Terminal 2: Device A creates group, pushes data
sync-cli init --name "Device A"
sync-cli pair --create    # Shows QR/code
sync-cli push "Hello from A"
sync-cli push "Second message"

# Terminal 3: Device B joins and syncs
sync-cli init --name "Device B"  
sync-cli pair --join XXXX-XXXX-XXXX-XXXX
sync-cli pull                    # Should receive both messages

# Stress test
for i in {1..100}; do sync-cli push "Blob $i"; done
sync-cli pull --after-cursor 50  # Get blobs 51-100
```

This is **impossible** to do quickly with a full Tauri app.

### 15.5 Integration Into Apps

Once `tauri-secure-sync` is stable, integrating into CashTable or Regime Tracker:

**During Development (side-by-side repos):**
```toml
# cashtable/src-tauri/Cargo.toml
[dependencies]
sync-client = { path = "../../tauri-secure-sync/sync-client" }
```

**For Production (git dependency):**
```toml
[dependencies]
sync-client = { git = "https://github.com/yourname/tauri-secure-sync", tag = "v0.1.0" }
```

**Or via Tauri Plugin:**
```toml
[dependencies]
tauri-plugin-sync = { git = "https://github.com/yourname/tauri-secure-sync", tag = "v0.1.0" }
```

### 15.6 Implementation Roadmap

| Phase | Crate | Deliverable | Test |
|-------|-------|-------------|------|
| **1** | `sync-types` | Envelope, Messages, Cursor types | Unit tests for serialization round-trip |
| **2** | `sync-relay` | Server + SQLite storage | `curl` health endpoint, iroh connection test |
| **3** | `sync-cli` | Basic push/pull commands | Script: push 10 blobs, pull all, verify |
| **4** | `sync-client` | Refined library API | CLI becomes thin wrapper around library |
| **5** | `tauri-plugin-sync` | Tauri commands | Import into CashTable, test in app |
| **6** | Integration | CashTable sync working | Full E2E: phone ↔ desktop sync |

### 15.7 Why This Order Matters

1. **Types first**: Everything depends on the wire format being stable
2. **Server second**: Need something to connect to
3. **CLI third**: Fastest way to prove the protocol works
4. **Library fourth**: CLI reveals API friction points
5. **Plugin fifth**: Just a thin wrapper once library is solid
6. **Integration last**: Don't touch app code until sync is bulletproof

---

## 16. References

- [Noise Protocol Specification](https://noiseprotocol.org/noise.html)
- [snow Rust Crate](https://github.com/mcginty/snow)
- [Syncthing BEP](https://docs.syncthing.net/specs/bep-v1.html) (inspiration)
- [Any-Sync Protocol](https://github.com/anyproto/any-sync)
- [WireGuard Protocol](https://www.wireguard.com/protocol/)
- [Tauri Plugin Development](https://v2.tauri.app/develop/plugins/)

---

## 17. Glossary

| Term | Definition |
|------|------------|
| **Blob** | Opaque encrypted data unit, sync's atomic element |
| **Cursor** | Monotonically increasing sequence number assigned by relay; used instead of timestamps for reliable ordering (see **Section 5.1**) |
| **Device ID** | Curve25519 public key identifying a device |
| **Sync Group** | Collection of devices sharing a group key |
| **Group Key** | Symmetric key derived from user passphrase |
| **Noise XX** | Mutual authentication handshake pattern |
| **Envelope** | Message wrapper containing routing info + encrypted payload |
| **Relay** | Server that routes blobs between devices |
| **TTL** | Time-to-live; how long relay stores a blob |
| **PADME** | Padding to Avoid Meaningful Distinction in Envelopes; traffic analysis mitigation (see **Section 12.3**) |
| **Forward Secrecy** | Property where compromise of current keys doesn't reveal past communications |
| **WAL** | Write-Ahead Logging; SQLite mode for concurrent access |
| **Invite** | One-time pairing code/QR for joining a sync group (see **Section 13**) |
| **Cargo Workspace** | Rust project structure with multiple crates sharing dependencies (see **Section 15**) |
| **sync-types** | Shared crate containing wire format and message definitions |
| **sync-client** | Library crate that apps import for sync functionality |
| **sync-relay** | Server binary crate that runs on infrastructure |
| **sync-cli** | Testing/verification tool for headless development |

---

*Specification complete. Ready for Claude Code implementation.*
