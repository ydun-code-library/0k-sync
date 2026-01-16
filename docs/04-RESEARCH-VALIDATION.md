# CrabNebula Sync - Research & Validation

**Version:** 1.0.0
**Date:** 2026-01-16
**Author:** James (LTIS Investments AB)
**Status:** Base document - requires deep research

---

## Purpose

This document provides justification for technology choices, identifies areas requiring deeper research, and collects references for validation.

**Legend:**
- ✅ **Validated** — Confirmed via web search or documentation
- 🔍 **Needs Research** — Requires deeper investigation
- ⚠️ **Risk** — Potential concern to evaluate
- 📚 **Reference** — Source documentation

---

## Table of Contents

1. [Technology Choice Justifications](#1-technology-choice-justifications)
2. [Security Analysis](#2-security-analysis)
3. [Performance Considerations](#3-performance-considerations)
4. [Competitive Analysis](#4-competitive-analysis)
5. [Open Questions](#5-open-questions)
6. [References](#6-references)

---

## 1. Technology Choice Justifications

### 1.1 iroh (P2P Networking)

**Choice:** [iroh](https://github.com/n0-computer/iroh) by n0-computer for Tier 1 MVP

**Status:** ✅ Validated

**Justification:**

| Factor | Evidence |
|--------|----------|
| Production-ready | "Running in production on millions of devices" ([source](https://github.com/n0-computer/iroh)) |
| Scale | "200k+ concurrent connections" ([Lambda Class interview](https://blog.lambdaclass.com/the-wisdom-of-iroh/)) |
| Rust native | Pure Rust, same ecosystem as Tauri |
| NAT traversal | Built-in hole punching + relay fallback |
| Latest version | v0.32.0 (Feb 2025) with WASM browser support |

**Key Features Used:**
- `iroh::Endpoint` — Connection management
- `iroh-blobs` — Content-addressed blob transfer (BLAKE3)
- `iroh-gossip` — Pub/sub for real-time notifications

**🔍 Needs Research:**

1. **iroh relay reliability** — What is the actual uptime of n0's public relays?
2. **iroh vs libp2p** — Detailed comparison for our use case
3. **iroh mobile performance** — Battery/CPU impact on iOS/Android
4. **iroh WASM limitations** — What doesn't work in browser mode?

**📚 References:**
- [iroh Documentation](https://iroh.computer/docs)
- [iroh GitHub](https://github.com/n0-computer/iroh)
- [iroh 0.32.0 Release Notes](https://www.iroh.computer/blog/iroh-0-32-0-browser-alpha-qad-and-n0-future)

---

### 1.2 Noise Protocol (Transport Encryption)

**Choice:** [snow](https://github.com/mcginty/snow) crate with XX handshake pattern

**Status:** ✅ Validated

**Justification:**

| Factor | Evidence |
|--------|----------|
| Battle-tested | WireGuard uses Noise Protocol |
| Spec compliance | Tracks Noise spec revision 34 (latest) |
| Mutual auth | XX pattern: both parties prove identity |
| Forward secrecy | From message 2 onwards |
| Pure Rust | No C dependencies (optional ring backend) |

**XX Handshake Pattern:**
```
XX:
  → e
  ← e, ee, s, es
  → s, se
```

**Why XX (not IK or NK):**
- Neither party knows the other's key in advance (pairing scenario)
- Both parties prove identity (mutual authentication)
- Perfect for device pairing where keys are exchanged via QR/code

**⚠️ Risk:**
- snow has NOT received formal security audit ([GitHub README](https://github.com/mcginty/snow))

**🔍 Needs Research:**

1. **snow audit status** — Any planned audits? Alternative audited implementations?
2. **Noise vs TLS 1.3** — Why not just use TLS for everything?
3. **XX vs IK pattern** — Could we use IK after initial pairing for faster reconnects?
4. **Noise Protocol adoption** — Who else uses it beyond WireGuard?

**📚 References:**
- [Noise Protocol Specification](https://noiseprotocol.org/noise.html)
- [snow crate](https://docs.rs/snow)
- [WireGuard Protocol](https://www.wireguard.com/protocol/)

---

### 1.3 Argon2id (Key Derivation)

**Choice:** [argon2](https://docs.rs/argon2) crate (RustCrypto) with Argon2id variant

**Status:** ✅ Validated

**Justification:**

| Factor | Evidence |
|--------|----------|
| OWASP recommended | "Argon2id with minimum 19 MiB memory, two iterations" ([OWASP](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)) |
| RFC standard | RFC 9106 recommends Argon2id |
| Attack resistance | Side-channel + time-memory tradeoff resistant |
| Rust implementation | 12.1M downloads, actively maintained |

**Parameters:**
```rust
// OWASP recommended minimum
Argon2id {
    m_cost: 19456,  // 19 MiB memory
    t_cost: 2,      // 2 iterations
    p_cost: 1,      // 1 parallelism
}
```

**🔍 Needs Research:**

1. **Mobile performance** — Is 19 MiB memory reasonable on low-end phones?
2. **Parameter tuning** — Should we adjust for different device classes?
3. **Argon2 vs scrypt** — Why Argon2 over scrypt for this use case?
4. **Key stretching frequency** — How often do we derive keys?

**📚 References:**
- [RFC 9106 - Argon2](https://www.rfc-editor.org/rfc/rfc9106.html)
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [argon2 crate](https://docs.rs/argon2)

---

### 1.4 ChaCha20-Poly1305 (Blob Encryption)

**Choice:** [chacha20poly1305](https://docs.rs/chacha20poly1305) crate (RustCrypto)

**Status:** ✅ Validated

**Justification:**

| Factor | Evidence |
|--------|----------|
| IETF standard | RFC 8439 |
| Performance | Faster than AES on devices without hardware AES |
| Security | AEAD (authenticated encryption) |
| Nonce size | 96-bit (sufficient for random nonces) |

**Why not AES-GCM:**
- ChaCha20 faster in software (no AES-NI required)
- Better for mobile devices
- Same security level (256-bit key)

**🔍 Needs Research:**

1. **Nonce collision probability** — With random 96-bit nonces, what's the practical limit?
2. **XChaCha20 vs ChaCha20** — Should we use 192-bit nonces for extra safety?
3. **AEAD alternatives** — AES-GCM-SIV for nonce misuse resistance?

**📚 References:**
- [RFC 8439 - ChaCha20-Poly1305](https://www.rfc-editor.org/rfc/rfc8439.html)
- [chacha20poly1305 crate](https://docs.rs/chacha20poly1305)

---

### 1.5 tokio-tungstenite (WebSocket)

**Choice:** [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) for WebSocket transport

**Status:** ✅ Validated

**Justification:**

| Factor | Evidence |
|--------|----------|
| Production use | "Used in production for real-time communication, video conferencing" |
| Tokio integration | Native async/await, no blocking |
| TLS support | native-tls and rustls backends |
| Maturity | Most popular Rust WebSocket library |

**⚠️ Risk:**
- ~30% slower than fastest WebSocket libraries (fastwebsockets)
- Acceptable for our sync use case (not real-time gaming)

**🔍 Needs Research:**

1. **Connection limits** — Max concurrent connections per process?
2. **Memory per connection** — Overhead for 1000 connections?
3. **Reconnection handling** — Built-in or manual implementation?
4. **fastwebsockets comparison** — Is the performance difference meaningful for us?

**📚 References:**
- [tokio-tungstenite GitHub](https://github.com/snapview/tokio-tungstenite)
- [Rust WebSocket ecosystem comparison](https://websocket.org/guides/languages/rust/)

---

### 1.6 SQLite (Relay Temporary Storage)

**Choice:** SQLite with WAL mode for relay's temporary blob buffer

**Status:** ✅ Validated

**Justification:**

| Factor | Evidence |
|--------|----------|
| Simplicity | Single file, no server process |
| WAL mode | Concurrent readers with single writer |
| Performance | Sufficient for 1000s of messages/second |
| Reliability | Most deployed database in the world |

**Configuration:**
```sql
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA busy_timeout=5000;
```

**⚠️ Risk:**
- Single writer bottleneck at extreme scale
- Not suitable for multi-region deployment

**🔍 Needs Research:**

1. **SQLite connection pooling** — Best practices with sqlx?
2. **WAL checkpoint tuning** — When to run checkpoints?
3. **Blob storage alternatives** — Redis for ephemeral data?
4. **Max practical throughput** — Blobs/second on typical hardware?

**📚 References:**
- [SQLite WAL Mode](https://www.sqlite.org/wal.html)
- [sqlx crate](https://docs.rs/sqlx)

---

### 1.7 Cloudflare Tunnel (Public Endpoint)

**Choice:** [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/) for exposing self-hosted relay

**Status:** ✅ Validated

**Justification:**

| Factor | Evidence |
|--------|----------|
| Free tier | Unlimited bandwidth for personal use |
| Security | DDoS protection, automatic TLS |
| Simplicity | Outbound-only connection (no port forwarding) |
| Reliability | Cloudflare's global network |

**⚠️ Risk:**
- Cloudflare sees unencrypted WebSocket frames (but our blobs are E2E encrypted)
- Dependency on Cloudflare availability
- No UDP in free tier

**🔍 Needs Research:**

1. **Cloudflare Tunnel alternatives** — Tailscale, ngrok, self-hosted options?
2. **Cloudflare reliability** — Historical uptime?
3. **Privacy implications** — What metadata does Cloudflare see?
4. **Rate limits** — Any undocumented limits on free tier?

**📚 References:**
- [Cloudflare Tunnel Documentation](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/)
- [Cloudflare Tunnel Alternatives](https://github.com/anderspitman/awesome-tunneling)

---

### 1.8 Tauri 2.0 (Plugin Framework)

**Choice:** Tauri 2.0 plugin system for sync integration

**Status:** ✅ Validated

**Justification:**

| Factor | Evidence |
|--------|----------|
| Mobile support | Android and iOS in Tauri 2.0 |
| Plugin ecosystem | First-class plugin support |
| Swift/Kotlin bindings | Native mobile code when needed |
| CrabNebula alignment | Official Tauri partner |

**🔍 Needs Research:**

1. **Plugin state management** — Best practices for async state?
2. **Mobile plugin limitations** — What doesn't work on iOS/Android?
3. **Plugin distribution** — npm, crates.io, or both?
4. **Existing sync plugins** — Any prior art to learn from?

**📚 References:**
- [Tauri 2.0 Plugin Development](https://v2.tauri.app/develop/plugins/)
- [Tauri 2.0 Release](https://v2.tauri.app/blog/tauri-20/)

---

## 2. Security Analysis

### 2.1 Threat Model Summary

| Threat | Mitigation | Status |
|--------|------------|--------|
| Relay reads data | E2E encryption (Group Key) | ✅ Addressed |
| MITM attack | Noise XX + TLS | ✅ Addressed |
| Replay attack | Monotonic cursors + nonces | ✅ Addressed |
| Device compromise | Per-device keys, Group Key rotation | ✅ Addressed |
| Traffic analysis | PADME padding (future) | 🔍 Needs implementation |
| Brute-force pairing | Rate limiting, short expiry | ✅ Addressed |

### 2.2 Security Research Needed

**🔍 Needs Research:**

1. **Formal security analysis** — Has Noise XX been formally verified?
2. **Key rotation protocol** — Best practices for Group Key rotation?
3. **Device revocation** — How to securely remove a compromised device?
4. **Metadata leakage** — What can relay infer from traffic patterns?
5. **Quantum resistance** — Future-proofing considerations?

---

### 2.3 Cryptographic Parameters

| Parameter | Value | Justification |
|-----------|-------|---------------|
| Group Key size | 256 bits | Standard for ChaCha20 |
| Nonce size | 96 bits (ChaCha20) | RFC 8439 standard |
| Argon2id memory | 19 MiB | OWASP minimum |
| Argon2id iterations | 2 | OWASP minimum |
| Device key (Curve25519) | 256 bits | Noise Protocol standard |

**🔍 Needs Research:**

1. **Parameter security margins** — Are OWASP minimums sufficient?
2. **NIST recommendations** — Any conflicts with our choices?
3. **Compliance requirements** — FIPS, SOC2 implications?

---

## 3. Performance Considerations

### 3.1 Benchmarks Needed

**🔍 Needs Research:**

| Benchmark | Target | Why |
|-----------|--------|-----|
| Encryption throughput | > 100 MB/s | Blob encryption speed |
| Noise handshake | < 100ms | Connection setup time |
| Argon2id derivation | < 500ms | Mobile key derivation |
| SQLite write | > 1000 blobs/s | Relay buffer throughput |
| WebSocket latency | < 50ms | Round-trip time |
| iroh hole punch | < 2s | P2P connection setup |

### 3.2 Scale Limits

**🔍 Needs Research:**

| Resource | Expected Limit | How to Verify |
|----------|----------------|---------------|
| Concurrent connections | 10,000+ | Load testing |
| Blobs per second | 1,000+ | Benchmark |
| Max blob size | 1 MB | By design |
| Memory per connection | < 10 KB | Profiling |
| Disk I/O | Depends on SSD | Benchmark |

### 3.3 Mobile Considerations

**🔍 Needs Research:**

1. **Battery impact** — Power consumption of background iroh node?
2. **Memory footprint** — RAM usage on low-end devices?
3. **Cold start time** — Time to establish sync after app launch?
4. **iOS background limits** — Actual time available for sync?

---

## 4. Competitive Analysis

### 4.1 Direct Competitors

**🔍 Needs Research:**

| Competitor | Approach | Strengths | Weaknesses |
|------------|----------|-----------|------------|
| **Firebase Realtime DB** | Cloud-first | Easy setup, scale | Vendor lock-in, no E2E |
| **Supabase Realtime** | Postgres + WebSocket | Open source | Server-centric |
| **AWS AppSync** | GraphQL subscriptions | Managed | Complex, expensive |
| **PouchDB/CouchDB** | Sync protocol | Mature | Heavy, Java/Erlang |
| **Replicache** | Client-side sync | Modern design | Complex setup |
| **PowerSync** | Postgres sync | SQL-based | New, less proven |

### 4.2 P2P/Local-First Solutions

**🔍 Needs Research:**

| Solution | Approach | Relevance |
|----------|----------|-----------|
| **Syncthing** | File sync | Different use case (files, not state) |
| **IPFS/libp2p** | Content-addressed P2P | Too complex for our needs |
| **Yjs/Automerge** | CRDT libraries | Complementary (we handle transport) |
| **Any-Sync** | Full CRDT sync | Inspiration, more complex |
| **ElectricSQL** | Postgres sync | Server-dependent |

### 4.3 Competitive Positioning

**Our differentiation:**
1. Zero-knowledge (E2E encryption)
2. No accounts (QR pairing)
3. Tauri-native (first-class integration)
4. Open source (no vendor lock-in)
5. Tiered (from free to enterprise)

**🔍 Needs Research:**

1. **Feature comparison matrix** — Detailed feature-by-feature
2. **Pricing comparison** — Cost at different scales
3. **Developer experience** — Setup complexity comparison
4. **Community sentiment** — What do devs say about alternatives?

---

## 5. Open Questions

### 5.1 Architecture Questions

| Question | Impact | Priority |
|----------|--------|----------|
| Should we support multiple relay backends simultaneously? | Redundancy | Medium |
| How to handle relay failover? | Reliability | Medium |
| Should offline buffer be on-device or relay-side? | Design | High |
| How to handle large sync groups (>100 devices)? | Scale | Low |

### 5.2 Protocol Questions

| Question | Impact | Priority |
|----------|--------|----------|
| Should cursors reset on Group Key rotation? | Compatibility | Medium |
| How to handle blob size > 1MB (chunking)? | Limitations | Low |
| Should we add blob compression (LZ4)? | Efficiency | Medium |
| Delta sync vs full blob sync? | Efficiency | Medium |

### 5.3 Product Questions

| Question | Impact | Priority |
|----------|--------|----------|
| How to price CrabNebula Sync? | Business | High |
| What metrics to expose in dashboard? | Product | Medium |
| How to handle abuse/spam on free tier? | Operations | High |
| Should we offer SLA for paid tiers? | Business | Medium |

### 5.4 Ecosystem Questions

| Question | Impact | Priority |
|----------|--------|----------|
| Integration with CrabNebula Cloud distribution? | Synergy | High |
| Plugin marketplace potential? | Growth | Low |
| Community contribution model? | Sustainability | Medium |

---

## 6. References

### 6.1 Specifications & Standards

| Name | URL | Status |
|------|-----|--------|
| Noise Protocol Specification | https://noiseprotocol.org/noise.html | ✅ Referenced |
| RFC 8439 (ChaCha20-Poly1305) | https://www.rfc-editor.org/rfc/rfc8439.html | ✅ Referenced |
| RFC 9106 (Argon2) | https://www.rfc-editor.org/rfc/rfc9106.html | ✅ Referenced |
| MessagePack Specification | https://msgpack.org/index.html | 🔍 To review |
| BLAKE3 Specification | https://github.com/BLAKE3-team/BLAKE3-specs | 🔍 To review |

### 6.2 Libraries & Tools

| Name | URL | Version |
|------|-----|---------|
| iroh | https://github.com/n0-computer/iroh | 0.32.0 |
| snow | https://github.com/mcginty/snow | 0.9.x |
| tokio-tungstenite | https://github.com/snapview/tokio-tungstenite | 0.21.x |
| argon2 (RustCrypto) | https://github.com/RustCrypto/password-hashes | 0.5.x |
| chacha20poly1305 | https://github.com/RustCrypto/AEADs | 0.10.x |
| sqlx | https://github.com/launchbadge/sqlx | 0.7.x |
| Tauri | https://github.com/tauri-apps/tauri | 2.x |

### 6.3 Related Projects & Inspiration

| Name | URL | Relevance |
|------|-----|-----------|
| WireGuard | https://www.wireguard.com/ | Noise Protocol usage |
| Syncthing | https://syncthing.net/ | Sync protocol design |
| Any-Sync | https://github.com/anyproto/any-sync | CRDT sync approach |
| Matrix | https://matrix.org/ | Federation model |
| Nostr | https://nostr.com/ | Decentralized relay network |

### 6.4 Documentation & Guides

| Name | URL | Status |
|------|-----|--------|
| Tauri Plugin Development | https://v2.tauri.app/develop/plugins/ | ✅ Referenced |
| OWASP Password Storage | https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html | ✅ Referenced |
| Cloudflare Tunnel | https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/ | ✅ Referenced |
| iroh Documentation | https://iroh.computer/docs | 🔍 To deep dive |

### 6.5 Academic Papers

**🔍 Needs Research:**

| Topic | Papers to Find |
|-------|----------------|
| Noise Protocol security | Formal verification papers |
| CRDT sync protocols | Academic foundations |
| P2P NAT traversal | STUN/TURN/ICE papers |
| E2E encryption systems | Signal Protocol analysis |
| Local-first software | Ink & Switch research |

---

## 7. Research Action Items

### 7.1 Immediate (Before Implementation)

- [ ] Verify iroh mobile performance (battery, memory)
- [ ] Confirm snow security status (audits, CVEs)
- [ ] Test Argon2id on low-end mobile devices
- [ ] Benchmark ChaCha20-Poly1305 throughput
- [ ] Verify Cloudflare Tunnel free tier limits

### 7.2 During Implementation

- [ ] Measure actual WebSocket connection overhead
- [ ] Profile SQLite performance under load
- [ ] Test iroh hole punching success rate
- [ ] Benchmark Noise handshake latency
- [ ] Verify Tauri plugin mobile compatibility

### 7.3 Before Launch

- [ ] Security review of cryptographic implementation
- [ ] Load testing at expected scale
- [ ] Mobile battery impact testing
- [ ] Competitive feature comparison update
- [ ] Pricing model validation

---

## 8. Validation Checklist

**Before marking this document complete:**

- [ ] All ✅ items verified with sources
- [ ] All 🔍 items have research tasks assigned
- [ ] All ⚠️ risks have mitigation plans
- [ ] All 📚 references are accessible
- [ ] Security analysis reviewed by second person
- [ ] Performance targets are realistic
- [ ] Competitive analysis is current

---

*Document: 04-RESEARCH-VALIDATION.md | Version: 1.0.0 | Date: 2026-01-16*
*Status: Base document - requires deep research completion*
