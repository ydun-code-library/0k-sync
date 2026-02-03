-- sync-relay SQLite schema
-- Version: 1.0.0
-- Uses WAL mode for concurrent read/write

PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA busy_timeout=5000;

-- Tracks the next cursor value per group (monotonic)
CREATE TABLE IF NOT EXISTS group_cursors (
    group_id BLOB PRIMARY KEY,
    next_cursor INTEGER NOT NULL DEFAULT 1
);

-- Stores encrypted blobs temporarily for offline devices
CREATE TABLE IF NOT EXISTS blobs (
    blob_id BLOB PRIMARY KEY,
    group_id BLOB NOT NULL,
    cursor INTEGER NOT NULL,
    sender_id BLOB NOT NULL,
    payload BLOB NOT NULL,
    timestamp INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    UNIQUE(group_id, cursor)
);

-- Tracks which devices have received which blobs
CREATE TABLE IF NOT EXISTS deliveries (
    blob_id BLOB NOT NULL,
    device_id BLOB NOT NULL,
    delivered_at INTEGER,
    PRIMARY KEY (blob_id, device_id)
);

-- Index for efficient cursor-based queries
CREATE INDEX IF NOT EXISTS idx_blobs_group_cursor ON blobs(group_id, cursor);

-- Index for cleanup task (find expired blobs)
CREATE INDEX IF NOT EXISTS idx_blobs_expires ON blobs(expires_at);

-- Index for group storage quota calculation
CREATE INDEX IF NOT EXISTS idx_blobs_group_id ON blobs(group_id);
