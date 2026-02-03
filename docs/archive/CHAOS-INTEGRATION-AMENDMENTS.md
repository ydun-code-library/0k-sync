# 0k-Sync — Chaos Testing Integration into Implementation Plan

**Document:** CHAOS-INTEGRATION-AMENDMENTS.md
**Version:** 1.0.0
**Date:** 2026-02-03
**Purpose:** Amend `03-IMPLEMENTATION-PLAN.md` to incorporate chaos testing deliverables from `06-CHAOS-TESTING-STRATEGY.md`
**Executor:** Moneypenny (Claude Code CLI)
**Verifier:** Q workstation

---

## Problem Statement

The implementation plan (`03-IMPLEMENTATION-PLAN.md`, v2.1.0) and the chaos testing strategy (`06-CHAOS-TESTING-STRATEGY.md`, v1.4.0) were written at different times. The chaos strategy explicitly references the implementation plan as a parent document and defines phase-by-phase chaos deliverables in Section 15. However, the implementation plan contains zero references to chaos testing — the word "chaos" does not appear in the document at all.

This means an agent following the implementation plan will build all six crates, pass every validation gate, tag every checkpoint, and never create a single chaos test or even the harness infrastructure. Section 15.1 of the chaos strategy warns against exactly this: "What you do NOT do is dump all 68 scenarios as a single sprint after the code is 'done.' By then it's too late — the architecture has already baked in assumptions that chaos testing would have caught."

The amendments below integrate chaos deliverables into the implementation plan so they are built alongside the code, not after it.

---

## Amendment 1: Add `tests/chaos/` to Project Structure

**Location:** Section 2.1 "Cargo Workspace" (lines 83–156)

**What to do:** After the `sync-relay/` entry and before the closing of the directory tree, add the `tests/chaos/` directory. This matches the harness architecture defined in Section 11.1 of the chaos strategy.

**Add this directory block after `sync-relay/`:**

```
├── tests/
│   └── chaos/                     # Chaos test harness (06-CHAOS-TESTING-STRATEGY.md)
│       ├── Cargo.toml
│       ├── docker-compose.chaos.yml
│       ├── Dockerfile.relay
│       ├── Dockerfile.cli
│       └── src/
│           ├── main.rs            # Test runner entry point
│           ├── topology.rs        # Docker Compose management
│           ├── toxiproxy.rs       # Toxiproxy HTTP API client
│           ├── pumba.rs           # Pumba command wrapper
│           ├── assertions.rs      # Sync state verification helpers
│           └── scenarios/
│               ├── transport.rs   # T-LAT, T-LOSS, T-CONN, T-BW (16 scenarios)
│               ├── encryption.rs  # E-HS, E-ENC, E-PQ (16 scenarios)
│               ├── sync.rs        # S-SM, S-CONC, S-CONV, S-BLOB (16 scenarios)
│               ├── content.rs     # C-STOR, C-COLL (6 scenarios)
│               └── adversarial.rs # A-PROTO, A-RES (10 scenarios)
```

Also add the `docs/` directory entries for `05-RELEASE-STRATEGY.md` and `06-CHAOS-TESTING-STRATEGY.md`, which are currently missing from the project structure listing.

**Verification:** The directory tree in Section 2.1 must include `tests/chaos/` with all files listed above. The `docs/` listing must include all six numbered documents (01 through 06).

---

## Amendment 2: Add `chaos-tests` to Workspace Cargo.toml

**Location:** Section 2.2 "Workspace Cargo.toml" (lines 160–171)

**What to do:** Add `tests/chaos` as a workspace member. Place it after `tauri-plugin-sync` and before the commented-out `sync-relay` line. Also add workspace dependencies required by the chaos harness.

**In the `[workspace]` members array, add:**

```toml
    "tests/chaos",        # Chaos test harness (Phases 1-2 skeleton, scenarios added per phase)
```

**In the `[workspace.dependencies]` section, add a new comment group after the existing dependencies:**

```toml
# Chaos testing infrastructure
toxiproxy-rs = "0.2"          # Toxiproxy API client (verify version on crates.io)
bollard = "0.16"              # Docker API client for topology management
```

**Verification:** Running `cargo metadata --format-version=1` from the workspace root must list `chaos-tests` as a workspace member. The Toxiproxy and Docker dependencies must be available to the chaos harness crate.

---

## Amendment 3: Add Chaos Harness Deliverables to Phase 1–2

**Location:** Phase 1 checkpoint (Section 4.5, lines 468–480) and Phase 2 checkpoint (Section 5.5, lines 820–833)

**Context:** Phases 1–2 build `sync-types` and `sync-core` — there is no network code to break yet. The chaos strategy (Section 15.2) specifies: "Build chaos harness skeleton: Docker Compose templates, Toxiproxy Rust wrapper, chaos controller scaffold, assertion helpers. No scenarios yet — there's no network code to break."

**What to do:** The chaos harness construction should be split across Phases 1 and 2 since both are infrastructure phases without network code. This keeps Phase 1 focused on wire types (harness compiles, no logic yet) and Phase 2 on pure logic (assertion helpers get their own unit tests).

**Add a new subsection to Phase 1, after Section 4.3 (TDD Sequence) and before Section 4.4 (Validation Gate):**

Title it "4.3.1 Chaos Harness: Skeleton" and write:

Create the `tests/chaos/` crate with its `Cargo.toml`, `docker-compose.chaos.yml`, `Dockerfile.relay`, and `Dockerfile.cli`. Implement `topology.rs` (Docker Compose management — start, stop, health-check containers), `toxiproxy.rs` (HTTP API client wrapping Toxiproxy's REST interface — add/remove/list toxics), and `pumba.rs` (command wrapper for container kill/pause/stop). These modules need only compile and have basic unit tests for their API surface. No scenario files yet.

The `docker-compose.chaos.yml` should define the base Pair topology: one relay, two clients, one Toxiproxy instance, all on an isolated bridge network with `internal: true`. Include `Dockerfile.relay` and `Dockerfile.cli` stubs that will be populated when those crates are buildable.

Deliverable: `tests/chaos/` compiles as a workspace member. `cargo test -p chaos-tests` runs and passes (testing Toxiproxy client URL construction, Docker Compose file parsing, Pumba command generation — all unit-level, no Docker required).

**Add a new subsection to Phase 2, after Section 5.3 (TDD Sequence) and before Section 5.4 (Validation Gate):**

Title it "5.3.1 Chaos Harness: Assertion Helpers" and write:

Implement `assertions.rs` with the four core verification functions that all chaos scenarios will depend on. These are pure functions that take sync state as input and return pass/fail — no I/O, matching Phase 2's "pure logic" philosophy:

1. `assert_blob_present(client_state, blob_hash)` — Verify a specific blob exists in a client's local store by hash.
2. `assert_no_data_loss(topology_state)` — Compare all clients' blob sets and verify every blob pushed by any client is present on all paired clients.
3. `assert_version_vectors_converged(topology_state)` — Verify all clients have identical version vectors after sync quiescence. This tests Invariant 4 from the chaos strategy.
4. `assert_no_plaintext_in_logs(relay_logs)` — Scan relay log output for any un-redacted sensitive fields. This tests Invariant 5.

Each assertion helper must have its own unit tests using fabricated state. These tests validate the assertions themselves — not the sync protocol. For example, `assert_no_data_loss` should fail when given a topology state where Client B is missing a blob that Client A pushed.

Deliverable: `assertions.rs` compiles with all four helpers and their unit tests. `cargo test -p chaos-tests` passes.

**Update Phase 1 validation gate** (Section 4.4) to include:

```bash
# Chaos harness compiles
cargo test -p chaos-tests --lib
```

**Update Phase 2 validation gate** (Section 5.4) to include:

```bash
# Chaos assertion helpers pass
cargo test -p chaos-tests
```

**Update Phase 1 checkpoint commit message** (Section 4.5) to mention: "Chaos harness skeleton (topology, Toxiproxy, Pumba wrappers)"

**Update Phase 2 checkpoint commit message** (Section 5.5) to mention: "Chaos assertion helpers (blob presence, data loss, version vector convergence, plaintext detection)"

**Verification:** After Phase 2 is complete, `cargo test -p chaos-tests` must pass. The harness must compile as a workspace member. All four assertion helpers must exist with unit tests. No chaos scenarios exist yet — that is correct at this stage.

---

## Amendment 4: Add Encryption Chaos Scenarios to Phase 3

**Location:** Phase 3 checkpoint section (Section 6.5, lines 1139–1154)

**Context:** Phase 3 builds `sync-client` including the Hybrid Noise Protocol handshake (clatter, ML-KEM-768 + X25519) and XChaCha20-Poly1305 encryption. The chaos strategy (Section 15.2) specifies: "Write E-HS-*, E-ENC-*, E-PQ-* scenario assertions against a mock transport. Also stub out T-* scenario signatures with `#[ignore]`."

**What to do:** Add a new subsection after Phase 3's TDD Sequence (Section 6.3) and before the Validation Gate (Section 6.4).

Title it "6.3.1 Chaos Scenarios: Encryption Layer" and write:

With the encryption layer now implemented, write chaos scenarios that test its resilience at the crypto layer using a mock transport — no Docker, no real network. These run in-process and inject chaos by corrupting, truncating, or reordering bytes between handshake messages.

Create `tests/chaos/src/scenarios/encryption.rs` with the following scenario assertions:

**Handshake chaos (E-HS-01 through E-HS-06, 6 scenarios):** These test the Hybrid Noise Protocol XX handshake under adversarial byte manipulation. Scenarios include: corrupted handshake payload (bit-flip in ML-KEM ciphertext), truncated handshake message (message cut mid-transmission), reordered handshake messages (message 2 arrives before message 1), replayed handshake message (duplicate of a previous handshake message), oversized handshake payload (payload exceeding maximum), and handshake timeout (initiator starts, responder never replies). Each must assert: handshake fails cleanly with an appropriate error, no partial state is retained, retry succeeds with fresh ephemeral keys.

**Session encryption chaos (E-ENC-01 through E-ENC-05, 5 scenarios):** These test the XChaCha20-Poly1305 session encryption after a successful handshake. Scenarios include: corrupted ciphertext (single bit flip in encrypted blob), truncated ciphertext (blob cut short), modified authentication tag (Poly1305 tag altered), nonce manipulation (replayed or zeroed nonce), and empty payload encryption (zero-length plaintext). Each must assert: decryption fails with authentication error (not garbage output), session remains usable for subsequent messages, no plaintext is leaked in error paths.

**Post-quantum specific chaos (E-PQ-01 through E-PQ-05, 5 scenarios):** These test ML-KEM-768 specific failure modes. Scenarios include: corrupted ML-KEM ciphertext (bit flip in encapsulated key), truncated ML-KEM ciphertext (short by 1 byte), ML-KEM decapsulation failure with valid X25519 (testing hybrid independence), key size boundary violations, and malformed public key. Each must assert: the hybrid handshake fails entirely (no fallback to X25519-only), error message identifies the post-quantum component, no key material is leaked.

Also create stub signatures for all 16 transport chaos scenarios (T-LAT-01 through T-LAT-04, T-LOSS-01 through T-LOSS-04, T-CONN-01 through T-CONN-05, T-BW-01 through T-BW-03) in `tests/chaos/src/scenarios/transport.rs`. Each stub should be annotated with `#[ignore]` and a comment explaining it requires Docker topology (Phase 6). The stub should contain the assertion description as a doc comment so the intent is captured now.

All 16 encryption scenarios must use a mock transport trait — the same `Transport` trait abstraction from `sync-client` — injecting chaos at the byte level. No network sockets, no Docker. This means they run in CI alongside unit tests.

**Update Phase 3 validation gate** (Section 6.4) to include:

```bash
# Encryption chaos scenarios pass (mock transport, no Docker)
cargo test -p chaos-tests -E 'test(/^encryption/)'

# Transport stubs compile but are skipped
cargo test -p chaos-tests -E 'test(/^transport/)' -- --ignored --list
```

**Update Phase 3 checkpoint commit message** (Section 6.5) to mention: "Encryption chaos scenarios (16 mock-based: E-HS, E-ENC, E-PQ), transport chaos stubs (16, #[ignore])"

**Verification:** After Phase 3 is complete, `cargo test -p chaos-tests -E 'test(/^encryption/)'` must pass all 16 scenarios. `cargo test -p chaos-tests -E 'test(/^transport/)' -- --ignored` must show 16 ignored tests. The mock transport must not use any real network connections.

---

## Amendment 5: Add Content Chaos Scenarios to Phase 3.5

**Location:** Phase 3.5 checkpoint criteria (Section 6.5.5, lines 1273–1284)

**Context:** Phase 3.5 builds `sync-content` with the encrypt-then-hash pipeline and iroh-blobs integration. The chaos strategy (Section 15.2) specifies: "Write S-BLOB-* assertions (hash verification after transfer), C-STOR-* stubs (disk full, bit rot). These test the content pipeline in isolation using mock storage backends."

**What to do:** Add a new subsection after Phase 3.5's Implementation Order (Section 6.5.4) and before the Checkpoint Criteria (Section 6.5.5).

Title it "6.5.4.1 Chaos Scenarios: Content Layer" and write:

With the content pipeline now implemented, write chaos scenarios that test blob integrity and storage resilience using mock storage backends — no Docker, no real disk I/O failures.

Create `tests/chaos/src/scenarios/content.rs` with the following scenario assertions:

**Blob integrity chaos (S-BLOB-01 through S-BLOB-04, 4 scenarios):** These test the encrypt-then-hash pipeline under adversarial conditions. Scenarios include: corrupted blob after encryption (bit flip in ciphertext between encrypt and hash stages), hash mismatch on retrieval (stored blob modified after BLAKE3 hash was computed), partial blob transfer (blob truncated mid-stream, simulating interrupted iroh-blobs transfer), and duplicate blob with different encryption (same plaintext encrypted twice must produce different ciphertext due to unique nonces, but decrypt to identical content). Each must assert: corruption is detected before any decrypted content is returned to the caller, BLAKE3 verification catches single-bit corruption, partial transfers are rejected not silently accepted.

**Storage chaos (C-STOR-01 through C-STOR-04, 4 scenarios):** These test storage backend failures. Scenarios include: storage write failure mid-blob (mock backend returns error after accepting half the data), storage read returns corrupted data (mock backend flips bits on read), storage quota exceeded (mock backend rejects writes above threshold), and storage unavailable then recovery (mock backend fails then becomes available). Each must assert: write failures are reported to the caller with the blob marked as unsent, read corruption is detected by hash verification, quota errors surface as actionable errors (not silent drops), recovery after storage failure does not skip any queued blobs.

**Collection chaos (C-COLL-01 through C-COLL-02, 2 scenarios):** These test collections of related blobs. Scenarios include: partial collection transfer (3 of 5 blobs in a collection arrive, other 2 fail) and collection metadata corruption (the manifest listing blob hashes is corrupted). Each must assert: partial collections are not presented as complete, metadata corruption is detected before any blob is served from the collection.

All 10 content chaos scenarios use mock storage backends, not real disk I/O. They run in-process alongside unit tests.

**Update Phase 3.5 checkpoint criteria** (Section 6.5.5) to include:

```bash
# Content chaos scenarios pass (mock storage, no Docker)
cargo test -p chaos-tests -E 'test(/^content/)'
cargo test -p chaos-tests -E 'test(/^sync::blob/)'
```

**Update Phase 3.5 checkpoint commit message** (Section 6.5.6) to mention: "Content chaos scenarios (10 mock-based: S-BLOB, C-STOR, C-COLL)"

**Verification:** After Phase 3.5 is complete, `cargo test -p chaos-tests -E 'test(/^content/)' -E 'test(/^sync::blob/)'` must pass all 10 scenarios. Mock storage must not perform real disk operations.

---

## Amendment 6: Add Sync Protocol Chaos Scenarios to Phase 4

**Location:** Phase 4 checkpoint (Section 7.5, lines 1413–1426)

**Context:** Phase 4 builds `sync-cli`, the headless testing tool. The chaos strategy (Section 15.2) specifies: "The CLI becomes the chaos test client driver. Write S-SM-*, S-CONC-*, S-CONV-* scenario logic using the CLI as the programmable client."

**What to do:** Add a new subsection after Phase 4's TDD Sequence (Section 7.3) and before the Validation Gate (Section 7.4).

Title it "7.3.1 Chaos Scenarios: Sync Protocol" and write:

With the CLI now available as a programmable client, write sync protocol chaos scenario logic. These scenarios define the assertions and orchestration for state machine, concurrency, and convergence testing. The scenario logic compiles and the assertions are complete, but scenarios that require a running relay cannot execute until Phase 6 brings up the Docker topology. Mark relay-dependent scenarios with `#[ignore = "requires relay (Phase 6)"]`.

Create `tests/chaos/src/scenarios/sync.rs` with the following scenario assertions:

**State machine chaos (S-SM-01 through S-SM-04, 4 scenarios):** These test the connection state machine under disruption. Scenarios include: disconnect during push (client loses connection while a push is in-flight), disconnect during pull (connection drops mid-pull-response), rapid connect/disconnect cycling (10 connect/disconnect cycles in 5 seconds), and state recovery after crash (client process killed, restarted, resumes from persisted state). Each must assert: no data loss after state machine recovers, pending operations are retried not silently dropped, cursor tracking remains consistent after reconnection.

**Concurrency chaos (S-CONC-01 through S-CONC-04, 4 scenarios):** These test concurrent operations from multiple clients. Scenarios include: simultaneous push from two clients (both push at the same instant), push during pull (Client A pushes while Client B is mid-pull), rapid sequential pushes (Client A pushes 100 blobs in 1 second), and three-client fan-out (Client A pushes, Clients B and C both pull simultaneously). Each must assert: all pushed blobs eventually arrive on all clients, no blob is duplicated, ordering within a single client's pushes is preserved.

**State convergence chaos (S-CONV-01 through S-CONV-04, 4 scenarios):** These test that clients reach identical state after disruption. Scenarios include: partition then reconciliation (Client A and Client B both push while partitioned, then reconnect), stale cursor recovery (client reconnects after long offline period with outdated cursor), version vector divergence (two clients' version vectors diverge then must converge), and selective pull (client pulls only blobs after a specific cursor, verifies no gaps). Each must assert: after chaos heals and sync quiesces, all clients have identical blob sets AND identical version vectors (Invariant 4), no silent data divergence.

All 12 sync chaos scenarios should have their assertion logic and orchestration code written now. Scenarios that can run with only mock transport (if any) should be runnable. Scenarios that require a relay must be marked `#[ignore]` with clear documentation that Phase 6 enables them.

**Update Phase 4 validation gate** (Section 7.4) to include:

```bash
# Sync chaos scenario logic compiles
cargo test -p chaos-tests -E 'test(/^sync/)' -- --ignored --list
```

**Update Phase 4 checkpoint commit message** (Section 7.5) to mention: "Sync chaos scenario logic (12 scenarios: S-SM, S-CONC, S-CONV — assertions written, relay-dependent tests #[ignore])"

**Verification:** After Phase 4 is complete, all 12 sync scenario signatures must exist in `tests/chaos/src/scenarios/sync.rs`. The code must compile. Relay-dependent scenarios must be clearly annotated as ignored pending Phase 6.

---

## Amendment 7: Add Full Chaos Activation to Phase 6

**Location:** Phase 6 section (Section 9, lines 1582–1650)

**Context:** Phase 6 builds `sync-relay`. The chaos strategy (Section 15.2) specifies: "This is where the full suite lights up. Wire all existing stubs and mocks to real Docker topology. Transport scenarios (T-*) go live. Adversarial scenarios (A-PROTO-*, A-RES-*) get implemented. Cross-platform (X-PLAT-*) follows when VM infrastructure is ready."

**What to do:** Add a new subsection after Phase 6's Test Strategy (Section 9.4) and before Section 10 (Testing Strategy).

Title it "9.5 Chaos Suite Activation" and write:

Phase 6 is where all 68 chaos scenarios become runnable against the real Docker topology. This is the most significant chaos testing milestone.

**Transport scenarios go live (16 scenarios):** Remove `#[ignore]` from all T-* scenarios (T-LAT-01 through T-LAT-04, T-LOSS-01 through T-LOSS-04, T-CONN-01 through T-CONN-05, T-BW-01 through T-BW-03). Wire them to the Docker topology using Toxiproxy for network manipulation and `tc netem` for kernel-level degradation. These require real TCP/QUIC connections between containers — mock transport is insufficient for testing actual network behavior.

**Adversarial scenarios (10 new scenarios):** Implement `tests/chaos/src/scenarios/adversarial.rs` with A-PROTO-01 through A-PROTO-05 (protocol violation: malformed messages, replay attacks, out-of-order operations, oversized payloads, unknown message types) and A-RES-01 through A-RES-05 (resource exhaustion: connection flooding, memory exhaustion via large payloads, file descriptor exhaustion, slow-client starvation, rapid reconnection storm). Each must assert: the relay rejects the attack, legitimate clients are not affected, the relay does not crash or leak resources.

**Cross-platform stubs (4 scenarios):** Create `tests/chaos/src/scenarios/cross_platform.rs` with X-PLAT-01 through X-PLAT-04 stubs. These require VM infrastructure that may not be available at Phase 6. Stub them with `#[ignore = "requires VM infrastructure"]`. Priority order per the chaos strategy: Linux-to-Linux first (always), Windows VM second (beta — highest platform-specific risk due to mandatory file locking), macOS VM third (RC), mobile deferred.

**Sync scenarios go live (12 scenarios):** Remove `#[ignore]` from all S-SM-*, S-CONC-*, S-CONV-* scenarios written in Phase 4. Wire them to the Docker topology with the relay running as a real container.

**Encryption scenarios graduate (16 scenarios):** The mock-based encryption scenarios from Phase 3 remain as-is for CI. Additionally, create "real topology" variants that run the same assertions against the Docker topology. This catches integration-level issues (buffer sizes, timeout interactions, flow control under real QUIC) that mocks miss. The mock versions run in CI (fast, no Docker). The real topology versions run on The Beast (slow, full fidelity). Both must pass.

**Docker topology buildout:** Populate `Dockerfile.relay` (building `sync-relay` binary) and `Dockerfile.cli` (building `sync-cli` binary) that were stubbed in Phase 1. Verify the `docker-compose.chaos.yml` Pair topology starts correctly. Add the Swarm topology variant for resource exhaustion scenarios (A-RES-*).

**Full validation run:** Execute all 68 scenarios, 50 iterations each, on The Beast. Target runtime: approximately 2 hours. All 68 must pass all iterations. One failure in 50 is a bug, not noise.

**Update Phase 6 validation gate to include:**

```bash
# Full chaos suite — smoke (CI-compatible, mock-based)
cargo test -p chaos-tests

# Full chaos suite — Docker topology (Beast only)
docker compose -f tests/chaos/docker-compose.chaos.yml build
cargo nextest run -p chaos-tests --no-capture

# Specific sections
cargo nextest run -p chaos-tests -E 'test(/^transport/)'
cargo nextest run -p chaos-tests -E 'test(/^encryption/)'
cargo nextest run -p chaos-tests -E 'test(/^sync/)'
cargo nextest run -p chaos-tests -E 'test(/^adversarial/)'
```

**Update Phase 6 checkpoint to mention:** "Full chaos suite operational — 68 scenarios across transport, encryption, sync, content, adversarial. Nightly runs on The Beast."

**Verification:** After Phase 6 is complete, `cargo nextest run -p chaos-tests` on The Beast must execute all 68 scenarios (minus X-PLAT-* if VM infra not ready). Zero failures across configured iteration counts.

---

## Amendment 8: Add Chaos Layer to Test Pyramid

**Location:** Section 10.1 "Test Pyramid" (lines 1655–1669) and Section 10.2 "Test Distribution" (lines 1671–1678)

**What to do:** The test pyramid currently has three layers: Unit, Integration, E2E. Chaos testing is a fourth dimension that cross-cuts these layers. Rather than adding it as a fourth pyramid layer (which would misrepresent its relationship), add it as a parallel dimension.

**After the existing test pyramid diagram, add a new subsection:**

Title it "10.1.1 Chaos Testing Dimension" and write:

Chaos testing is not a fourth layer in the pyramid — it is a parallel dimension that applies at multiple levels. Mock-based chaos (encryption, content) runs at the integration test level. Docker-topology chaos (transport, sync, adversarial) runs at the E2E level. The chaos strategy document (06-CHAOS-TESTING-STRATEGY.md) defines 68 scenarios across 6 categories with phase-by-phase authoring.

```
Standard Tests          Chaos Tests
─────────────          ───────────
Unit (fast)            [no chaos — pure logic]
Integration (medium)   Mock chaos: encryption, content (in-process)
E2E (slow)             Docker chaos: transport, sync, adversarial (Beast)
```

Mock-based chaos scenarios run in CI alongside integration tests. Docker-topology chaos runs nightly on The Beast. See 06-CHAOS-TESTING-STRATEGY.md for the full scenario inventory, iteration counts, and pass criteria.

**Update the Test Distribution table** (Section 10.2) to add a row:

| Layer | Location | Count | Speed |
|-------|----------|-------|-------|
| Chaos (mock) | `tests/chaos/src/scenarios/` | 26 | < 5 min (CI) |
| Chaos (Docker) | `tests/chaos/` + Docker | 68 total | ~2 hrs (Beast) |

**Verification:** Section 10 must reference 06-CHAOS-TESTING-STRATEGY.md and the two chaos test modes (mock/CI and Docker/Beast).

---

## Amendment 9: Add Chaos Smoke Job to CI Pipeline

**Location:** Section 11.2 "CI Pipeline" (lines 1924–1954)

**What to do:** The current CI YAML defines a single `test` job with formatting, Clippy, unit tests, integration tests, and a build step. Add a second job for chaos smoke testing, matching Section 12.2 of the chaos strategy.

**After the existing `test` job, add a `chaos-smoke` job:**

```yaml
  chaos-smoke:
    runs-on: ubuntu-latest
    needs: [test]  # Only run if standard tests pass
    services:
      toxiproxy:
        image: ghcr.io/shopify/toxiproxy:latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Mock-based chaos (encryption + content)
        run: cargo test -p chaos-tests -E 'test(/^encryption|^content|^sync::blob/)'

      - name: Smoke chaos (Docker topology, 3 key scenarios)
        run: |
          docker compose -f tests/chaos/docker-compose.chaos.yml up -d
          cargo nextest run -p chaos-tests -E 'test(packet_loss_20_percent) | test(handshake_disruption_retry) | test(concurrent_push_no_lost_writes)' --no-capture
          docker compose -f tests/chaos/docker-compose.chaos.yml down -v
```

The three smoke scenarios correspond to chaos strategy Section 12.2: T-LOSS-02, E-HS-01, S-CONC-01. Target: under 10 minutes total.

**Also add a `chaos-nightly` workflow file reference** (as a comment or note) pointing to the nightly Beast cron defined in chaos strategy Section 12.4. This does not run in GitHub Actions — it runs on The Beast via `scripts/chaos-run.sh`. But the implementation plan should acknowledge it exists.

**Verification:** The CI pipeline YAML in Section 11.2 must include a `chaos-smoke` job. The three smoke scenario names must match the chaos strategy's CI smoke selection.

---

## Amendment 10: Add Chaos Gate to Validation Checklist

**Location:** Section 11.1 "Gate Checklist (Every Phase)" (lines 1907–1920)

**What to do:** The current gate checklist has 8 items. Add a 9th for chaos testing that applies conditionally — not every phase produces chaos scenarios, but the check should be present so Q doesn't skip it.

**Add to the checklist:**

```markdown
- [ ] Chaos scenarios pass (if applicable for this phase — see 06-CHAOS-TESTING-STRATEGY.md Section 15)
```

**Verification:** The gate checklist must include a chaos line item.

---

## Amendment 11: Update Summary Table

**Location:** Section "Summary" (lines 1997–2008)

**What to do:** The summary table has two problems: it is missing Phase 3.5 (sync-content) and it has no chaos testing column. Fix both.

**Replace the existing summary table with:**

| Phase | Crate | Key Deliverable | Test Focus | Chaos Deliverables |
|-------|-------|-----------------|------------|--------------------|
| 1 | sync-types | Wire format | Serialization roundtrip | Harness skeleton (topology, Toxiproxy, Pumba wrappers) |
| 2 | sync-core | State machine | Pure logic (no I/O) | Assertion helpers (blob presence, data loss, convergence, plaintext detection) |
| 3 | sync-client | Client library | Encryption, transport | 16 encryption chaos scenarios (mock), 16 transport stubs (#[ignore]) |
| 3.5 | sync-content | Content transfer | Encrypt-then-hash, iroh-blobs | 10 content chaos scenarios (mock: S-BLOB, C-STOR, C-COLL) |
| 4 | sync-cli | Testing tool | E2E headless | 12 sync chaos scenarios (logic written, relay-dependent #[ignore]) |
| 5 | framework-integration | Example: Tauri plugin | Commands, events | None (framework-specific, not protocol-level) |
| 6 | sync-relay | Custom relay | Message routing | Full suite activation: 68 scenarios, Docker topology, nightly runs |

**Verification:** The summary table must include all 7 phases (1, 2, 3, 3.5, 4, 5, 6) and a "Chaos Deliverables" column that matches the chaos strategy's Section 15 phase mapping.

---

## Amendment 12: Update Document Version and Footer

**Location:** Document footer (line 2012)

**What to do:** The footer currently reads `Version: 2.0.0 | Date: 2026-01-16`. This was already flagged in the pre-flight audit as not matching the header version (2.1.0). Update to reflect these chaos integration amendments.

**Replace with:**

```
*Document: 03-IMPLEMENTATION-PLAN.md | Version: 2.2.0 | Date: 2026-02-03*
```

Also update the header version (line 3 area) to `Version: 2.2.0`.

**In the document's changelog or version history (if one exists), add an entry:**

```
v2.2.0 (2026-02-03): Integrated chaos testing deliverables from 06-CHAOS-TESTING-STRATEGY.md.
Added tests/chaos/ to project structure, chaos harness in Phases 1-2, encryption chaos in
Phase 3, content chaos in Phase 3.5, sync chaos in Phase 4, full activation in Phase 6.
Added chaos smoke CI job, chaos gate to validation checklist, chaos layer to test pyramid.
```

**Verification:** Header and footer versions must both read `2.2.0`. Date must be `2026-02-03`.

---

## Amendment 13: Add Cross-Reference in Chaos Strategy

**Location:** This is a change to `06-CHAOS-TESTING-STRATEGY.md`, not the implementation plan.

**What to do:** Section 15 of the chaos strategy references the implementation plan phases but does not note that the implementation plan cross-references back. After these amendments are applied, add a note at the top of Section 15 in the chaos strategy:

```
> **Cross-reference:** As of v2.2.0, 03-IMPLEMENTATION-PLAN.md includes chaos deliverables
> in each phase's validation gate and checkpoint, matching the phase-by-phase mapping below.
```

Update the chaos strategy version to `1.5.0` and date to `2026-02-03`.

**Verification:** Both documents must reference each other. The implementation plan references the chaos strategy by document name. The chaos strategy references the implementation plan by document name and version.

---

## Execution Order

Moneypenny should apply these amendments in this exact order, as later amendments depend on earlier structural changes:

1. **Amendment 1** — Project structure (creates the space)
2. **Amendment 2** — Workspace Cargo.toml (makes it a real crate)
3. **Amendment 3** — Phase 1–2 harness (builds the foundation)
4. **Amendment 4** — Phase 3 encryption chaos (first scenarios)
5. **Amendment 5** — Phase 3.5 content chaos (content scenarios)
6. **Amendment 6** — Phase 4 sync chaos (protocol scenarios)
7. **Amendment 7** — Phase 6 full activation (everything lights up)
8. **Amendment 8** — Test pyramid update (documentation alignment)
9. **Amendment 9** — CI pipeline update (automation)
10. **Amendment 10** — Validation checklist update (gate enforcement)
11. **Amendment 11** — Summary table update (overview alignment)
12. **Amendment 12** — Version and footer (bookkeeping)
13. **Amendment 13** — Cross-reference in chaos strategy (bidirectional linking)

---

## Verification Checklist (for Q)

After all amendments are applied, Q should verify:

- [ ] The word "chaos" appears in `03-IMPLEMENTATION-PLAN.md` (it currently appears zero times)
- [ ] `tests/chaos/` exists in the project structure diagram (Section 2.1)
- [ ] `tests/chaos` is a workspace member in `Cargo.toml` (Section 2.2)
- [ ] Phase 1 mentions chaos harness skeleton
- [ ] Phase 2 mentions chaos assertion helpers
- [ ] Phase 3 mentions 16 encryption scenarios and 16 transport stubs
- [ ] Phase 3.5 mentions 10 content chaos scenarios
- [ ] Phase 4 mentions 12 sync chaos scenarios
- [ ] Phase 5 explicitly notes no chaos deliverables (framework-level, not protocol)
- [ ] Phase 6 mentions full 68-scenario activation
- [ ] Test pyramid (Section 10) includes chaos dimension
- [ ] CI pipeline (Section 11.2) includes `chaos-smoke` job
- [ ] Validation gate checklist (Section 11.1) includes chaos line item
- [ ] Summary table includes Phase 3.5 AND a Chaos Deliverables column
- [ ] Header and footer versions both read `2.2.0`
- [ ] `06-CHAOS-TESTING-STRATEGY.md` has a cross-reference note in Section 15
- [ ] Total chaos scenario count across all phases sums to 68 (16 encryption + 16 transport + 10 content + 12 sync + 10 adversarial + 4 cross-platform)

---

*Document: CHAOS-INTEGRATION-AMENDMENTS.md | Version: 1.0.0 | Date: 2026-02-03*
