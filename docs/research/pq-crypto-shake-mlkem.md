# Post-Quantum Crypto: SHAKE256 + ML-KEM Research

**Date:** 2026-02-06
**Triggered by:** Feedback from Matthias (felsweg) on SHA-3/Keccak/sponge functions
**Status:** Research complete, informing future PQ implementation

---

## Executive Summary

ML-KEM (FIPS 203, formerly CRYSTALS-Kyber) uses SHAKE internally but the security comes from lattice math, not the hash function. For 0k-sync's post-quantum path:

- **Inside ML-KEM**: SHAKE128/256 is mandatory (handled by the crate)
- **Noise handshake**: Keep BLAKE2s or SHA-512 (clatter doesn't support SHAKE)
- **Application layer**: Keep BLAKE3 (5-10x faster, no security downside)

No benefit to "SHAKE everywhere" — different layers have different needs.

---

## How ML-KEM Uses SHAKE Internally

ML-KEM (FIPS 203) uses **both SHAKE128 and SHAKE256** for different internal functions:

| Function | Algorithm | Purpose |
|----------|-----------|---------|
| **H** | SHA3-256 | Hash function (32-byte output) |
| **G** | SHA3-512 | Hash function (64 bytes split to two 32-byte outputs) |
| **J** | SHAKE256 | XOF for implicit rejection / challenge generation |
| **PRF** | SHAKE256 | Pseudorandom function for noise sampling |
| **XOF** | SHAKE128 | Matrix generation (SampleNTT) |

### Why Two SHAKE Variants?

- **SHAKE128** for XOF: Used where large amounts of pseudorandom data are needed. 168-byte rate vs SHAKE256's 136-byte rate = faster for bulk output.
- **SHAKE256** for PRF/J: Used for security-critical operations where 256-bit security is desired.

---

## FIPS 203 Requirements

For FIPS 203 compliance:

1. SHAKE functions must comply with FIPS 202 (SHA-3 standard)
2. Specific function bindings are **mandatory** — cannot substitute BLAKE3
3. These are handled internally by the ML-KEM implementation crate

---

## Clatter Hash Support

The `clatter` crate (Noise + ML-KEM) supports:

| Hash | Supported |
|------|-----------|
| SHA-256/512 | Yes |
| BLAKE2 | Yes |
| SHAKE128/256 | **No** |

SHAKE would require custom implementation, but there's no security benefit — Noise uses HMAC/HKDF internally which expects block-based hashes.

---

## Performance: SHAKE vs BLAKE3

| Algorithm | Cycles/byte (modern CPU) |
|-----------|--------------------------|
| BLAKE3 | ~0.5-1 |
| SHAKE128 | ~4-6 |
| SHAKE256 | ~5-8 |
| SHA3-256 | ~8-11 |

For key derivation (32-64 bytes), BLAKE3 is 5-10x faster with equivalent post-quantum security.

---

## Recommended Architecture

```
Application Layer
├── Content hashing: BLAKE3 (fast, parallel)
├── Key derivation: BLAKE3 derive_key mode
│
Noise Handshake Layer (clatter)
├── Hash: SHA-512 or BLAKE2s (Noise-standard)
├── DH: X25519 (classical)
├── KEM: ML-KEM-768 (post-quantum)
├── Cipher: ChaChaPoly
│
ML-KEM-768 Internal (fips203 crate)
├── XOF: SHAKE128 (matrix generation)
├── PRF: SHAKE256 (noise sampling)
├── J: SHAKE256 (implicit rejection)
├── H: SHA3-256, G: SHA3-512
```

---

## Key Insights

1. **ML-KEM handles SHAKE internally** — transparent to us
2. **No "pure SHAKE stack" benefit** — different layers serve different purposes
3. **BLAKE3 for app layer** — faster, equally secure for symmetric operations
4. **Compliance ≠ security** — FIPS compliance often means older algorithms

---

## References

- [FIPS 203 - ML-KEM Standard](https://csrc.nist.gov/pubs/fips/203/final)
- [FIPS 202 - SHA-3 Standard](https://csrc.nist.gov/pubs/fips/202/final)
- [clatter crate](https://docs.rs/clatter)
- [BLAKE3 benchmarks](https://github.com/BLAKE3-team/BLAKE3)
- [Keccak Team performance](https://keccak.team/sw_performance.html)
