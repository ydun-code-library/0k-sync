# Research Report: iroh Ecosystem Deep Dive & relay-sync Architecture Validation

**Date:** 2026-02-01
**Author:** James / Claude (Research Pair)
**Status:** DECISION-GRADE — Informs foundation-level spec changes before implementation
**Scope:** Full iroh ecosystem audit, overlap analysis, integration recommendations

---

## Reality Check (2026-02-03)

> **This section added post-implementation to document actual versions used.**

The recommendations in this document remain valid, but version numbers have evolved:

| Recommendation | Actual Implementation |
|---------------|----------------------|
| iroh ~0.95, approaching 1.0 RC | **iroh 0.96** — pre-1.0, stable API |
| iroh-blobs ~0.97 | **iroh-blobs 0.98** — verified streaming works |
| "approaching 1.0 RC" | 1.0 RC not yet released as of 2026-02-03 |

**Cargo Patch Required:** iroh 0.96 requires a cargo patch for `curve25519-dalek 5.0.0-pre.1`:
```toml
[patch.crates-io]
curve25519-dalek = { git = "https://github.com/ydun-code-library/curve25519-dalek", branch = "fix/digest-import-5.0.0-pre.1" }
```

**E2E Validation Completed:** Cross-device sync tested successfully (Mac Mini → Beast server) using iroh QUIC transport with the versions above.

The original analysis below remains accurate regarding architecture, security model, and integration approach. Only version numbers needed adjustment.

---

## 1. Executive Summary

After a thorough audit of iroh's entire ecosystem — core networking, blobs, gossip, docs, discovery, relay infrastructure, security model, and 1.0 roadmap — the conclusion is nuanced:

**We are NOT reinventing the wheel with our custom sync protocol.** Our wire protocol solves a fundamentally different problem (zero-knowledge relay-buffered sync with post-quantum crypto and device lifecycle management) that no iroh protocol addresses.

**We ARE missing an opportunity** by not leveraging iroh-blobs for large content transfer, and potentially iroh-gossip for real-time peer notification when devices are online simultaneously.

**There is a critical security divergence:** iroh explicitly does NOT support post-quantum cryptography. Their FAQ states this is a deliberate tradeoff for usability. This makes our hybrid crypto layer (Appendix B) not just a nice-to-have — it's the differentiator that justifies the custom protocol stack for enterprise and defense markets.

**Foundation-level changes required:** The spec needs a new layer for large content transfer (photos, documents, voice memos) using iroh-blobs, and the transport architecture should be restructured to fully leverage iroh's connectivity stack while keeping our protocol as a clean layer on top.

**Dependency sovereignty: CONFIRMED SAFE.** iroh is dual-licensed MIT/Apache 2.0 with every infrastructure component (relay, DNS server) fully open source and self-hostable. With our planned self-hosted infrastructure (own iroh-relay, own iroh-dns-server, mDNS, DHT fallback), n0 disappearing would have zero runtime impact. The entire stack — from transport relay to DNS discovery — runs on infrastructure we control, with air-gap capability for defense/enterprise deployments.

---

## 2. iroh Ecosystem Inventory

### 2.1 What iroh Actually Is (as of v0.95, approaching 1.0 RC)

iroh is a **peer-to-peer QUIC connectivity library**, not a sync framework. The core value proposition: dial any device by a 32-byte Ed25519 public key (EndpointId), get an encrypted QUIC connection, regardless of NATs, firewalls, or network topology.

**Core crate (`iroh`):**
- QUIC connections via Quinn
- Automatic hole punching (UDP, ICE-like)
- Relay server fallback (WebSocket-tunneled QUIC datagrams)
- Connection migration (WiFi → cellular seamless)
- ALPN-based protocol routing (like HTTP path routing, but at connection level)

**Discovery mechanisms:**
- DNS Discovery (default): Custom DNS server resolving EndpointIds → relay URLs
- Pkarr: Signed DNS records published to relay servers
- mDNS Local Discovery: LAN-only, zero infrastructure, uses swarm-discovery crate
- BitTorrent DHT: Distributed, no central dependency (optional feature flag)

**Key architecture fact:** iroh endpoints maintain exactly ONE TCP connection to a "home relay." This relay forwards encrypted datagrams when direct connections fail. The relay CANNOT decrypt traffic — it only sees EndpointIds and forwards opaque bytes.

### 2.2 Protocol Ecosystem

| Crate | Purpose | Maturity | Our Interest |
|-------|---------|----------|--------------|
| `iroh` | QUIC connectivity, relays, discovery | Pre-1.0 RC (0.95) | **Already using** — transport layer |
| `iroh-blobs` | BLAKE3 content-addressed blob transfer | Pre-1.0 (0.97) | **HIGH — large content transfer** |
| `iroh-gossip` | Topic-based pub/sub broadcast (HyParView + PlumTree) | Stable | **MEDIUM — real-time notification** |
| `iroh-docs` | CRDT key-value store (range-based set reconciliation) | Not 1.0 ready | **LOW — different trust model** |
| `iroh-automerge` | Automerge document sync | Experimental | **NONE — wrong abstraction** |
| `iroh-willow` | Willow protocol implementation | In construction | **WATCH — future interest** |
| `iroh-relay` | Relay server implementation | Production | **Already planning to use** |

### 2.3 iroh-blobs Deep Dive

This is the most relevant protocol we're NOT currently using.

**What it provides:**
- Content-addressed storage: every blob identified by 32-byte BLAKE3 hash
- Verified streaming: integrity checked every 16 KiB chunk via Bao outboards
- Resumable downloads: bitfield tracking of which chunks are present
- Range requests: fetch specific byte ranges of large blobs
- HashSeq: sequence of hashes for chunked/multi-part content
- Provider/Requester model: any endpoint can be both
- Tag-based garbage collection: persistent tags protect blobs from GC
- FsStore for persistent storage, MemStore for in-memory
- Downloader component: coordinates multi-source downloads

**What it does NOT provide:**
- Encryption (blobs are plaintext to anyone who has the hash)
- Access control (anyone with the hash can request the blob)
- Ordering or sequencing
- Offline buffering
- Group semantics
- Push notifications

### 2.4 iroh-gossip Deep Dive

Topic-based broadcast using epidemic broadcast trees.

**What it provides:**
- Topic-scoped swarms (32-byte TopicId)
- Broadcast to all peers in a topic
- Self-optimizing message routing (eager/lazy sets, PlumTree)
- ~5 active connections per topic (efficient)
- Auto-recovery from node failures (HyParView membership)
- State machine design (proto module has no I/O)

**What it does NOT provide:**
- Message persistence (fire-and-forget broadcast)
- Offline delivery
- Ordering guarantees
- Encryption beyond transport
- Device management

### 2.5 iroh-docs Deep Dive

Multi-writer CRDT key-value store.

**What it provides:**
- Namespace-scoped replicas (NamespaceId = Ed25519 public key)
- Author-signed entries (AuthorId = Ed25519 public key)
- Range-based set reconciliation for efficient sync
- Entries: (namespace, author, key) → (BLAKE3 hash, size, timestamp)
- Content stored in iroh-blobs (docs is a "meta protocol" on top of blobs + gossip)
- Persistent storage via redb

**Why it's NOT a replacement for our protocol:**
1. **Not zero-knowledge.** Entry metadata (keys, authors, timestamps) is visible to any peer with namespace access. The relay sees everything.
2. **Timestamp-based conflict resolution** (LWW), not cursor-ordered.
3. **No device pairing/revocation** as protocol operations.
4. **No TTL or ephemeral buffering.** Data persists until explicitly deleted.
5. **No push notification hooks.**
6. **Peer-to-peer topology**, not client-relay hub-spoke.
7. **No post-quantum crypto.**

### 2.6 Security Model & Post-Quantum Gap

**iroh's cryptography:**
- Ed25519 for endpoint identity (signing)
- X25519 / P-256 for ECDH key exchange
- TLS 1.3 via QUIC (Quinn)
- Self-signed certificates with EndpointId verification

**From iroh's FAQ (verbatim finding):** They explicitly state they do NOT support post-quantum cryptography. They acknowledge the 37× key size increase of Xyber, impact on UDP packet sizes, DNS discovery fragmentation, and EndpointId length. Their position: serving existing use cases efficiently today is more important than quantum resistance.

**Our position:** This is exactly the gap our Appendix B (Hybrid Cryptographic Compliance) fills. For financial data (CashTable, Vault Ledger), defense/enterprise contexts (ZK-Vault), and health data (PrivateHealth), the "harvest now, decrypt later" threat model makes hybrid PQ compliance from day one a hard requirement. iroh's transport encryption is fine for the wire, but our E2E layer must be hybrid-compliant independently.

### 2.7 iroh 1.0 Timeline & Version Strategy

**Current state:** v0.95 released, v0.96 expected as last canary (adding multipath). 1.0.0-rc.0 expected late 2025 / early 2026.

**Scope of 1.0:** Core networking only (Endpoint, connections, relays, discovery). iroh-blobs and iroh-gossip will follow with their own 1.0 tracks. iroh-docs is explicitly NOT 1.0 ready.

**Our version strategy (validated):** Pin to 0.35.x stable for production was the prior recommendation. Given that 1.0-rc.0 is imminent (the issue tracker from Dec 2025 shows it at rc.0), we should now target the RC series directly. The breaking changes between 0.35 and 0.9x are substantial (Node → Endpoint rename, new error types, new Watcher API, metrics changes) but well-documented. Better to eat the migration cost once into the RC than build on 0.35 and migrate later.

**Updated recommendation:** Target `iroh 1.0.0-rc.x` (or the imminent stable 1.0) for our transport layer from the start.

---

## 3. Overlap Analysis: What We're Building vs. What Exists

### 3.1 Things We Are NOT Reinventing

| Capability | Our Spec | iroh Equivalent | Verdict |
|-----------|----------|----------------|---------|
| QUIC transport | Use iroh | `iroh::Endpoint` | ✅ Already decided |
| Hole punching | Use iroh | Built into Endpoint | ✅ Already decided |
| Relay fallback | Use iroh relays | `iroh-relay` | ✅ Already decided |
| DNS discovery | Use iroh | `discovery::dns` | ✅ Already decided |
| LAN discovery | Not in spec | `discovery::mdns` | ⚠️ **Free optimization we should add** |
| Large file transfer | Not in spec | `iroh-blobs` | ❌ **Gap — must add** |

### 3.2 Things We ARE Building That Don't Exist in iroh

| Capability | Our Protocol | iroh Status | Verdict |
|-----------|-------------|-------------|---------|
| Zero-knowledge relay buffering | Cursor-ordered encrypted blob mailbox | No equivalent | ✅ Novel, necessary |
| Post-quantum hybrid crypto | clatter + ML-KEM-768 from day one | Explicitly not supported | ✅ Key differentiator |
| Device pairing protocol | QR/code exchange, key derivation | Not addressed | ✅ Novel |
| Device revocation | Protocol-level REVOKE_DEVICE/DEVICE_REVOKED | Not addressed | ✅ Novel |
| Cursor-based ordering | Relay-assigned monotonic u64, gap-free | iroh-docs uses timestamps | ✅ Better for our model |
| TTL auto-cleanup | Per-blob TTL on relay | iroh-blobs uses tag-based GC | ✅ Different lifecycle |
| Push notification hooks | REGISTER_PUSH/UNREGISTER_PUSH | Not addressed | ✅ Mobile-essential |
| Sync group management | HELLO/WELCOME handshake | iroh-docs namespaces (different model) | ✅ Different trust model |
| E2E encryption with group keys | XChaCha20-Poly1305 + Argon2id | Transport only (TLS) | ✅ Zero-knowledge requirement |

### 3.3 Things We Should Adopt Instead of Building

| Capability | What We'd Build | What iroh Provides | Recommendation |
|-----------|----------------|-------------------|----------------|
| Photo/file transfer | Custom chunked upload in sync protocol | iroh-blobs: verified streaming, resumable, range requests | **Adopt iroh-blobs** |
| Real-time "new data" notification | Our NOTIFY (0x31) message | iroh-gossip: topic broadcast | **Consider for v1.1** (our NOTIFY is simpler and works for v1.0) |
| Local network optimization | Not addressed | iroh mDNS: same-LAN direct sync | **Adopt from day one** — zero cost, big UX win |

---

## 4. Foundation-Level Spec Change: Large Content Transfer Layer

### 4.1 The Problem

Our sync protocol is designed for small encrypted blobs (app state, JSON entries, ~100 KB sweet spot). The moment a Private Suite app needs to sync a photo (2-10 MB), a voice memo (1-5 MB), a document scan (0.5-3 MB), or a PDF attachment, we're pushing megabytes through a protocol optimized for kilobytes.

### 4.2 Affected Applications

| App | Content Type | Size Range | Frequency |
|-----|------------|------------|-----------|
| Innermost | Journal photos, voice memos | 1-15 MB | High (daily entries) |
| PrivateHealth | Meal photos, progress pics, medical doc scans | 1-10 MB | High |
| Vault Ledger | Receipt scans, invoice PDFs | 0.5-5 MB | Medium |
| Stash | Saved article images, PDF attachments | 0.5-20 MB | Medium |
| Circle | Contact photos | 50-500 KB | Low (small enough for sync relay) |
| KeyVault | None | — | None |

At least 4 of 6 apps need a large content transfer mechanism.

### 4.3 Proposed Architecture: Encrypt-Then-Hash with iroh-blobs

**The composition model:**

1. **App layer** creates content (e.g., journal photo)
2. **Encryption layer** encrypts content with the sync group's content key → produces ciphertext
3. **iroh-blobs layer** hashes ciphertext with BLAKE3, stores locally → produces `ContentHash`
4. **Sync protocol layer** sends a small metadata blob through the existing cursor-ordered channel:

```
ContentReference {
    blob_id: UUID,            // client-generated, our protocol's ID
    content_hash: [u8; 32],   // BLAKE3 hash of CIPHERTEXT (iroh-blobs address)
    encryption_nonce: [u8; 24], // XChaCha20-Poly1305 nonce
    content_size: u64,        // original plaintext size
    encrypted_size: u64,      // ciphertext size
    mime_type: String,        // "image/jpeg", "audio/opus", "application/pdf"
    thumbnail_hash: Option<[u8; 32]>, // optional small preview (also encrypted)
}
```

5. **Receiving device** gets the ContentReference through normal sync, then fetches the actual encrypted blob via iroh-blobs using the `content_hash`
6. **Receiving device** decrypts locally with the group content key + nonce

**Key properties of this design:**

- **iroh-blobs sees only ciphertext.** The BLAKE3 hash is of encrypted bytes. No metadata leaks.
- **Resumable.** If phone sleeps mid-download, iroh-blobs resumes from the last verified chunk.
- **Verified.** Every 16 KiB chunk is integrity-checked during streaming.
- **Decoupled lifecycle.** The sync relay handles the small metadata reference (with TTL). The large blob is transferred P2P between devices, or buffered in iroh-blobs' local store.
- **No relay load.** Large files never touch the sync relay. They travel device-to-device via iroh's direct QUIC connection (or via iroh relay as fallback, but that's iroh's problem, not ours).

### 4.4 Content Key Derivation

The group already has `GroupSecret` for sync blob encryption. For large content, derive a separate key to allow independent rotation:

```
content_key = HKDF-SHA256(
    ikm = GroupSecret,
    salt = "private-suite-content-v1",
    info = blob_id || "content-encryption"
) → 32 bytes for XChaCha20-Poly1305
```

Same key for all devices in the group. Same rotation lifecycle as the group key.

### 4.5 Garbage Collection Strategy

- **Provider side:** Device keeps the encrypted blob in iroh-blobs FsStore, tagged with a persistent tag linked to the ContentReference blob_id
- **Requester side:** After successful download + decrypt + local storage, the iroh-blobs tag can be dropped (blob becomes eligible for GC)
- **Provider cleanup:** When all devices in the group have ACKed the ContentReference (via normal sync protocol DELETE flow), the provider drops its tag → blob gets GC'd
- **Orphan protection:** If ContentReference is deleted from sync before all devices fetch, the provider keeps the tag for a configurable grace period (e.g., 30 days)

### 4.6 Mobile Considerations

- **Background fetch:** On iOS/Android, the app can register for background fetch to download content references, but the actual iroh-blobs download should be deferred to WiFi / charging
- **Thumbnail-first:** The optional `thumbnail_hash` in ContentReference allows showing a preview immediately (small enough to include in the sync blob or as a tiny iroh-blobs fetch)
- **Progressive disclosure:** Show "Photo attached" → show thumbnail → download full resolution on demand
- **Storage quota:** Each app manages its own content storage budget. Old content can be evicted from iroh-blobs and re-fetched on demand (the ContentReference persists in the sync log)

### 4.7 Offline / Asynchronous Transfer

**Challenge:** iroh-blobs requires the provider to be online when the requester wants to fetch. With our user's personal devices, both devices might rarely be online simultaneously.

**Solutions, in priority order:**

1. **mDNS direct transfer:** When both devices are on the same LAN (most common scenario — same household WiFi), iroh discovers them via mDNS and transfers directly. This is the happy path for most photo/file sync.

2. **iroh relay passthrough:** When devices are on different networks but both online, iroh's relay forwards the encrypted blobs. This is transparent to our code.

3. **Scheduled sync windows:** The app can maintain a "pending downloads" queue and attempt fetches periodically. iroh-blobs handles resumption natively.

4. **Future: relay-hosted blob cache (v2.0+):** If we eventually run our own infrastructure, the relay could optionally cache encrypted blobs for a TTL period, acting as both sync relay AND blob cache. This is a deployment optimization, not a protocol change.

---

## 5. Secondary Recommendations

### 5.1 mDNS Local Discovery (Add to v1.0)

**Cost:** Near zero. Add `discovery-local-network` feature flag to iroh dependency, configure MdnsDiscovery on the endpoint.

**Benefit:** When two devices are on the same WiFi, they discover each other without DNS, relay, or internet access. Sync happens at LAN speed via direct QUIC. This is particularly valuable for the initial device pairing flow (both devices likely on same network) and for the "home sync" scenario where laptop and phone are on the same WiFi.

**Integration:** The sync protocol doesn't need to change. iroh handles discovery underneath. Our protocol runs over the QUIC connection regardless of how it was established (relay, holepunch, or mDNS direct).

### 5.2 iroh-gossip for Real-Time Notification (Defer to v1.1)

**What it could replace:** Our NOTIFY (0x31) and PRESENCE (0x30) messages are custom implementations of "hey, I have new data" and "I'm alive." iroh-gossip provides this as a topic-scoped broadcast with self-healing membership.

**Why defer:** Our NOTIFY/PRESENCE are simple, well-specified, and tightly integrated with the sync state machine. Replacing them with gossip adds a dependency (gossip needs bootstrap peers, swarm management) for minimal benefit when we only have 2-5 devices in a sync group. Gossip shines at scale (hundreds of peers) which isn't our scenario.

**Future value:** If Private Suite ever supports shared spaces (e.g., family photo sharing, team vaults), gossip becomes more interesting for multi-user notification.

### 5.3 iroh-docs (Do NOT Adopt)

Despite surface-level similarity, iroh-docs is wrong for our use case:
- Multi-writer CRDT with visible metadata ≠ zero-knowledge encrypted relay
- Range-based set reconciliation assumes both peers can read entry keys ≠ our encrypted blob model
- Timestamp-based LWW ≠ our cursor-based ordering
- Not 1.0 ready (n0 says explicitly)
- Would require wrapping every iroh-docs operation with our encryption, defeating the purpose

Our custom protocol is the right choice. iroh-docs would be useful if we were building a collaborative editor, not a privacy-preserving sync system.

---

## 6. Updated Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    Private Suite App                      │
│                  (Innermost, Stash, etc.)                │
├─────────────────────────────────────────────────────────┤
│  Layer 4: Application Sync Logic                         │
│  - App-specific merge strategies                         │
│  - Schema definitions                                    │
│  - UI sync status                                        │
├─────────────────────────────────────────────────────────┤
│  Layer 3: Content Transfer (NEW)                         │
│  - ContentReference metadata via sync protocol           │
│  - Encrypt-then-hash for large blobs                     │
│  - iroh-blobs for verified streaming transfer            │
│  - Thumbnail generation & progressive loading            │
├─────────────────────────────────────────────────────────┤
│  Layer 2: Sync Protocol (relay-sync)                     │
│  - Cursor-ordered encrypted blobs                        │
│  - Device pairing / revocation                           │
│  - Sync groups with HELLO/WELCOME                        │
│  - TTL-based relay buffering                             │
│  - Push notification hooks                               │
│  - E2E: XChaCha20-Poly1305 + Argon2id                  │
├─────────────────────────────────────────────────────────┤
│  Layer 1: Hybrid Transport Security                      │
│  - Noise XX handshake via clatter                        │
│  - Curve25519 + ML-KEM-768 hybrid KEM                   │
│  - Post-quantum compliance from day one                  │
│  - Session key establishment                             │
├─────────────────────────────────────────────────────────┤
│  Layer 0: iroh Transport                                 │
│  - QUIC via Quinn (authenticated, encrypted)             │
│  - Hole punching + relay fallback                        │
│  - Discovery: DNS + mDNS (NEW) + optional DHT           │
│  - Connection migration                                  │
│  - ALPN-based protocol routing                           │
│  ┌─────────────────────────────────────────────────┐    │
│  │ ALPNs registered:                                │    │
│  │   /private-sync/1  → sync protocol handler      │    │
│  │   /iroh-bytes/4    → iroh-blobs handler (NEW)   │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

---

## 7. Impact on Existing Specifications

### 7.1 Technical Specification v2.1.0

**Changes required:**
- Add Section: "Large Content Transfer Protocol" describing the iroh-blobs integration
- Add ContentReference message type to the wire protocol (new message type 0x70 CONTENT_REF, 0x71 CONTENT_ACK)
- Update transport section to reference iroh 1.0 RC instead of "iroh-first MVP"
- Add mDNS discovery as a transport-level feature
- Add ALPN registration table showing both our sync ALPN and iroh-blobs ALPN

**No changes required to:**
- Core wire protocol (cursor ordering, group semantics, device lifecycle)
- Encryption model (same XChaCha20-Poly1305 + Argon2id, extended for content key derivation)
- Relay server design (large content bypasses the relay entirely)

### 7.2 Implementation Plan v2.0.0

**Changes required:**
- Phase 1 (sync-types): Add ContentReference type definition
- Phase 3 (sync-client): Add iroh-blobs integration, content encryption, content key derivation
- Phase 5 (tauri-plugin-sync): Add content transfer progress events, thumbnail handling
- New sub-phase: iroh-blobs store initialization alongside sync client

**No changes required to:**
- Phase 2 (sync-core): State machine doesn't handle content transfer
- Phase 4 (sync-cli): Can be extended later for testing content transfer
- Phase 6 (sync-relay): Relay doesn't handle large content

### 7.3 Appendix B: Hybrid Cryptographic Compliance v1.1.0

**No changes required.** The content encryption uses the same primitives (XChaCha20-Poly1305 is symmetric, already quantum-resistant). The content key derivation uses HKDF-SHA256 from the GroupSecret, which inherits the hybrid protection of the group key establishment.

### 7.4 Cargo Workspace Structure

**Updated crate structure:**

```
relay-sync/
├── sync-types/          # Wire format, messages, ContentReference
├── sync-core/           # State machine, pure logic
├── sync-crypto/         # GroupKey, GroupSecret, ContentKey derivation
├── sync-client/         # Connection, encryption, cursor tracking
│   └── (depends on iroh-blobs for content transfer)
├── sync-content/        # NEW: Content transfer coordinator
│   ├── encrypt.rs       # Encrypt-then-hash pipeline
│   ├── transfer.rs      # iroh-blobs provider/requester wrapper
│   ├── thumbnail.rs     # Preview generation
│   └── lifecycle.rs     # GC coordination, quota management
├── sync-cli/            # Headless testing tool
├── tauri-plugin-sync/   # Tauri command bridge
└── sync-relay/          # Server implementation (future)
```

---

## 8. Risk Assessment

### 8.1 iroh Pre-1.0 Stability

**Risk:** iroh hasn't cut 1.0 yet. API could change between RC and stable.

**Mitigation:** The RC series (0.96+) is specifically designed for production feedback. Breaking changes from RC to 1.0 are expected to be minimal. Pin to specific RC version, audit changelogs before bumping.

**Fallback:** If iroh 1.0 delays significantly, we can ship with 0.95.x (the v0.35 stable recommendation from earlier is now outdated — the 0.9x series is more appropriate given timeline alignment).

### 8.2 iroh-blobs Maturity

**Risk:** iroh-blobs docs.rs page states "this version is not yet considered production quality."

**Mitigation:** We're using iroh-blobs for device-to-device transfer of encrypted blobs within a small device group (2-5 devices). This is a much gentler use case than production CDN-scale blob serving. The core transfer protocol is unchanged from v0.35 (Get request wire format compatible). The new API is the unstable part, not the transfer mechanism.

**Fallback:** For MVP, content transfer could use a simpler custom chunked-upload over raw QUIC streams, with iroh-blobs integration added in v1.1. But this means losing resumable downloads, verified streaming, and range requests — all of which are genuinely hard to build correctly.

### 8.3 Post-Quantum on iroh Transport

**Risk:** iroh's transport uses non-PQ TLS. Our Noise handshake adds hybrid PQ on top, but the initial relay connection (WebSocket over HTTPS) is classical-only.

**Assessment:** Acceptable for v1.0. The relay sees only ciphertext (our E2E encryption is PQ-hybrid). The transport layer exposure is connection metadata (who's talking to whom), not content. An attacker who harvests relay traffic gets encrypted blobs they can't decrypt even with a quantum computer. The metadata exposure (endpoint IDs, connection timing) is a lower-tier threat.

**Future mitigation:** When iroh eventually adds PQ support (they acknowledge it's on the horizon), we get it for free at the transport layer.

### 8.4 Mobile Background Transfer

**Risk:** iOS and Android aggressively kill background processes. iroh-blobs transfers could be interrupted.

**Mitigation:** This is exactly why iroh-blobs' resumable downloads matter. The client saves bitfield state (which chunks are downloaded), and resumes on next app foreground. Combined with our "thumbnail-first, full-resolution-on-demand" strategy, users never see broken sync — they see a preview immediately and full content loads when the app is active.

---

## 9. Decision Summary

| Decision | Status | Rationale |
|----------|--------|-----------|
| Keep custom sync protocol | **CONFIRMED** | Zero-knowledge relay, PQ crypto, cursor ordering, device lifecycle — none exists in iroh |
| Integrate iroh-blobs for large content | **NEW — ADOPT** | Verified streaming, resumable downloads, no wheel reinvention |
| Add mDNS local discovery | **NEW — ADOPT** | Zero cost, major UX improvement for same-LAN sync |
| Target iroh 1.0 RC | **UPDATED** | RC is imminent, better than building on 0.35 and migrating |
| Add ContentReference to wire protocol | **NEW** | Foundation-level: must be in spec before Phase 1 implementation |
| Adopt iroh-gossip | **DEFER to v1.1** | Our NOTIFY is simpler and sufficient for 2-5 device groups |
| Adopt iroh-docs | **REJECT** | Wrong trust model, wrong topology, not 1.0 ready |
| Add sync-content crate | **NEW** | Clean separation of content transfer concerns |
| Self-host iroh-relay from day one | **NEW — ADOPT** | Zero dependency on n0 infrastructure, required for sovereignty |
| Self-host iroh-dns-server | **NEW — ADOPT** | Own discovery infra on dns.ydun.io, DHT as decentralized fallback |
| Disable n0 default relays in production | **NEW** | Relay map points only to our infrastructure |

---

## 10. Next Steps

1. **Spec Amendment:** Draft "Appendix C: Large Content Transfer Protocol" covering ContentReference, encrypt-then-hash flow, iroh-blobs integration, GC lifecycle, and mobile considerations
2. **Wire Protocol Update:** Add CONTENT_REF (0x70) and CONTENT_ACK (0x71) message types to Technical Spec v2.2.0
3. **Implementation Plan Update:** Add sync-content crate to workspace, iroh-blobs dependency to sync-client, mDNS configuration to transport setup
4. **Validate with CrabNebula:** Check if CashTable needs large content transfer (receipt attachments?) — if yes, this benefits the internship work too

---

## 11. Dependency Risk & Sovereignty: What If n0 Disappears?

### 11.1 Licensing

iroh and all n0-computer crates are **dual-licensed MIT OR Apache 2.0** — the standard Rust ecosystem permissive license (same as Tokio, serde, Axum). This means:

- Fork freely, modify freely, redistribute freely, embed in commercial products
- No copyleft obligations, no patent traps (Apache 2.0 includes explicit patent grant)
- No requirement to open-source derivative works
- The license is irrevocable — even if n0 disappears, every published version retains its license

This is the strongest possible position for a dependency. No relicensing risk, no CLA concerns.

### 11.2 Company Background

n0 (number 0, Inc.) describes itself as "partly venture capital and partly founder backed" — meaning founders have also invested personal capital. They generate revenue through:

- **n0des.iroh.computer** — Managed iroh infrastructure service
- **n0ps** (n0.computer/n0ps) — Operational services for iroh deployments
- Re-launching an "AWS for iroh protocols" service in 2025/2026

They are not purely VC-funded burning runway toward an exit. The founder-backing and existing revenue are positive signals. However, as a startup, the risk of shutdown, acquisition, or pivot is non-zero, so we plan accordingly.

### 11.3 Infrastructure We Depend On (and Don't)

**Things n0 runs that we could use:**

| Service | URL | Our Dependency Level | Self-Hostable? |
|---------|-----|---------------------|----------------|
| Public relay servers | `use1-1.relay.iroh.network` etc. | **NONE** — we run our own relays | Yes — `iroh-relay` crate, compiled binary in every release |
| DNS discovery | `dns.iroh.link` | **Fallback only** — we run our own DNS server | Yes — `iroh-dns-server` crate, open source |
| n0des managed service | `n0des.iroh.computer` | **NONE** — we don't use this | N/A |
| Pkarr relay | Used by DNS discovery | **Indirect, via our DNS server** | Yes — Pkarr is an open project, any server can relay |

**Key insight:** By running our own iroh-relay and iroh-dns-server (both open source, both shipped as compiled binaries on every release), we have **zero runtime dependency on n0's infrastructure.**

### 11.4 What Happens If n0 Shuts Down Tomorrow

**Immediate impact: None.** Here's why:

1. **Relay servers:** We operate our own. n0's public relays going offline doesn't affect our users.

2. **DNS discovery:** We operate our own `iroh-dns-server` on our domain. If `dns.iroh.link` goes offline, our endpoints discover via `dns.ydun.io` (or whatever we configure).

3. **mDNS discovery:** Zero infrastructure required. Works on the local network with no external dependency whatsoever.

4. **BitTorrent DHT discovery:** Fully decentralized. The BitTorrent mainline DHT has been running since 2005 with millions of nodes. It doesn't depend on n0 or anyone else.

5. **crates.io is immutable:** Once a crate version is published, it cannot be unpublished or altered. Every version of iroh we pin to stays available forever (crates.io is backed by the Rust Foundation).

6. **Source code on GitHub:** Even if n0 deletes their repo (unlikely), GitHub forks persist. The repo has 7.6K stars and 101 forks as of January 2026.

**Medium-term impact (3-12 months):** No new releases, no bug fixes upstream. We'd need to:
- Maintain a fork of the specific version we depend on
- Apply security patches ourselves
- The relay server is intentionally simple (stateless packet forwarder) — maintenance burden is low
- The DNS server is a small codebase — maintenance burden is low

**Long-term impact (1+ years):** The QUIC ecosystem evolves (Quinn, rustls), and we'd need to keep our fork compatible. This is the real cost, but it's manageable given the clean layered architecture.

### 11.5 Fork Viability Assessment

Could we maintain a fork of iroh if needed?

| Component | Complexity | Our Capability | Verdict |
|-----------|-----------|----------------|---------|
| `iroh` (core networking) | Medium-High | Rust is our primary language | Feasible — we only use Endpoint, Connection, ALPN routing |
| `iroh-relay` | Low | Stateless binary, ~few thousand LOC | Easy to maintain |
| `iroh-dns-server` | Low | Small Rust server with redb/SQLite | Easy to maintain |
| `iroh-blobs` | Medium | Content-addressed transfer, Bao verification | Moderate — well-specified protocol, no magic |
| Quinn (QUIC) | High | Deep networking code | N/A — Quinn is a separate project with its own maintainers |

**Verdict:** We wouldn't need to fork the whole ecosystem. Pin the version, fork only if a critical security issue needs patching. Quinn (the QUIC implementation) is maintained independently and widely used (Cloudflare, Mozilla), so even if iroh's wrapper disappears, the transport layer is safe.

### 11.6 Mitigation Plan (Zero Extra Cost)

These steps cost nothing to implement and eliminate vendor dependency:

| Step | When | Effort |
|------|------|--------|
| Pin iroh version in `Cargo.lock` | Day 1 | Default Rust practice |
| Run own `iroh-relay` instance(s) | Day 1 (already planned) | Compiled binary, any VPS |
| Run own `iroh-dns-server` on `dns.ydun.io` | Day 1 | Same as relay deployment |
| Enable mDNS discovery | Day 1 | Feature flag, zero infrastructure |
| Enable DHT discovery as fallback | Day 1 | Feature flag, zero infrastructure |
| Configure relay map to use ONLY our relays (no n0 defaults) | Day 1 | One config change |
| Mirror iroh source in private Git | Optional | `git clone` once |

After implementing this plan, our entire stack runs on infrastructure we control. n0's existence becomes a bonus (upstream improvements, community, bug fixes) rather than a dependency.

### 11.7 Comparison: iroh Dependency Risk vs. Alternatives

| Dependency | License | Self-Hostable | Shutdown Impact | Fork Viability |
|-----------|---------|---------------|-----------------|----------------|
| **iroh** | MIT/Apache 2.0 | Everything | Zero (with our plan) | High |
| libp2p | MIT/Apache 2.0 | Most things | Low | Medium (larger, more complex) |
| Syncthing relay | MPL 2.0 | Yes | Low | Medium (Go, not our stack) |
| Nostr relays | Various | Yes | Low (decentralized) | High |
| Cloud SDK (AWS/GCP) | Proprietary | No | Critical | Impossible |
| CrabNebula Cloud | Proprietary service | No (service) | High | N/A |

iroh is among the lowest-risk options available. Permissive license, full self-hosting, stateless infrastructure, open standards underneath.

---

## 12. Our Self-Hosted Infrastructure

### 12.1 Current Infrastructure

We already operate self-hosted infrastructure aligned with the local-first, privacy-preserving philosophy:

**The Beast (Home Server):**
- Primary development and services host
- Runs Qdrant vector database, Ollama for local AI inference
- Accessible via Cloudflare Tunnels for public endpoints
- Proven operational model for always-on services

**Cloudflare Tunnels:**
- Zero-cost public ingress for self-hosted services
- DDoS protection and global edge (free tier)
- TLS termination handled automatically
- Already used for existing Ydun.io services

**GitHub-Based Distribution:**
- GitHub Pages for static update manifests (rate-limit safe)
- GitHub Releases for artifact hosting
- GitHub Actions for CI/CD pipelines
- Validated as 50-70% cheaper than CrabNebula Cloud for desktop app distribution

### 12.2 iroh Infrastructure Deployment Plan

Building on the existing self-hosted stack, the iroh infrastructure slots in naturally:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Ydun.io Infrastructure                            │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Cloudflare Edge                           │   │
│  │  DDoS protection · Global anycast · Free TLS · WebSocket    │   │
│  └────────┬──────────────────┬──────────────────┬──────────────┘   │
│           │                  │                  │                   │
│  ┌────────▼────────┐ ┌──────▼───────┐ ┌────────▼────────┐         │
│  │  iroh-relay     │ │ iroh-dns-    │ │  sync-relay     │         │
│  │  (QUIC + WS)    │ │ server       │ │  (our custom)   │         │
│  │                 │ │ dns.ydun.io  │ │                 │         │
│  │  Stateless      │ │              │ │  Cursor-ordered │         │
│  │  packet fwd     │ │  Pkarr +     │ │  encrypted      │         │
│  │                 │ │  DNS lookup   │ │  blob mailbox   │         │
│  └─────────────────┘ └──────────────┘ └─────────────────┘         │
│                                                                     │
│  Host: The Beast (home server) / VPS (production scale)            │
│  Tunneling: Cloudflare Tunnel (free tier MVP → Pro at scale)       │
│  Cost: $0/month at MVP                                              │
└─────────────────────────────────────────────────────────────────────┘
```

### 12.3 Deployment Tiers

**Tier 0: Development & Dogfooding** (Current)
- All services on The Beast via Cloudflare Tunnel
- Single region (Linköping, Sweden)
- Cost: $0/month
- Suitable for: personal use, Private Suite alpha testing, CashTable prototype

**Tier 1: MVP / Beta Launch**
- iroh-relay + iroh-dns-server + sync-relay on The Beast (primary)
- Optional: One VPS in EU (Hetzner, ~€5/month) for redundancy
- Cloudflare free tier for edge/DDoS
- mDNS + DHT discovery as infrastructure-free fallbacks
- Cost: $0-5/month
- Suitable for: beta testers, early adopters, CrabNebula demo

**Tier 2: Production Scale**
- iroh-relay instances in 2-3 regions (EU, US-East, APAC)
- Dedicated iroh-dns-server with replication
- sync-relay with SQLite blob store
- Cloudflare Pro ($25/month) for better routing
- Deployment: Docker containers on Fly.io or Hetzner
- Cost: $30-100/month
- Suitable for: public launch, enterprise pilots

**Tier 3: Enterprise / Defense**
- Air-gapped deployment option (all services on client's infrastructure)
- No internet dependency (mDNS-only mode for classified environments)
- Custom relay configuration for compliance requirements
- Cost: Per-contract
- Suitable for: defense contractors, government, regulated industries

### 12.4 Air-Gap Capability

A critical differentiator for enterprise/defense markets: the entire stack can run with zero internet connectivity.

**Air-gapped mode requires:**
- iroh-relay on local network (for devices that can't establish direct QUIC)
- mDNS discovery only (no DNS, no DHT, no internet)
- sync-relay on local network
- All devices on same LAN or connected via internal network

**What this means:** A defense contractor can deploy Private Suite applications on a classified network with no external connections. Devices discover each other via mDNS, sync through a locally-hosted relay, and all data stays within the security perimeter. No cloud, no SaaS, no external DNS queries.

This is impossible with any cloud-first sync solution (iCloud, Google, Dropbox). It's our structural advantage for regulated markets.

### 12.5 Infrastructure Sovereignty Summary

| Concern | Our Position |
|---------|-------------|
| Who owns the servers? | We do (The Beast, our VPS accounts) |
| Who controls the domain? | We do (ydun.io) |
| Can users self-host? | Yes — relay is a single binary |
| Can it run air-gapped? | Yes — mDNS + local relay, zero internet |
| Cloud vendor lock-in? | None — Docker containers, any provider |
| Does n0 going away break us? | No — see Section 11 |
| Does Cloudflare going away break us? | No — swap to any reverse proxy / direct connection |
| Does GitHub going away break us? | Code mirrors, artifacts on own CDN — migration path exists |

**Philosophy:** Every external service we use is replaceable. The architecture has no single point of failure that we don't control. This isn't paranoia — it's a product feature for privacy-conscious users and a selling point for enterprise customers who need exactly these guarantees.
