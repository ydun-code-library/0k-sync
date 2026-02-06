# OPRF for Passphrase Hardening Research

**Date:** 2026-02-06
**Triggered by:** Feedback from Matthias (felsweg) on OPRF
**Status:** Research complete, potential future enhancement

---

## Executive Summary

OPRF (Oblivious Pseudorandom Function) can eliminate offline dictionary attacks on passphrases by requiring server cooperation for each guess. Used by WhatsApp and Signal for backup/PIN protection.

**Trade-off:** Requires relay to be online for pairing. Changes trust model.

**Recommendation:** Consider as optional enhancement for enterprise tier, not replacement for current Argon2id-only flow.

---

## What is OPRF?

A two-party protocol where:
- Client has input `x` (passphrase)
- Server has secret key `k`
- Together they compute `PRF(k, x)` such that:
  - Client learns only the output
  - Server learns nothing about the input

### 2HashDH Construction (RFC 9497)

1. **Client blinds**: `blinded = Hash(passphrase) * r` (random `r`)
2. **Server evaluates**: `evaluated = blinded^k`
3. **Client unblinds**: `result = evaluated^(1/r) = Hash(passphrase)^k`

The server never sees the passphrase. The client can't compute the result without the server.

---

## Threat Model Change

### Current 0k-Sync (Argon2id Only)

```
passphrase --[Argon2id + salt]--> GroupSecret --[HKDF]--> GroupKey
```

**Vulnerability:** Offline dictionary attack possible if attacker captures ciphertext + salt.

### With OPRF

```
passphrase --[blind]--> relay --[OPRF(k, .)]--> client --[unblind + Argon2id]--> GroupSecret
```

| Attack | Argon2id Only | With OPRF |
|--------|---------------|-----------|
| Offline dictionary attack | Possible (slowed by Argon2) | **Impossible** |
| Pre-computation attack | Possible if salt known | **Impossible** |
| Online brute-force | Rate-limited | Rate-limited + hard caps |

---

## What OPRF Changes

### New Dependencies

| Dependency | Impact |
|------------|--------|
| Server availability | Pairing **requires** relay online |
| Server key custody | Relay must protect OPRF key |
| Trust in server | Must trust relay won't collude |
| Protocol complexity | Two round-trips vs local computation |

### Zero-Knowledge Compatibility

OPRF is **compatible** with zero-knowledge relay:
- Relay still never sees plaintext messages
- Relay still can't derive encryption keys
- Relay gains: custody of OPRF key (new trust requirement)

---

## Real-World Deployments

### WhatsApp E2EE Backups

- Uses 2HashDH OPRF in HSM-based Backup Key Vault
- **5-attempt hard limit** then key permanently destroyed
- Password never sent to server
- `opaque-ke` Rust crate audited for this deployment

### Signal Secure Value Recovery

- OPRF with Intel SGX enclaves
- Distributed across Raft consensus groups
- **5-guess hard limit** enforced by enclave
- Exploring threshold OPRF across data centers

---

## Rust Implementation

### opaque-ke crate

- **Crate:** [opaque-ke](https://crates.io/crates/opaque-ke)
- **RFC:** Implements RFC 9807 (OPAQUE protocol)
- **Audit:** NCC Group (June 2021), sponsored by WhatsApp
- **Status:** Production-ready, version 4.0.1

### voprf crate

- **Crate:** [voprf](https://crates.io/crates/voprf)
- **RFC:** Implements RFC 9497 (OPRF/VOPRF/POPRF)
- **Audit:** No public audit
- **Status:** Maintained by Meta

**Recommendation:** Use `opaque-ke` for production (audited, WhatsApp-proven).

---

## Recommendation for 0k-Sync

### Hybrid Approach

OPRF as **optional enhancement**, not replacement:

1. **Default mode:** Current Argon2id-only
   - Works offline
   - No additional trust requirements
   - Good for personal use, privacy maximalists

2. **Enterprise mode:** OPRF-hardened pairing
   - Requires relay cooperation
   - Prevents offline dictionary attacks
   - Good for compliance environments

### If Implementing OPRF

1. Use `opaque-ke` crate (audited)
2. Consider threshold OPRF across multiple relays (for multi-relay failover work)
3. Add attempt limiting (5 max) with permanent lockout
4. Document trust model change clearly
5. Make it opt-in, not default

---

## References

- [RFC 9497 - OPRF Using Prime-Order Groups](https://datatracker.ietf.org/doc/rfc9497/)
- [RFC 9807 - OPAQUE Protocol](https://datatracker.ietf.org/doc/rfc9807/)
- [WhatsApp E2EE Backups Engineering](https://engineering.fb.com/2021/09/10/security/whatsapp-e2ee-backups/)
- [Signal Secure Value Recovery](https://signal.org/blog/secure-value-recovery/)
- [opaque-ke crate](https://docs.rs/opaque-ke)
- [Wikipedia - OPRF](https://en.wikipedia.org/wiki/Oblivious_pseudorandom_function)
