# CrabNebula Sync - Executive Summary

**Version:** 1.0.0
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
| **Build custom** | 6-12 months of networking, encryption, conflict resolution |
| **Skip sync** | Users stuck on single device, competitive disadvantage |

### The Tauri Ecosystem Gap

[Tauri](https://tauri.app) has emerged as the leading framework for building lightweight, secure desktop and mobile applications. CrabNebula already provides **distribution** (CDN, updates) via CrabNebula Cloud.

**Missing piece:** Sync. Tauri developers have no turnkey solution for multi-device synchronization.

---

## 2. Solution: CrabNebula Sync

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

- ❌ **Data storage** — Relay is pass-through only, not a database
- ❌ **Hole punching** — All connections outbound to relay (simpler)
- ❌ **User accounts** — Zero-knowledge pairing via QR/codes
- ❌ **Proprietary dependencies** — 100% open source stack

---

## 3. Product Tiers

Six tiers serve the full market, from hobbyist to enterprise:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│  Tier 1: Vibe Coder          Free, iroh public network, zero setup     │
│  Tier 2: Home Developer      Self-hosted container, your hardware       │
│  Tier 3: Vercel-style        Deploy to PaaS, developer pays platform   │
│  ──────────────────────────────────────────────────────────────────    │
│  Tier 4: Community Sync      CrabNebula hosted, free/cheap with limits │
│  Tier 5: Cloud               CrabNebula dedicated, usage-based pricing │
│  Tier 6: Enterprise          Customer deploys, license + support       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
         Open Source (Tiers 1-3)              CrabNebula Revenue (Tiers 4-6)
```

### Tier Details

| Tier | Relay Infrastructure | User Cost | CrabNebula Revenue |
|------|---------------------|-----------|-------------------|
| **1. Vibe Coder** | [iroh](https://github.com/n0-computer/iroh) public network | $0 | $0 |
| **2. Home Developer** | Self-hosted Docker container | Electricity | $0 |
| **3. Vercel-style** | Container on Vercel/Railway/Fly.io | ~$5-50/mo to platform | $0 |
| **4. Community Sync** | CrabNebula shared infrastructure | Free (100MB) or $5/mo | Subscription |
| **5. Cloud** | CrabNebula dedicated per-app | Usage-based | Usage fees |
| **6. Enterprise** | Customer infrastructure + license | Annual license | License + support |

### Developer Experience (All Tiers)

```rust
// One line to add sync to any Tauri app
tauri::Builder::default()
    .plugin(tauri_plugin_sync::init())
    .run(tauri::generate_context!())
```

```typescript
// Frontend usage (any framework)
await sync.enable();
await sync.push(encryptedBlob);
const blobs = await sync.pull();
```

Changing tiers = changing one config value. No code changes.

---

## 4. Technical Architecture

### Protocol Stack

```
┌─────────────────────────────────────────┐
│  Layer 4: Application Messages          │  Push, Pull, Ack, Presence
├─────────────────────────────────────────┤
│  Layer 3: Envelope                      │  Routing, cursor, timestamp
├─────────────────────────────────────────┤
│  Layer 2: E2E Encryption                │  Group Key (ChaCha20-Poly1305)
├─────────────────────────────────────────┤
│  Layer 1: Transport Encryption          │  Noise Protocol XX (snow crate)
├─────────────────────────────────────────┤
│  Layer 0: Transport                     │  WebSocket / QUIC
└─────────────────────────────────────────┘
```

### Security Model

| Property | How Achieved |
|----------|--------------|
| **Zero-knowledge relay** | E2E encryption with Group Key; relay sees only ciphertext |
| **No accounts** | Devices pair via QR code or short code; no email/password |
| **Forward secrecy** | Noise Protocol XX handshake pattern |
| **Replay protection** | Monotonic cursors + nonces |

### Why No Hole Punching?

Direct P2P would require NAT traversal complexity. Instead:

- All devices make **outbound** WebSocket connections to relay
- NAT allows outbound connections (no hole punching needed)
- Relay has public endpoint via Cloudflare Tunnel (free tier)
- Trade-off: Slightly higher latency, dramatically simpler implementation

For Tier 1 (iroh), hole punching is handled by iroh's infrastructure—not our code.

---

## 5. Technology Choices

### 100% Open Source Stack

| Component | Choice | License | Rationale |
|-----------|--------|---------|-----------|
| Transport encryption | [snow](https://github.com/mcginty/snow) (Noise Protocol) | Apache-2.0 | Battle-tested (WireGuard uses Noise) |
| P2P networking | [iroh](https://github.com/n0-computer/iroh) | MIT | Production-proven, 200k+ concurrent connections |
| WebSocket | [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) | MIT | Mature, production-ready |
| Key derivation | [argon2](https://docs.rs/argon2) (RustCrypto) | Apache-2.0/MIT | OWASP recommended |
| Plugin framework | [Tauri 2.0](https://v2.tauri.app/develop/plugins/) | MIT/Apache-2.0 | Native mobile support |
| Public endpoint | [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/) | Free tier | DDoS protection, TLS termination |

**No vendor lock-in.** Every component is replaceable with alternatives.

---

## 6. Business Model

### CrabNebula Revenue Streams

| Tier | Model | Target Market |
|------|-------|---------------|
| **Community Sync** | Freemium (100MB free, $5/mo pro) | Indie developers |
| **Cloud** | Usage-based (per GB, per 1K connections) | Funded startups, SaaS |
| **Enterprise** | Annual license + support contract | Banks, healthcare, government |

### Competitive Positioning

| Competitor | Weakness | CrabNebula Advantage |
|------------|----------|---------------------|
| **Firebase** | Vendor lock-in, no E2E encryption | Zero-knowledge, open source |
| **Supabase** | Server-centric, requires accounts | Local-first, no accounts |
| **Custom solutions** | 6-12 month build time | One-line integration |
| **Syncthing** | File sync only, not app state | App state sync, cursor-based |

### Strategic Fit

CrabNebula Cloud already provides **distribution** for Tauri apps. Sync is the natural complement:

```
┌─────────────────────────────────────────────────────────────────┐
│                    CrabNebula Cloud                              │
│                                                                  │
│   ┌─────────────────┐              ┌─────────────────┐          │
│   │  Distribution   │              │      Sync       │          │
│   │  (existing)     │              │   (proposed)    │          │
│   │                 │              │                 │          │
│   │  • CDN          │              │  • Relay        │          │
│   │  • Updates      │              │  • E2E encrypt  │          │
│   │  • Analytics    │              │  • Multi-device │          │
│   └─────────────────┘              └─────────────────┘          │
│                                                                  │
│           Build → Distribute → Sync → Complete Tauri Platform   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 7. Implementation Approach

### Test-Driven Development

Every phase starts with tests:

| Phase | Crate | Test Strategy |
|-------|-------|---------------|
| 1 | sync-types | Serialization round-trip |
| 2 | sync-core | Pure logic, no I/O (instant tests) |
| 3 | sync-client | Two nodes syncing locally |
| 4 | sync-cli | Headless integration tests |
| 5 | tauri-plugin-sync | Import into test app |
| 6 | sync-relay | Custom relay (future) |

### MVP Timeline

**Tier 1 (iroh-based) MVP:** Weeks, not months. iroh handles the hard networking.

**Full product (Tiers 1-5):** Incremental delivery. Each tier builds on the previous.

---

## 8. Summary

| Question | Answer |
|----------|--------|
| What is it? | Zero-knowledge sync relay for Tauri apps |
| Who is it for? | Tauri developers (hobbyist to enterprise) |
| Why build it? | Fills the sync gap in local-first ecosystem |
| How does it scale? | Client constant, relay tier changes |
| What's the moat? | Deep Tauri integration, open source trust |
| How does CrabNebula profit? | Tiers 4-6: hosted infrastructure + enterprise |

**CrabNebula Sync completes the Tauri platform: Build → Distribute → Sync.**

---

## References

- [iroh by n0-computer](https://github.com/n0-computer/iroh) — P2P networking library
- [Noise Protocol](https://noiseprotocol.org/noise.html) — Encryption framework
- [Tauri 2.0 Plugins](https://v2.tauri.app/develop/plugins/) — Plugin development guide
- [CrabNebula Cloud](https://crabnebula.dev/) — Existing distribution platform

---

*Document: 01-EXECUTIVE-SUMMARY.md | Version: 1.0.0 | Date: 2026-01-16*
