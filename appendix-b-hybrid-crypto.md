# Appendix B: Hybrid Cryptographic Compliance

> **IMPLEMENTATION STATUS (2026-02-05):** This appendix describes the DESIGN for hybrid post-quantum cryptography. The clatter Noise Protocol layer described here is **NOT YET IMPLEMENTED** in code. Current transport security is provided by iroh QUIC (TLS 1.3). E2E encryption uses XChaCha20-Poly1305. See security audit finding F-002.

**Version:** 1.1.0
**Date:** 2026-01-18
**Parent Document:** 0k-Sync Specification v2.1.0

---

## B.1 Scope

This appendix addresses post-quantum readiness for 0k-Sync, combining classical algorithms with NIST post-quantum standards so security holds if either is broken.

---

## B.2 Layer Analysis

**Layer 1 (Transport Encryption):** The Noise XX handshake uses Curve25519—the primary quantum vulnerability since Shor's algorithm breaks ECDH. Hybrid treatment is mandatory.

**Layer 0 (TLS 1.3):** Handled by infrastructure. Most providers are rolling out X25519Kyber768. Verify provider support; no code changes required.

**Layer 2 (E2E Encryption):** XChaCha20-Poly1305 with 256-bit keys is already quantum-resistant. Grover's algorithm only halves effective strength, leaving 128-bit equivalent security. No changes needed.

**Device Identity (Section 4.3):** Currently Ed25519. For enterprise tiers requiring signed audit logs, pair with ML-DSA-65. Lower tiers can defer.

---

## B.3 Algorithm Selections

| Layer | Classical | Post-Quantum | Notes |
|-------|-----------|--------------|-------|
| Key Exchange | X25519 | ML-KEM-768 | Combined in hybrid handshake |
| Signatures (enterprise) | Ed25519 | ML-DSA-65 | 3.3KB sigs—audit logs only |
| Symmetric | XChaCha20-Poly1305 | N/A | Already sufficient |

ML-KEM-768 provides NIST Security Level 3. ML-KEM-1024 adds ~50% ciphertext overhead with marginal benefit for this threat model.

---

## B.4 Integration Strategy

**Recommended: Use `clatter` crate with HybridHandshake**

The `clatter` crate (https://github.com/jmlepisto/clatter) provides a pure Rust, `no_std` compatible Noise implementation with built-in hybrid support. This eliminates the need for custom KEM wrapper integration.

Replace `snow` with `clatter` and use the `HybridHandshake` type with `noise_hybrid_XX` pattern. The hybrid handshake combines both DH and KEM operations in the same messages, achieving true hybrid security with minimal effect on round trips.

**Protocol name format:**

```
Noise_hybridXX_X25519+MLKEM768_ChaChaPoly_BLAKE2s
```

**Why this works:**

- `clatter` handles DH and KEM public key ordering automatically (DH first, then KEM)
- All `mix_hash` and `mix_key` operations conducted in correct order
- Preserves relative ordering of operations with respect to key material transmissions
- Verified against Cacophony and Snow test vectors

**Prior art:**

- Signal protocol integrated X25519+Kyber for key exchanges
- Katzenpost decryption mix network uses pqXX with X25519+Kyber768 in production
- University of Luxembourg researchers evaluated hybrid Noise on mobile devices—execution times nearly identical to classical under normal network conditions

---

## B.5 Compliance Mapping

| Framework | Position |
|-----------|----------|
| FIPS 140-3 | ML-KEM approved in FIPS 203; hybrid accepted |
| PCI-DSS 4.0 | Hybrid exceeds "strong cryptography" requirement |
| SOC 2 | Demonstrates security commitment |
| CNSA 2.0 | Aligns with NSA 2025-2030 transition guidance |

For enterprise positioning, hybrid compliance removes a migration burden that future adopters would otherwise inherit.

---

## B.6 Dependencies

**Primary:** Replace `snow` with `clatter`

```toml
[dependencies]
clatter = "2.1"
```

`clatter` includes ML-KEM support via feature flags. No additional KEM crates required for the handshake layer.

**For enterprise signatures:** `ml-dsa` crate if audit log signing needed.

**Binary impact:** ML-KEM-768 adds ~150KB; ML-DSA-65 adds ~200KB. Negligible for desktop, verify mobile budgets.

**Performance:** Sub-millisecond encapsulation/decapsulation. University of Luxembourg benchmarks show hybrid handshakes nearly indistinguishable from classical under normal network conditions.

---

## B.7 Configuration Extension

Add `hybrid_mode` to `SyncConfig`:

- **Classical** — No post-quantum (development only)
- **Hybrid** — ML-KEM-768 + X25519 combined (recommended default)
- **PostQuantumOnly** — Future, not recommended until ecosystem matures

Enterprise tier should enforce Hybrid as minimum.

```rust
pub enum HybridMode {
    Classical,        // snow, Noise_XX
    Hybrid,           // clatter, Noise_hybridXX
    PostQuantumOnly,  // clatter, Noise_pqXX (future)
}
```

---

## B.8 Migration Path

Relay and clients must upgrade together—hybrid client cannot handshake with classical-only relay.

1. Release relay with hybrid support (backward compatible, accepts both)
2. Release clients with hybrid default
3. After adoption threshold, deprecate classical-only

Group Key rotation is not required—symmetric layer is already quantum-resistant.

---

## B.9 Implementation Checklist

- [ ] Replace `snow` dependency with `clatter`
- [ ] Update handshake pattern from `noise_XX` to `noise_hybrid_XX`
- [ ] Configure ML-KEM-768 for KEM operations
- [ ] Verify iroh transport integration with `clatter`
- [ ] Update protocol version negotiation in HELLO message
- [ ] Add `hybrid_mode` to `SyncConfig` and `RelayConfig`
- [ ] Test interoperability between hybrid and classical modes during transition
- [ ] Benchmark handshake latency on target mobile devices

---

## B.10 Remaining Considerations

1. **Version negotiation** — Include hybrid capability flag in HELLO message for backward compatibility during rollout
2. **iOS memory** — Verify ML-KEM memory usage within iOS extension constraints (~55MB limit per Section 4.1)
3. **Audit logging** — If enterprise tier requires signed logs, add ML-DSA-65 hybrid signatures separately

---

## B.11 References

- `clatter` crate: https://github.com/jmlepisto/clatter
- PQNoise paper: https://eprint.iacr.org/2022/539
- University of Luxembourg evaluation: https://orbilu.uni.lu/bitstream/10993/63642/1/SECITC2024.pdf
- NIST FIPS 203 (ML-KEM): https://csrc.nist.gov/pubs/fips/203/final

---

*Appendix B | v1.1.0 | 2026-01-18*
