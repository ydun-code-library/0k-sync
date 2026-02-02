# 0k-Sync - Executive Summary

**Version:** 2.0.0
**Date:** 2026-01-16
**Author:** James (LTIS Investments AB)
**Audience:** Technical Executives, Architects, Decision Makers

---

## 1. Problem Statement

### The Sync Gap in Local-First Applications

Modern applications increasingly adopt local-first architecture—data lives on the device, works offline, and syncs opportunistically. This approach delivers superior user experience: instant responsiveness, offline capability, and data ownership.

However, **sync is hard**. Developers face a painful choice:

| Option | Problem |
|--------|---------|
| **Firebase/Supabase** | Vendor lock-in, data leaves device unencrypted, recurring costs |
| **Build custom** | Months of networking, encryption, conflict resolution work |
| **Skip sync** | Users stuck on single device, competitive disadvantage |

### The Tauri Ecosystem Gap

[Tauri](https://tauri.app) has emerged as the leading framework for building lightweight, secure desktop and mobile applications. CrabNebula already provides **distribution** (CDN, updates) via CrabNebula Cloud.

**Missing piece:** Sync. Tauri developers have no turnkey solution for multi-device synchronization.

---

## 2. Solution: 0k-Sync

### Core Architecture

A **zero-knowledge relay** that passes encrypted blobs between devices. The relay never sees plaintext data—it's a dumb pipe.

```
Device A                     RELAY                      Device B
    │                          │                           │
    │── encrypted blob ───────►│                           │
    │                          │── encrypted blob ────────►│
    │                          │                           │
    │                          │◄───── ACK ────────────────│
    │                          │   (blob deleted)          │
```

**Key insight:** The client library stays constant. Only the relay tier changes.

### What We're Building

| Component | Purpose |
|-----------|---------|
| **tauri-plugin-sync** | Drop-in Tauri plugin—one line to add sync |
| **sync-client** | Rust library for E2E encryption, pairing, cursor tracking |
| **sync-relay** | Stateless message router with temporary buffering |

### What We're NOT Building

- **Data storage** — Relay is pass-through only, not a database
- **Conflict resolution** — CRDTs are app responsibility (we transport blobs)
- **User accounts** — Zero-knowledge pairing via QR/codes
- **Proprietary dependencies** — 100% open source stack

---

## 3. Technical Validation Status

### Production Readiness Gates

Three gates must be addressed before GA release:

| Gate | Status | Action Required |
|------|--------|-----------------|
| **Security Audit** | ⚠️ Blocked | `snow` crate requires targeted code review OR swap to HACL* verified bindings |
| **Enterprise Compliance** | ⚠️ Blocked | "FIPS Mode" fallback using AES-GCM/PBKDF2 for regulated markets |
| **Infrastructure** | ✅ Ready | Cloudflare Tunnel validated; Fly.io hybrid for production SLA |

### Validated Technology Choices

| Component | Choice | Version | Validation |
|-----------|--------|---------|------------|
| P2P networking | [iroh](https://github.com/n0-computer/iroh) | **Pin v0.35.x** | 200K+ connections, Delta Chat production |
| Transport encryption | [snow](https://github.com/mcginty/snow) (Noise XX) | **v0.9.7+** | Security advisories fixed |
| Blob encryption | XChaCha20-Poly1305 | RustCrypto | 192-bit nonces, no coordination needed |
| Key derivation | Argon2id | RustCrypto | Device-adaptive parameters |
| WebSocket | tokio-tungstenite | 0.21.x | 120K connections benchmarked |
| Storage | SQLite + WAL | via sqlx | 70K+ writes/sec |

**iroh Version Strategy:**
- Current stable: v0.35.x (pin for production)
- Canary series: v0.90+ (breaking changes, track for direction)
- Migration: Plan upgrade sprint when 1.0 RC ships

---

## 4. Architecture Overview

### Protocol Stack

```
┌─────────────────────────────────────────┐
│  Layer 4: Application Messages          │  Push, Pull, Ack, Presence
├─────────────────────────────────────────┤
│  Layer 3: Envelope                      │  Routing, cursor, timestamp
├─────────────────────────────────────────┤
│  Layer 2: E2E Encryption                │  XChaCha20-Poly1305 (Group Key)
├─────────────────────────────────────────┤
│  Layer 1: Transport Encryption          │  Noise Protocol XX (snow v0.9.7+)
├─────────────────────────────────────────┤
│  Layer 0: Transport                     │  WebSocket / QUIC (iroh)
└─────────────────────────────────────────┘
```

### Cryptographic Primitives

| Function | Algorithm | Notes |
|----------|-----------|-------|
| DH | Curve25519 | Via Noise Protocol |
| Cipher | XChaCha20-Poly1305 | 192-bit nonce (not 96-bit) |
| Hash | BLAKE2s | Noise Protocol |
| KDF | Argon2id | Device-adaptive: 12-64 MiB based on RAM |

**Why XChaCha20 (not standard ChaCha20)?**
- 192-bit nonces eliminate collision risk (safe threshold: 2^80 vs 2^32)
- Random nonce generation safe without cross-device coordination
- Negligible performance overhead (one HChaCha20 block)

### Security Model

| Property | How Achieved |
|----------|--------------|
| **Zero-knowledge relay** | E2E encryption with Group Key; relay sees only ciphertext |
| **No accounts** | Devices pair via QR code or short code; no email/password |
| **Forward secrecy** | Noise Protocol XX handshake pattern |
| **Replay protection** | Monotonic cursors + nonces |

### Mobile Architecture: Wake-on-Push

Mobile platforms kill WebSocket connections within ~30 seconds of backgrounding. Solution:

```
FOREGROUND MODE           BACKGROUND MODE           WAKE EVENT
─────────────────         ──────────────────        ───────────
• iroh endpoint ACTIVE    • All sockets CLOSED      • Push notification
• P2P connections OPEN    • Push token ACTIVE       • BGAppRefreshTask
• Real-time sync ON       • Zero network usage      • User returns
```

**Key principle:** Never block on app close. Fire-and-forget flush with 500ms timeout. Stranded commits sync on next launch.

---

## 5. Product Tiers

Six tiers serve the full market:

| Tier | Infrastructure | Target User |
|------|----------------|-------------|
| **1. Vibe Coder** | iroh public network | Hobbyist, zero setup |
| **2. Home Developer** | Self-hosted Docker | Privacy-focused |
| **3. Vercel-style** | Container on PaaS | Startup on budget |
| **4. Community Sync** | CrabNebula shared | Indie developer |
| **5. Cloud** | CrabNebula dedicated | Funded startup |
| **6. Enterprise** | Customer infrastructure | Regulated industry |

**Developer Experience (All Tiers):**

```rust
// One line to add sync
tauri::Builder::default()
    .plugin(tauri_plugin_sync::init())
    .run(tauri::generate_context!())
```

Changing tiers = changing one config value. No code changes.

---

## 6. Implementation Approach

### Crate Structure

```
sync-types     → Wire format (Envelope, Messages)
    ↓
sync-core      → Pure logic (state machine, no I/O)
    ↓
sync-client    → Library (crypto, transport)
    ↓
├── sync-cli       → Testing tool (headless E2E)
└── tauri-plugin   → Tauri integration
    ↓
sync-relay     → Custom relay (future, Tiers 2-6)
```

### Test-Driven Development

| Crate | Test Strategy |
|-------|---------------|
| sync-types | Serialization round-trip |
| sync-core | Pure logic, instant tests (no I/O) |
| sync-client | Crypto verification, mock transport |
| sync-cli | E2E headless scripts |
| tauri-plugin | Tauri mock runtime |

### Critical Implementation Requirements

From research validation:

1. **Thundering Herd Mitigation** — Client-side exponential backoff with jitter on reconnect
2. **Device-Adaptive Argon2** — 12 MiB (low-end) to 64 MiB (desktop) based on available RAM
3. **XChaCha20 Nonces** — 192-bit random nonces, never 96-bit
4. **snow v0.9.7+** — RUSTSEC-2024-0011 and RUSTSEC-2024-0347 fixed
5. **iroh v0.35.x** — Pin until 1.0 RC; v0.90+ is unstable canary

---

## 7. Risk Summary

| Risk | Severity | Mitigation |
|------|----------|------------|
| snow unaudited | High | HACL* swap or limited scope audit before GA |
| FIPS compliance gap | Critical (for Gov/Finance) | Feature flag for AES-GCM/PBKDF2 build |
| iroh API instability | Medium | Pin v0.35.x; plan 1.0 migration sprint |
| Mobile battery impact | Medium | Wake-on-Push architecture; quantify in beta |
| Thundering herd | Medium | Client-side jitter required |

---

## 8. Summary

| Question | Answer |
|----------|--------|
| What is it? | Zero-knowledge sync relay for Tauri apps |
| Who is it for? | Tauri developers (hobbyist to enterprise) |
| Why build it? | Fills the sync gap in local-first ecosystem |
| How does it scale? | Client constant, relay tier changes |
| What's validated? | iroh, XChaCha20, Argon2id, Noise XX |
| What's blocked? | Security audit, FIPS compliance |

**0k-Sync completes the Tauri platform: Build → Distribute → Sync.**

---

## References

- [iroh by n0-computer](https://github.com/n0-computer/iroh) — P2P networking (v0.35.x)
- [snow crate](https://github.com/mcginty/snow) — Noise Protocol (v0.9.7+)
- [Noise Protocol](https://noiseprotocol.org/noise.html) — Encryption framework
- [Tauri 2.0 Plugins](https://v2.tauri.app/develop/plugins/) — Plugin development
- [04-RESEARCH-VALIDATION.md](./04-RESEARCH-VALIDATION.md) — Full technology validation

---

*Document: 01-EXECUTIVE-SUMMARY.md | Version: 2.0.0 | Date: 2026-01-16*
