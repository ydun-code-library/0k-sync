# CrabNebula Sync - Technical Specification

**Version:** 2.1.0
**Date:** 2026-01-16
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
9. [Tauri Plugin](#9-tauri-plugin)
10. [Mobile Lifecycle Considerations](#10-mobile-lifecycle-considerations)
11. [Tier-Specific Configuration](#11-tier-specific-configuration)
12. [Error Handling](#12-error-handling)
13. [Configuration Reference](#13-configuration-reference)
14. [Best Practices](#14-best-practices)
15. [Device Revocation](#15-device-revocation)
16. [Push Notification Integration](#16-push-notification-integration)

---

## 1. Overview

### 1.1 Purpose

CrabNebula Sync provides secure, zero-knowledge synchronization between multiple instances of a Tauri application across devices (desktop, mobile, web).

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
│                            Tauri Application                             │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                         Application Code                            │ │
│  │  • Business logic, UI, local database                              │ │
│  └───────────────────────────────┬────────────────────────────────────┘ │
│                                  │ push(blob) / pull()                  │
│  ┌───────────────────────────────▼────────────────────────────────────┐ │
│  │                      tauri-plugin-sync                              │ │
│  │  • Tauri commands (sync_push, sync_pull, sync_status)              │ │
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
| **tauri-plugin-sync** | Tauri command bridge, state management, events |
| **sync-client** | E2E encryption, connection, cursor tracking, pairing |
| **sync-types** | Wire format, message definitions, shared types |
| **sync-core** | Pure logic (state machine, buffer), no I/O |
| **sync-relay** | Connection management, message routing, temp buffer |

### 2.3 Data Flow

```
                    PUSH FLOW
                    ─────────
App Data ──► Serialize ──► Encrypt (Group Key) ──► Envelope ──►
         ──► Transport Encrypt (Noise) ──► WebSocket ──► Relay ──►
         ──► Forward to online peers ──► Buffer for offline peers

                    PULL FLOW
                    ─────────
Relay ──► WebSocket ──► Transport Decrypt (Noise) ──► Envelope ──►
      ──► Decrypt (Group Key) ──► Deserialize ──► App Data
```

---

## 3. Protocol Stack

### 3.1 Layer Diagram

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 5: Application                                       │
│  • App-specific serialization (JSON, MessagePack, etc.)     │
│  • CRDT operations, business logic                          │
├─────────────────────────────────────────────────────────────┤
│  Layer 4: Sync Messages                                     │
│  • PUSH, PULL, ACK, PRESENCE, NOTIFY                        │
│  • Cursor-based ordering                                    │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: Envelope                                          │
│  • Routing (group_id, sender_id)                            │
│  • Metadata (cursor, timestamp, nonce)                      │
│  • MessagePack serialization                                │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: E2E Encryption                                    │
│  • Group Key encryption (XChaCha20-Poly1305, 192-bit nonce) │
│  • Derived from user passphrase via Argon2id                │
├─────────────────────────────────────────────────────────────┤
│  Layer 1: Transport Encryption                              │
│  • Noise Protocol XX handshake                              │
│  • Per-session keys, forward secrecy                        │
├─────────────────────────────────────────────────────────────┤
│  Layer 0: Transport                                         │
│  • WebSocket (Tiers 2-6) or QUIC (Tier 1 via iroh)         │
│  • TLS 1.3 (via Cloudflare or native)                       │
└─────────────────────────────────────────────────────────────┘
```

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
│  Sync Group Key (256-bit)                                  │
│           │                                                 │
│           ▼  HKDF-SHA256                                   │
│     ┌─────┴─────┐                                          │
│     │           │                                          │
│     ▼           ▼                                          │
│  Encryption   Authentication                               │
│  Key (256b)   Key (256b)                                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

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

**Cryptographic Primitives:**

| Function | Algorithm | Crate |
|----------|-----------|-------|
| DH | Curve25519 | snow **v0.9.7+** |
| Cipher | ChaChaPoly | snow **v0.9.7+** |
| Hash | BLAKE2s | snow **v0.9.7+** |

> ⚠️ **snow version requirement:** Use v0.9.7 or later. Earlier versions have security advisories (RUSTSEC-2024-0011, RUSTSEC-2024-0347).

### 4.3 Device Identity

- Each device generates Curve25519 keypair on first launch
- Public key = Device ID (32 bytes, base64 for display)
- Private key stored in OS keychain (via tauri-plugin-keyring or keyring crate)

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
    /// Tier 1: iroh public network
    Iroh,

    /// Tiers 2-3: Self-hosted or PaaS
    Relay { url: String },

    /// Tiers 4-5: CrabNebula managed
    CrabNebula { api_key: String },

    /// Tier 6: Enterprise
    Enterprise { url: String, auth: EnterpriseAuth },
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
| Accept WebSocket connections | Store data long-term |
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
     relay: "wss://sync.example.com",
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
   - relay URL
5. Connect to relay
6. Sync begins
```

### 8.4 QR Code Format

```
URL: cashtable://sync?invite=BASE64_PAYLOAD

Payload (before base64):
{
  "v": 1,
  "r": "wss://sync.example.com",
  "g": "base64(group_id)",
  "s": "base64(group_secret)",
  "c": "base64(creator_pubkey)",
  "e": 1705000000
}
```

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

## 9. Tauri Plugin

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
// @anthropic/tauri-plugin-sync-api

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
import { sync } from '@anthropic/tauri-plugin-sync-api';

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

> ⚠️ **Critical Reality:** On iOS and modern Android, WebSocket connections are killed within ~30 seconds of the app being backgrounded. You cannot rely on persistent connections for sync.

**iOS `applicationWillTerminate`** does NOT guarantee enough execution time to:
1. Establish a WebSocket connection
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
// In your Tauri app
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

Sync should be triggered on these Tauri lifecycle events:

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sync::init())
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
                // App came to foreground - pull new data, push pending
                let sync_client = window.state::<SyncState>();
                tauri::async_runtime::spawn(async move {
                    sync_client.sync_pending_then_pull().await;
                });
            }
            WindowEvent::CloseRequested { .. } => {
                // App closing - DO NOT block waiting for sync!
                // Just mark pending items; they'll sync next launch
                let sync_client = window.state::<SyncState>();
                sync_client.mark_pending_for_next_launch();
                // Fire-and-forget attempt (may not complete)
                tauri::async_runtime::spawn(async move {
                    let _ = tokio::time::timeout(
                        std::time::Duration::from_millis(500),
                        sync_client.quick_flush()
                    ).await;
                });
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
| Background sync | iOS/Android kill background WebSockets | Push notifications |
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
> - **Pin v0.35.x** for production stability
> - v0.90+ is "canary series" with frequent breaking changes
> - Plan migration sprint when 1.0 RC ships (expected mid-2026)

### 10.2 Tier 2: Home Developer

```rust
SyncConfig {
    backend: RelayBackend::Relay {
        url: "wss://sync.home.local".into(),
    },
    ..Default::default()
}
```

Self-hosted Docker container + Cloudflare Tunnel.

### 10.3 Tier 3: Vercel-style

```rust
SyncConfig {
    backend: RelayBackend::Relay {
        url: "wss://my-relay.fly.dev".into(),
    },
    ..Default::default()
}
```

Container deployed to Vercel/Railway/Fly.io.

### 10.4 Tier 4-5: CrabNebula

```rust
SyncConfig {
    backend: RelayBackend::CrabNebula {
        api_key: "cn_live_xxxx".into(),
    },
    ..Default::default()
}
```

API key determines tier (community vs cloud).

### 10.5 Tier 6: Enterprise

```rust
SyncConfig {
    backend: RelayBackend::Enterprise {
        url: "wss://sync.corp.internal".into(),
        auth: EnterpriseAuth::Oidc {
            issuer: "https://auth.corp.com".into(),
            client_id: "sync-client".into(),
        },
    },
    ..Default::default()
}
```

Customer-deployed with enterprise auth integration.

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
backend = "relay"
relay_url = "wss://sync.example.com"
auto_reconnect = true
reconnect_delay_ms = 1000
max_reconnect_delay_ms = 30000
```

### 13.3 Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `SYNC_RELAY_URL` | Override relay URL | Config file |
| `SYNC_API_KEY` | CrabNebula API key | None |
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

### 16.2 Plugin API (Day 1 Requirements)

The Tauri plugin MUST expose push token hooks from Day 1, even if backend integration comes later:

```rust
// tauri-plugin-sync/src/lib.rs

#[tauri::command]
async fn sync_register_push_token(
    state: State<'_, SyncState>,
    token: String,
    platform: PushPlatform,
) -> Result<(), String>;

#[tauri::command]
async fn sync_unregister_push_token(
    state: State<'_, SyncState>,
) -> Result<(), String>;

pub enum PushPlatform {
    Apns,      // Apple Push Notification Service
    Fcm,       // Firebase Cloud Messaging
    Web,       // Web Push (future)
}
```

```typescript
// JavaScript API
export interface SyncPlugin {
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
    // Pass to Tauri via invoke
    TauriInvoke.call("plugin:sync|register_push_token",
                     args: ["token": token, "platform": "apns"])
}

func application(_ application: UIApplication,
                 didReceiveRemoteNotification userInfo: [AnyHashable: Any],
                 fetchCompletionHandler: @escaping (UIBackgroundFetchResult) -> Void) {
    // Silent push received - trigger sync
    TauriInvoke.call("plugin:sync|handle_push_wake")
    fetchCompletionHandler(.newData)
}
```

**Android (FCM):**
```kotlin
// In your Firebase messaging service
class SyncFirebaseService : FirebaseMessagingService() {
    override fun onNewToken(token: String) {
        // Pass to Tauri
        TauriInvoke.call("plugin:sync|register_push_token",
                        mapOf("token" to token, "platform" to "fcm"))
    }

    override fun onMessageReceived(message: RemoteMessage) {
        if (message.data["type"] == "sync") {
            // Trigger sync in background
            TauriInvoke.call("plugin:sync|handle_push_wake")
        }
    }
}
```

### 16.6 Implementation Phases

| Phase | Scope | Dependency |
|-------|-------|------------|
| **Day 1** | Plugin API hooks (register/unregister) | None |
| **Day 1** | Message types (REGISTER_PUSH, etc.) | sync-types |
| **Phase 5** | Relay stores push tokens | sync-relay |
| **Phase 5** | Relay sends to APNS/FCM | Push service integration |
| **Future** | Web Push support | Service workers |

**Key Point:** The client-side hooks MUST exist from Day 1 so apps can register tokens. The relay-side push sending can come later, but the API contract must be stable.

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
tokio = { version = "1", features = ["rt", "sync", "time"] }
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
snow = "0.9.7"                    # v0.9.7+ required (security fixes)
argon2 = "0.5"
chacha20poly1305 = "0.10"        # Supports XChaCha20
rand = "0.8"
thiserror = "1"
tracing = "0.1"
```

### sync-relay
```toml
[dependencies]
sync-types = { path = "../sync-types" }
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
snow = "0.9.7"                    # v0.9.7+ required (security fixes)
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio"] }
axum = "0.7"
tower = "0.4"
tracing = "0.1"
tracing-subscriber = "0.3"
config = "0.14"
```

### tauri-plugin-sync
```toml
[dependencies]
sync-client = { path = "../sync-client" }
tauri = "2"
tauri-plugin = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

*Document: 02-SPECIFICATION.md | Version: 2.1.0 | Date: 2026-01-16*
