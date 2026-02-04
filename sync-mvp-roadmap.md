# 0k-Sync - MVP Roadmap

**Version:** 1.0.0  
**Date:** 2026-01-18  
**Target:** CashTable Launch

---

## Overview

This document defines the minimum viable implementation for CashTable launch and identifies features that can ship as post-launch updates based on real user data.

---

## MVP Requirements (Launch Blockers)

### Core Protocol

| Component | Status | Notes |
|-----------|--------|-------|
| sync-types crate | Required | Message definitions, wire format |
| sync-client crate | Required | E2E encryption, connection management |
| sync-relay server | Required | iroh Endpoint, message routing, temp buffer |
| Framework integration | Required | App-specific bridge (e.g., Tauri plugin for CashTable) |

### Cryptography

| Feature | Status | Notes |
|---------|--------|-------|
| Hybrid Noise handshake | Required | `clatter` with `noise_hybrid_XX` |
| X25519 + ML-KEM-768 | Required | Quantum-resistant from day one |
| XChaCha20-Poly1305 (Group Key) | Required | E2E blob encryption |
| Argon2id key derivation | Required | Device-adaptive parameters |

### Pairing Flow

| Feature | Status | Notes |
|---------|--------|-------|
| QR code pairing | Required | Primary flow for mobile |
| Short code pairing | Required | Fallback when camera unavailable |
| 10-minute invite expiry | Required | Security baseline |
| Single-use invites | Required | Delete on claim |

### Sync Operations

| Feature | Status | Notes |
|---------|--------|-------|
| PUSH / PUSH_ACK | Required | Upload encrypted blobs |
| PULL / PULL_RESPONSE | Required | Download blobs after cursor |
| NOTIFY | Required | Real-time updates to online peers |
| Cursor-based ordering | Required | No timestamps for ordering |
| Reconnection with jitter | Required | Prevent thundering herd |

### Mobile Lifecycle

| Feature | Status | Notes |
|---------|--------|-------|
| Sync on app launch | Required | Connect + push pending + pull new |
| Sync on foreground | Required | Resume sync when app visible |
| Fire-and-forget flush on close | Required | 500ms timeout, non-blocking |
| Local-first persistence | Required | SQLite, never lose data |
| Sync status indicators | Required | UI shows pending/synced/failed |

### Security

| Feature | Status | Notes |
|---------|--------|-------|
| SecureModuleLoader equivalent | Required | Verify relay identity |
| No plaintext on relay | Required | Zero-knowledge architecture |
| Device keypair in OS keychain | Required | Platform secure storage |

### Configuration

| Feature | Status | Notes |
|---------|--------|-------|
| Relay URL configuration | Required | Point to Managed or self-hosted |
| Auto-reconnect | Required | Default enabled |
| Console logging (debug builds) | Required | Disabled in production |

---

## MVP Scope Boundaries

### In Scope for Launch

- Single relay tier (Managed or self-hosted)
- Desktop + mobile platforms
- Two-device sync minimum
- App-specific blob format (CashTable handles serialization)

### Explicitly Out of Scope for Launch

- Multi-relay federation
- Web client
- Offline-first conflict resolution (app responsibility)
- Background sync on mobile
- Push notifications

---

## Post-Launch Updates

### Phase 2: Observability (Week 1-2 post-launch)

Ship instrumentation to collect real data before optimizing.

| Feature | Priority | Depends On |
|---------|----------|------------|
| Sync success/failure metrics | High | Launch |
| Time-to-sync measurements | High | Launch |
| Stranded commit frequency | High | Launch |
| Reconnection attempt tracking | Medium | Launch |
| Blob size distribution | Medium | Launch |

### Phase 3: Device Management (Week 2-4)

| Feature | Priority | Depends On |
|---------|----------|------------|
| List devices in sync group | High | Launch |
| Device revocation (REVOKE_DEVICE) | High | Launch |
| Force delete stuck blobs | Medium | Device revocation |
| Device last-seen tracking | Medium | Launch |

### Phase 4: Tuning (Week 4-6)

Based on Phase 2 metrics.

| Feature | Priority | Depends On |
|---------|----------|------------|
| Reconnection backoff optimization | Medium | Observability data |
| Quick flush timeout tuning | Low | Observability data |
| Buffer TTL adjustment | Low | Usage patterns |
| Presence heartbeat interval | Low | User feedback |

### Phase 5: Push Notifications (Week 6-8)

Only if observability shows users need background sync.

| Feature | Priority | Depends On |
|---------|----------|------------|
| REGISTER_PUSH / UNREGISTER_PUSH | Medium | User demand signal |
| APNS integration (iOS) | Medium | Push token storage |
| FCM integration (Android) | Medium | Push token storage |
| Silent push for sync wake | Medium | APNS/FCM integration |

**Decision gate:** Ship push notifications only if >20% of syncs are delayed >24h due to app not being opened.

### Phase 6: Enterprise Features (Future)

For future 0k-Sync adopters requiring enterprise compliance. Not CashTable scope.

| Feature | Priority | Depends On |
|---------|----------|------------|
| ML-DSA-65 hybrid signatures | Low | Enterprise customer demand |
| Audit log signing | Low | Enterprise customer demand |
| OIDC authentication | Low | Enterprise customer demand |
| Custom relay deployment | Low | Enterprise customer demand |

---

## Implementation Timeline

### MVP (5-6 weeks)

| Week | Focus | Deliverables |
|------|-------|--------------|
| 1 | Protocol foundation | sync-types, basic relay scaffolding |
| 2 | Crypto layer | clatter integration, hybrid handshake |
| 3 | Client implementation | sync-client, connection management |
| 4 | Tauri integration | tauri-plugin-sync, JS API |
| 5 | Pairing + polish | QR/short code flows, error handling |
| 6 | Buffer | Testing, edge cases, CashTable integration |

### Post-Launch

| Week | Phase | Focus |
|------|-------|-------|
| 1-2 | Phase 2 | Instrumentation, metrics collection |
| 2-4 | Phase 3 | Device management UI |
| 4-6 | Phase 4 | Tuning based on data |
| 6-8 | Phase 5 | Push notifications (if needed) |

---

## Risk Mitigation

### Launch Risks

| Risk | Mitigation |
|------|------------|
| Hybrid handshake issues | Test against clatter test vectors early |
| Mobile lifecycle edge cases | Spec assumes worst case; ship and observe |
| Relay scaling | Single Managed instance handles early load |

### Post-Launch Risks

| Risk | Mitigation |
|------|------------|
| Push notifications complex | API hooks exist from day one; backend later |
| Device revocation edge cases | Ship basic version; harden with real scenarios |
| Tuning without data | Phase 2 instrumentation is non-negotiable |

---

## Success Criteria

### Launch

- [ ] Two devices can pair via QR code
- [ ] Blobs sync within 5 seconds when both online
- [ ] No data loss when app closed mid-sync
- [ ] Hybrid handshake completes successfully
- [ ] Relay handles 100 concurrent connections

### 30 Days Post-Launch

- [ ] Observability dashboard showing sync health
- [ ] Device revocation working in production
- [ ] <1% sync failure rate
- [ ] Median time-to-sync <2 seconds

### 60 Days Post-Launch

- [ ] Tuning applied based on real data
- [ ] Push notifications shipped (if metrics justify)
- [ ] Zero reported data loss incidents

---

## Dependencies

### External

| Dependency | Version | Risk |
|------------|---------|------|
| clatter | 2.1+ | Low - stable release |
| iroh | 1.0 | Low - production ready |
| sqlx (relay) | 0.8+ | Low - mature |
| tauri | 2.0+ | Low - Managed expertise |

### Internal

| Dependency | Owner | Risk |
|------------|-------|------|
| CashTable blob format | CashTable team | Define early |
| Managed Cloud hosting | Managed | Coordinate deployment |
| Mobile build pipeline | Managed | Verify Tauri mobile works |

---

## Open Decisions

| Decision | Options | Deadline |
|----------|---------|----------|
| Relay hosting | Managed Cloud vs self-hosted for launch | Week 1 |
| Blob format | JSON vs MessagePack for CashTable | Week 1 |
| Tier configuration | Hardcode Managed tier vs configurable | Week 2 |

---

*MVP Roadmap | v1.0.0 | 2026-01-18*
