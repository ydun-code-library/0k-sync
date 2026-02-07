# 0k-Sync — Distributed Testing Guide

**Version:** 1.0.0
**Date:** 2026-02-07
**Author:** Q (Mac Mini)
**Audience:** Maintainers, operators, CI pipeline
**Parent Documents:** 06-CHAOS-TESTING-STRATEGY.md, E2E-TESTING-GUIDE.md

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture](#2-architecture)
3. [Permanent Relay Setup](#3-permanent-relay-setup)
4. [Running Tests](#4-running-tests)
5. [Relay Observability](#5-relay-observability)
6. [Test Categories](#6-test-categories)
7. [Adding New Tests](#7-adding-new-tests)
8. [Troubleshooting](#8-troubleshooting)
9. [File Reference](#9-file-reference)

---

## 1. Overview

Distributed testing validates 0k-sync across real machines, real networks, and real latency — not simulated chaos on a Docker bridge. The test infrastructure spans three machines on a Tailscale mesh:

- **Q** (Mac Mini, macOS) — Test orchestrator, runs `cargo test`
- **Beast** (91GB server, Linux) — Hosts 3 relay instances in Docker
- **Guardian** (Raspberry Pi, ARM Linux) — Edge device under test

There are two tiers of chaos tests:

| Tier | Tests | Where They Run | What They Test |
|------|-------|----------------|----------------|
| Single-host Docker | 28 scenarios | Beast only | Toxiproxy-mediated chaos (latency, loss, partition) |
| Distributed | 37 scenarios | Q → Beast + Guardian | Real multi-machine sync, relay failover, edge device behavior |

Both tiers coexist. Single-host tests use `docker-compose.chaos.yml`. Distributed tests use `docker-compose.distributed.yml`.

---

## 2. Architecture

```
Q (100.114.70.54) — Mac Mini, macOS
  ├── Test Orchestrator (cargo test -p chaos-tests)
  └── Client-Q (native sync-cli binary, built locally)

Beast (100.71.79.25) — 91GB RAM, Linux 6.8.0
  ├── Relay-1 (Docker, HTTP :8090, QUIC ephemeral)
  ├── Relay-2 (Docker, HTTP :8091, QUIC ephemeral)
  ├── Relay-3 (Docker, HTTP :8092, QUIC ephemeral)
  └── Client-Beast (Docker container, sleep infinity)

Guardian (100.115.186.91) — Raspberry Pi, Linux ARM
  └── Client-Guardian (cross-compiled aarch64 binary)
```

**Network:** Tailscale mesh. All nodes directly routable. Traffic goes over real internet (Tailscale WireGuard tunnels). iroh endpoints publish via Pkarr/DNS discovery.

**Key design decisions:**

1. **Permanent relays.** The 3 relays on Beast start once and stay running. Tests connect to them, not start them. This enables rapid iteration, load testing, and cost analysis.

2. **Per-test isolation via passphrase.** Each test generates a unique passphrase, creating a unique sync group. Multiple tests can run against the same relays without data collision.

3. **SSH orchestration.** All remote commands run via `tokio::process::Command` shelling out to `ssh`. No SSH crate needed — Tailscale handles authentication.

4. **ARM cross-compilation on Beast.** Guardian's binary is cross-compiled on Beast (Linux → ARM Linux via `cross`), then SCP'd to Guardian. Q is macOS and can't cross-compile for ARM Linux easily.

---

## 3. Permanent Relay Setup

### 3.1 Starting Relays (One-Time)

From Q or any machine with SSH access to Beast:

```bash
ssh jimmyb@100.71.79.25 "cd ~/0k-sync && git pull && \
  docker compose -f tests/chaos/docker-compose.distributed.yml \
  -p dist-chaos up -d --build --wait"
```

This builds 3 relay images and starts them with:
- `RUST_LOG=sync_relay=debug,iroh=warn` — full debug logging
- `NET_ADMIN` capability — for tc netem chaos injection
- Health checks on each relay (5s interval, 3s timeout, 5 retries)

Verify all 3 are healthy:

```bash
ssh jimmyb@100.71.79.25 "curl -s http://localhost:8090/health && echo && \
  curl -s http://localhost:8091/health && echo && \
  curl -s http://localhost:8092/health"
```

### 3.2 Discovering Endpoint IDs

Each relay logs its iroh Endpoint ID at startup:

```bash
ssh jimmyb@100.71.79.25 "docker compose -p dist-chaos logs relay-1 2>&1 | grep 'Endpoint ID'"
ssh jimmyb@100.71.79.25 "docker compose -p dist-chaos logs relay-2 2>&1 | grep 'Endpoint ID'"
ssh jimmyb@100.71.79.25 "docker compose -p dist-chaos logs relay-3 2>&1 | grep 'Endpoint ID'"
```

The test harness discovers these automatically — you only need to check manually when debugging.

### 3.3 Stopping Relays

```bash
ssh jimmyb@100.71.79.25 "cd ~/0k-sync && \
  docker compose -f tests/chaos/docker-compose.distributed.yml \
  -p dist-chaos down -v --remove-orphans"
```

### 3.4 Rebuilding After Code Changes

```bash
ssh jimmyb@100.71.79.25 "cd ~/0k-sync && git pull && \
  docker compose -f tests/chaos/docker-compose.distributed.yml \
  -p dist-chaos up -d --build --wait"
```

The `--build` flag rebuilds the Docker images. The `--wait` flag blocks until all health checks pass.

---

## 4. Running Tests

All distributed tests are annotated `#[ignore = "requires distributed"]` and run from Q.

### 4.1 Prerequisites

1. **3 relays running on Beast** (see Section 3.1)
2. **SSH keys configured:** `ssh jimmyb@100.71.79.25` and `ssh jamesb@100.115.186.91` must work without password prompts
3. **sync-cli built locally:** `cargo build -p zerok-sync-cli --release`
4. **Guardian binary available:** The harness handles this automatically (cross-compiles on Beast, SCPs to Guardian)

### 4.2 Run All Distributed Tests

```bash
cargo test -p chaos-tests distributed -- --ignored
```

This runs all 37 distributed tests: 5 SSH primitives, 16 infrastructure tests, 16 scenario tests.

### 4.3 Run Specific Test Categories

```bash
# SSH primitives only
cargo test -p chaos-tests distributed::ssh -- --ignored

# Harness infrastructure tests
cargo test -p chaos-tests distributed::harness -- --ignored

# Multi-relay failover scenarios
cargo test -p chaos-tests mr_ -- --ignored

# Cross-machine sync scenarios
cargo test -p chaos-tests cm_ -- --ignored

# Edge device (Guardian) scenarios
cargo test -p chaos-tests edge_ -- --ignored

# Network partition & convergence
cargo test -p chaos-tests net_ -- --ignored
cargo test -p chaos-tests conv_ -- --ignored
```

### 4.4 Run a Single Test

```bash
cargo test -p chaos-tests mr_01_relay_crash_failover -- --ignored
```

### 4.5 Run All Chaos Tests (Single-Host + Distributed)

This only works on Beast (single-host tests require Docker):

```bash
cargo test -p chaos-tests -- --ignored
```

### 4.6 Run Non-Ignored Tests (Unit Tests Only)

```bash
cargo test -p chaos-tests
```

This runs the pure unit tests (SSH parsing, endpoint ID extraction, etc.) without any infrastructure.

---

## 5. Relay Observability

### 5.1 Health Endpoint

Each relay exposes `/health` on its HTTP port:

```bash
curl -s http://100.71.79.25:8090/health | python3 -m json.tool
```

Response:

```json
{
    "status": "ok",
    "version": "0.1.0",
    "connections": 3,
    "groups": 2,
    "uptime_seconds": 7200,
    "total_blobs": 150,
    "storage_bytes": 51200,
    "groups_with_data": 5
}
```

| Field | Meaning |
|-------|---------|
| `connections` | Currently active QUIC sessions |
| `groups` | Groups with active sessions |
| `uptime_seconds` | Seconds since relay started |
| `total_blobs` | Blobs stored in SQLite |
| `storage_bytes` | Total ciphertext bytes in database |
| `groups_with_data` | Distinct groups with stored data |

### 5.2 Prometheus Metrics

Each relay exposes `/metrics` in Prometheus text format:

```bash
curl -s http://100.71.79.25:8090/metrics
```

**Gauges (current state):**

| Metric | Description |
|--------|-------------|
| `sync_relay_connections_active` | Active QUIC sessions now |
| `sync_relay_groups_active` | Active sync groups now |
| `sync_relay_storage_blobs` | Blobs in database now |
| `sync_relay_storage_bytes` | Ciphertext bytes in database now |
| `sync_relay_storage_groups` | Groups with stored data now |
| `sync_relay_info{version="..."}` | Server version |

**Counters (monotonic since startup):**

| Metric | Description |
|--------|-------------|
| `sync_relay_pushes_total` | Total PUSH requests handled |
| `sync_relay_pulls_total` | Total PULL requests handled |
| `sync_relay_connections_total` | Total connections accepted |
| `sync_relay_bytes_received_total` | Total ciphertext bytes received |
| `sync_relay_bytes_sent_total` | Total ciphertext bytes sent |
| `sync_relay_blobs_stored_total` | Total blobs stored since startup |
| `sync_relay_rate_limit_hits_total` | Total rate limit rejections |
| `sync_relay_errors_total` | Total protocol errors |

### 5.3 Live Logs

```bash
# Follow all relay logs
ssh jimmyb@100.71.79.25 "docker compose -p dist-chaos logs -f"

# Follow a specific relay
ssh jimmyb@100.71.79.25 "docker compose -p dist-chaos logs -f relay-1"

# Last 100 lines from relay-2
ssh jimmyb@100.71.79.25 "docker compose -p dist-chaos logs --tail 100 relay-2"
```

With `RUST_LOG=sync_relay=debug`, you'll see:
- Every connection accept/close
- Every HELLO handshake (device ID, group ID, pending count)
- Every PUSH (blob ID, cursor, group, bytes)
- Every PULL (blob count, bytes, cursor range)
- Every NOTIFY delivery
- Rate limit hits
- Cleanup task activity

### 5.4 Comparing All 3 Relays

Quick health dashboard:

```bash
for port in 8090 8091 8092; do
  echo "=== Relay on :$port ==="
  ssh jimmyb@100.71.79.25 "curl -s http://localhost:$port/health" | python3 -m json.tool
  echo
done
```

Quick metrics comparison:

```bash
for port in 8090 8091 8092; do
  echo "=== Relay :$port ==="
  ssh jimmyb@100.71.79.25 "curl -s http://localhost:$port/metrics" | grep -E "^sync_relay_(pushes|pulls|bytes|connections_total|storage_bytes)"
  echo
done
```

---

## 6. Test Categories

### 6.1 SSH Primitives (5 tests)

| Test | What |
|------|------|
| `ssh_exec_beast_whoami` | SSH to Beast, verify user |
| `ssh_exec_beast_docker_version` | Docker available on Beast |
| `ssh_exec_guardian_whoami` | SSH to Guardian, verify user |
| `ssh_exec_nonexistent_command` | Error handling for bad commands |
| `ssh_scp_round_trip` | SCP file to Beast and back |

### 6.2 Harness Infrastructure (16 tests)

| Test | What |
|------|------|
| `distributed_connect_to_relays` | Connect to 3 permanent relays, discover Endpoint IDs |
| `distributed_relay_health_checks` | All 3 relays respond to health checks |
| `distributed_guardian_binary_exists` | ARM binary present on Guardian |
| `distributed_guardian_cli_version` | sync-cli runs on Guardian |
| `distributed_guardian_init` | sync-cli init works on Guardian |
| `distributed_init_pair_q` | Init and pair on Q (local) |
| `distributed_init_pair_guardian` | Init and pair on Guardian (SSH) |
| `distributed_init_pair_beast_container` | Init and pair in Beast container |
| `distributed_push_pull_q_to_guardian` | Q pushes, Guardian pulls, data matches |
| `distributed_configure_multi_relay` | All clients have all 3 relay addresses |
| `distributed_netem_relay_latency` | tc netem latency on relay container |
| `distributed_netem_guardian_loss` | tc netem packet loss on Guardian |
| `distributed_partition_beast_guardian` | iptables partition between machines |
| `distributed_heal_partition` | Remove iptables partition |

### 6.3 Multi-Relay Failover — MR (4 tests)

| Test | What |
|------|------|
| `mr_01_relay_crash_failover` | Kill relay-1, verify failover to relay-2/3 |
| `mr_02_fan_out_all_relays` | Push fan-out reaches all 3 relays |
| `mr_03_relay_restart_new_endpoint` | Restart relay, verify new Endpoint ID |
| `mr_04_all_relays_down` | Kill all 3, verify error, restart 1, verify recovery |

### 6.4 Cross-Machine Sync — CM (4 tests)

| Test | What |
|------|------|
| `cm_01_q_push_guardian_pull` | Q pushes 10 messages, Guardian pulls all 10 |
| `cm_02_bidirectional_sync` | Q pushes 5, Guardian pushes 5, both see all 10 |
| `cm_03_three_way_sync` | Q + Beast + Guardian all push, all see everything |
| `cm_04_concurrent_push_pull` | 20 rapid pushes from Q, Guardian receives all |

### 6.5 Edge Device — EDGE (4 tests)

| Test | What |
|------|------|
| `edge_01_guardian_high_latency` | 500ms + 100ms jitter on Guardian |
| `edge_02_guardian_bandwidth_limit` | 128kbps bandwidth limit on Guardian |
| `edge_03_guardian_partition_recovery` | Block Guardian, push 10 msgs, unblock, catch up |
| `edge_04_guardian_slow_relay_fast_client` | 200ms relay latency, bidirectional push/pull |

### 6.6 Network Partition & Convergence — NET/CONV (4 tests)

| Test | What |
|------|------|
| `net_01_partition_q_beast` | Block Q↔Beast, verify error, heal, verify recovery |
| `net_02_selective_relay_partition` | 100% loss on relay-1 only, clients fail over |
| `net_03_asymmetric_chaos` | Relay-1: 200ms, Relay-2: 10% loss, Relay-3: clean |
| `conv_01_convergence_after_multi_failure` | Kill relay + partition Guardian + push + heal → converge |

---

## 7. Adding New Tests

### 7.1 Harness API

```rust
use crate::distributed::harness::{DistributedHarness, Machine, ChaosTarget};
use crate::netem::NetemConfig;

// Connect to permanent relays (fail-fast if not running)
let harness = DistributedHarness::connect().await?;

// Init and pair all 3 clients (Q, Beast, Guardian)
harness.init_and_pair_all().await?;

// Push/pull from any machine
harness.push(Machine::Q, "hello").await?;
let output = harness.pull(Machine::Guardian).await?;

// Kill/restart relays (tests that modify relays must restore them)
harness.kill_relay(0).await?;
harness.restart_relay(0).await?;

// Chaos injection (tc netem)
let netem = NetemConfig::new().delay(200).jitter(50).loss(5.0);
harness.inject_netem(ChaosTarget::Relay(0), &netem).await?;
harness.inject_netem(ChaosTarget::Guardian, &netem).await?;
harness.clear_netem(ChaosTarget::Relay(0)).await?;

// Network partition (iptables on Beast)
harness.partition("100.115.186.91", "100.71.79.25").await?;
harness.heal_partition("100.115.186.91", "100.71.79.25").await?;

// Collect relay logs for zero-knowledge assertion
let logs = harness.all_relay_logs().await?;
for log in &logs {
    let result = assert_no_plaintext_in_logs(log);
    assert!(result.passed);
}

// Clean up client state (does NOT touch relays)
harness.cleanup().await?;
```

### 7.2 Test Template

```rust
#[tokio::test]
#[ignore = "requires distributed"]
async fn my_new_scenario() {
    let harness = DistributedHarness::connect().await.expect("connect failed");

    DistributedHarness::ensure_guardian_binary()
        .await
        .expect("ensure_guardian_binary failed");

    harness.init_and_pair_all().await.expect("init_and_pair_all failed");

    // --- Test logic here ---

    // Always clean up
    harness.cleanup().await.expect("cleanup failed");
}
```

### 7.3 Conventions

- All distributed tests go in `tests/chaos/src/scenarios/distributed.rs`
- All tests must be `#[ignore = "requires distributed"]`
- Tests that kill/restart relays must restore them before returning
- Tests that inject netem/iptables must clear them before returning
- Use `settle()` (3s) for normal propagation delays
- Use `settle_long()` (10s) for chaos recovery scenarios
- Unique message prefixes prevent cross-test interference (UUID per message)

---

## 8. Troubleshooting

### "RelaysNotRunning" Error

The harness checks relay health on connect. If relays aren't running:

```bash
# Start them
ssh jimmyb@100.71.79.25 "cd ~/0k-sync && \
  docker compose -f tests/chaos/docker-compose.distributed.yml \
  -p dist-chaos up -d --build --wait"
```

### Port Conflicts on Beast

```bash
# Check what's using the ports
ssh jimmyb@100.71.79.25 "ss -tlnp | grep -E '8090|8091|8092'"

# Kill orphaned containers
ssh jimmyb@100.71.79.25 "docker ps -a --filter 'name=dist-chaos' --format '{{.Names}}'"
ssh jimmyb@100.71.79.25 "docker compose -p dist-chaos down -v --remove-orphans"
```

### SSH Failures

```bash
# Test SSH manually
ssh jimmyb@100.71.79.25 "whoami"   # Should print "jimmyb"
ssh jamesb@100.115.186.91 "whoami"  # Should print "jamesb"

# Check Tailscale
tailscale status
```

### Guardian Binary Not Found

The harness auto-builds and SCPs the ARM binary. If it fails:

```bash
# Manual cross-compile on Beast
ssh jimmyb@100.71.79.25 "export PATH=\$HOME/.cargo/bin:\$PATH && \
  cd ~/0k-sync && cargo install cross 2>/dev/null || true && \
  cross build --target aarch64-unknown-linux-gnu -p zerok-sync-cli --release"

# Manual SCP to Guardian
ssh jimmyb@100.71.79.25 "scp ~/0k-sync/target/aarch64-unknown-linux-gnu/release/sync-cli \
  jamesb@100.115.186.91:/tmp/0k-sync-test/sync-cli"
```

### Tests Hang

The most likely cause is a relay that's not responding. Check health:

```bash
for port in 8090 8091 8092; do
  echo -n "Relay :$port — "
  ssh jimmyb@100.71.79.25 "curl -sf http://localhost:$port/health" && echo "OK" || echo "DOWN"
done
```

If a relay is down after a `kill_relay` test, restart it:

```bash
ssh jimmyb@100.71.79.25 "cd ~/0k-sync && \
  docker compose -f tests/chaos/docker-compose.distributed.yml \
  -p dist-chaos up -d --wait relay-1"
```

### Stale iptables Rules

If a test crashed mid-partition, clean up manually:

```bash
ssh jimmyb@100.71.79.25 "sudo iptables -L INPUT -n | grep DROP"
ssh jimmyb@100.71.79.25 "sudo iptables -F INPUT && sudo iptables -F OUTPUT"
```

### Stale netem Rules

```bash
# On a relay container
ssh jimmyb@100.71.79.25 "docker exec dist-chaos-relay-1-1 tc qdisc del dev eth0 root 2>/dev/null; echo done"

# On Guardian
ssh jamesb@100.115.186.91 "sudo tc qdisc del dev eth0 root 2>/dev/null; echo done"
```

---

## 9. File Reference

### Test Code

| File | Purpose |
|------|---------|
| `tests/chaos/src/distributed/mod.rs` | Module root |
| `tests/chaos/src/distributed/ssh.rs` | SSH execution primitives (`SshTarget`, `exec`, `scp`) |
| `tests/chaos/src/distributed/config.rs` | Machine IPs, paths, ports, timeouts |
| `tests/chaos/src/distributed/harness.rs` | `DistributedHarness` orchestrator |
| `tests/chaos/src/scenarios/distributed.rs` | 16 scenario tests (MR, CM, EDGE, NET, CONV) |

### Infrastructure

| File | Purpose |
|------|---------|
| `tests/chaos/docker-compose.distributed.yml` | 3-relay + client-beast topology for Beast |
| `tests/chaos/Dockerfile.relay` | Relay Docker image (shared with single-host tests) |
| `tests/chaos/Dockerfile.cli` | CLI Docker image (for client-beast container) |

### Relay Observability

| Endpoint | URL (relay-1) | Format |
|----------|---------------|--------|
| Health | `http://100.71.79.25:8090/health` | JSON |
| Metrics | `http://100.71.79.25:8090/metrics` | Prometheus text |
| Logs | `docker compose -p dist-chaos logs relay-1` | Text (tracing fmt) |

### Machine Reference

| Machine | Tailscale IP | SSH | Role |
|---------|-------------|-----|------|
| Q | `100.114.70.54` | — (local) | Test orchestrator |
| Beast | `100.71.79.25` | `ssh jimmyb@100.71.79.25` | Relay host |
| Guardian | `100.115.186.91` | `ssh jamesb@100.115.186.91` | Edge device |

---

**See also:**
- `06-CHAOS-TESTING-STRATEGY.md` — Single-host chaos testing strategy (68 scenarios)
- `E2E-TESTING-GUIDE.md` — Manual E2E testing between Q and Beast
- `CLAUDE.md` — Quick reference for common commands
