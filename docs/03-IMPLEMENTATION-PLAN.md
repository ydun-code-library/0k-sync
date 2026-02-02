# 0k-Sync - Test-Driven Implementation Plan

**Version:** 2.1.0
**Date:** 2026-02-02
**Author:** James (LTIS Investments AB)
**Audience:** Implementing Developers
**Methodology:** Test-Driven Development (TDD) + Jimmy's Workflow

---

## Table of Contents

1. [Implementation Philosophy](#1-implementation-philosophy)
2. [Project Structure](#2-project-structure)
3. [Phase Overview](#3-phase-overview)
4. [Phase 1: sync-types](#4-phase-1-sync-types)
5. [Phase 2: sync-core](#5-phase-2-sync-core)
6. [Phase 3: sync-client](#6-phase-3-sync-client)
6.5. [Phase 3.5: sync-content](#65-phase-35-sync-content)
7. [Phase 4: sync-cli](#7-phase-4-sync-cli)
8. [Phase 5: Framework Integration (Tauri Example)](#8-phase-5-framework-integration-tauri-example)
9. [Phase 6: sync-relay (Future)](#9-phase-6-sync-relay-future)
10. [Testing Strategy](#10-testing-strategy)
11. [Validation Gates](#11-validation-gates)
12. [Rollback Procedures](#12-rollback-procedures)

---

## 1. Implementation Philosophy

### 1.1 TDD: Tests First, Always

```
┌─────────────────────────────────────────────────────────────────┐
│                     TDD Cycle                                    │
│                                                                  │
│     ┌─────────┐                                                 │
│     │  RED    │  Write failing test                             │
│     │         │  (test doesn't compile or fails)                │
│     └────┬────┘                                                 │
│          │                                                       │
│          ▼                                                       │
│     ┌─────────┐                                                 │
│     │  GREEN  │  Write minimal code to pass                     │
│     │         │  (just enough, no more)                         │
│     └────┬────┘                                                 │
│          │                                                       │
│          ▼                                                       │
│     ┌─────────┐                                                 │
│     │REFACTOR │  Improve code quality                           │
│     │         │  (tests still pass)                             │
│     └────┬────┘                                                 │
│          │                                                       │
│          └──────────────► Repeat                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Jimmy's Workflow Integration

Each phase follows:

| Stage | Action | Validation |
|-------|--------|------------|
| 🔴 **RED** | Write tests, implement code | Tests pass |
| 🟢 **GREEN** | Run full test suite, verify behavior | All checks pass |
| 🔵 **CHECKPOINT** | Document, tag, prepare rollback | Ready for next phase |

### 1.3 Key Principles

1. **No code without a test** — If it's not tested, it doesn't exist
2. **Pure logic first** — sync-core has zero I/O, instant tests
3. **Integration last** — Prove components work before combining
4. **Headless testing** — sync-cli enables testing without GUI
5. **Fail fast** — Catch issues early, not in production

---

## 2. Project Structure

### 2.1 Cargo Workspace

```
0k-sync/
├── Cargo.toml                    # Workspace root
├── README.md
├── docs/
│   ├── 01-EXECUTIVE-SUMMARY.md
│   ├── 02-SPECIFICATION.md
│   ├── 03-IMPLEMENTATION-PLAN.md  # This file
│   └── 04-RESEARCH-VALIDATION.md
│
├── sync-types/                    # Phase 1: Wire format
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── envelope.rs
│       ├── messages.rs
│       ├── ids.rs
│       └── error.rs
│
├── sync-core/                     # Phase 2: Pure logic
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── state.rs              # Connection state machine
│       ├── buffer.rs             # Message queue
│       ├── cursor.rs             # Cursor tracking
│       └── pairing.rs            # Invite generation/parsing
│
├── sync-client/                   # Phase 3: Client library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── client.rs             # SyncClient implementation
│       ├── connection.rs         # WebSocket/iroh transport
│       ├── crypto.rs             # E2E encryption
│       ├── storage.rs            # Local persistence
│       └── transport/
│           ├── mod.rs
│           ├── iroh.rs           # Tier 1
│           └── websocket.rs      # Tiers 2-6
│
├── sync-content/                  # Phase 3.5: Large content transfer
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── encrypt.rs            # Encrypt-then-hash pipeline
│       ├── transfer.rs           # iroh-blobs provider/requester wrapper
│       ├── thumbnail.rs          # Preview generation
│       └── lifecycle.rs          # GC coordination, quota management
│
├── sync-cli/                      # Phase 4: Testing tool
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
│
├── tauri-plugin-sync/             # Phase 5: Tauri plugin
│   ├── Cargo.toml
│   ├── src/
│   │   └── lib.rs
│   ├── guest-js/                 # JavaScript bindings
│   │   ├── package.json
│   │   └── src/
│   │       └── index.ts
│   └── build.rs
│
└── sync-relay/                    # Phase 6: Custom relay (future)
    ├── Cargo.toml
    ├── src/
    │   ├── main.rs
    │   ├── server.rs
    │   ├── storage.rs
    │   └── config.rs
    └── Dockerfile
```

### 2.2 Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "sync-types",
    "sync-core",
    "sync-client",
    "sync-content",
    "sync-cli",
    "tauri-plugin-sync",
    # "sync-relay",  # Enable when implementing Phase 6
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/your-org/0k-sync"

[workspace.dependencies]
# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rmp-serde = "1"
uuid = { version = "1", features = ["v4", "serde"] }

# Error handling
thiserror = "1"

# Async runtime
tokio = { version = "1", features = ["full"] }

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Cryptography (PINNED VERSIONS - security critical)
clatter = "2.1"                  # Hybrid Noise Protocol (ML-KEM-768 + X25519)
chacha20poly1305 = "0.10"        # XChaCha20-Poly1305 with 192-bit nonces
argon2 = "0.5"                   # Key derivation with device-adaptive parameters

# P2P networking (PINNED VERSION - iroh 1.0 RC)
iroh = "1.0"                     # iroh 1.0 RC - stable transport layer
iroh-blobs = "1.0"               # Content-addressed storage with BLAKE3/Bao

# Random number generation
rand = "0.8"
getrandom = "0.2"

# Content key derivation
hkdf = "0.12"
sha2 = "0.10"
blake3 = "1"                     # Hash ciphertext for iroh-blobs content address
```

---

## 3. Phase Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Implementation Phases                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Phase 1: sync-types        ──────────────────────────┐                 │
│  (wire format)                                         │                 │
│                                                        ▼                 │
│  Phase 2: sync-core         ◄─────────────────────────┘                 │
│  (pure logic)                                          │                 │
│                                                        ▼                 │
│  Phase 3: sync-client       ◄──────────────────────────┘                │
│  (library)                                             │                 │
│                                                        ▼                 │
│  Phase 3.5: sync-content    ◄──────────────────────────┘                │
│  (large content transfer)                              │                 │
│                                                        │                 │
│                              ┌─────────────────────────┼────────┐       │
│                              ▼                         ▼        │       │
│  Phase 4: sync-cli     Phase 5: tauri-plugin     (parallel)     │       │
│  (testing tool)        (Tauri integration)                      │       │
│                              │                         │        │       │
│                              └─────────────────────────┼────────┘       │
│                                                        ▼                 │
│  Phase 6: sync-relay (FUTURE)                                           │
│  (custom relay)                                                         │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Phase Dependencies

| Phase | Depends On | Blocks |
|-------|------------|--------|
| 1. sync-types | None | All others |
| 2. sync-core | sync-types | sync-client |
| 3. sync-client | sync-types, sync-core | sync-content, sync-cli |
| 3.5. sync-content | sync-types, sync-client | tauri-plugin |
| 4. sync-cli | sync-client | None (testing tool) |
| 5. tauri-plugin | sync-client, sync-content | None (end product) |
| 6. sync-relay | sync-types | None (future) |

---

## 4. Phase 1: sync-types

### 4.1 Objective

Define wire format types that all crates share. This is the foundation—get it right first.

### 4.2 Deliverables

| File | Contents |
|------|----------|
| `ids.rs` | `DeviceId`, `GroupId`, `BlobId`, `Cursor` |
| `envelope.rs` | `Envelope` struct |
| `messages.rs` | All message types (Hello, Push, Pull, etc.) |
| `error.rs` | `SyncError` enum |
| `lib.rs` | Public exports |

### 4.3 TDD Sequence

#### Step 1: Define IDs (RED → GREEN → REFACTOR)

```rust
// sync-types/src/ids.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_roundtrip() {
        let original = DeviceId::random();
        let bytes = original.as_bytes();
        let restored = DeviceId::from_bytes(bytes).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn device_id_base64_display() {
        let id = DeviceId::random();
        let display = id.to_string();
        assert_eq!(display.len(), 43); // 32 bytes = 43 base64 chars (no padding)
    }

    #[test]
    fn group_id_from_secret() {
        let secret = b"test-passphrase-for-sync-group";
        let group = GroupId::from_secret(secret);
        assert_eq!(group.as_bytes().len(), 32);
    }

    #[test]
    fn blob_id_is_uuid_v4() {
        let id = BlobId::new();
        assert_eq!(id.as_bytes().len(), 16);
    }

    #[test]
    fn cursor_ordering() {
        let c1 = Cursor::new(100);
        let c2 = Cursor::new(200);
        assert!(c1 < c2);
        assert!(c2 > c1);
    }
}
```

**Implementation:**
```rust
// sync-types/src/ids.rs

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId([u8; 32]);

impl DeviceId {
    pub fn random() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("getrandom failed");
        Self(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(bytes);
            Some(Self(arr))
        } else {
            None
        }
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        write!(f, "{}", URL_SAFE_NO_PAD.encode(self.0))
    }
}

// Similar implementations for GroupId, BlobId, Cursor...
```

#### Step 2: Define Envelope (RED → GREEN → REFACTOR)

```rust
// sync-types/src/envelope.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serialize_roundtrip() {
        let envelope = Envelope {
            version: 1,
            msg_type: MessageType::Push as u8,
            sender_id: DeviceId::random(),
            group_id: GroupId::from_secret(b"test"),
            cursor: Cursor::new(42),
            timestamp: 1705000000,
            nonce: [0u8; 24],
            payload: vec![1, 2, 3, 4],
        };

        let bytes = envelope.to_bytes().unwrap();
        let restored = Envelope::from_bytes(&bytes).unwrap();

        assert_eq!(envelope.version, restored.version);
        assert_eq!(envelope.cursor, restored.cursor);
        assert_eq!(envelope.payload, restored.payload);
    }

    #[test]
    fn envelope_msgpack_is_compact() {
        let envelope = Envelope::minimal();
        let bytes = envelope.to_bytes().unwrap();
        // MessagePack should be much smaller than JSON
        assert!(bytes.len() < 200);
    }
}
```

#### Step 3: Define Messages (RED → GREEN → REFACTOR)

```rust
// sync-types/src/messages.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_roundtrip() {
        let hello = Hello {
            version: 1,
            device_name: "Test Device".into(),
            group_id: GroupId::from_secret(b"test"),
            last_cursor: Cursor::new(0),
        };

        let bytes = rmp_serde::to_vec(&hello).unwrap();
        let restored: Hello = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(hello.device_name, restored.device_name);
    }

    #[test]
    fn push_with_payload() {
        let push = Push {
            blob_id: BlobId::new(),
            payload: vec![0u8; 1000],
            ttl: 3600,
        };

        let bytes = rmp_serde::to_vec(&push).unwrap();
        let restored: Push = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(push.payload.len(), restored.payload.len());
    }

    // Tests for all message types...
}
```

### 4.4 Validation Gate

```bash
# All tests pass
cargo test -p sync-types

# No warnings
cargo clippy -p sync-types -- -D warnings

# Formatted
cargo fmt -p sync-types --check

# Documentation builds
cargo doc -p sync-types --no-deps
```

### 4.5 Checkpoint

```bash
git add sync-types/
git commit -m "Add sync-types crate with wire format definitions

- DeviceId, GroupId, BlobId, Cursor types
- Envelope structure with MessagePack serialization
- All message types (Hello, Push, Pull, etc.)
- 100% test coverage for serialization roundtrips"

git tag v0.1.0-phase1
```

---

## 5. Phase 2: sync-core

### 5.1 Objective

Implement pure logic with **zero I/O**. This enables instant unit testing without mocking networks.

### 5.2 Key Design: Pure Functions

```rust
// sync-core/src/state.rs

/// Connection state machine - NO I/O, just state transitions
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Handshaking,
    Connected { cursor: Cursor },
    Reconnecting { attempt: u32 },
}

/// Events that can occur
pub enum Event {
    ConnectRequested,
    ConnectSucceeded,
    ConnectFailed { error: String },
    HandshakeCompleted { cursor: Cursor },
    MessageReceived { msg: Message },
    Disconnected { reason: String },
    ReconnectTimer,
}

/// Actions to perform (executed by sync-client)
pub enum Action {
    Connect { url: String },
    SendMessage { msg: Message },
    StartReconnectTimer { delay: Duration },
    EmitEvent { event: SyncEvent },
}

impl ConnectionState {
    /// Pure state transition - no side effects
    pub fn on_event(&self, event: Event) -> (Self, Vec<Action>) {
        match (self, event) {
            (Disconnected, Event::ConnectRequested) => (
                Connecting,
                vec![Action::Connect { url: "...".into() }]
            ),
            // ... all transitions
        }
    }
}
```

### 5.3 TDD Sequence

#### Step 1: Connection State Machine

```rust
// sync-core/src/state.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_disconnected() {
        let state = ConnectionState::new();
        assert!(matches!(state, ConnectionState::Disconnected));
    }

    #[test]
    fn connect_request_transitions_to_connecting() {
        let state = ConnectionState::Disconnected;
        let (new_state, actions) = state.on_event(Event::ConnectRequested);

        assert!(matches!(new_state, ConnectionState::Connecting));
        assert!(actions.iter().any(|a| matches!(a, Action::Connect { .. })));
    }

    #[test]
    fn connect_failure_triggers_reconnect() {
        let state = ConnectionState::Connecting;
        let (new_state, actions) = state.on_event(Event::ConnectFailed {
            error: "timeout".into()
        });

        assert!(matches!(new_state, ConnectionState::Reconnecting { attempt: 1 }));
        assert!(actions.iter().any(|a| matches!(a, Action::StartReconnectTimer { .. })));
    }

    #[test]
    fn reconnect_backoff_increases_with_jitter() {
        // Thundering herd mitigation: exponential backoff + random jitter
        let state = ConnectionState::Reconnecting { attempt: 1 };
        let (_, actions) = state.on_event(Event::ConnectFailed {
            error: "timeout".into()
        });

        if let Some(Action::StartReconnectTimer { delay }) = actions.first() {
            assert!(*delay >= Duration::from_secs(2)); // Base exponential backoff
        } else {
            panic!("Expected reconnect timer");
        }
    }

    #[test]
    fn reconnect_jitter_prevents_thundering_herd() {
        // Multiple clients reconnecting should have different delays
        let mut delays = Vec::new();

        for _ in 0..100 {
            let state = ConnectionState::Reconnecting { attempt: 3 };
            let (_, actions) = state.on_event(Event::ReconnectTimer);

            if let Some(Action::StartReconnectTimer { delay }) = actions.first() {
                delays.push(*delay);
            }
        }

        // With 0-5000ms jitter, delays should vary
        let min = delays.iter().min().unwrap();
        let max = delays.iter().max().unwrap();

        // At least 2 seconds variance due to jitter
        assert!(*max - *min >= Duration::from_secs(2),
            "Jitter should create at least 2s variance in delays");
    }

    #[test]
    fn reconnect_delay_capped_at_30_seconds() {
        // Even after many attempts, delay should be capped
        let state = ConnectionState::Reconnecting { attempt: 10 };
        let (_, actions) = state.on_event(Event::ConnectFailed {
            error: "timeout".into()
        });

        if let Some(Action::StartReconnectTimer { delay }) = actions.first() {
            // Cap: base (30s) + jitter (up to 5s) = max 35s
            assert!(*delay <= Duration::from_secs(35),
                "Reconnect delay must be capped at ~30s + jitter");
        } else {
            panic!("Expected reconnect timer");
        }
    }

    #[test]
    fn successful_connect_resets_attempts() {
        let state = ConnectionState::Reconnecting { attempt: 5 };
        let (new_state, _) = state.on_event(Event::HandshakeCompleted {
            cursor: Cursor::new(100)
        });

        assert!(matches!(new_state, ConnectionState::Connected { .. }));
    }
}
```

#### Step 2: Message Buffer

```rust
// sync-core/src/buffer.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_queues_messages() {
        let mut buffer = MessageBuffer::new(100);
        let msg = Push { blob_id: BlobId::new(), payload: vec![1,2,3], ttl: 0 };

        buffer.enqueue(msg.clone());

        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn buffer_respects_max_size() {
        let mut buffer = MessageBuffer::new(2);

        buffer.enqueue(make_push());
        buffer.enqueue(make_push());
        let overflow = buffer.enqueue(make_push());

        assert!(overflow.is_err());
        assert_eq!(buffer.len(), 2);
    }

    #[test]
    fn buffer_dequeues_in_order() {
        let mut buffer = MessageBuffer::new(100);
        let msg1 = make_push_with_id(1);
        let msg2 = make_push_with_id(2);

        buffer.enqueue(msg1);
        buffer.enqueue(msg2);

        let first = buffer.dequeue().unwrap();
        assert_eq!(get_push_id(&first), 1);
    }

    #[test]
    fn buffer_marks_pending_until_ack() {
        let mut buffer = MessageBuffer::new(100);
        let msg = make_push();
        let blob_id = msg.blob_id;

        buffer.enqueue(msg);
        let pending = buffer.dequeue().unwrap();

        assert!(buffer.is_pending(&blob_id));

        buffer.ack(&blob_id);
        assert!(!buffer.is_pending(&blob_id));
    }
}
```

#### Step 3: Cursor Tracking

```rust
// sync-core/src/cursor.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_tracker_starts_at_zero() {
        let tracker = CursorTracker::new();
        assert_eq!(tracker.last_cursor(), Cursor::new(0));
    }

    #[test]
    fn cursor_tracker_updates_on_receive() {
        let mut tracker = CursorTracker::new();

        tracker.received(Cursor::new(5));
        tracker.received(Cursor::new(3)); // Out of order
        tracker.received(Cursor::new(7));

        assert_eq!(tracker.last_cursor(), Cursor::new(7));
    }

    #[test]
    fn cursor_tracker_detects_gaps() {
        let mut tracker = CursorTracker::new();

        tracker.received(Cursor::new(1));
        tracker.received(Cursor::new(2));
        tracker.received(Cursor::new(5)); // Gap: 3, 4 missing

        assert!(tracker.has_gaps());
        assert_eq!(tracker.missing(), vec![Cursor::new(3), Cursor::new(4)]);
    }
}
```

#### Step 4: Pairing Logic

```rust
// sync-core/src/pairing.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_roundtrip() {
        let invite = Invite::create(
            "wss://relay.example.com",
            GroupId::random(),
            GroupSecret::random(),
        );

        let encoded = invite.to_qr_payload();
        let decoded = Invite::from_qr_payload(&encoded).unwrap();

        assert_eq!(invite.relay_url, decoded.relay_url);
        assert_eq!(invite.group_id, decoded.group_id);
    }

    #[test]
    fn short_code_format() {
        let invite = Invite::create(
            "wss://relay.example.com",
            GroupId::random(),
            GroupSecret::random(),
        );

        let code = invite.to_short_code();

        // Format: XXXX-XXXX-XXXX-XXXX
        assert_eq!(code.len(), 19);
        assert_eq!(&code[4..5], "-");
        assert_eq!(&code[9..10], "-");
        assert_eq!(&code[14..15], "-");
    }

    #[test]
    fn short_code_splits_correctly() {
        let code = "ABCD-EFGH-IJKL-MNOP";
        let (lookup, decrypt) = Invite::split_short_code(code).unwrap();

        assert_eq!(lookup, "ABCDEFGH");
        assert_eq!(decrypt, "IJKLMNOP");
    }

    #[test]
    fn invite_expires() {
        let invite = Invite::create_with_ttl(
            "wss://relay.example.com",
            GroupId::random(),
            GroupSecret::random(),
            Duration::from_secs(0), // Already expired
        );

        assert!(invite.is_expired());
    }
}
```

### 5.4 Validation Gate

```bash
# All tests pass (should be instant - no I/O!)
cargo test -p sync-core

# Verify no I/O dependencies
cargo tree -p sync-core | grep -E "tokio|async|socket|network"
# Should return nothing

# Clippy clean
cargo clippy -p sync-core -- -D warnings
```

### 5.5 Checkpoint

```bash
git add sync-core/
git commit -m "Add sync-core crate with pure logic

- ConnectionState machine (no I/O)
- MessageBuffer with pending tracking
- CursorTracker with gap detection
- Invite generation/parsing
- All tests pass instantly (no network mocking)"

git tag v0.1.0-phase2
```

---

## 6. Phase 3: sync-client

### 6.1 Objective

Build the client library that apps will use. This is where I/O happens.

### 6.2 Key Design: Transport Abstraction

```rust
// sync-client/src/transport/mod.rs

#[async_trait]
pub trait Transport: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    async fn recv(&mut self) -> Result<Vec<u8>>;
}

// Tier 1: iroh
pub struct IrohTransport { /* ... */ }

// Tiers 2-6: WebSocket
pub struct WebSocketTransport { /* ... */ }
```

### 6.3 TDD Sequence

#### Step 1: Crypto Module

> ⚠️ **Critical Version Pins:**
> - `clatter = "2.1"` — Hybrid Noise Protocol with ML-KEM-768 + X25519
> - XChaCha20-Poly1305 — 192-bit nonces (not 96-bit ChaCha20)
> - Device-adaptive Argon2id — 12 MiB to 64 MiB based on RAM

```rust
// sync-client/src/crypto.rs

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================
    // XChaCha20-Poly1305 Tests (192-bit nonces)
    // ===========================================

    #[test]
    fn group_key_derivation_uses_device_adaptive_argon2() {
        // Detect available RAM and use appropriate parameters
        let ram_mb = detect_available_ram_mb();
        let secret = GroupSecret::from_passphrase("my-secure-passphrase");

        let start = std::time::Instant::now();
        let key = GroupKey::derive(&secret, ram_mb);
        let elapsed = start.elapsed();

        assert_eq!(key.encryption_key().len(), 32);
        assert_eq!(key.auth_key().len(), 32);

        // Device-adaptive: should take 200-500ms
        assert!(elapsed >= Duration::from_millis(150));
        assert!(elapsed <= Duration::from_secs(1));
    }

    #[test]
    fn argon2_parameters_scale_with_ram() {
        // Low-end mobile: 12 MiB
        let params_low = Argon2Params::for_ram_mb(1500);
        assert_eq!(params_low.memory_mib(), 12);
        assert_eq!(params_low.iterations(), 3);

        // Mid-range mobile: 19 MiB
        let params_mid = Argon2Params::for_ram_mb(3000);
        assert_eq!(params_mid.memory_mib(), 19);
        assert_eq!(params_mid.iterations(), 2);

        // High-end mobile: 46 MiB
        let params_high = Argon2Params::for_ram_mb(6000);
        assert_eq!(params_high.memory_mib(), 46);
        assert_eq!(params_high.iterations(), 1);

        // Desktop: 64 MiB
        let params_desktop = Argon2Params::for_ram_mb(16000);
        assert_eq!(params_desktop.memory_mib(), 64);
        assert_eq!(params_desktop.iterations(), 3);
    }

    #[test]
    fn xchacha20_uses_192_bit_nonces() {
        let key = GroupKey::derive(&GroupSecret::random(), 16000);
        let plaintext = b"Hello, sync world!";

        let (ciphertext, nonce) = key.encrypt(plaintext).unwrap();

        // XChaCha20 uses 24-byte (192-bit) nonces, not 12-byte (96-bit)
        assert_eq!(nonce.len(), 24, "Must use 192-bit nonces for XChaCha20");

        let decrypted = key.decrypt(&ciphertext, &nonce).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn random_192_bit_nonces_are_safe() {
        // 192-bit nonces have 2^80 birthday bound (vs 2^32 for 96-bit)
        // Safe to generate randomly without coordination
        let key = GroupKey::derive(&GroupSecret::random(), 16000);
        let plaintext = b"Same message";

        let (ct1, nonce1) = key.encrypt(plaintext).unwrap();
        let (ct2, nonce2) = key.encrypt(plaintext).unwrap();

        // Different random nonces
        assert_ne!(nonce1, nonce2);
        // Different ciphertext
        assert_ne!(ct1, ct2);

        // Both decrypt correctly
        assert_eq!(key.decrypt(&ct1, &nonce1).unwrap(), plaintext.as_slice());
        assert_eq!(key.decrypt(&ct2, &nonce2).unwrap(), plaintext.as_slice());
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let key1 = GroupKey::derive(&GroupSecret::random(), 16000);
        let key2 = GroupKey::derive(&GroupSecret::random(), 16000);
        let plaintext = b"Secret message";

        let (ciphertext, nonce) = key1.encrypt(plaintext).unwrap();
        let result = key2.decrypt(&ciphertext, &nonce);

        assert!(result.is_err());
    }

    // ===========================================
    // Hybrid Noise Protocol Tests (clatter v2.1+)
    // ===========================================

    #[test]
    fn noise_hybrid_xx_handshake_succeeds() {
        // Using clatter v2.1+ for hybrid ML-KEM-768 + X25519
        let initiator = NoiseSession::new_initiator();
        let responder = NoiseSession::new_responder();

        // Noise XX pattern: -> e, <- e, ee, s, es, -> s, se
        let msg1 = initiator.write_message(&[]).unwrap();
        let msg2 = responder.read_message(&msg1).unwrap();
        let msg2_out = responder.write_message(&msg2).unwrap();
        let msg3 = initiator.read_message(&msg2_out).unwrap();
        let msg3_out = initiator.write_message(&msg3).unwrap();
        responder.read_message(&msg3_out).unwrap();

        assert!(initiator.is_transport_ready());
        assert!(responder.is_transport_ready());
    }

    #[test]
    fn noise_provides_forward_secrecy() {
        let initiator = NoiseSession::new_initiator();
        let responder = NoiseSession::new_responder();

        // Complete handshake
        complete_handshake(&mut initiator, &mut responder);

        // Get transport keys
        let transport_key_1 = initiator.get_transport_key();

        // New session with same static keys
        let initiator2 = NoiseSession::new_initiator();
        let responder2 = NoiseSession::new_responder();
        complete_handshake(&mut initiator2, &mut responder2);

        let transport_key_2 = initiator2.get_transport_key();

        // Different ephemeral keys = different transport keys
        // This is forward secrecy
        assert_ne!(transport_key_1, transport_key_2);
    }
}
```

#### Step 2: Client Implementation

```rust
// sync-client/src/client.rs

#[cfg(test)]
mod tests {
    use super::*;

    // Use mock transport for unit tests
    struct MockTransport {
        sent: Vec<Vec<u8>>,
        recv_queue: VecDeque<Vec<u8>>,
    }

    #[tokio::test]
    async fn client_connects_and_sends_hello() {
        let mut mock = MockTransport::new();
        mock.expect_recv(welcome_message());

        let config = SyncConfig::test_config();
        let mut client = SyncClient::with_transport(config, mock);

        let result = client.connect().await;

        assert!(result.is_ok());
        assert!(client.transport.sent_contains_hello());
    }

    #[tokio::test]
    async fn push_encrypts_and_sends() {
        let mut mock = MockTransport::new();
        mock.expect_recv(push_ack_message(42));

        let client = connected_client(mock).await;
        let data = b"test payload";

        let result = client.push(data).await.unwrap();

        assert_eq!(result.cursor, Cursor::new(42));
        // Verify payload was encrypted (not plaintext)
        let sent = client.transport.last_sent();
        assert!(!sent.windows(data.len()).any(|w| w == data));
    }

    #[tokio::test]
    async fn pull_returns_decrypted_blobs() {
        let mut mock = MockTransport::new();
        let encrypted_blob = encrypt_test_data(b"secret data");
        mock.expect_recv(pull_response_message(vec![encrypted_blob]));

        let client = connected_client(mock).await;

        let result = client.pull(Cursor::new(0)).await.unwrap();

        assert_eq!(result.blobs.len(), 1);
        assert_eq!(result.blobs[0].data, b"secret data");
    }
}
```

#### Step 3: Integration Test (Two Clients)

> **iroh Version Strategy:**
> - Using **iroh 1.0 RC** for production stability
> - Stable API, ready for production use
> - Self-hosted infrastructure option via iroh-relay and iroh-dns-server

```rust
// sync-client/tests/integration.rs

#[tokio::test]
async fn two_clients_sync_via_iroh() {
    // This test uses iroh 1.0 RC - requires network
    // Uses mDNS for local discovery when on same LAN
    if std::env::var("RUN_NETWORK_TESTS").is_err() {
        return; // Skip in CI without network
    }

    let group_secret = GroupSecret::random();

    // Client A
    let config_a = SyncConfig {
        backend: RelayBackend::Iroh,
        group_key: Some(GroupKey::derive(&group_secret)),
        ..Default::default()
    };
    let mut client_a = SyncClient::new(config_a).await.unwrap();

    // Client B
    let config_b = SyncConfig {
        backend: RelayBackend::Iroh,
        group_key: Some(GroupKey::derive(&group_secret)),
        ..Default::default()
    };
    let mut client_b = SyncClient::new(config_b).await.unwrap();

    // A pushes
    let data = b"Hello from A!";
    let push_result = client_a.push(data).await.unwrap();

    // B pulls
    let pull_result = client_b.pull(Cursor::new(0)).await.unwrap();

    assert_eq!(pull_result.blobs.len(), 1);
    assert_eq!(pull_result.blobs[0].data, data.as_slice());
}
```

### 6.4 Validation Gate

```bash
# Unit tests (fast, no network)
cargo test -p sync-client --lib

# Integration tests (requires network)
RUN_NETWORK_TESTS=1 cargo test -p sync-client --test integration

# Clippy
cargo clippy -p sync-client -- -D warnings
```

### 6.5 Checkpoint

```bash
git add sync-client/
git commit -m "Add sync-client library

- SyncClient with push/pull/subscribe API
- GroupKey E2E encryption (XChaCha20-Poly1305, 192-bit nonces)
- Device-adaptive Argon2id key derivation (12-64 MiB)
- Hybrid Noise Protocol XX (clatter v2.1+, ML-KEM-768 + X25519)
- Transport abstraction (iroh 1.0 RC, WebSocket)
- Thundering herd mitigation with jitter
- Integration test: two clients syncing"

git tag v0.1.0-phase3
```

---

## 6.5 Phase 3.5: sync-content

### 6.5.1 Objective

Provide large content transfer capabilities using iroh-blobs. Small sync messages (<64KB) go through the relay; large content (photos, documents, audio) transfers directly between devices via iroh-blobs with encrypt-then-hash.

### 6.5.2 Deliverables

| File | Contents |
|------|----------|
| `lib.rs` | Public API, ContentTransfer struct |
| `encrypt.rs` | Encrypt-then-hash pipeline, content key derivation |
| `transfer.rs` | iroh-blobs provider/requester wrapper |
| `thumbnail.rs` | Preview generation (optional, platform-specific) |
| `lifecycle.rs` | GC coordination, quota management |

### 6.5.3 TDD Sequence

```rust
// sync-content/src/encrypt.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_key_derivation() {
        // HKDF-SHA256 from GroupSecret
        let group_secret = [0u8; 32];
        let blob_id = [1u8; 16];

        let content_key = derive_content_key(&group_secret, &blob_id);

        assert_eq!(content_key.len(), 32);
        // Different blob_id → different key
        let other_key = derive_content_key(&group_secret, &[2u8; 16]);
        assert_ne!(content_key, other_key);
    }

    #[test]
    fn test_encrypt_then_hash() {
        let content_key = [0u8; 32];
        let plaintext = b"Hello, World!";

        let (ciphertext, nonce) = encrypt_content(&content_key, plaintext)?;
        let content_hash = blake3::hash(&ciphertext);

        // Hash is of CIPHERTEXT, not plaintext
        assert_ne!(content_hash, blake3::hash(plaintext));

        // Decrypt succeeds
        let decrypted = decrypt_content(&content_key, &nonce, &ciphertext)?;
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_content_reference_creation() {
        let content_key = [0u8; 32];
        let plaintext = b"Large file content...";

        let content_ref = ContentRef::from_plaintext(&content_key, plaintext)?;

        assert!(content_ref.content_size > 0);
        assert!(content_ref.encrypted_size >= content_ref.content_size);
        assert_eq!(content_ref.encryption_nonce.len(), 24); // XChaCha20
    }
}

// sync-content/src/transfer.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_content_provider() {
        // Setup iroh-blobs store
        let store = iroh_blobs::store::mem::Store::new();

        // Add encrypted content
        let ciphertext = b"encrypted content";
        let hash = store.import_bytes(ciphertext.into()).await?;

        // Hash matches BLAKE3 of ciphertext
        assert_eq!(hash, blake3::hash(ciphertext));
    }

    #[tokio::test]
    async fn test_content_download() {
        // Two stores (simulating two devices)
        let store_a = iroh_blobs::store::mem::Store::new();
        let store_b = iroh_blobs::store::mem::Store::new();

        // Device A provides content
        let ciphertext = b"encrypted content";
        let hash = store_a.import_bytes(ciphertext.into()).await?;

        // Device B downloads (via iroh connection in real scenario)
        // Test that content verification works
        let downloaded = store_b.export_bytes(&hash).await?;
        assert_eq!(downloaded.as_ref(), ciphertext);
    }
}
```

### 6.5.4 Implementation Order

| Step | Test | Implementation |
|------|------|----------------|
| 1 | Content key derivation | `derive_content_key()` with HKDF-SHA256 |
| 2 | Encrypt-then-hash | `encrypt_content()`, `decrypt_content()` |
| 3 | ContentRef creation | `ContentRef::from_plaintext()` |
| 4 | iroh-blobs integration | `ContentProvider`, `ContentRequester` |
| 5 | GC coordination | `ContentLifecycle::cleanup_orphaned()` |

### 6.5.5 Checkpoint Criteria

```bash
cargo test -p sync-content
# All tests pass

# Verify:
# - Content key derivation is deterministic (same inputs → same key)
# - Encrypt-then-hash produces verifiable ciphertext
# - iroh-blobs store/retrieve works correctly
# - ContentRef round-trips through MessagePack
```

### 6.5.6 Git Checkpoint

```bash
git add sync-content/
git commit -m "Add sync-content crate

Phase 3.5 complete:
- Content key derivation (HKDF-SHA256 from GroupSecret)
- Encrypt-then-hash pipeline (XChaCha20-Poly1305 → BLAKE3)
- iroh-blobs integration for content-addressed storage
- ContentRef struct for sync relay metadata
- GC coordination for orphaned content cleanup"

git tag v0.1.0-phase3.5
```

---

## 7. Phase 4: sync-cli

### 7.1 Objective

Build a command-line tool for headless testing. This enables scripted E2E tests without GUI.

### 7.2 Commands

```
sync-cli init --name "Device A"
sync-cli pair --create
sync-cli pair --join XXXX-XXXX-XXXX-XXXX
sync-cli push "message"
sync-cli push --file data.json
sync-cli pull
sync-cli pull --after-cursor 100
sync-cli status
sync-cli devices
```

### 7.3 TDD Sequence

```rust
// sync-cli/src/main.rs

#[cfg(test)]
mod tests {
    use super::*;
    use assert_cmd::Command;

    #[test]
    fn init_creates_device_identity() {
        let temp_dir = tempdir().unwrap();

        Command::cargo_bin("sync-cli")
            .unwrap()
            .args(["init", "--name", "Test Device", "--data-dir", temp_dir.path().to_str().unwrap()])
            .assert()
            .success();

        assert!(temp_dir.path().join("device.key").exists());
    }

    #[test]
    fn pair_create_shows_code() {
        let temp_dir = setup_initialized_device();

        let output = Command::cargo_bin("sync-cli")
            .unwrap()
            .args(["pair", "--create", "--data-dir", temp_dir.path().to_str().unwrap()])
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should show XXXX-XXXX-XXXX-XXXX format
        assert!(stdout.contains("-"));
    }

    #[test]
    fn push_pull_roundtrip() {
        // Requires two initialized devices and network
        if std::env::var("RUN_NETWORK_TESTS").is_err() {
            return;
        }

        let device_a = setup_device("A");
        let device_b = setup_device("B");

        // A creates group
        let code = create_invite(&device_a);

        // B joins
        join_invite(&device_b, &code);

        // A pushes
        Command::cargo_bin("sync-cli")
            .unwrap()
            .args(["push", "Hello from A", "--data-dir", device_a.path().to_str().unwrap()])
            .assert()
            .success();

        // B pulls
        let output = Command::cargo_bin("sync-cli")
            .unwrap()
            .args(["pull", "--data-dir", device_b.path().to_str().unwrap()])
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Hello from A"));
    }
}
```

### 7.4 Validation Gate

```bash
# CLI builds
cargo build -p sync-cli

# Unit tests
cargo test -p sync-cli

# Manual verification
./target/debug/sync-cli --help
./target/debug/sync-cli init --name "Test"
./target/debug/sync-cli pair --create
```

### 7.5 Checkpoint

```bash
git add sync-cli/
git commit -m "Add sync-cli testing tool

- init: Create device identity
- pair --create/--join: Pairing flow
- push/pull: Data sync
- status: Connection info
- Enables headless E2E testing"

git tag v0.1.0-phase4
```

---

## 8. Phase 5: Framework Integration (Tauri Example)

### 8.1 Objective

Wrap sync-client for your framework of choice. This section uses Tauri as the example, but the pattern applies to Electron, React Native, Flutter, etc.

> ⚠️ **Mobile Lifecycle:** The plugin MUST handle mobile app lifecycle correctly. See Section 10.4 for mobile-specific test requirements. Key rules:
> - Never block on close
> - Fire-and-forget flush with 500ms timeout
> - Persist pending items for next launch
> - Emit sync status to UI

### 8.2 TDD Sequence

```rust
// tauri-plugin-sync/src/lib.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::test::{mock_builder, MockRuntime};

    #[test]
    fn plugin_initializes() {
        let app = mock_builder()
            .plugin(init())
            .build()
            .unwrap();

        assert!(app.state::<SyncState>().is_ok());
    }

    #[tokio::test]
    async fn sync_enable_command() {
        let app = mock_builder()
            .plugin(init())
            .build()
            .unwrap();

        let result: Result<(), String> = tauri::test::invoke(
            &app,
            "plugin:sync|enable",
            (),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn sync_create_invite_returns_code() {
        let app = mock_builder()
            .plugin(init())
            .build()
            .unwrap();

        // Enable first
        let _: () = tauri::test::invoke(&app, "plugin:sync|enable", ()).await.unwrap();

        // Create invite
        let invite: Invite = tauri::test::invoke(
            &app,
            "plugin:sync|create_invite",
            (),
        ).await.unwrap();

        assert!(!invite.short_code.is_empty());
        assert!(invite.expires_at > 0);
    }
}
```

### 8.3 JavaScript Bindings

```typescript
// tauri-plugin-sync/guest-js/src/index.ts

import { invoke } from '@tauri-apps/api/core';

export interface Invite {
  qrCode: string;
  shortCode: string;
  expiresAt: number;
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

export async function enable(): Promise<void> {
  return invoke('plugin:sync|enable');
}

export async function disable(): Promise<void> {
  return invoke('plugin:sync|disable');
}

export async function createInvite(): Promise<Invite> {
  return invoke('plugin:sync|create_invite');
}

export async function joinInvite(invite: string): Promise<void> {
  return invoke('plugin:sync|join_invite', { invite });
}

export async function push(data: Uint8Array): Promise<PushResult> {
  return invoke('plugin:sync|push', { data: Array.from(data) });
}

export async function pull(afterCursor?: number): Promise<PullResult> {
  return invoke('plugin:sync|pull', { afterCursor });
}

// ... events, status, etc.
```

### 8.4 Validation Gate

```bash
# Rust tests
cargo test -p tauri-plugin-sync

# Build JS bindings
cd tauri-plugin-sync/guest-js && npm run build

# Integration test with real Tauri app
cd examples/test-app && cargo tauri dev
```

### 8.5 Checkpoint

```bash
git add tauri-plugin-sync/
git commit -m "Add tauri-plugin-sync (example framework integration)

- Tauri 2.0 plugin structure
- Commands: enable, disable, create_invite, join_invite, push, pull
- JavaScript/TypeScript bindings
- Event emission to frontend
- Pattern can be adapted to other frameworks (Electron, React Native, etc.)"

git tag v0.1.0-phase5
```

---

## 9. Phase 6: sync-relay (Future)

### 9.1 Objective

Build custom relay for self-hosted deployments (Tiers 2-6).

**Status:** Deferred. MVP uses iroh public network.

### 9.2 When to Implement

Implement sync-relay when:
- iroh-based MVP is stable and validated
- Users request self-hosted option
- Managed Cloud wants Tiers 4-6 control

### 9.3 Implementation Outline

```rust
// sync-relay/src/main.rs

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;

    // SQLite for temporary buffer
    let db = Database::connect(&config.database).await?;

    // WebSocket server
    let server = RelayServer::new(config, db);

    // Health/metrics endpoints
    let api = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics));

    // Run both
    tokio::select! {
        _ = server.run() => {},
        _ = axum::serve(api_listener, api) => {},
    }

    Ok(())
}
```

### 9.4 Test Strategy

```rust
#[tokio::test]
async fn relay_routes_between_clients() {
    let relay = TestRelay::start().await;

    let client_a = connect_client(&relay, "A").await;
    let client_b = connect_client(&relay, "B").await;

    // A pushes
    client_a.send(push_message(b"hello")).await;

    // B receives notify
    let notify = client_b.recv().await;
    assert!(matches!(notify, Message::Notify { .. }));

    // B pulls
    client_b.send(pull_message(0)).await;
    let response = client_b.recv().await;
    assert!(matches!(response, Message::PullResponse { .. }));
}
```

---

## 10. Testing Strategy

### 10.1 Test Pyramid

```
                    ▲
                   ╱ ╲
                  ╱   ╲
                 ╱ E2E ╲         Few, slow, high confidence
                ╱───────╲
               ╱         ╲
              ╱Integration╲      Some, medium speed
             ╱─────────────╲
            ╱               ╲
           ╱   Unit Tests    ╲   Many, fast, isolated
          ╱───────────────────╲
```

### 10.2 Test Distribution

| Layer | Location | Count | Speed |
|-------|----------|-------|-------|
| Unit | Each crate's `src/*.rs` | Many | < 1s total |
| Integration | `tests/*.rs` | Some | < 30s |
| E2E | `sync-cli` scripts | Few | < 2min |

### 10.3 Test Commands

```bash
# All unit tests (fast)
cargo test --workspace --lib

# All tests including integration
cargo test --workspace

# With coverage
cargo tarpaulin --workspace --out Html

# E2E script
./scripts/e2e-test.sh
```

### 10.4 Mobile Lifecycle Testing

Testing mobile-specific sync behavior requires special consideration:

#### 10.4.1 Stranded Commits Test

```rust
// sync-client/tests/mobile_lifecycle.rs

#[tokio::test]
async fn stranded_commits_persist_locally() {
    let client = setup_client().await;
    let db = client.local_db();

    // Simulate data created while "offline"
    let tx = create_test_transaction();
    db.insert(&tx).await.unwrap();
    db.mark_pending_sync(tx.id).await.unwrap();

    // Verify pending state persists
    let pending = db.get_pending_sync().await.unwrap();
    assert!(pending.contains(&tx.id));

    // Simulate "app restart" - pending items should still be there
    let client2 = setup_client_same_db().await;
    let pending2 = client2.local_db().get_pending_sync().await.unwrap();
    assert!(pending2.contains(&tx.id));
}
```

#### 10.4.2 Quick Flush Timeout Test

```rust
#[tokio::test]
async fn quick_flush_respects_timeout() {
    let client = setup_client().await;

    // Queue multiple items
    for i in 0..100 {
        client.queue_for_sync(make_blob(i)).await;
    }

    // Quick flush with 500ms timeout (simulating app close)
    let start = Instant::now();
    let result = tokio::time::timeout(
        Duration::from_millis(500),
        client.quick_flush()
    ).await;

    // Should complete within timeout, not block indefinitely
    assert!(start.elapsed() < Duration::from_secs(1));

    // Some items may not have synced - that's expected
    let pending = client.pending_count().await;
    // We don't assert pending == 0 because quick_flush may not finish all
}
```

#### 10.4.3 Sync Status Indicator Test

```rust
#[test]
fn sync_status_reflects_actual_state() {
    let mut tracker = SyncStatusTracker::new();

    // Initially offline
    assert_eq!(tracker.status(), SyncStatus::Offline);

    // Connected but nothing synced yet
    tracker.on_connected();
    assert_eq!(tracker.status(), SyncStatus::Connected);

    // Push succeeded
    tracker.on_push_acked(BlobId::new());
    assert_eq!(tracker.status(), SyncStatus::Synced);

    // Push failed
    tracker.on_push_failed(BlobId::new());
    assert_eq!(tracker.status(), SyncStatus::PendingSync);

    // Disconnected
    tracker.on_disconnected();
    assert_eq!(tracker.status(), SyncStatus::Offline);
}
```

#### 10.4.4 Tauri Lifecycle Integration Test

```rust
// tauri-plugin-sync/tests/lifecycle.rs

#[tokio::test]
async fn plugin_handles_focus_events() {
    let app = mock_tauri_app_with_sync_plugin();

    // Simulate app losing focus (backgrounded)
    app.emit_window_event(WindowEvent::Focused(false));

    // Plugin should NOT block or panic
    // Just mark state for next foreground

    // Simulate app gaining focus (foregrounded)
    app.emit_window_event(WindowEvent::Focused(true));

    // Plugin should trigger sync in background
    // Verify sync was triggered (non-blocking)
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(app.state::<SyncState>().sync_triggered());
}

#[tokio::test]
async fn plugin_handles_close_gracefully() {
    let app = mock_tauri_app_with_sync_plugin();

    // Queue some data
    let state = app.state::<SyncState>();
    state.queue_blob(b"pending data").await;

    // Simulate close request
    let start = Instant::now();
    app.emit_window_event(WindowEvent::CloseRequested {
        api: CloseRequestApi::new(),
    });

    // Should NOT block for more than 500ms
    assert!(start.elapsed() < Duration::from_secs(1));
}
```

#### 10.4.5 UI Status Indicator Mapping

```rust
// sync-core/src/status.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_emoji_mapping() {
        assert_eq!(SyncStatus::Synced.emoji(), "☁️✓");
        assert_eq!(SyncStatus::PendingSync.emoji(), "☁️⏳");
        assert_eq!(SyncStatus::SyncFailed.emoji(), "☁️✗");
        assert_eq!(SyncStatus::Offline.emoji(), "📴");
    }

    #[test]
    fn status_user_message() {
        assert_eq!(
            SyncStatus::PendingSync.user_message(),
            "Changes saved locally. Will sync when connection restored."
        );
    }
}
```

### 10.5 E2E Test Script

```bash
#!/bin/bash
# scripts/e2e-test.sh

set -e

echo "=== E2E Test: Two Devices Sync ==="

# Build CLI
cargo build -p sync-cli --release
CLI=./target/release/sync-cli

# Setup temp directories
DIR_A=$(mktemp -d)
DIR_B=$(mktemp -d)

# Initialize devices
$CLI init --name "Device A" --data-dir "$DIR_A"
$CLI init --name "Device B" --data-dir "$DIR_B"

# A creates invite
INVITE=$($CLI pair --create --data-dir "$DIR_A" --output code)
echo "Invite code: $INVITE"

# B joins
$CLI pair --join "$INVITE" --data-dir "$DIR_B"

# A pushes
$CLI push "Hello from A" --data-dir "$DIR_A"
$CLI push "Second message" --data-dir "$DIR_A"

# Give time for sync
sleep 2

# B pulls
OUTPUT=$($CLI pull --data-dir "$DIR_B")
echo "$OUTPUT"

# Verify
if echo "$OUTPUT" | grep -q "Hello from A"; then
    echo "✅ E2E test passed"
else
    echo "❌ E2E test failed"
    exit 1
fi

# Cleanup
rm -rf "$DIR_A" "$DIR_B"
```

---

## 11. Validation Gates

### 11.1 Gate Checklist (Every Phase)

```markdown
## Phase N Validation Gate

- [ ] All unit tests pass: `cargo test -p <crate>`
- [ ] No clippy warnings: `cargo clippy -p <crate> -- -D warnings`
- [ ] Formatted: `cargo fmt -p <crate> --check`
- [ ] Documentation builds: `cargo doc -p <crate> --no-deps`
- [ ] No new dependencies without justification
- [ ] Integration tests pass (if applicable)
- [ ] Manual verification complete
- [ ] Git tag created: `vX.Y.Z-phaseN`
```

### 11.2 CI Pipeline

```yaml
# .github/workflows/ci.yml

name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Check formatting
        run: cargo fmt --all --check

      - name: Clippy
        run: cargo clippy --workspace -- -D warnings

      - name: Unit tests
        run: cargo test --workspace --lib

      - name: Integration tests
        run: cargo test --workspace
        env:
          RUN_NETWORK_TESTS: "1"

      - name: Build all
        run: cargo build --workspace --release
```

---

## 12. Rollback Procedures

### 12.1 Phase Rollback

If a phase fails validation:

```bash
# Reset to previous checkpoint
git reset --hard v0.1.0-phase$(N-1)

# Or revert specific commits
git revert <commit-hash>
```

### 12.2 Dependency Rollback

If a dependency causes issues:

```bash
# Check what changed
cargo tree -p <crate> --depth 1

# Pin to previous version in Cargo.toml
<dependency> = "=X.Y.Z"

# Update lockfile
cargo update -p <dependency>
```

### 12.3 Breaking Change Rollback

If a breaking change reaches users:

1. **Immediate:** Revert commit, publish patch
2. **Document:** Add to CHANGELOG with migration guide
3. **Prevent:** Add regression test

---

## Summary

| Phase | Crate | Key Deliverable | Test Focus |
|-------|-------|-----------------|------------|
| 1 | sync-types | Wire format | Serialization roundtrip |
| 2 | sync-core | State machine | Pure logic (no I/O) |
| 3 | sync-client | Client library | Encryption, transport |
| 4 | sync-cli | Testing tool | E2E headless |
| 5 | framework-integration | Example: Tauri plugin | Commands, events |
| 6 | sync-relay | Custom relay | Message routing |

**Remember:** Tests first. Every time. No exceptions.

---

*Document: 03-IMPLEMENTATION-PLAN.md | Version: 2.0.0 | Date: 2026-01-16*
