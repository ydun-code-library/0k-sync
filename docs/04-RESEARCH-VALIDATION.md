# 0k-Sync - Research & Validation

**Version:** 2.2.0
**Date:** 2026-02-03
**Author:** James (LTIS Investments AB)
**Status:** Decision-Ready Document

---

## Purpose

This document provides justification for technology choices, validates assumptions with evidence, and documents risk mitigations.

**Legend:**
- ✅ **Validated** — Confirmed via research and documentation
- ⚠️ **Risk** — Concern requiring mitigation
- 📚 **Reference** — Source documentation

---

## Executive Summary

0k-Sync is **technically viable** for production. The stack (iroh + Noise Protocol) offers superior connectivity and throughput compared to alternatives. Three gates require attention before GA:

| Gate | Status | Action Required |
|------|--------|-----------------|
| **Security Audit** | ✅ Resolved | Using `clatter` for hybrid Noise (ML-KEM-768 + X25519) - post-quantum ready |
| **Enterprise Compliance** | ⚠️ Blocked | "FIPS Mode" fallback using AES-GCM/PBKDF2 for regulated markets |
| **Infrastructure** | ✅ Ready | Cloudflare Tunnel validated; self-hosted iroh-relay option |

**Recommendation:** Proceed with MVP development. Cloudflare free tier appropriate for personal/MVP use.

---

## Table of Contents

1. [Technology Choice Justifications](#1-technology-choice-justifications)
2. [Security Analysis](#2-security-analysis)
3. [Performance Validation](#3-performance-validation)
4. [Mobile Strategy](#4-mobile-strategy)
5. [Infrastructure Strategy](#5-infrastructure-strategy)
6. [Compliance Strategy](#6-compliance-strategy)
7. [Competitive Analysis](#7-competitive-analysis)
8. [Risk Matrix](#8-risk-matrix)
9. [References](#9-references)

---

## 1. Technology Choice Justifications

### 1.1 iroh (P2P Networking)

**Choice:** [iroh](https://github.com/n0-computer/iroh) by n0-computer for Tier 1 MVP

**Status:** ✅ Validated for Production

| Factor | Evidence | Source |
|--------|----------|--------|
| Scale | 200K+ concurrent connections | Lambda Class interview |
| Hole-punch success | ~90% (vs libp2p's 70% ± 7.1%) | n0 engineering, Dec 2022 |
| Relay fallback | 100% connectivity guarantee | Architecture design |
| Production deployment | Delta Chat 1.48 on 100K+ devices | Delta Chat blog (Nov 2024) |
| Rust native | Pure Rust, same ecosystem as Tauri | — |

**Version Strategy:** Using iroh 0.96 (latest crates.io release as of Feb 2026)

iroh 1.0 has not yet shipped. The current stable release is 0.96.

- **Production:** iroh 0.96 (requires cargo patch for curve25519-dalek)
- **Content Transfer:** iroh-blobs 0.98 for large file transfer
- **Discovery:** mDNS (LAN), DNS, optional DHT

**⚠️ Dependency Note:** iroh 0.96 pulls curve25519-dalek 5.0.0-pre.1 which has a build issue. Requires cargo patch:
```toml
[patch.crates-io]
curve25519-dalek = { git = "https://github.com/ydun-code-library/curve25519-dalek", branch = "fix/digest-import-5.0.0-pre.1" }
```
See PR #878 for upstream fix.

**Key Features Used:**
- `iroh::Endpoint` — Connection management
- `iroh-blobs` — Content-addressed blob transfer (BLAKE3)
- `iroh-gossip` — Pub/sub for real-time notifications

**📚 References:**
- [iroh Documentation](https://iroh.computer/docs)
- [iroh GitHub](https://github.com/n0-computer/iroh)
- [iroh vs libp2p comparison](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)

---

### 1.2 Hybrid Noise Protocol (Transport Encryption)

**Choice:** [clatter](https://github.com/jmwample/clatter) crate with hybrid XX handshake pattern

**Status:** ✅ Validated (Post-Quantum Ready)

| Factor | Evidence |
|--------|----------|
| Battle-tested pattern | XX from WireGuard, WhatsApp, Lightning Network |
| Hybrid post-quantum | ML-KEM-768 + X25519 (NIST PQC + classical) |
| Spec compliance | Tracks Noise spec revision 34 (latest) |
| Mutual auth | XX pattern: both parties prove identity |
| Forward secrecy | From message 2 onwards |
| Pure Rust | No C dependencies |

**Hybrid XX Handshake Pattern:**
```
Noise_XX_25519+Kyber768_ChaChaPoly_BLAKE2s:
  → e (X25519 ephemeral + ML-KEM-768 encapsulation)
  ← e, ee, s, es
  → s, se
```

**Why Hybrid (not pure classical):**
- **Future-proof:** ML-KEM-768 provides quantum resistance
- **No regression:** X25519 maintains classical security
- **Harvest-now-decrypt-later defense:** Data encrypted today stays safe
- **NIST standardized:** ML-KEM (FIPS 203) is the PQC standard

**Why XX (not IK or NK):**
- Neither party knows the other's key in advance (pairing scenario)
- Both parties prove identity (mutual authentication)
- Perfect for device pairing where keys are exchanged via QR/code

**Why clatter over snow:**

| Factor | snow | clatter |
|--------|------|---------|
| Post-quantum | ❌ X25519 only | ✅ ML-KEM-768 hybrid |
| Classical security | ✅ | ✅ (X25519 fallback) |
| Maintenance | Active | Active |
| API compatibility | — | Similar to snow |

**Decision (2026-02-02):** Migrated from `snow` to `clatter` for hybrid post-quantum support. This resolves the previous audit concern by providing defense-in-depth (if classical crypto breaks, PQC remains; if PQC has issues, classical remains).

**📚 References:**
- [Noise Protocol Specification](https://noiseprotocol.org/noise.html)
- [clatter crate](https://github.com/jmwample/clatter)
- [NIST FIPS 203 (ML-KEM)](https://csrc.nist.gov/pubs/fips/203/final)
- [WireGuard Protocol](https://www.wireguard.com/protocol/)

---

### 1.3 Argon2id (Key Derivation)

**Choice:** [argon2](https://docs.rs/argon2) crate (RustCrypto) with Argon2id variant

**Status:** ✅ Validated (with device-adaptive parameters)

| Factor | Evidence |
|--------|----------|
| OWASP recommended | "Argon2id with minimum 19 MiB memory, two iterations" |
| RFC standard | RFC 9106 recommends Argon2id |
| Attack resistance | Side-channel + time-memory tradeoff resistant |
| Rust implementation | 12.1M downloads, actively maintained |

**Device-Adaptive Parameters:**

OWASP minimum (19 MiB, 2 iterations) performs well on modern devices but hits 800ms+ on low-end mobile.

| Device Class | Detection Signal | Memory | Iterations | Target Time |
|--------------|------------------|--------|------------|-------------|
| Low-end mobile | RAM < 2GB | 12 MiB | 3 | 300-500ms |
| Mid-range mobile | RAM 2-4GB | 19 MiB | 2 | 200-400ms |
| High-end mobile | RAM > 4GB | 46 MiB | 1 | 200-400ms |
| Desktop | Always | 64 MiB | 3 | 200-500ms |

**iOS Constraint:** AutoFill extension processes have ~55 MiB usable memory. Configurations above 46 MiB fail intermittently.

```rust
fn select_argon2_params() -> Params {
    let available_ram = get_available_memory_mb();
    match available_ram {
        0..=2048 => Params::new(12 * 1024, 3, 1, None).unwrap(),    // 12 MiB
        2049..=4096 => Params::new(19 * 1024, 2, 1, None).unwrap(), // 19 MiB (OWASP)
        _ => Params::new(46 * 1024, 1, 1, None).unwrap(),           // 46 MiB
    }
}
```

**📚 References:**
- [RFC 9106 - Argon2](https://www.rfc-editor.org/rfc/rfc9106.html)
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)

---

### 1.4 XChaCha20-Poly1305 (Blob Encryption)

**Choice:** [chacha20poly1305](https://docs.rs/chacha20poly1305) crate (RustCrypto) with **XChaCha20** (192-bit nonce)

**Status:** ✅ Validated

| Factor | Evidence |
|--------|----------|
| IETF standard | RFC 8439 + extended nonce |
| Performance | 1.18-1.75 GB/s (x86_64 AVX2), 92 MB/s floor (ARM) |
| Security | AEAD (authenticated encryption) |
| Nonce safety | 192-bit eliminates collision risk |

**Why XChaCha20 (not standard ChaCha20):**

| Nonce Size | 50% Collision | Safe Threshold (2^-32) |
|------------|---------------|------------------------|
| 96-bit (standard) | 2^48 (~281T) | 2^32 (~4.3B) |
| 192-bit (XChaCha20) | 2^96 | 2^80 |

**Recommendation:** Use XChaCha20-Poly1305. Random nonce generation is safe. No cross-device coordination required. Performance overhead is negligible (one HChaCha20 block).

**Why not AES-GCM:**
- ChaCha20 faster in software (no AES-NI required)
- Better for mobile devices without hardware acceleration
- Same security level (256-bit key)

**📚 References:**
- [RFC 8439 - ChaCha20-Poly1305](https://www.rfc-editor.org/rfc/rfc8439.html)
- [chacha20poly1305 crate](https://docs.rs/chacha20poly1305)

---

### 1.5 tokio-tungstenite (WebSocket)

**Choice:** [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) for WebSocket transport

**Status:** ⏸️ Deferred — Not used in current architecture

> **Amendment (2026-02-03):** The transport architecture was simplified to iroh QUIC for all tiers (see 02-SPECIFICATION.md v2.3.0). WebSocket is no longer part of the transport stack. tokio-tungstenite research is retained here for reference if a WebSocket transport adapter is ever needed (e.g., for environments that block QUIC), but it is not a current dependency.

| Factor | Evidence |
|--------|----------|
| Scale | 120K connections, 1M msg/s (Sockudo benchmark, 4-vCPU) |
| Tokio integration | Native async/await, no blocking |
| TLS support | native-tls and rustls backends |
| Memory | 8-10KB per connection with 4KB buffers |

**⚠️ Risk: Thundering Herd** (applies to any reconnection scenario)

After relay restart, all clients reconnect simultaneously, potentially crashing database or exhausting limits.

**Required Mitigation: Client-Side Jitter** (implemented for iroh transport)

```rust
async fn reconnect_with_backoff(attempt: u32) {
    let base_delay = Duration::from_millis(100 * 2u64.pow(attempt.min(6)));
    let jitter = Duration::from_millis(rand::thread_rng().gen_range(0..5000));
    let delay = (base_delay + jitter).min(Duration::from_secs(120));
    tokio::time::sleep(delay).await;
}
```

**📚 References:**
- [tokio-tungstenite GitHub](https://github.com/snapview/tokio-tungstenite)

---

### 1.6 SQLite (Relay Temporary Storage)

**Choice:** SQLite with WAL mode for relay's temporary blob buffer

**Status:** ✅ Validated

| Factor | Evidence |
|--------|----------|
| Simplicity | Single file, no server process |
| WAL mode | Concurrent readers with single writer |
| Performance | 70,000-100,000+ writes/s with WAL mode |
| Reliability | Most deployed database in the world |

**Configuration:**
```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
PRAGMA mmap_size = 30000000000;
PRAGMA cache_size = -65536;
```

**Scale Ceiling:** SQLite handles ~50,000 messages/second with single-writer. Beyond this, migrate to PostgreSQL for horizontal scaling.

**📚 References:**
- [SQLite WAL Mode](https://www.sqlite.org/wal.html)
- [sqlx crate](https://docs.rs/sqlx)

---

### 1.7 Tauri 2.0 (Plugin Framework)

**Choice:** Tauri 2.0 plugin system for sync integration

**Status:** ✅ Validated

| Factor | Evidence |
|--------|----------|
| Mobile support | Android and iOS in Tauri 2.0 |
| Plugin ecosystem | First-class plugin support |
| Swift/Kotlin bindings | Native mobile code when needed |
| Managed Cloud alignment | Official Tauri partner |

**📚 References:**
- [Tauri 2.0 Plugin Development](https://v2.tauri.app/develop/plugins/)
- [Tauri 2.0 Release](https://v2.tauri.app/blog/tauri-20/)

---

## 2. Security Analysis

### 2.1 Threat Model Coverage

| Threat | Mitigation | Status |
|--------|------------|--------|
| Relay sees plaintext | Double encryption (Noise + Group Key) | ✅ Mitigated |
| MITM on pairing | QR code contains full key material | ✅ Mitigated |
| Replay attacks | Nonce tracking, Noise counters | ✅ Mitigated |
| Key compromise (forward secrecy) | Noise provides PFS from message 2 | ✅ Mitigated |
| Metadata leakage | Relay sees: device IDs, timestamps, blob sizes | ⚠️ Partial |
| Quantum threats | No PQ algorithms | ⚠️ Future risk |
| Brute-force pairing | Rate limiting, short expiry | ✅ Mitigated |
| Traffic analysis | PADME padding (future) | ⚠️ Not implemented |

### 2.2 Cryptographic Primitives

| Primitive | Standard | Security Level | FIPS Status |
|-----------|----------|----------------|-------------|
| XChaCha20-Poly1305 | RFC 8439 + extended nonce | 256-bit | ❌ Not approved |
| Curve25519 (X25519) | RFC 7748 | ~128-bit | ❌ Not approved |
| Argon2id | RFC 9106 | Configurable | ❌ Not approved |
| Ed25519 | FIPS 186-5 | ~128-bit | ✅ Approved (Feb 2023) |
| BLAKE3 | N/A (new) | 256-bit | ❌ Not approved |

See [Section 6: Compliance Strategy](#6-compliance-strategy) for FIPS mitigation path.

---

## 3. Performance Validation

### 3.1 Throughput Targets

| Target | Verdict | Measured Performance |
|--------|---------|---------------------|
| ChaCha20-Poly1305 > 100 MB/s | ✅ **ACHIEVED** | 1.18-1.75 GB/s (x86_64 AVX2), 92 MB/s floor (ARM) |
| Noise XX handshake < 100ms | ✅ **ACHIEVED** | <1ms crypto time; network RTT dominates |
| Argon2id < 500ms mobile | ⚠️ **MARGINAL** | 200-400ms modern devices; 800ms+ low-end |
| SQLite writes > 1000/s | ✅ **ACHIEVED** | 70,000-100,000+ writes/s with WAL mode |
| WebSocket < 10KB/conn | ⏸️ **DEFERRED** | Research retained; WebSocket not in current architecture |
| iroh hole punch | ✅ **ACHIEVED** | ~90% success rate, <2s typical |

### 3.2 Platform Support Matrix

| Platform | P2P Direct | Relay | iroh-blobs | iroh-gossip |
|----------|------------|-------|------------|-------------|
| Desktop (Native) | ✅ | ✅ | ✅ | ✅ |
| iOS/Android | ✅ | ✅ | ✅ | ✅ |
| Browser (WASM) | ❌ | ✅ | 🚧 Porting | 🚧 Porting |

**Browser Limitation:** Browsers cannot send UDP, so hole-punching unavailable. All traffic routes through relays while maintaining E2E encryption.

---

## 4. Mobile Strategy

### 4.1 Mobile Viability Assessment

| Aspect | iOS | Android | Risk Level |
|--------|-----|---------|------------|
| iroh-ffi bindings | ✅ Swift via UniFFI | ✅ Kotlin via UniFFI | Low |
| Background execution | ⚠️ Limited (30s max) | ⚠️ Doze mode restrictions | Medium |
| Battery impact | ❓ Requires validation | ❓ Requires validation | High |
| Production precedent | ✅ Delta Chat | ✅ Delta Chat | Low |

### 4.2 Wake-on-Push Architecture

**Problem:** Maintaining persistent P2P connections on mobile drains battery and violates OS power management policies.

**Solution:** Hybrid Push + P2P Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                 MOBILE LIFECYCLE STATE MACHINE               │
└─────────────────────────────────────────────────────────────┘

                    ┌──────────────────┐
                    │   APP LAUNCH     │
                    └────────┬─────────┘
                             │
                             ▼
               ┌─────────────────────────┐
               │     FOREGROUND MODE     │
               │  • iroh endpoint ACTIVE │
               │  • P2P connections OPEN │
               │  • Real-time sync ON    │
               └────────────┬────────────┘
                            │
               ┌────────────┴────────────┐
               │    APP BACKGROUNDED     │
               └────────────┬────────────┘
                            │
                            ▼
               ┌─────────────────────────┐
               │   GRACE PERIOD (10s)    │
               │  • Flush pending syncs  │
               │  • Upload cursor state  │
               └────────────┬────────────┘
                            │
                            ▼
               ┌─────────────────────────┐
               │    BACKGROUND MODE      │
               │  • iroh endpoint CLOSED │
               │  • Push token ACTIVE    │
               │  • Zero network usage   │
               └────────────┬────────────┘
                            │
         ┌──────────────────┼──────────────────┐
         │                  │                  │
         ▼                  ▼                  ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│  USER RETURNS   │ │  PUSH RECEIVED  │ │ SCHEDULED SYNC  │
│  (App opened)   │ │  (APNS/FCM)     │ │ (BGAppRefresh)  │
└────────┬────────┘ └────────┬────────┘ └────────┬────────┘
         │                   │                   │
         └───────────────────┴───────────────────┘
                            │
                            ▼
               ┌─────────────────────────┐
               │     FOREGROUND MODE     │
               │       (Full sync)       │
               └─────────────────────────┘
```

### 4.3 Battery Impact Mitigation

| Strategy | Implementation | Expected Impact |
|----------|----------------|-----------------|
| Aggressive socket teardown | Close all connections on background | -90% background drain |
| Silent push for wake | Use content-available push | ~0.5% per wake event |
| Batched sync windows | Combine with BGAppRefreshTask | -50% vs continuous |
| Adaptive sync frequency | Reduce polling in low-battery mode | Variable |

**Validation Required:** Instrument with MetricKit (iOS) and Battery Historian (Android) during beta.

---

## 5. Infrastructure Strategy

### 5.1 Relay Hosting Comparison

| Factor | Cloudflare Tunnel | Fly.io | Self-Hosted (VPS) |
|--------|-------------------|--------|-------------------|
| **Free tier** | ✅ Generous (1000 tunnels) | ⚠️ $5 credit, then pay | ❌ ~$5-20/mo minimum |
| **WebSocket support** | ✅ Native | ✅ Native | ✅ Full control |
| **Global edge** | ✅ 330+ cities | ✅ 30+ regions | ❌ Single region |
| **DDoS protection** | ✅ Enterprise-grade | ⚠️ Basic | ❌ DIY |
| **Latency** | ⚠️ Variable | ✅ Excellent | ✅ Predictable |
| **Pricing predictability** | ✅ Flat tiers | ⚠️ Usage-based | ✅ Fixed monthly |

### 5.2 Cloudflare Strengths and Weaknesses

**Strengths:**
- Unmatched free tier (1,000 tunnels, no bandwidth caps)
- Global network (#1 in 48% of top networks by TCP connection time)
- Zero infrastructure (just `cloudflared` daemon)
- DDoS mitigation (405 Tbps capacity)

**Weaknesses:**
- Latency variability (some ISPs route suboptimally)
- WebSocket quirks (community reports of disconnection issues)
- No SLA on free/Pro tier
- Centralized dependency

### 5.3 Recommended Architecture

**Hybrid Approach:** Cloudflare as edge/DDoS layer, Fly.io (or self-hosted) as compute layer.

```
    User Device              Cloudflare Edge              Fly.io Compute
   ┌───────────┐            ┌───────────────┐            ┌───────────────┐
   │  Tauri    │──iroh/QUIC──▶│  Cloudflare   │────────────▶│   Relay      │
   │   App     │            │    Proxy      │  Tunnel or  │   Server     │
   │           │◀────────────│  (DDoS prot)  │◀────────────│  (Rust app)  │
   └───────────┘            └───────────────┘             └───────────────┘
```

### 5.4 Decision Matrix

| Use Case | Recommended Infrastructure | Rationale |
|----------|---------------------------|-----------|
| Personal project / vibe coding | Cloudflare Tunnel alone | Simplest, free |
| MVP / early startup | Cloudflare Tunnel alone | Free tier covers needs |
| Production with SLA needs | Cloudflare + Fly.io | Reliability + DDoS |
| Enterprise / regulated | Cloudflare Enterprise + dedicated | SLA, compliance |

---

## 6. Compliance Strategy

### 6.1 FIPS Compliance Gap

**Current Stack:** Not FIPS 140-2/3 compliant.

| Algorithm | Current | FIPS Alternative | Performance Delta |
|-----------|---------|------------------|-------------------|
| Key Exchange | X25519 | ECDH P-256 | ~2x slower |
| Symmetric Encryption | ChaCha20-Poly1305 | AES-256-GCM | Faster with AES-NI |
| Key Derivation | Argon2id | PBKDF2-HMAC-SHA256 | ~10x faster (less secure) |
| Signatures | Ed25519 | Ed25519 | Same (FIPS approved) |

### 6.2 Market Impact

| Market Segment | FIPS Required | Revenue Impact |
|----------------|---------------|----------------|
| Indie/Startup developers | No | $0 |
| SMB SaaS | Rarely | Low |
| Enterprise (general) | Sometimes | Medium |
| U.S. Federal Government | **Yes** | Gate |
| Healthcare (HIPAA) | Often required | Medium-High |
| Financial services | Often required | Medium-High |

### 6.3 Mitigation: FIPS Mode Build Flag

**Recommendation:** Implement compile-time feature flag for "Enterprise Build" with FIPS-approved algorithms.

```rust
// Cargo.toml
[features]
default = ["modern-crypto"]
modern-crypto = ["chacha20poly1305", "x25519-dalek", "argon2"]
fips-mode = ["aes-gcm", "p256", "pbkdf2"]
```

**Timeline:** Implement after MVP, before enterprise sales motion.

---

## 7. Competitive Analysis

### 7.1 Direct Competitors

| Competitor | Approach | Strengths | Weaknesses |
|------------|----------|-----------|------------|
| **Firebase Realtime DB** | Cloud-first | Easy setup, scale | Vendor lock-in, no E2E |
| **Supabase Realtime** | Postgres + WebSocket | Open source | Server-centric |
| **AWS AppSync** | GraphQL subscriptions | Managed | Complex, expensive |
| **PouchDB/CouchDB** | Sync protocol | Mature | Heavy, Java/Erlang |
| **Replicache** | Client-side sync | Modern design | Complex setup |
| **PowerSync** | Postgres sync | SQL-based | New, less proven |

### 7.2 P2P/Local-First Solutions

| Solution | Approach | Relevance |
|----------|----------|-----------|
| **Syncthing** | File sync | Different use case (files, not state) |
| **IPFS/libp2p** | Content-addressed P2P | Too complex for our needs |
| **Yjs/Automerge** | CRDT libraries | Complementary (we handle transport) |
| **Any-Sync** | Full CRDT sync | Inspiration, more complex |
| **ElectricSQL** | Postgres sync | Server-dependent |

### 7.3 Our Differentiation

1. **Zero-knowledge** (E2E encryption)
2. **No accounts** (QR pairing)
3. **Tauri-native** (first-class integration)
4. **Open source** (no vendor lock-in)
5. **Tiered** (from free to enterprise)

---

## 8. Risk Matrix

| Risk Area | Severity | Probability | Mitigation Strategy | Timeline |
|-----------|----------|-------------|---------------------|----------|
| **Post-Quantum Transition** | Low | Low | Using clatter with ML-KEM-768 hybrid (future-proof) | ✅ Resolved |
| **Regulatory** (FIPS gap) | Critical | 100% (in Gov) | Develop "Enterprise Build" with AES/PBKDF2 | Before Enterprise |
| **Infrastructure** (self-hosted) | Low | Low | iroh-relay and iroh-dns-server available | ✅ Resolved |
| **Mobile Battery** | Medium | High | Implement Wake-on-Push architecture | MVP |
| **Mobile Performance** (Argon2id) | Medium | High | Dynamic parameter tuning | MVP |
| **API Stability** (iroh) | Low | Medium | Using iroh 0.96 (pre-1.0, minor API changes possible) | ⚠️ Monitoring |
| **Dependency** (curve25519-dalek) | Low | Low | Cargo patch for build fix (PR #878 upstream) | ✅ Resolved |
| **Thundering Herd** | Medium | Medium | Client-side exponential backoff with jitter | MVP |
| **Relay SPOF** | High | Low | Deploy redundant relays | Beta |

---

## 9. References

### 9.1 Specifications & Standards

| Name | URL |
|------|-----|
| Noise Protocol Specification | https://noiseprotocol.org/noise.html |
| RFC 8439 (ChaCha20-Poly1305) | https://www.rfc-editor.org/rfc/rfc8439.html |
| RFC 9106 (Argon2) | https://www.rfc-editor.org/rfc/rfc9106.html |
| RFC 7748 (X25519) | https://www.rfc-editor.org/rfc/rfc7748.html |
| MessagePack Specification | https://msgpack.org/index.html |

### 9.2 Libraries & Tools

| Name | URL | Version |
|------|-----|---------|
| iroh | https://github.com/n0-computer/iroh | **0.96** (requires cargo patch) |
| iroh-blobs | https://github.com/n0-computer/iroh-blobs | **0.98** |
| clatter | https://github.com/jmwample/clatter | 2.2 |
| tokio-tungstenite | https://github.com/snapview/tokio-tungstenite | ⏸️ Deferred |
| argon2 (RustCrypto) | https://github.com/RustCrypto/password-hashes | 0.5.x |
| chacha20poly1305 | https://github.com/RustCrypto/AEADs | 0.10.x |
| sqlx | https://github.com/launchbadge/sqlx | 0.7.x |

### 9.3 Security Notes

| Note | Details |
|------|---------|
| Hybrid Noise | clatter provides ML-KEM-768 + X25519 (post-quantum + classical) |
| snow migration | Migrated from snow to clatter (2026-02-02) for PQC support |
| iroh stability | Using iroh 0.96 (latest crates.io); 1.0 not yet released |
| curve25519-dalek | Requires cargo patch for digest 0.11 compatibility (PR #878) |

### 9.4 Related Projects & Inspiration

| Name | URL | Relevance |
|------|-----|-----------|
| WireGuard | https://www.wireguard.com/ | Noise Protocol usage |
| Syncthing | https://syncthing.net/ | Sync protocol design |
| Any-Sync | https://github.com/anyproto/any-sync | CRDT sync approach |
| Delta Chat | https://delta.chat/ | iroh production usage |

### 9.5 Academic Papers

| Paper | Topic | URL |
|-------|-------|-----|
| Noise*: Verified High-Performance Protocols | Formal verification | https://eprint.iacr.org/2022/607.pdf |
| Analyzing the Noise Protocol Framework | Security analysis | https://www.iacr.org/archive/pkc2020/12110122/12110122.pdf |

---

## Appendix A: Validation Checklist

**Before MVP Release:**

- [x] iroh pinned to stable version (1.0 RC)
- [x] clatter for hybrid Noise (ML-KEM-768 + X25519)
- [ ] XChaCha20-Poly1305 implemented (not standard ChaCha20)
- [ ] Device-adaptive Argon2id parameters implemented
- [ ] Client-side reconnection jitter implemented
- [ ] Mobile lifecycle handlers implemented (iOS + Android)
- [ ] Push notification integration complete

**Before Beta Exit:**

- [ ] Security audit strategy executed (HACL* swap or limited audit)
- [ ] Cloudflare Pro subscription active OR self-hosted relay deployed
- [ ] Load testing at 10x expected Beta users
- [ ] Battery impact quantified on target devices

**Before GA:**

- [ ] FIPS Mode feature flag implemented (if enterprise target)
- [ ] Redundant relay infrastructure deployed
- [ ] 99.9% uptime demonstrated over 30 days
- [ ] Security audit report published

---

---

## Changelog

**v2.2.0 (2026-02-03):** Marked tokio-tungstenite/WebSocket research as deferred per transport architecture simplification to iroh QUIC (all tiers). Updated architecture diagram to show iroh/QUIC.

**v2.1.0 (2026-02-02):** Updated iroh to 1.0 RC stable. Migrated from snow to clatter for hybrid Noise (ML-KEM-768 + X25519).

---

*Document: 04-RESEARCH-VALIDATION.md | Version: 2.2.0 | Date: 2026-02-03*
*Status: Decision-Ready | Next Review: Before Beta Exit*
