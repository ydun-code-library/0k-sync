# 0k-Sync — Release Strategy

**Version:** 1.0.0
**Date:** 2026-02-02
**Author:** James (LTIS Investments AB)
**Audience:** Maintainers, contributors, integrators
**Parent Documents:** 02-SPECIFICATION.md, 03-IMPLEMENTATION-PLAN.md, sync-mvp-roadmap.md

---

## Table of Contents

1. [Branding & Crate Naming](#1-branding--crate-naming)
2. [Versioning Strategy](#2-versioning-strategy)
3. [Release Milestones](#3-release-milestones)
4. [Crate Publishing](#4-crate-publishing)
5. [Distribution Channels](#5-distribution-channels)
6. [CI/CD Pipeline](#6-cicd-pipeline)
7. [Quality Gates](#7-quality-gates)
8. [Changelog & Documentation](#8-changelog--documentation)
9. [Breaking Change Policy](#9-breaking-change-policy)
10. [Security Response](#10-security-response)
11. [Open Source Launch](#11-open-source-launch)
12. [Dependency Management](#12-dependency-management)

---

## 1. Branding & Crate Naming

### 1.1 Product Name

**0k-Sync** (pronounced "zero-k sync"). The "0k" means **zero knowledge** — the relay never sees plaintext data.

Usage across contexts:

| Context | Name | Example |
|---------|------|---------|
| Product / marketing | 0k-Sync | "0k-Sync: zero-knowledge relay protocol" |
| Branded domain | `0ksync.io` | Redirects (302) to `ydun.io/0k-sync` |
| Landing page | `ydun.io/0k-sync` | Canonical content lives here |
| GitHub repository | `0k-sync` | `github.com/ydun-code-library/0k-sync` |
| Workspace directory | `0k-sync/` | Local development root |
| Documentation | 0k-Sync | All spec docs use this |
| Tactical/research context | relay-sync | Appendix D uses broader protocol name |

### 1.2 Crate Naming for crates.io

**Constraint:** crates.io requires the first character to be alphabetic. `0k-sync-types` is invalid.

**Decision required — two viable options:**

**Option A: `zerok-sync-*`** (recommended)

| Workspace Dir | crates.io Name | Rust Import |
|---------------|----------------|-------------|
| `sync-types/` | `zerok-sync-types` | `zerok_sync_types` |
| `sync-core/` | `zerok-sync-core` | `zerok_sync_core` |
| `sync-client/` | `zerok-sync-client` | `zerok_sync_client` |
| `sync-content/` | `zerok-sync-content` | `zerok_sync_content` |
| `sync-cli/` | `zerok-sync-cli` | (binary, not imported) |
| `sync-relay/` | `zerok-sync-relay` | (binary, not imported) |
| `tauri-plugin-sync/` | `tauri-plugin-zerok-sync` | `tauri_plugin_zerok_sync` |

Cargo.toml mapping:

```toml
# sync-types/Cargo.toml
[package]
name = "zerok-sync-types"  # crates.io name
publish = true

# Workspace root references by path (unaffected)
[dependencies]
zerok-sync-types = { path = "../sync-types" }
```

**Option B: `oksync-*`**

Shorter, but loses the "zero" meaning. `oksync-types`, `oksync-client`, etc. Less immediately recognisable as the 0k-Sync product.

**Recommendation:** Option A. `zerok` is unambiguous, searchable, and maps directly to the brand. Reserve the `zerok-sync` prefix on crates.io immediately upon first publish.

### 1.3 npm Package Naming

For JavaScript bindings (Tauri plugin guest-js, future WASM):

| Package | npm Name |
|---------|----------|
| Tauri plugin JS bindings | `@0k-sync/tauri-plugin` |
| Future WASM client | `@0k-sync/client` |

npm allows leading digits in scoped packages. The `@0k-sync` scope should be reserved on npmjs.com early.

### 1.4 Docker Image Naming

| Image | Registry | Name |
|-------|----------|------|
| Relay server | GitHub Container Registry | `ghcr.io/ydun-code-library/0k-sync-relay` |
| Relay server | Docker Hub (mirror) | `ydun/0k-sync-relay` |

---

## 2. Versioning Strategy

### 2.1 Semantic Versioning

All crates follow [SemVer 2.0.0](https://semver.org/). Given `MAJOR.MINOR.PATCH`:

- **MAJOR** — Breaking API changes (pre-1.0: any breaking change bumps MINOR)
- **MINOR** — New features, backward-compatible
- **PATCH** — Bug fixes, documentation, internal refactors

### 2.2 Workspace-Synchronized Versioning

All crates in the workspace share the same version number. This is simpler than independent versioning and appropriate for a tightly-coupled protocol implementation.

```toml
# Workspace root Cargo.toml
[workspace.package]
version = "0.1.0"
```

Rationale: sync-types, sync-core, sync-client, and sync-content are designed as a cohesive unit. A change to the wire format (sync-types) requires coordinated updates across all crates. Independent versioning would create a combinatorial compatibility nightmare.

Exception: `tauri-plugin-zerok-sync` may diverge from the core workspace version once stable, since it tracks both 0k-Sync and Tauri release cycles.

### 2.3 Pre-Release Tags

| Tag Format | Meaning | Example |
|------------|---------|---------|
| `X.Y.Z-alpha.N` | Feature-incomplete, API unstable | `0.1.0-alpha.1` |
| `X.Y.Z-beta.N` | Feature-complete, API stabilising | `0.1.0-beta.1` |
| `X.Y.Z-rc.N` | Release candidate, API frozen | `0.1.0-rc.1` |
| `X.Y.Z` | General availability | `0.1.0` |

### 2.4 Git Tagging

Every published version gets a git tag:

```
v0.1.0-alpha.1
v0.1.0-beta.1
v0.1.0-rc.1
v0.1.0
```

Phase-completion tags from development (e.g., `v0.1.0-phase1`) are internal milestones, not published releases.

---

## 3. Release Milestones

### 3.1 Milestone Definitions

**Alpha (v0.1.0-alpha.x)** — "It compiles and the tests pass."

Core protocol works. API will change. Not for production. Published to crates.io so early adopters and CashTable integration can begin.

| Criteria | Required |
|----------|----------|
| sync-types complete with round-trip tests | ✅ |
| sync-core state machine functional | ✅ |
| sync-client connects, pushes, pulls | ✅ |
| Hybrid Noise handshake (clatter) working | ✅ |
| E2E encryption verified | ✅ |
| sync-cli headless testing works | ✅ |
| API surface documented (`cargo doc`) | ✅ |
| CI passing on all commits | ✅ |

**Beta (v0.1.0-beta.x)** — "CashTable uses it."

First real consumer integrated. API stabilising based on real usage. Wire format frozen (backward-compatible changes only from this point).

| Criteria | Required |
|----------|----------|
| All alpha criteria | ✅ |
| CashTable integration working | ✅ |
| Wire format frozen | ✅ |
| Pairing flow (QR + short code) complete | ✅ |
| Mobile lifecycle handling tested | ✅ |
| Reconnection with jitter verified | ✅ |
| sync-relay server running on Beast | ✅ |
| Docker image published | ✅ |
| Observability hooks present | ✅ |

**Release Candidate (v0.1.0-rc.x)** — "We trust it with real data."

API frozen. Only bug fixes from this point. External testing invited.

| Criteria | Required |
|----------|----------|
| All beta criteria | ✅ |
| API frozen (no new public surface) | ✅ |
| No known data-loss bugs | ✅ |
| E2E test suite comprehensive | ✅ |
| CHANGELOG complete | ✅ |
| README with quick-start guide | ✅ |
| Performance baseline established | ✅ |
| Two weeks of real-world CashTable usage | ✅ |

**General Availability (v0.1.0)** — "Ship it."

| Criteria | Required |
|----------|----------|
| All RC criteria | ✅ |
| Zero P0 bugs open | ✅ |
| Security audit of clatter integration patterns | ✅ |
| Migration guide from RC (if breaking changes occurred) | ✅ |

### 3.2 Timeline Alignment with MVP Roadmap

| Week | Implementation Phase | Release Milestone |
|------|---------------------|-------------------|
| 1–2 | sync-types + sync-core | Internal only (no publish) |
| 3 | sync-client + clatter | **v0.1.0-alpha.1** to crates.io |
| 4 | tauri-plugin-sync | v0.1.0-alpha.2 (with framework bindings) |
| 5 | Pairing + polish | v0.1.0-alpha.3 |
| 6 | CashTable integration | **v0.1.0-beta.1** |
| 8–10 | Real-world testing | **v0.1.0-rc.1** |
| 10+ | Stabilisation | **v0.1.0** GA |

---

## 4. Crate Publishing

### 4.1 Publishing Order

Crates must be published in dependency order. A single `cargo publish` won't handle workspace interdependencies.

```
1. zerok-sync-types      (no internal deps)
2. zerok-sync-core       (depends on types)
3. zerok-sync-content    (depends on types)
4. zerok-sync-client     (depends on types, core, content)
5. zerok-sync-cli        (depends on client) — binary only
6. zerok-sync-relay      (depends on types) — binary only
7. tauri-plugin-zerok-sync (depends on client)
```

Wait for crates.io index propagation (~60 seconds) between each publish.

### 4.2 Publishing Script

```bash
#!/bin/bash
# scripts/publish.sh — Publish all crates in dependency order
set -euo pipefail

VERSION=$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "zerok-sync-types") | .version')
echo "Publishing 0k-Sync v${VERSION}"

CRATES=(
    sync-types
    sync-core
    sync-content
    sync-client
    sync-cli
    sync-relay
    tauri-plugin-sync
)

for crate in "${CRATES[@]}"; do
    echo "Publishing ${crate}..."
    cargo publish -p "zerok-${crate}" --allow-dirty
    echo "Waiting for index propagation..."
    sleep 90
done

echo "All crates published. Tagging..."
git tag -a "v${VERSION}" -m "Release v${VERSION}"
git push origin "v${VERSION}"
```

### 4.3 What Gets Published

| Crate | Published to crates.io | Reason |
|-------|----------------------|--------|
| zerok-sync-types | ✅ Yes | Library — used by downstream apps |
| zerok-sync-core | ✅ Yes | Library — used by downstream apps |
| zerok-sync-content | ✅ Yes | Library — used by downstream apps |
| zerok-sync-client | ✅ Yes | **Primary integration point** for developers |
| zerok-sync-cli | ✅ Yes | Binary — `cargo install zerok-sync-cli` for testing |
| zerok-sync-relay | ✅ Yes | Binary — `cargo install zerok-sync-relay` for self-hosting |
| tauri-plugin-zerok-sync | ✅ Yes | Library — Tauri developers add via `cargo add` |

All crates are published. The cli and relay are binaries installable via `cargo install`. The libraries are the developer integration surface.

### 4.4 Crate Categories and Keywords

```toml
# Shared across workspace
[package]
categories = ["cryptography", "network-programming"]
keywords = ["sync", "encryption", "privacy", "local-first", "zero-knowledge"]
```

---

## 5. Distribution Channels

### 5.1 Channel Matrix

| Channel | What | Audience | When |
|---------|------|----------|------|
| **ydun.io/0k-sync** | Landing page + docs links | Everyone | Launch (GA) |
| **ydun.io** | Product listing, links, context | Visitors to Ydun.io | Launch (GA) |
| **GitHub** (`ydun-code-library/0k-sync`) | Public repo, source, issues | Contributors, developers | Alpha onwards |
| **GitHub Releases** | Binaries (cli, relay) + changelog | All developers | Every release |
| **crates.io** | All Rust crates | Rust developers | Every release |
| **GHCR** | `0k-sync-relay` Docker image | Self-hosters (Tier 2–3) | Every release |
| **npm** | `@0k-sync/tauri-plugin` | Tauri JS developers | Every release with JS changes |

### 5.2 Web Presence

**Dedicated landing page:** `ydun.io/0k-sync` (or subdomain `0k-sync.ydun.io` — decide at launch)

The landing page serves as the public-facing entry point for developers and evaluators. It should convey the value proposition without requiring them to read the full specification. Content:

- One-line pitch: "Zero-knowledge sync for local-first apps. Your relay sees nothing."
- Architecture overview diagram (the 3-layer stack from exec summary)
- Quick-start code snippet (`cargo add zerok-sync-client`)
- Links to: GitHub repo, crates.io, API docs (`docs.rs`), Docker image
- Protocol summary: what's encrypted, what the relay sees (nothing), what the client controls (everything)
- Use case examples: Tauri apps, cross-device sync, self-hosted relays
- License badge (MIT/Apache-2.0)

**ydun.io integration:** 0k-Sync listed as a product/project on the main Ydun.io site with brief description and link to the landing page. Positioned within the Ydun.io portfolio alongside Private Suite and other projects.

**Domain:** `0ksync.io` — registered as the branded short URL. Redirects (302 temporary) to `ydun.io/0k-sync`. All external-facing URLs (crate metadata, GitHub About, README badges, conference slides) use `0ksync.io` as the canonical link. Content stays on ydun.io — one site to maintain, full SEO weight on the parent domain. Use 302 (not 301) so the redirect destination can change later without fighting browser/search engine caches. If the project ever warrants a standalone site, the domain is already in circulation and just needs repointing.

**Documentation hosting:** Primary API docs via `docs.rs` (auto-generated from crate publishes). Protocol specification and guides hosted in the GitHub repo `/docs` directory, linked from the landing page.

### 5.3 GitHub Releases

Every tagged release gets a GitHub Release with:
- Pre-built binaries for: `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
- SHA256 checksums file
- CHANGELOG excerpt for the version
- Docker image link

### 5.4 Docker Distribution

```dockerfile
# Multi-stage build
FROM rust:1.76-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build -p zerok-sync-relay --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/zerok-sync-relay /usr/local/bin/
EXPOSE 8080
CMD ["zerok-sync-relay", "--config", "/etc/0k-sync/relay.toml"]
```

Image tags:

| Tag | Meaning |
|-----|---------|
| `latest` | Latest stable GA release |
| `0.1.0` | Specific version |
| `0.1.0-beta.1` | Pre-release (for testing) |
| `edge` | Latest commit on `main` (CI-built, unstable) |

---

## 6. CI/CD Pipeline

### 6.1 Pipeline Architecture

Three tiers, optimised for cost (GitHub Actions minutes are finite):

**Tier 1 — Every Push (fast, <5 min)**

Runs on every push to any branch. Catches obvious breakage immediately.

```yaml
name: CI
on: [push]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Check formatting
        run: cargo fmt --all --check
      - name: Clippy
        run: cargo clippy --workspace -- -D warnings
      - name: Unit tests
        run: cargo nextest run --workspace --lib
```

**Tier 2 — Pull Requests (thorough, <15 min)**

Full test suite including integration tests. Gate for merging to `main`.

```yaml
name: PR
on: [pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Full test suite
        run: cargo nextest run --workspace
        env:
          RUN_NETWORK_TESTS: "1"
      - name: Build all targets
        run: cargo build --workspace --release
      - name: Doc tests
        run: cargo test --workspace --doc
      - name: Security audit
        run: cargo audit
```

**Tier 3 — Release (comprehensive, <30 min)**

Triggered by version tags. Cross-platform builds, publishing, Docker image.

```yaml
name: Release
on:
  push:
    tags: ['v*']
jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Build
        run: cargo build -p zerok-sync-cli -p zerok-sync-relay --release --target ${{ matrix.target }}
      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: binaries-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/zerok-sync-*

  publish-crates:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Publish to crates.io
        run: ./scripts/publish.sh
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

  publish-docker:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build and push Docker image
        uses: docker/build-push-action@v5
        with:
          push: true
          tags: ghcr.io/ydun-code-library/0k-sync-relay:${{ github.ref_name }}

  github-release:
    needs: [build, publish-crates]
    runs-on: ubuntu-latest
    steps:
      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          generate_release_notes: true
          files: binaries-*/zerok-sync-*
```

### 6.2 Branch Strategy

| Branch | Purpose | Merge Target | CI Tier |
|--------|---------|-------------|---------|
| `main` | Stable, releasable | — | Tier 1 + Tier 2 |
| `dev/*` | Feature development | `main` via PR | Tier 1 |
| `release/v*` | Release preparation | `main` via PR | Tier 2 |
| `hotfix/*` | Critical fixes | `main` via PR | Tier 2 |

Tags on `main` trigger Tier 3.

### 6.3 Caching Strategy

Rust builds are slow. Cache aggressively:

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: "ci-${{ matrix.target }}"
    cache-on-failure: true
```

Cache the `~/.cargo/registry`, `~/.cargo/git`, and `target/` directories. Expected savings: 3–8 minutes per job.

---

## 7. Quality Gates

### 7.1 Gate Matrix

Every gate must pass before the corresponding release milestone is published:

| Gate | Alpha | Beta | RC | GA |
|------|-------|------|-----|-----|
| `cargo fmt --check` | ✅ | ✅ | ✅ | ✅ |
| `cargo clippy -- -D warnings` | ✅ | ✅ | ✅ | ✅ |
| Unit tests pass | ✅ | ✅ | ✅ | ✅ |
| Integration tests pass | ✅ | ✅ | ✅ | ✅ |
| `cargo doc` builds without warnings | ✅ | ✅ | ✅ | ✅ |
| `cargo audit` — no known vulnerabilities | — | ✅ | ✅ | ✅ |
| Wire format backward-compatibility test | — | ✅ | ✅ | ✅ |
| CashTable integration smoke test | — | ✅ | ✅ | ✅ |
| Cross-platform build succeeds (4 targets) | — | — | ✅ | ✅ |
| Performance regression test (<10% drift) | — | — | ✅ | ✅ |
| CHANGELOG reviewed and complete | — | — | ✅ | ✅ |
| Zero P0 bugs | — | — | — | ✅ |
| Security pattern review | — | — | — | ✅ |

### 7.2 Automated Gate Enforcement

Quality gates are enforced via GitHub branch protection on `main`:

- Required status checks: Tier 1 + Tier 2 CI jobs
- Required reviews: 1 (self-review acceptable for solo development; raise to 2 when team grows)
- No force pushes to `main`
- Linear history (squash merge)

### 7.3 Manual Checkpoints

Before each milestone publication, the maintainer runs through a manual checklist:

```markdown
## Release Checklist — v0.1.0-{milestone}

- [ ] All CI green on the release commit
- [ ] CHANGELOG.md updated with release notes
- [ ] Version bumped in workspace Cargo.toml
- [ ] `cargo publish --dry-run` succeeds for all crates
- [ ] Quick smoke test: build CLI, connect to local relay, push/pull a blob
- [ ] Git tag created and pushed
- [ ] GitHub Release drafted with changelog excerpt
```

---

## 8. Changelog & Documentation

### 8.1 Changelog Format

Follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/):

```markdown
# Changelog

## [Unreleased]

### Added
- New feature description

### Changed
- Modified behaviour description

### Fixed
- Bug fix description

### Security
- Security fix description (with advisory link if applicable)

## [0.1.0-alpha.1] — 2026-XX-XX

### Added
- Initial release: sync-types, sync-core, sync-client, sync-cli, sync-relay
- Hybrid Noise handshake via clatter (ML-KEM-768 + X25519)
- Content-addressed blob storage via iroh-blobs
- E2E encryption with XChaCha20-Poly1305
```

### 8.2 Documentation Requirements Per Release

| Document | Alpha | Beta | GA |
|----------|-------|------|-----|
| API docs (`cargo doc`) | ✅ | ✅ | ✅ |
| README with quick-start | ✅ | ✅ | ✅ |
| CHANGELOG | ✅ | ✅ | ✅ |
| Protocol specification (in-repo) | Draft | Stable | Final |
| Integration guide (Tauri) | — | ✅ | ✅ |
| Self-hosting guide (relay) | — | ✅ | ✅ |
| Landing page content | — | Draft | ✅ |
| Architecture decision records | As needed | As needed | As needed |

### 8.3 Commit Convention

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(sync-client): add reconnection with exponential backoff
fix(sync-types): correct MessagePack deserialization for empty vaults
docs(readme): add quick-start guide
chore(ci): add cross-platform build matrix
test(sync-core): add property-based state machine tests
security(clatter): update ML-KEM-768 parameter validation
```

Scope is the crate name without the `zerok-` prefix. This feeds directly into automated changelog generation.

---

## 9. Breaking Change Policy

### 9.1 Pre-1.0 (Current Phase)

Before reaching 1.0, breaking changes are expected and allowed:

- Any breaking change bumps the MINOR version (e.g., `0.1.0` → `0.2.0`)
- Breaking changes documented in CHANGELOG under a clear **Breaking** heading
- Migration guidance provided inline or in a separate migration guide
- Wire format changes after beta require a format version bump and backward-compatibility shim for one release

### 9.2 Post-1.0 (Future)

After 1.0, stricter rules apply:

- Breaking changes require a MAJOR version bump
- Deprecation before removal: deprecated items must survive at least one MINOR release with compiler warnings before removal in the next MAJOR
- Wire format changes handled via version negotiation (already designed into the protocol header)

### 9.3 What Counts as Breaking

| Change | Breaking? |
|--------|-----------|
| Removing a public function/type | ✅ Yes |
| Changing a function signature | ✅ Yes |
| Adding a required field to a public struct | ✅ Yes |
| Changing wire format encoding | ✅ Yes |
| Adding a new optional field | ❌ No |
| Adding a new public function | ❌ No |
| Internal refactors (no public API change) | ❌ No |
| Changing error message text | ❌ No |
| Bumping MSRV | ⚠️ Minor (document in CHANGELOG) |

### 9.4 Wire Format Stability

The wire format (MessagePack messages defined in sync-types) has its own stability lifecycle:

| Phase | Wire Format Status | Implication |
|-------|-------------------|-------------|
| Alpha | Unstable | Can change freely between alpha releases |
| Beta | Frozen | Only additive changes (new optional fields) |
| RC/GA | Stable | Version negotiation required for any change |

---

## 10. Security Response

### 10.1 Reporting

Security vulnerabilities should be reported via GitHub Security Advisories (private disclosure) on the `ydun-code-library/0k-sync` repository. Do not file public issues for security vulnerabilities.

Contact: security@ydun.io (forwarded to James)

### 10.2 Response Timeline

| Severity | Response | Fix | Disclosure |
|----------|----------|-----|------------|
| Critical (data exposure, key leak) | 24 hours | 72 hours | After fix published |
| High (auth bypass, DoS) | 48 hours | 1 week | After fix published |
| Medium (information leak, edge case) | 1 week | 2 weeks | After fix published |
| Low (minor, theoretical) | 2 weeks | Next release | With fix |

### 10.3 RUSTSEC Advisory Database

For vulnerabilities in 0k-Sync crates, file an advisory with [RustSec](https://rustsec.org/):

1. Create advisory in `rustsec/advisory-db` via PR
2. Reference the CVE if assigned
3. Include affected version range and patched version
4. `cargo audit` will then flag the vulnerability for all downstream users

### 10.4 Dependency Vulnerability Response

When `cargo audit` flags a dependency vulnerability:

- **Critical/High in direct dependency:** Patch or pin within 72 hours, publish patch release
- **Critical/High in transitive dependency:** Update `Cargo.lock`, publish patch release
- **Medium/Low:** Address in next scheduled release
- **No fix available:** Document mitigation, consider alternative dependency, notify users via advisory

### 10.5 Security-Critical Dependencies

These dependencies require extra scrutiny. Any update must be reviewed manually, not auto-merged:

| Dependency | Role | Risk |
|------------|------|------|
| `clatter` | Hybrid Noise handshake | Cryptographic correctness |
| `fips203` (via clatter) | ML-KEM-768 | Post-quantum security |
| `chacha20poly1305` | Symmetric encryption | Data confidentiality |
| `argon2` | Key derivation | Key security |
| `iroh` | Network transport | Connection security |

---

## 11. Open Source Launch

### 11.1 Repository Setup

**Organisation:** `ydun-code-library` on GitHub
**Repository:** `ydun-code-library/0k-sync` — public from day one (alpha onwards)
**License:** Dual MIT/Apache-2.0 (standard Rust ecosystem choice)

Repository structure at launch:

```
0k-sync/
├── .github/
│   ├── workflows/          # CI/CD pipelines
│   ├── ISSUE_TEMPLATE/     # Bug report, feature request templates
│   ├── PULL_REQUEST_TEMPLATE.md
│   └── SECURITY.md         # Vulnerability reporting instructions
├── docs/
│   ├── 01-EXECUTIVE-SUMMARY.md
│   ├── 02-SPECIFICATION.md
│   ├── 03-IMPLEMENTATION-PLAN.md
│   ├── 04-RESEARCH-VALIDATION.md
│   └── architecture/       # Diagrams, decision records
├── sync-types/
├── sync-core/
├── sync-client/
├── sync-content/
├── sync-cli/
├── sync-relay/
├── tauri-plugin-sync/
├── scripts/
│   └── publish.sh
├── Cargo.toml              # Workspace root
├── Cargo.lock
├── README.md
├── CHANGELOG.md
├── LICENSE-MIT
├── LICENSE-APACHE
└── CONTRIBUTING.md
```

### 11.2 Web Presence & Landing Page

**Landing page:** `ydun.io/0k-sync` — dedicated page for the project

Launch timing: landing page goes live at **beta** (when CashTable demonstrates real usage). Content exists in draft form from alpha. The page serves as the canonical entry point for developers discovering the project.

Page content sections:

1. **Hero:** One-liner + protocol architecture diagram
2. **Why 0k-Sync:** Zero-knowledge by design, local-first, post-quantum ready
3. **Quick Start:** `cargo add zerok-sync-client` + 10-line integration example
4. **How It Works:** Encrypted blob diagram — what the relay sees (nothing) vs. what clients control (everything)
5. **Self-Host:** Docker one-liner for running your own relay
6. **Links:** GitHub repo, crates.io, docs.rs, npm, Docker image
7. **Built by Ydun.io:** Connection to the broader Ydun.io portfolio

**ydun.io main site integration:**

0k-Sync listed as a project on ydun.io with:
- Brief description (1–2 sentences)
- Technology badges (Rust, post-quantum, local-first)
- Status indicator (alpha/beta/stable)
- Link to dedicated landing page
- Link to GitHub repository

### 11.3 Launch Sequence

| Phase | Action | Public Visibility |
|-------|--------|-------------------|
| Pre-alpha | Private development, docs only | None |
| **Alpha** | Push to `ydun-code-library/0k-sync` (public), first crates.io publish | GitHub repo public, crates.io indexed |
| Alpha → Beta | Announce on Ydun.io blog/social | Developers discover via search |
| **Beta** | Landing page live on ydun.io, README polished | Discoverable, inviting contributions |
| RC | Announce to Tauri community, Rust subreddit | Broader developer awareness |
| **GA** | Full launch: landing page finalised, all channels active | Production-ready messaging |

### 11.4 Community & Contribution

**CONTRIBUTING.md** covers:

- How to set up the development environment
- How to run tests (`cargo nextest run --workspace`)
- Commit convention (Conventional Commits)
- PR process (fork → branch → PR → review)
- Code of conduct reference
- Where to ask questions (GitHub Discussions)

**Issue labels:**

| Label | Meaning |
|-------|---------|
| `good-first-issue` | Suitable for new contributors |
| `help-wanted` | Actively seeking contributions |
| `bug` | Confirmed bug |
| `enhancement` | Feature request |
| `security` | Security-related (use private disclosure for vulnerabilities) |
| `documentation` | Documentation improvements |
| `breaking-change` | Will require a version bump |

### 11.5 Licensing

Dual-licensed under MIT and Apache-2.0. Every source file includes the SPDX header:

```rust
// SPDX-License-Identifier: MIT OR Apache-2.0
```

This is the de facto standard for the Rust ecosystem and maximises compatibility with downstream projects.

---

## 12. Dependency Management

### 12.1 Pinning Strategy

| Dependency Type | Strategy | Rationale |
|----------------|----------|-----------|
| **Security-critical** (clatter, fips203, chacha20poly1305, argon2) | Exact pin (`=X.Y.Z`) | Cryptographic code must not change without explicit review |
| **Core infrastructure** (iroh, iroh-blobs) | Compatible range (`~X.Y`) | Allow patch updates, review minor updates |
| **Framework** (tauri) | Compatible range (`~X.Y`) | Track Tauri releases |
| **Dev dependencies** (nextest, criterion) | Caret range (`^X.Y`) | Latest compatible, less critical |
| **Build dependencies** (serde, tokio) | Caret range (`^X.Y`) | Broad ecosystem compatibility |

### 12.2 Update Schedule

| Cadence | Action |
|---------|--------|
| **Daily** | `cargo audit` in CI (automated) |
| **Weekly** | Review Dependabot PRs, merge non-security patches |
| **Monthly** | Full `cargo update`, review all changes, run full test suite |
| **On advisory** | Immediate patch for critical/high severity |

### 12.3 Minimum Supported Rust Version (MSRV)

MSRV tracks the latest stable Rust minus 2 releases. At time of writing: **Rust 1.83+** (adjustable as the project matures).

MSRV is enforced in CI:

```yaml
- uses: dtolnay/rust-toolchain@1.83
```

MSRV bumps are documented in CHANGELOG and considered a minor breaking change.

### 12.4 Lock File Policy

`Cargo.lock` is committed to the repository. This is standard practice for applications and workspace projects (as opposed to standalone libraries). It ensures reproducible builds across all environments.

### 12.5 Dependency Audit Criteria

Before adding any new dependency, evaluate:

1. **Maintenance:** Active maintainer(s), recent commits, responsive to issues
2. **Security surface:** Does it handle untrusted input? Does it do crypto?
3. **Transitive weight:** How many transitive dependencies does it pull in?
4. **License compatibility:** Must be MIT, Apache-2.0, BSD, or similarly permissive
5. **Alternatives:** Is there a lighter or better-maintained option?

For security-critical dependencies (crypto, networking), additionally verify:
- Published audit history
- Known CVE response track record
- Maintainer identity (not anonymous for crypto code)

---

## Appendix A: Quick Reference

### Release Checklist (copy-paste for each release)

```markdown
## Release v{VERSION}

### Pre-flight
- [ ] All CI green on release commit
- [ ] `cargo publish --dry-run` succeeds for all crates
- [ ] CHANGELOG.md updated
- [ ] Version bumped in workspace Cargo.toml
- [ ] README quick-start verified

### Publish
- [ ] Run `scripts/publish.sh`
- [ ] Verify crates appear on crates.io
- [ ] Create git tag: `git tag -a v{VERSION} -m "Release v{VERSION}"`
- [ ] Push tag: `git push origin v{VERSION}`
- [ ] Verify GitHub Actions release pipeline completes
- [ ] Verify Docker image published to GHCR

### Post-publish
- [ ] GitHub Release created with changelog excerpt
- [ ] Landing page updated (if applicable)
- [ ] ydun.io project status updated (if milestone change)
- [ ] Announce (if beta/RC/GA milestone)
```

### URL Reference

| Resource | URL |
|----------|-----|
| **Branded Domain** | `0ksync.io` → redirects to landing page |
| GitHub Repository | `github.com/ydun-code-library/0k-sync` |
| Landing Page | `ydun.io/0k-sync` |
| crates.io (types) | `crates.io/crates/zerok-sync-types` |
| crates.io (client) | `crates.io/crates/zerok-sync-client` |
| API Documentation | `docs.rs/zerok-sync-client` |
| npm (Tauri plugin) | `npmjs.com/package/@0k-sync/tauri-plugin` |
| Docker Image | `ghcr.io/ydun-code-library/0k-sync-relay` |
| Security Reporting | `github.com/ydun-code-library/0k-sync/security` |

---

*Document: 05-RELEASE-STRATEGY.md | Version: 1.0.0 | Date: 2026-02-02*