# 0k-Sync - Executive Summary

**Version:** 2.1.0
**Date:** 2026-02-02
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

### The Local-First Ecosystem Gap

Local-first frameworks (Tauri, Electron, React Native, Flutter) have matured for building desktop and mobile applications. Developers can build, package, and distribute apps easily.

**Missing piece:** Sync. Local-first developers have no turnkey, zero-knowledge solution for multi-device synchronization that doesn't require trusting a third party with their users' data.

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
| **sync-client** | Rust library for E2E encryption, pairing, cursor tracking |
| **sync-content** | Large file transfer via iroh-blobs (encrypt-then-hash) |
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
| **Security Audit** | ⏳ In Progress | Audit complete (2026-02-05). Remediation in progress. Noise Protocol (clatter) planned but not yet implemented. |
| **Enterprise Compliance** | ⚠️ Blocked | "FIPS Mode" fallback using AES-GCM/PBKDF2 for regulated markets |
| **Infrastructure** | ✅ Ready | Cloudflare Tunnel validated; self-hosted iroh-relay option |

### Validated Technology Choices

| Component | Choice | Version | Validation |
|-----------|--------|---------|------------|
| P2P networking | [iroh](https://github.com/n0-computer/iroh) | **0.96** | 200K+ connections, stable API |
| Transport encryption | iroh QUIC (TLS 1.3) | **0.96** | Wire encryption via iroh |
| **Planned:** Noise Protocol | [clatter](https://github.com/jmwample/clatter) | **2.2** | Hybrid ML-KEM-768 + X25519 (not yet implemented) |
| Large content | [iroh-blobs](https://github.com/n0-computer/iroh-blobs) | **0.98** | BLAKE3/Bao verified streaming |
| Blob encryption | XChaCha20-Poly1305 | RustCrypto | 192-bit nonces, no coordination needed |
| Key derivation | Argon2id | RustCrypto | Device-adaptive parameters |
| Transport | [iroh](https://github.com/n0-computer/iroh) | **0.96** | QUIC P2P + relay fallback |
| Storage | SQLite + WAL | via sqlx | 70K+ writes/sec |

**iroh Version Strategy:**
- Production: iroh 0.96 (requires cargo patch for curve25519-dalek)
- Content transfer: iroh-blobs 0.98 for large files
- Discovery: mDNS (LAN), DNS, optional DHT

> **⚠️ Dependency Note:** iroh 0.96 requires a cargo patch for curve25519-dalek 5.0.0-pre.1.
> See workspace Cargo.toml `[patch.crates-io]` section. PR #878 submitted upstream.

---

## 4. Architecture Overview

### Protocol Stack

```
┌─────────────────────────────────────────┐
│  Layer 4: Application Sync Logic        │  Push, Pull, Ack, Presence
├─────────────────────────────────────────┤
│  Layer 3: Content Transfer              │  iroh-blobs (large files), encrypt-then-hash
├─────────────────────────────────────────┤
│  Layer 2: Sync Protocol (0k-Sync)       │  Envelope, routing, cursor
├─────────────────────────────────────────┤
│  Layer 1: Transport Security            │  iroh QUIC (TLS 1.3) — Noise XX planned, not yet implemented
├─────────────────────────────────────────┤
│  Layer 0: Transport                     │  iroh (QUIC), mDNS, DHT discovery
└─────────────────────────────────────────┘
```

### Cryptographic Primitives

| Function | Algorithm | Notes |
|----------|-----------|-------|
| Cipher | XChaCha20-Poly1305 | 192-bit nonce (not 96-bit) |
| Transport | iroh QUIC (TLS 1.3) | Wire encryption |
| KDF | Argon2id | Device-adaptive: 19-64 MiB based on RAM (OWASP minimum: 19 MiB) |

**Why XChaCha20 (not standard ChaCha20)?**
- 192-bit nonces eliminate collision risk (safe threshold: 2^80 vs 2^32)
- Random nonce generation safe without cross-device coordination
- Negligible performance overhead (one HChaCha20 block)

### Security Model

| Property | How Achieved |
|----------|--------------|
| **Zero-knowledge relay** | E2E encryption with Group Key; relay sees only ciphertext |
| **No accounts** | Devices pair via QR code or short code; no email/password |
| **Forward secrecy** | iroh QUIC TLS (transport level). Noise Protocol XX planned for application-level forward secrecy. |
| **Replay protection** | Monotonic cursors + nonces |

### Mobile Architecture: Wake-on-Push

Mobile platforms kill background network connections within ~30 seconds of backgrounding. Solution:

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
| **4. Community Sync** | Managed Cloud shared | Indie developer |
| **5. Cloud** | Managed Cloud dedicated | Funded startup |
| **6. Enterprise** | Customer infrastructure | Regulated industry |

**Developer Experience (All Tiers):**

```rust
// One line to add sync
use sync_client::SyncClient;

let client = SyncClient::new(SyncConfig::default()).await?;
client.push(encrypted_blob).await?;
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
└── integrations   → Framework bindings (optional)
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
| integrations | Framework-specific tests |

### Critical Implementation Requirements

From research validation:

1. **Thundering Herd Mitigation** — Client-side exponential backoff with jitter on reconnect
2. **Device-Adaptive Argon2** — 12 MiB (low-end) to 64 MiB (desktop) based on available RAM
3. **XChaCha20 Nonces** — 192-bit random nonces, never 96-bit
4. **Noise Protocol (planned)** — Hybrid clatter with ML-KEM-768 + X25519 designed but not yet implemented
5. **iroh 0.96** — Pre-1.0 but stable, requires cargo patch (see Cargo.toml)

---

## 7. Risk Summary

| Risk | Severity | Mitigation |
|------|----------|------------|
| Post-quantum transition | Medium | Hybrid Noise (clatter ML-KEM-768) designed but not yet implemented |
| FIPS compliance gap | Critical (for Gov/Finance) | Feature flag for AES-GCM/PBKDF2 build |
| iroh ecosystem maturity | Low | Using stable 0.96; self-hosted option available |
| Mobile battery impact | Medium | Wake-on-Push architecture; quantify in beta |
| Thundering herd | Medium | Client-side jitter required |

---

## 8. Summary

| Question | Answer |
|----------|--------|
| What is it? | Zero-knowledge sync protocol for local-first apps |
| Who is it for? | Local-first developers (any framework) |
| Why build it? | Fills the sync gap in local-first ecosystem |
| How does it scale? | Client constant, relay tier changes |
| What's validated? | iroh 0.96 (E2E tested), XChaCha20-Poly1305, Argon2id. Noise Protocol (clatter) not yet implemented. |
| What's blocked? | FIPS compliance (enterprise only) |

**0k-Sync completes local-first apps: Build → Store Locally → Sync Securely.**

---

## References

- [iroh by n0-computer](https://github.com/n0-computer/iroh) — P2P networking (0.96)
- [iroh-blobs](https://github.com/n0-computer/iroh-blobs) — Content-addressed storage (0.98)
- [clatter crate](https://github.com/jmwample/clatter) — Hybrid Noise Protocol (planned, not yet implemented)
- [Noise Protocol](https://noiseprotocol.org/noise.html) — Encryption framework (planned)
- [04-RESEARCH-VALIDATION.md](./04-RESEARCH-VALIDATION.md) — Full technology validation

---

*Document: 01-EXECUTIVE-SUMMARY.md | Version: 2.2.0 | Date: 2026-02-02*
