# Multi-Relay Fan-Out Specification

**Version:** 1.0.0
**Date:** 2026-02-06
**Author:** James (LTIS Investments AB) / Q
**Phase:** 6.5
**Status:** Design Complete — Implementation Pending

---

## 1. Overview

A single relay is a single point of failure. Multi-relay fan-out eliminates this by having clients push encrypted blobs to 2-3 regional relays simultaneously. Any one relay holding the data is sufficient for other devices to pull.

**Key constraint:** All fan-out logic lives in the client. Relay servers are unchanged — they remain zero-knowledge, independent, and stateless. No inter-relay communication.

### 1.1 Relationship to iroh RelayMap

iroh's `RelayMap` is a **transport-level** concern — it routes QUIC datagrams through relay servers for NAT traversal. A client's iroh endpoint uses the RelayMap to reach peers when direct connections fail.

0k-sync's multi-relay is an **application-level** concern — the client pushes the same encrypted blob to multiple independent sync-relay instances for redundancy. These are different layers:

```
Application Layer (0k-sync multi-relay)
├── Push same blob to Relay A, B, C for redundancy
├── Pull from whichever relay is reachable
│
Transport Layer (iroh RelayMap)
├── Route QUIC packets through relay for NAT traversal
├── Failover between iroh relays for connectivity
```

They may share the same physical relay servers, but they serve different purposes and operate independently.

---

## 2. Architecture

### 2.1 Push Flow (Fan-Out)

The client pushes the same encrypted blob to all configured relays concurrently. Success is reported as soon as the primary relay acknowledges. Secondaries are fire-and-forget with background retry on failure.

```
Phone ──push──→ Relay A (primary, London)      → PushAck → "Done!" to user
       └─push──→ Relay B (secondary, Singapore) → PushAck → logged, ignored
       └─push──→ Relay C (secondary, Sydney)    → PushAck → logged, ignored
```

If a secondary push fails, it is logged via `tracing::warn`. The data is safe on the primary. Secondary failures do not block or fail the user's push operation.

### 2.2 Pull Flow (Failover)

The client pulls from its connected relay. If that relay is unreachable, it tries the next relay in preference order. Each relay has its own cursor space, so the client tracks cursors per-relay.

```
Laptop ──pull──→ Relay B (nearest) → PullResponse → done
If B down:
Laptop ──pull──→ Relay C → PullResponse → done
If C down:
Laptop ──pull──→ Relay A → PullResponse → done
```

### 2.3 Connect Flow (Failover)

On initial connect, the client tries each relay in order until one succeeds the HELLO/Welcome handshake. The successful relay becomes the "active" relay for that session.

If all relays fail, the client returns `ClientError::AllRelaysFailed`.

### 2.4 Per-Relay Cursors

Each relay assigns cursors independently. Relay A might assign cursor 5 to a blob, while Relay B assigns cursor 3 to the same blob (different arrival order). The client tracks a `HashMap<RelayAddress, Cursor>`:

- "Last cursor from Relay A was 5"
- "Last cursor from Relay B was 3"
- "Last cursor from Relay C was 7"

When pulling from a specific relay, the client uses that relay's cursor. When pulling from a relay for the first time, cursor starts at 0.

### 2.5 Relay Server Changes

**None.** Each relay instance continues to operate independently:

- Own SQLite database
- Own cursor space
- Own TTL-based cleanup (default 7 days, hourly)
- Own per-group storage quotas (default 100 MB)
- Own `mark_delivered_batch` tracking

No awareness of other relays. No inter-relay protocol. No shared state.

### 2.6 Cleanup

No changes. Each relay independently:

1. Stores blobs with TTL (default 7 days)
2. Runs hourly cleanup to delete expired blobs
3. Enforces per-group storage quotas (default 100 MB)
4. Tracks delivery per-device with `mark_delivered_batch`

With 3 relays, each manages its own lifecycle. Total storage is 3x single-relay, but within VPS disk budgets.

---

## 3. Invite Format v3

### 3.1 Current Format (v2)

```json
{
  "version": 2,
  "relay_node_id": "<32-byte hex>",
  "group_id": "<32-byte hex>",
  "group_secret": "<32-byte hex>",
  "salt": "<16-byte hex>",
  "created_at": 1738800000,
  "expires_at": 1738886400
}
```

### 3.2 New Format (v3)

```json
{
  "version": 3,
  "relay_node_ids": ["<32-byte hex>", "<32-byte hex>", "<32-byte hex>"],
  "group_id": "<32-byte hex>",
  "group_secret": "<32-byte hex>",
  "salt": "<16-byte hex>",
  "created_at": 1738800000,
  "expires_at": 1738886400
}
```

### 3.3 Backward Compatibility

- **v3 client reading v2 invite:** The `relay_node_id` field (singular) is deserialized as a one-element `relay_node_ids` vector via serde alias
- **v2 client reading v3 invite:** Will fail to deserialize (expected — v2 clients don't support multi-relay)
- **v1 invites:** Rejected as before (`UnsupportedVersion(1)`)

### 3.4 QR Code Size

A v3 invite with 3 relay node IDs adds ~128 bytes (2 additional 32-byte hex-encoded node IDs). QR codes support up to 2,953 bytes. Current v2 invites are ~300 bytes. No size concern.

---

## 4. Configuration Format

### 4.1 CLI GroupConfig (group.json)

**New format:**

```json
{
  "group_name": "my-group",
  "relay_addresses": ["node-id-1", "node-id-2", "node-id-3"],
  "cursors": {
    "node-id-1": 42,
    "node-id-2": 38,
    "node-id-3": 45
  },
  "group_secret_hex": "...",
  "salt_hex": "..."
}
```

**Backward compatibility with old format:**

```json
{
  "group_name": "my-group",
  "relay_address": "node-id-1",
  "cursor": 42,
  "group_secret_hex": "...",
  "salt_hex": "..."
}
```

Old format loads successfully — `relay_address` maps to single-element `relay_addresses`, and `cursor` maps to `cursors` with the single relay address as key.

### 4.2 Client SyncConfig

```toml
[sync]
backend = "sync-relay"
relay_addresses = ["primary-node-id", "secondary-node-id"]
auto_reconnect = true
reconnect_delay_ms = 1000
max_reconnect_delay_ms = 30000
```

Replaces the singular `relay_node_id` field from the spec.

---

## 5. Failure Mode Matrix

| Scenario | Primary | Secondary(s) | User Impact | Client Action |
|----------|---------|--------------|-------------|---------------|
| Normal operation | Up | Up | None | Fan-out to all, primary ack |
| Secondary down | Up | 1+ down | None | Primary ack, warn about secondary |
| Primary down (push) | Down | Up | Slightly slower | Connect fails over to secondary, push there |
| Primary down (pull) | Down | Up | None | Pull from secondary using that relay's cursor |
| All relays down | Down | Down | Sync offline | Return `AllRelaysFailed`, queue for retry |
| Primary slow | Slow | Fast | None | Primary ack is slow but secondaries complete |
| Relay flapping | Up/down | Varies | Brief interruption | Reconnect on next operation, cursor resumes |

---

## 6. Deduplication

Relays do **not** deduplicate blobs. If the same encrypted blob is pushed twice (e.g., during retry), it is stored twice with different cursors. The client handles deduplication if needed — but since blobs are encrypted with random nonces, identical plaintext produces different ciphertext, so relay-level dedup is not meaningful.

---

## 7. Cost Model

At scale (100,000 users, 40 global relays, 3x fan-out):

| Metric | Value |
|--------|-------|
| Average push/day/user | ~10 small (1 KB) + 1 large (100 KB) |
| Single-relay bandwidth | ~2.9 TB/mo |
| 3x fan-out bandwidth | ~8.7 TB/mo |
| Per-relay monthly cost | ~$14/mo (Hetzner CX22 or OVH equivalent) |
| 40 relays total cost | ~$570/mo |
| Revenue (100K × $5.99) | ~$599,000/mo |

Fan-out bandwidth is negligible relative to VPS capacity. The cost model scales linearly with relay count, not with fan-out multiplier.
