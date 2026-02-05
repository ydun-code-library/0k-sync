# E2E Testing Guide

**Last Updated:** 2026-02-05
**Purpose:** How to run end-to-end tests between Q and Beast

---

## Quick Start

### Prerequisites

1. **Beast relay running** — Docker container on `100.71.79.25:8090`
2. **CLI initialized** — `cargo run -p zerok-sync-cli -- init --name <device-name>`
3. **Group joined** — Both devices must share the same group secret

### Health Check

```bash
# Check relay is running
curl http://100.71.79.25:8090/health
# Expected: {"status":"ok","version":"0.1.0","connections":0,"groups":0,"uptime_seconds":...}
```

---

## Setup (One-Time)

### 1. Start Relay on Beast

```bash
ssh jimmyb@100.71.79.25

# If relay not running:
docker run -d -p 8090:8080 -v relay-data:/data --name relay --stop-signal SIGINT 0k-sync-relay

# Get the relay's NodeId (needed for client config):
docker logs relay 2>&1 | grep "Endpoint ID"
# Example: Endpoint ID: 49929a4c3efe228802b762f9db92851f17ab64ed2500c8cb12340fc8583f6848
```

### 2. Initialize CLI on Q (Mac Mini)

```bash
cd /Users/ydun.io/Projects/Personal/0k-sync
cargo run -p zerok-sync-cli -- init --name "Q-Mac-Mini"
```

### 3. Create or Join a Sync Group

**Create new group (first device):**
```bash
cargo run -p zerok-sync-cli -- pair --create --passphrase "your-secret-passphrase" --relay <RELAY_NODE_ID>
```

**Join existing group (second device):**
```bash
cargo run -p zerok-sync-cli -- pair --join --passphrase "your-secret-passphrase" --relay <RELAY_NODE_ID>
```

The `<RELAY_NODE_ID>` is the 64-character hex string from `docker logs relay`.

---

## Running E2E Tests

### Push a Message

```bash
cargo run -p zerok-sync-cli -- push "Hello from Q at $(date +%H:%M:%S)"
```

Expected output:
```
Pushing 32 bytes...
Connecting to peer: 49929a4c...
Push successful!

  Blob ID: ba23cbee-206f-4bb9-bd44-efe8e656c332
  Cursor:  1
```

### Pull Messages

```bash
cargo run -p zerok-sync-cli -- pull --after-cursor 0
```

Expected output:
```
Pulling data after cursor 0...
Connecting to peer: 49929a4c...
Received 1 blob(s):

  [1] Hello from Q at 21:58:18

Cursor updated to 1
```

### Check Status

```bash
cargo run -p zerok-sync-cli -- status
```

---

## Troubleshooting

### "Connection timeout"

The relay NodeId in your config doesn't match the running relay. Docker containers get a new NodeId each time they're created.

**Fix:** Update `relay_address` in your group config:

```bash
# Find config location (macOS)
cat ~/Library/Application\ Support/io.zerok.sync-cli/group.json

# Get new NodeId from relay
ssh jimmyb@100.71.79.25 "docker logs relay 2>&1 | grep 'Endpoint ID'"

# Edit group.json and update relay_address
```

### "Port already in use" on Beast

```bash
# Check what's using 8080/8090
ssh jimmyb@100.71.79.25 "ss -tlnp | grep -E '8080|8090'"

# Kill old relay process if running outside Docker
ssh jimmyb@100.71.79.25 "pkill -f zerok-sync-relay"

# Use different port
docker run -d -p 8091:8080 -v relay-data:/data --name relay --stop-signal SIGINT 0k-sync-relay
```

### Rebuild Relay After Code Changes

```bash
ssh jimmyb@100.71.79.25 "cd ~/0k-sync && git pull && export PATH=\$HOME/.cargo/bin:\$PATH && docker build -t 0k-sync-relay . && docker stop relay && docker rm relay && docker run -d -p 8090:8080 -v relay-data:/data --name relay --stop-signal SIGINT 0k-sync-relay"
```

---

## Cross-Machine Test (Q ↔ Beast)

To test two CLI instances syncing:

**On Q (Mac Mini):**
```bash
cargo run -p zerok-sync-cli -- push "Message from Q"
```

**On Beast:**
```bash
cd ~/0k-sync
export PATH=$HOME/.cargo/bin:$PATH
cargo run -p zerok-sync-cli -- pull --after-cursor 0
```

Both devices must have:
- Same `group_secret_hex` in their `group.json`
- Same `relay_address` pointing to the running relay

---

## Config File Locations

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/io.zerok.sync-cli/` |
| Linux | `~/.local/share/io.zerok.sync-cli/` |

Files:
- `device.json` — Device identity (ID, name, created timestamp)
- `group.json` — Sync group (relay address, group secret, cursor)

---

## Beast Reference

| Item | Value |
|------|-------|
| Tailscale IP | `100.71.79.25` |
| SSH | `ssh jimmyb@100.71.79.25` |
| Relay port | `8090` (HTTP health), QUIC port is ephemeral |
| Repo | `~/0k-sync` |
| Rust | `export PATH=$HOME/.cargo/bin:$PATH` (not on PATH by default) |

---

**See also:** `NEXT-SESSION-START-HERE.md` for session continuity notes
