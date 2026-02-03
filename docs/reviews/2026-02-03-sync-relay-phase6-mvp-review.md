# sync-relay Phase 6 MVP — Code Review

**Date:** 2026-02-03
**Reviewer:** James Barclay
**Status:** Feedback for Q to address
**Verdict:** Solid MVP. Clean architecture, good test coverage, a handful of things to tighten before Phase 6 completion.

---

## Files Reviewed

- `lib.rs`, `main.rs`, `config.rs`, `error.rs`, `server.rs`
- `protocol.rs`, `session.rs`
- `storage/mod.rs`, `storage/sqlite.rs`, `storage/schema.sql`
- `http/mod.rs`, `http/health.rs`, `http/metrics.rs`
- `cleanup.rs`, `Cargo.toml`, `relay.toml.example`

---

## What's Good

### Architecture is clean
Eight modules, each with a single responsibility. Config → Error → Storage → Server → Protocol → Session → HTTP → Cleanup. No circular dependencies, no god objects. Exactly what you'd want from a relay that's supposed to be a "dumb pipe."

### Storage layer is well-designed
The atomic cursor assignment using `INSERT ... ON CONFLICT DO UPDATE ... RETURNING next_cursor - 1` is elegant — guarantees monotonicity without separate locking. WAL mode, proper indexes on `(group_id, cursor)` and `expires_at`, in-memory mode for testing. The `BlobStorage` trait means you could swap SQLite for something else without touching session logic. 12+ tests covering cursors, delivery tracking, cleanup, group isolation.

### Session state machine is correct
`AwaitingHello → Active → Closing` with proper pattern matching on `(state, message)` tuples. Device ID derived from iroh's `connection.remote_id()` (Ed25519 public key) — aligns with the spec's zero-account model. Cleanup on disconnect calls `unregister_session`. Length-prefixed framing (4-byte big-endian) is simple and robust.

### Protocol integration is right
`ProtocolHandler` trait implemented correctly, sessions spawned per connection so the accept loop isn't blocked. ALPN `/0k-sync/1` matches the client side. `send.finish()` called after each response to signal stream end.

### Test coverage is appropriate for MVP
30 tests across storage (12), server (6+), HTTP (3), cleanup (2+), protocol (2), health serialization, metrics format. All the critical paths (cursor assignment, delivery tracking, expiration, session registration) are tested.

---

## Issues to Fix

### 1. Quota enforcement not wired up (Medium)

**Problem:** Config defines `max_blob_size` and `max_group_storage`. Error types include `QuotaExceeded` and `BlobTooLarge`. The storage layer implements `get_group_storage()`. But `handle_push` only checks `push.payload.len() > MAX_MESSAGE_SIZE` (the 1MB protocol constant). It never checks `config.storage.max_blob_size` or calls `get_group_storage()`. If you set `max_blob_size = 524288` in relay.toml, the relay still accepts up to 1MB.

**Fix:** Add both checks in `handle_push` before `store_blob`:
```rust
// Check configured blob size limit
if push.payload.len() > self.relay.config().storage.max_blob_size {
    return Err(ProtocolError::BlobTooLarge);
}

// Check group storage quota
let current_storage = self.relay.storage().get_group_storage(&group_id).await?;
if current_storage + push.payload.len() > self.relay.config().storage.max_group_storage {
    return Err(ProtocolError::QuotaExceeded);
}
```

**Location:** `sync-relay/src/session.rs` in `handle_push()`

---

### 2. cleanup_expired does N+1 queries (Low-Medium)

**Problem:** The cleanup method fetches all expired blob IDs, then loops through them running individual `DELETE FROM deliveries WHERE blob_id = ?` queries. With 1,000 expired blobs that's 1,001 queries.

**Fix:** Single query approach:
```sql
DELETE FROM deliveries WHERE blob_id IN (
    SELECT blob_id FROM blobs WHERE expires_at <= ?1
);
DELETE FROM blobs WHERE expires_at <= ?1;
```
Two queries instead of N+1. Or add `ON DELETE CASCADE` via a foreign key if you're willing to enable foreign keys in SQLite.

**Location:** `sync-relay/src/storage/sqlite.rs` in `cleanup_expired()`

---

### 3. handle_pull marks delivery one blob at a time (Low-Medium)

**Problem:** Same pattern — each blob in the pull response gets its own `mark_delivered` call. With a pull limit of 100, that's 100 separate INSERT queries.

**Fix:** Batch into a single transaction. `sqlx` supports `begin()` / `commit()`. Or build a single query with multiple value tuples.

**Location:** `sync-relay/src/session.rs` in `handle_pull()`

---

### 4. Error mapping in session is lossy (Low)

**Problem:** `StorageError` gets mapped to `ProtocolError::InvalidMessage { reason: e.to_string() }` everywhere. A disk-full error or SQLite corruption looks identical to a malformed message from the client's perspective. More importantly, the tracing output only shows it as a "stream error" when it's actually infrastructure failing.

**Fix:** Add a `ProtocolError::Internal(String)` variant for storage/server failures vs. client-caused errors. Helps with monitoring and debugging.

**Location:** `sync-relay/src/error.rs` and `sync-relay/src/session.rs`

---

### 5. notify_group is a stub (Known/Acknowledged)

**Problem:** Currently logs "Would notify N devices" but doesn't actually send NOTIFY messages. Clients must poll with PULL. STATUS.md lists this as remaining work. Not a bug, just incomplete — but it means the relay is pull-only until this lands, which affects latency for online-online sync.

**Fix:** Implement actual notification delivery. Requires tracking send channels per session.

**Location:** `sync-relay/src/server.rs` in `notify_group()`

---

### 6. total_sessions uses try_read with fallback to 0 (Low)

**Problem:** If a `RwLock` is write-locked during a health check (a session is registering/unregistering at that exact moment), that group's sessions count as 0. Health/metrics will momentarily undercount. Unlikely to matter in practice, but could be confusing if you're watching the dashboard and numbers flicker.

**Fix:** Either maintain an `AtomicUsize` counter alongside the DashMap, or accept the inaccuracy for monitoring (it self-corrects on next request).

**Location:** `sync-relay/src/server.rs` in `total_sessions()`

---

### 7. Graceful shutdown incomplete (Low-Medium)

**Problem:** `main.rs` calls `iroh::protocol::Router::builder(endpoint).accept(ALPN, protocol).spawn()` and binds it to `router`, which keeps it alive. The HTTP server runs via `tokio::spawn`, the cleanup task runs via `tokio::spawn`. If the process gets killed, sessions don't get a chance to clean up gracefully.

**Fix:** Add `tokio::signal::ctrl_c()` in a `tokio::select!` alongside the HTTP server. On signal, drop the router (stops accepting connections), abort cleanup, and exit cleanly. Important for Docker deployments where containers get SIGTERM on stop.

**Location:** `sync-relay/src/main.rs`

---

## Not Issues (Just Observations)

- **`limits.rs` commented out in lib.rs** — Acknowledged as remaining Phase 6 work. Config and error types are ready, just needs implementation with `governor` crate.
- **No Dockerfile yet** — Listed as remaining. The spec has a reference Dockerfile that needs updating for the iroh port (the spec says `EXPOSE 8080` but relay also needs UDP for QUIC on 4433).
- **`discovery_handler` returns static string** — Fine for MVP. Full implementation would return the relay's NodeId for DNS-based discovery. Low priority since clients will be configured with the NodeId directly.
- **28 chaos test stubs ignored** — Waiting for relay integration. Correct approach — they'll be activated when the relay is testable end-to-end.

---

## Summary Table

| # | Issue | Severity | Effort | When |
|---|-------|----------|--------|------|
| 1 | Quota enforcement not wired up | Medium | 30 min | Before Phase 6 completion |
| 2 | Cleanup N+1 queries | Low-Medium | 15 min | Phase 6 completion |
| 3 | Pull delivery batching | Low-Medium | 15 min | Phase 6 completion |
| 4 | Error variant for internal errors | Low | 10 min | Phase 6 completion |
| 5 | notify_group implementation | Known | 1-2 hrs | Phase 6 completion |
| 6 | total_sessions accuracy | Low | Optional | Nice-to-have |
| 7 | Graceful shutdown | Low-Medium | 30 min | Before Docker/deployment |

**Total estimated effort:** ~3-4 hours

---

## Conclusion

None of these are architectural problems. The bones are exactly right — clean trait boundaries, proper async patterns, correct state machine, good test foundation. These are all "finish the wiring" items that Q should knock out alongside rate limiting and Docker.

---

## Action Items for Q

1. [ ] Fix quota enforcement in `handle_push` (Issue #1)
2. [ ] Batch cleanup queries (Issue #2)
3. [ ] Batch delivery marking in `handle_pull` (Issue #3)
4. [ ] Add `ProtocolError::Internal` variant (Issue #4)
5. [ ] Implement graceful shutdown (Issue #7)
6. [ ] Implement `notify_group` (Issue #5)
7. [ ] (Optional) Fix `total_sessions` accuracy (Issue #6)
8. [ ] Implement rate limiting (`limits.rs`)
9. [ ] Create Dockerfile
10. [ ] Integration tests (two sync-cli through relay)
11. [ ] Activate 28 chaos test stubs

---

**Next Session:** Address issues #1-4, #7 first (quick wins), then rate limiting, then Docker.
