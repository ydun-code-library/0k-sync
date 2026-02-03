# 0k-Sync — WebSocket Removal & Transport Architecture Correction

**Document:** WEBSOCKET-REMOVAL-AMENDMENT.md
**Version:** 1.0.0
**Date:** 2026-02-03
**Purpose:** Remove WebSocket transport from the entire 0k-Sync architecture, replace with iroh QUIC throughout
**Affects:** 02-SPECIFICATION.md, 03-IMPLEMENTATION-PLAN.md, 04-RESEARCH-VALIDATION.md
**Executor:** Moneypenny (Claude Code CLI)
**Verifier:** Q workstation

---

## The Problem

The 0k-Sync specification was originally written with a WebSocket-based client-server transport model: clients connect to a custom relay via `wss://`, the relay accepts WebSocket connections, `tokio-tungstenite` appears in dependency lists for both sync-client and sync-relay.

The iroh deep dive (2026-02-02) replaced the transport layer with iroh — QUIC via Quinn, P2P with relay fallback, mDNS/DNS/DHT discovery. The layer stack (Section 3.1) was updated. But the WebSocket assumption was never removed from the rest of the documents. The result is two contradictory transport architectures coexisting in the same spec.

This contradiction matters because:

1. **Phase 3 builds dead code.** The impl plan tells Q to create `websocket.rs` and `WebSocketTransport` with `tokio-tungstenite`. There is nothing for it to connect to — `sync-relay` (Phase 6) is deferred, and even when built, it should be an iroh Endpoint, not a WebSocket server.

2. **Phase 6 builds the wrong relay.** The impl plan and spec describe `sync-relay` as a WebSocket server using `tokio-tungstenite` + `axum`. But the whole point of the iroh decision is that the transport is QUIC all the way down. The custom relay should be an iroh Endpoint that accepts QUIC connections and speaks the 0k-sync protocol, not a WebSocket server.

3. **The dependency list is wrong.** `tokio-tungstenite` appears in sync-client and sync-relay Cargo.toml. Neither needs it. The entire transport is iroh.

4. **The value proposition is undermined.** The architecture is "Rust + SQLite." That means: Rust for everything, iroh for transport, SQLite for storage. No Node.js runtime, no Deno runtime, no WebSocket server framework. Apps using 0k-sync embed a Rust crate — they don't need to run a separate WebSocket server or connect to one.

The fix is simple: iroh QUIC is the only transport. All tiers. The custom sync-relay (Phase 6) is an iroh Endpoint, not a WebSocket server.

---

## Architecture After This Amendment

```
                    0k-Sync Transport Stack (All Tiers)
                    ────────────────────────────────────

    App (Tauri/CLI)                    Relay Infrastructure
   ┌─────────────────┐                ┌─────────────────────┐
   │  sync-client    │                │  Tier 1: iroh-relay │ ← dumb datagram forwarder
   │  ┌─────────┐   │    QUIC/iroh   │  (public network)   │
   │  │  iroh   │───┼────────────────►│                     │
   │  │Endpoint │   │                │  Tier 2-6:          │
   │  └─────────┘   │                │  sync-relay         │ ← smart: cursors, groups,
   │                 │                │  (iroh Endpoint     │   buffering, routing
   │  + SQLite       │                │   + SQLite)         │   also an iroh Endpoint
   └─────────────────┘                └─────────────────────┘

   No WebSocket. No tokio-tungstenite. No wss:// URLs.
   All connections are QUIC via iroh.
```

**Tier 1 (MVP):** Client iroh Endpoint → iroh public network (P2P direct or iroh-relay fallback). No custom server. "No relay infrastructure needed."

**Tiers 2-6 (Phase 6):** Client iroh Endpoint → custom sync-relay (which is also an iroh Endpoint accepting QUIC connections). The sync-relay is a Rust binary with SQLite. It speaks the 0k-sync protocol (HELLO, PUSH, PULL, etc.) over iroh connections. Deployable as a Docker container behind Cloudflare Tunnel. Identified by NodeId, discovered via DNS.

**Why this works for custom relays:** iroh Endpoints can accept incoming connections via `endpoint.accept()`. The sync-relay runs as a long-lived iroh Endpoint, accepts connections from clients, and runs the sync protocol over those connections. Same encryption (Noise handshake over QUIC), same transport (iroh), same discovery (DNS/mDNS). The only difference from Tier 1 is that clients connect to a known NodeId instead of discovering peers.

---

## Amendments to 02-SPECIFICATION.md

### S1. Data Flow Diagram (Section 2.3, ~line 108)

**Replace:**
```
         ──► Transport Encrypt (Noise) ──► WebSocket ──► Relay ──►
...
Relay ──► WebSocket ──► Transport Decrypt (Noise) ──► Envelope ──►
```

**With:**
```
         ──► Transport Encrypt (Noise) ──► iroh (QUIC) ──► Peer/Relay ──►
...
Peer/Relay ──► iroh (QUIC) ──► Transport Decrypt (Noise) ──► Envelope ──►
```

**Verification:** The data flow must reference iroh/QUIC, not WebSocket.

---

### S2. Relay Server Responsibilities (Section 7.1, ~line 628)

**Replace:**
```
| Accept WebSocket connections | Store data long-term |
```

**With:**
```
| Accept iroh connections (QUIC) | Store data long-term |
```

**Verification:** Section 7.1 must say "iroh connections" not "WebSocket connections."

---

### S3. RelayBackend Enum (Section 6.2, ~line 550)

The current enum uses `url: String` with `wss://` URLs. Replace with iroh-native addressing.

**Replace the entire `RelayBackend` enum with:**

```rust
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

**Rationale:** NodeId is iroh's native addressing — a 32-byte public key that uniquely identifies an Endpoint. DNS discovery maps a hostname to a NodeId. The `relay_url` field is for the iroh-relay (QUIC datagram relay), not a WebSocket URL.

**Verification:** The enum must use `NodeId`, not `url: String`. No `wss://` URLs.

---

### S4. Tier Configuration Examples (Section 11, ~lines 1101–1170)

**Replace all tier config examples:**

**Tier 1 (unchanged — already correct):**
```rust
SyncConfig {
    backend: RelayBackend::Iroh,
    ..Default::default()
}
```

**Tier 2 — replace `wss://sync.home.local` with:**
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

**Tier 3 — replace `wss://my-relay.fly.dev` with:**
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

**Tier 4-5 (rename variant from ManagedCloud — fix space-in-enum):**
```rust
SyncConfig {
    backend: RelayBackend::ManagedCloud {
        api_key: "cn_live_xxxx".into(),
    },
    ..Default::default()
}
```

API key authenticates to CrabNebula managed infrastructure. Discovery service resolves API key to a sync-relay NodeId.

**Tier 6 — replace `wss://sync.corp.internal` with:**
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

**Verification:** All tier configs must use NodeId addressing. Zero `wss://` URLs in the entire section.

---

### S5. Pairing Protocol Examples (Sections 8.2–8.4, ~lines 765, 783, 796)

The pairing invite format currently includes `"wss://sync.example.com"` as the relay field, and Section 8.3 step 4 stores the "relay URL."

**In Section 8.2 (Create Sync Group), replace the invite payload relay field:**
```
     relay: "wss://sync.example.com",
```
**With:**
```
     relay_node: "sync-relay-node-id",  // NodeId of sync-relay, or omitted for Tier 1
```

**In Section 8.3 (Join Sync Group), step 4, replace:**
```
   - relay URL
```
**With:**
```
   - relay NodeId (if present — Tier 1 uses iroh public network, no relay NodeId needed)
```

**In Section 8.4 (QR Code Format), replace:**
```json
  "r": "wss://sync.example.com",
```
**With:**
```json
  "r": "sync-relay-node-id-or-discovery-url",
```

Add a note: "For Tier 1 (iroh public network), the `r` field is omitted — peers discover each other via the public iroh relay network. For Tiers 2-6, `r` contains the sync-relay's NodeId or an HTTPS discovery URL (e.g., `https://sync.example.com/.well-known/iroh`) that resolves to a NodeId."

**Verification:** No `wss://` URLs in Sections 8.2–8.4. "relay URL" replaced with "relay NodeId."

---

### S6. Mobile Lifecycle Section (Section 10.1, ~line 983)

**Replace:**
```
> ⚠️ **Critical Reality:** On iOS and modern Android, WebSocket connections 
> are killed within ~30 seconds of the app being backgrounded. You cannot 
> rely on persistent connections for sync.
```

**With:**
```
> ⚠️ **Critical Reality:** On iOS and modern Android, background network 
> connections (including QUIC/iroh) are killed within ~30 seconds of the 
> app being backgrounded. You cannot rely on persistent connections for sync.
```

**Replace line 986:**
```
1. Establish a WebSocket connection
```

**With:**
```
1. Establish an iroh connection
```

**Replace line 1092:**
```
| Background sync | iOS/Android kill background WebSockets | Push notifications |
```

**With:**
```
| Background sync | iOS/Android kill background connections | Push notifications |
```

**Verification:** Zero instances of "WebSocket" in Section 10.

---

### S7. Configuration Sections (Section 13, ~lines 1248–1260)

**In Section 13.2 (Client Configuration), replace:**
```toml
[sync]
backend = "relay"
relay_url = "wss://sync.example.com"
```

**With:**
```toml
[sync]
backend = "sync-relay"
relay_node_id = "your-sync-relay-node-id"
# or for DNS-based discovery:
# relay_discovery = "https://sync.example.com/.well-known/iroh"
```

**In Section 13.3 (Environment Variables), replace:**
```
| `SYNC_RELAY_URL` | Override relay URL | Config file |
```

**With:**
```
| `SYNC_RELAY_NODE_ID` | Override relay NodeId | Config file |
```

**Verification:** No `wss://` or `RELAY_URL` in Section 13. Uses NodeId addressing.

---

### S8. Appendix A Dependencies — sync-client (~line 1865)

**Remove this line entirely:**
```toml
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
```

sync-client uses iroh for transport. iroh handles QUIC, TLS, everything. No WebSocket client needed.

**Verification:** `tokio-tungstenite` must not appear in sync-client dependencies.

---

### S9. Appendix A Dependencies — sync-relay (~line 1895)

**Replace:**
```toml
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
```

**With:**
```toml
iroh = "1.0"                     # Endpoint for accepting client connections (QUIC)
```

The sync-relay is an iroh Endpoint. It accepts QUIC connections from clients, not WebSocket connections. `axum` stays — it serves health/metrics HTTP endpoints, not WebSocket upgrades.

**Verification:** sync-relay dependencies must include `iroh` and must not include `tokio-tungstenite`.

---

### S10. Update Spec Version

Update header to `Version: 2.3.0` and date to `2026-02-03`. Update footer to match.

Add changelog entry:
```
v2.3.0 (2026-02-03): Removed WebSocket transport from all tiers. All connections now use
iroh QUIC. sync-relay (Phase 6) redesigned as iroh Endpoint instead of WebSocket server.
Removed tokio-tungstenite dependency. Updated RelayBackend enum to use NodeId addressing.
Fixed Managed Cloud enum variant (space in name). Updated data flow diagram, tier configs,
pairing format, mobile lifecycle section, CLI config, and dependency lists.
```

**Verification:** Header and footer both read `2.3.0`.

---

## Amendments to 03-IMPLEMENTATION-PLAN.md

### I1. Project Structure — Remove websocket.rs (Section 2.1, ~line 118–124)

**Replace:**
```
│       ├── connection.rs         # WebSocket/iroh transport
│       └── transport/
│           ├── mod.rs
│           ├── iroh.rs           # Tier 1
│           └── websocket.rs      # Tiers 2-6
```

**With:**
```
│       ├── connection.rs         # iroh transport management
│       └── transport/
│           ├── mod.rs
│           └── iroh.rs           # All tiers (QUIC via iroh)
```

`websocket.rs` is gone. The transport abstraction trait stays in `mod.rs` (good design for testing with mocks), but the only production implementation is `IrohTransport`.

**Verification:** No `websocket.rs` in the project structure.

---

### I2. Workspace Dependencies — Remove tokio-tungstenite (Section 2.2, ~line 196 area)

Check whether `tokio-tungstenite` appears in the workspace dependencies. If it does, remove it. It should not be a workspace dependency.

**Verification:** `tokio-tungstenite` must not appear in the workspace Cargo.toml.

---

### I3. Transport Abstraction — Remove WebSocketTransport (Section 6.2, ~line 900–916)

**Replace:**
```rust
// Tier 1: iroh
pub struct IrohTransport { /* ... */ }

// Tiers 2-6: WebSocket
pub struct WebSocketTransport { /* ... */ }
```

**With:**
```rust
// All tiers: iroh (QUIC)
pub struct IrohTransport { /* ... */ }

// For testing: in-process mock transport
#[cfg(test)]
pub struct MockTransport { /* ... */ }
```

The `Transport` trait itself stays — it is the right abstraction for testing (mock transports, chaos injection). But the only production implementation is `IrohTransport`. For Tier 1, it connects via the iroh public relay network. For Tiers 2-6 (Phase 6), it connects to a known sync-relay NodeId via iroh. Same transport, different discovery target.

**Verification:** No `WebSocketTransport` in the codebase. `IrohTransport` is the only production transport.

---

### I3.5. Phase 2 Invite Tests — Replace wss:// URLs (~lines 792, 807, 833)

The `Invite` struct tests use `"wss://relay.example.com"` as the relay URL parameter. With iroh addressing, invites carry a NodeId (or nothing, for Tier 1).

**Replace all three occurrences of:**
```rust
"wss://relay.example.com",
```

**With:**
```rust
test_relay_node_id(),  // Returns a deterministic NodeId for testing
```

Also update the `Invite::create` function signature and the assertion on line 800 to use `relay_node_id` instead of `relay_url`:

```rust
assert_eq!(invite.relay_node_id, decoded.relay_node_id);
```

This aligns the Invite struct with the `RelayBackend` enum change (S3). The invite encodes a NodeId for the sync-relay (Tiers 2-6) or omits the relay field entirely (Tier 1).

**Verification:** `grep "wss://" 03-IMPLEMENTATION-PLAN.md` returns zero results.

---

### I4. Phase 3 Checkpoint — Remove WebSocket mention (~line 1226)

**Replace:**
```
- Transport abstraction (iroh 1.0 RC, WebSocket)
```

**With:**
```
- Transport abstraction (iroh 1.0 RC)
```

Also update the Phase 3 commit message to remove any WebSocket reference.

**Verification:** Phase 3 checkpoint mentions iroh only, not WebSocket.

---

### I5. Phase 6 Implementation Outline — Replace WebSocket Server (~line 1713–1741)

**Replace the Phase 6 implementation outline with:**

```rust
// sync-relay/src/main.rs

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;

    // SQLite for temporary buffer
    let db = Database::connect(&config.database).await?;

    // iroh Endpoint — accepts QUIC connections from clients
    let endpoint = Endpoint::builder()
        .discovery(config.discovery()?)
        .relay_mode(config.relay_mode()?)
        .secret_key(config.secret_key()?)
        .bind()
        .await?;

    let relay = SyncRelay::new(config, db, endpoint);

    // Health/metrics endpoints (HTTP only — no WebSocket upgrade)
    let api = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/.well-known/iroh", get(node_info));  // NodeId discovery

    // Run both
    tokio::select! {
        _ = relay.accept_connections() => {},
        _ = axum::serve(api_listener, api) => {},
    }

    Ok(())
}
```

Key differences from the old outline:
- `Endpoint::builder()` replaces WebSocket server setup
- `relay.accept_connections()` loops on `endpoint.accept()` to accept incoming iroh connections
- The HTTP server (axum) is ONLY for health/metrics/discovery — not for WebSocket upgrades
- `/.well-known/iroh` endpoint publishes the relay's NodeId for DNS-based discovery

**Verification:** Phase 6 outline must use `Endpoint`, not WebSocket. `axum` must serve health/metrics only.

---

### I6. Update Implementation Plan Version

Increment from `2.2.0` to `2.3.0`. Update date to `2026-02-03`.

Add changelog entry:
```
v2.3.0 (2026-02-03): Removed WebSocket transport. All tiers use iroh QUIC. Removed
websocket.rs from project structure, WebSocketTransport from Phase 3, tokio-tungstenite
from dependencies. Phase 6 sync-relay redesigned as iroh Endpoint.
```

**Verification:** Version reads `2.3.0` in header and footer.

---

## Amendments to 04-RESEARCH-VALIDATION.md

### R1. tokio-tungstenite Section (Section 1.5, ~line 214)

**Do NOT delete this section** — it contains valid research. Instead, add a status marker:

**Replace:**
```
### 1.5 tokio-tungstenite (WebSocket)

**Choice:** [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) for WebSocket transport

**Status:** ✅ Validated
```

**With:**
```
### 1.5 tokio-tungstenite (WebSocket)

**Choice:** [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) for WebSocket transport

**Status:** ⏸️ Deferred — Not used in current architecture

> **Amendment (2026-02-03):** The transport architecture was simplified to iroh QUIC 
> for all tiers (see 02-SPECIFICATION.md v2.3.0). WebSocket is no longer part of the 
> transport stack. tokio-tungstenite research is retained here for reference if a 
> WebSocket transport adapter is ever needed (e.g., for environments that block QUIC), 
> but it is not a current dependency.
```

**Verification:** Section 1.5 exists but is marked as deferred, not validated.

---

### R2. Architecture Diagram (Section 5.3, ~line 467)

**Replace:**
```
   │  Tauri    │──WebSocket─▶│  Cloudflare   │────────────▶│   Relay      │
```

**With:**
```
   │  Tauri    │──iroh/QUIC──▶│  Cloudflare   │────────────▶│   Relay      │
```

**Verification:** Diagram must show iroh/QUIC, not WebSocket.

---

### R3. WebSocket Memory Row (Section 3, ~line 335)

**Replace:**
```
| WebSocket < 10KB/conn | ✅ **ACHIEVED** | 8-10KB with 4KB buffers configured |
```

**With:**
```
| WebSocket < 10KB/conn | ⏸️ **DEFERRED** | Research retained; WebSocket not in current architecture |
```

**Verification:** WebSocket benchmark row is marked deferred, not achieved.

---

### R4. Dependency Table (Section 7, ~line 591)

**Remove or mark as deferred** the `tokio-tungstenite` row from the dependency matrix.

**Verification:** tokio-tungstenite is not listed as an active dependency.

---

### R5. Remaining WebSocket Mentions (lines 440, 456, 531) — No Change Needed

Three WebSocket mentions in the research doc describe other products or comparison criteria, not 0k-sync's architecture:
- Line 440: Comparison table — "WebSocket support" as a feature across competing solutions
- Line 456: Risk note about WebSocket quirks in other implementations  
- Line 531: Supabase Realtime described as "Postgres + WebSocket"

These are factual descriptions of the competitive landscape and should remain unchanged.

---

### R6. Update Research Doc Version

Increment version and add amendment note:
```
v2.1.0 (2026-02-03): Marked tokio-tungstenite/WebSocket research as deferred per
transport architecture simplification to iroh QUIC (all tiers).
```

**Verification:** Version updated with changelog entry.

---

## Summary of What Gets Removed

| Item | Document | Action |
|------|----------|--------|
| `websocket.rs` | Impl Plan (project structure) | **Remove** |
| `WebSocketTransport` | Impl Plan (Phase 3 transport) | **Remove** |
| `tokio-tungstenite` | Spec (sync-client deps) | **Remove** |
| `tokio-tungstenite` | Spec (sync-relay deps) | **Replace with `iroh`** |
| `wss://` URLs | Spec (6 occurrences) | **Replace with NodeId** |
| "Accept WebSocket connections" | Spec (relay responsibilities) | **Replace with "iroh connections"** |
| "WebSocket" in data flow | Spec (Section 2.3) | **Replace with "iroh (QUIC)"** |
| "WebSocket" in mobile section | Spec (Section 10) | **Replace with "connections"** |
| WebSocket server outline | Impl Plan (Phase 6) | **Replace with iroh Endpoint** |
| tokio-tungstenite validated | Research doc | **Mark deferred** |

## Summary of What Gets Added

| Item | Document | Purpose |
|------|----------|---------|
| `iroh::NodeId` addressing | Spec (RelayBackend enum) | Native iroh peer addressing |
| `iroh = "1.0"` | Spec (sync-relay deps) | Relay is an iroh Endpoint |
| `MockTransport` | Impl Plan (Phase 3) | Testing without real network |
| `/.well-known/iroh` | Impl Plan (Phase 6) | NodeId discovery endpoint |
| Deferred note on tungstenite | Research doc | Preserve research, mark unused |

---

## What This Fixes Beyond WebSocket

This amendment also fixes the `Managed Cloud` enum variant with a space in the name (already flagged in the pre-flight audit as issue #5). The corrected enum uses `ManagedCloud` — valid Rust.

---

## Execution Order

1. **S1–S10** — Spec amendments (transport architecture is the source of truth)
2. **I1–I6** — Impl plan amendments (follows from spec changes)
3. **R1–R5** — Research doc amendments (marks research as deferred, preserves it)

---

## Verification Checklist (for Q)

After all amendments are applied:

- [ ] `grep -i "websocket" 02-SPECIFICATION.md` returns zero results
- [ ] `grep -i "tungstenite" 02-SPECIFICATION.md` returns zero results
- [ ] `grep "wss://" 02-SPECIFICATION.md` returns zero results
- [ ] `grep -i "websocket" 03-IMPLEMENTATION-PLAN.md` returns zero results
- [ ] `grep -i "tungstenite" 03-IMPLEMENTATION-PLAN.md` returns zero results
- [ ] `grep "wss://" 03-IMPLEMENTATION-PLAN.md` returns zero results
- [ ] `RelayBackend` enum uses `NodeId`, not `url: String`
- [ ] `ManagedCloud` has no space (fixes pre-flight audit issue #5)
- [ ] Data flow diagram (Section 2.3) says "iroh (QUIC)" not "WebSocket"
- [ ] Relay responsibilities (Section 7.1) says "iroh connections" not "WebSocket"
- [ ] Mobile lifecycle (Section 10.1) says "connections" not "WebSocket"
- [ ] Phase 3 project structure has no `websocket.rs`
- [ ] Phase 3 transport has `IrohTransport` + `MockTransport`, no `WebSocketTransport`
- [ ] Phase 6 outline uses `Endpoint::builder()`, not WebSocket server
- [ ] sync-client dependencies: iroh present, tokio-tungstenite absent
- [ ] sync-relay dependencies: iroh present, tokio-tungstenite absent
- [ ] Research doc Section 1.5 marked "⏸️ Deferred"
- [ ] All three documents have updated version numbers

---

## Why Not Keep WebSocket as a Future Option in the Codebase?

Because it adds surface area Q has to maintain, test, and reason about when it provides zero value for any current or planned deployment. The Transport trait abstraction means WebSocket can be added later as a new `impl Transport` if a legitimate use case emerges (browser clients, QUIC-hostile corporate networks). The research is preserved in the validation doc. But the codebase should not contain dead transport implementations.

The principle: build what you need, not what you might need. The Transport trait is the escape hatch. Use it when the need is real.

---

*Document: WEBSOCKET-REMOVAL-AMENDMENT.md | Version: 1.0.0 | Date: 2026-02-03*
