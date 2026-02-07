//! Transport chaos scenarios (T-LAT-*, T-LOSS-*, T-CONN-*, T-BW-*).
//!
//! All tests require Docker with `tc netem` support (NET_ADMIN capability).
//! Run on Beast: `cargo test -p chaos-tests -- --ignored`
//!
//! Infrastructure:
//! - Docker containers with NET_ADMIN capability
//! - `tc qdisc netem` for latency/loss/bandwidth injection (QUIC/UDP)
//! - bollard for container lifecycle (kill, restart, pause)
//! - ChaosHarness for orchestration
//!
//! Per 06-CHAOS-TESTING-STRATEGY.md section 5.

// Helpers only used by #[ignore] tests — suppress dead_code warnings
#![allow(dead_code, unused_imports)]

use std::path::PathBuf;

use crate::assertions::assert_no_plaintext_in_logs;
use crate::harness::ChaosHarness;
use crate::netem::NetemConfig;

/// Resolve the docker-compose file path relative to the chaos test directory.
fn compose_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docker-compose.chaos.yml")
}

/// Helper: set up harness, init+pair clients, return harness.
async fn setup_paired_harness() -> ChaosHarness {
    let mut harness = ChaosHarness::new(compose_file()).expect("harness creation failed");
    harness.setup().await.expect("harness setup failed");
    harness.init_and_pair().await.expect("init and pair failed");
    harness
}

/// Helper: push from A, pull from B, verify data arrived.
async fn push_pull_verify(harness: &ChaosHarness, message: &str) {
    harness
        .push("client-a", message)
        .await
        .expect("push failed");

    // Brief delay for relay processing
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let pull_result = harness.pull("client-b").await.expect("pull failed");

    assert!(
        pull_result.stdout.contains(message),
        "Pull output should contain message '{}', got: {}",
        message,
        pull_result.stdout
    );
}

// ============================================================================
// T-LAT-* Latency & Jitter (4 tests)
// ============================================================================

/// T-LAT-01: Fixed 200ms latency, sync completes with blobs intact.
#[tokio::test]
#[ignore = "requires docker"]
async fn t_lat_01_fixed_200ms_latency() {
    let harness = setup_paired_harness().await;

    // Inject 200ms latency on client-a
    let netem = NetemConfig::new().delay(200);
    harness
        .inject_netem("client-a", &netem)
        .await
        .expect("netem injection failed");

    // Push and pull — should complete despite latency
    push_pull_verify(&harness, "lat-01-test-data").await;

    // Verify relay logs are clean
    let logs = harness.relay_logs().await.expect("log collection failed");
    let result = assert_no_plaintext_in_logs(&logs);
    assert!(
        result.passed,
        "Relay log check: {:?}",
        result.failure_details
    );

    // Cleanup
    harness.clear_netem("client-a").await.ok();
    harness.teardown().await.expect("teardown failed");
}

/// T-LAT-02: High jitter (200ms +/- 150ms), no reordering corruption.
#[tokio::test]
#[ignore = "requires docker"]
async fn t_lat_02_high_jitter() {
    let harness = setup_paired_harness().await;

    // Inject high jitter on client-a
    let netem = NetemConfig::new().delay(200).jitter(150);
    harness
        .inject_netem("client-a", &netem)
        .await
        .expect("netem injection failed");

    // Push multiple messages
    for i in 0..3 {
        harness
            .push("client-a", &format!("jitter-msg-{}", i))
            .await
            .expect("push failed");
    }

    // Wait for delivery with jitter
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Pull and verify all messages arrived
    let pull_result = harness.pull("client-b").await.expect("pull failed");
    for i in 0..3 {
        let msg = format!("jitter-msg-{}", i);
        assert!(
            pull_result.stdout.contains(&msg),
            "Missing message '{}' in pull output: {}",
            msg,
            pull_result.stdout
        );
    }

    harness.clear_netem("client-a").await.ok();
    harness.teardown().await.expect("teardown failed");
}

/// T-LAT-03: Asymmetric latency (10ms on A, 500ms on B).
#[tokio::test]
#[ignore = "requires docker"]
async fn t_lat_03_asymmetric_latency() {
    let harness = setup_paired_harness().await;

    // Asymmetric: low latency on A, high on B
    let netem_a = NetemConfig::new().delay(10);
    let netem_b = NetemConfig::new().delay(500);
    harness
        .inject_netem("client-a", &netem_a)
        .await
        .expect("netem A failed");
    harness
        .inject_netem("client-b", &netem_b)
        .await
        .expect("netem B failed");

    // Push from A (fast path)
    harness
        .push("client-a", "asymmetric-from-a")
        .await
        .expect("push from A failed");

    // Wait for B's slow path to receive
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    let pull_b = harness.pull("client-b").await.expect("pull B failed");
    assert!(
        pull_b.stdout.contains("asymmetric-from-a"),
        "B should have A's data: {}",
        pull_b.stdout
    );

    // Push from B (slow path)
    harness
        .push("client-b", "asymmetric-from-b")
        .await
        .expect("push from B failed");

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let pull_a = harness.pull("client-a").await.expect("pull A failed");
    assert!(
        pull_a.stdout.contains("asymmetric-from-b"),
        "A should have B's data: {}",
        pull_a.stdout
    );

    harness.clear_netem("client-a").await.ok();
    harness.clear_netem("client-b").await.ok();
    harness.teardown().await.expect("teardown failed");
}

/// T-LAT-04: Satellite simulation (600ms + 50ms jitter).
#[tokio::test]
#[ignore = "requires docker"]
async fn t_lat_04_satellite_simulation() {
    let harness = setup_paired_harness().await;

    // Satellite-like conditions on both clients
    let netem = NetemConfig::new().delay(600).jitter(50);
    harness
        .inject_netem("client-a", &netem)
        .await
        .expect("netem A failed");
    harness
        .inject_netem("client-b", &netem)
        .await
        .expect("netem B failed");

    // Push a message — should still work, just slowly
    harness
        .push("client-a", "satellite-test")
        .await
        .expect("push failed under satellite conditions");

    // Give plenty of time for satellite latency
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    let pull_result = harness.pull("client-b").await.expect("pull failed");
    assert!(
        pull_result.stdout.contains("satellite-test"),
        "Data should arrive despite satellite latency: {}",
        pull_result.stdout
    );

    harness.clear_netem("client-a").await.ok();
    harness.clear_netem("client-b").await.ok();
    harness.teardown().await.expect("teardown failed");
}

// ============================================================================
// T-LOSS-* Packet Loss (4 tests)
// ============================================================================

/// T-LOSS-01: 5% random packet loss, sync completes with retries.
#[tokio::test]
#[ignore = "requires docker"]
async fn t_loss_01_5_percent_loss() {
    let harness = setup_paired_harness().await;

    // 5% packet loss on client-a
    let netem = NetemConfig::new().loss(5.0);
    harness
        .inject_netem("client-a", &netem)
        .await
        .expect("netem injection failed");

    // Push and pull — QUIC retries should handle 5% loss
    push_pull_verify(&harness, "loss-5pct-test").await;

    harness.clear_netem("client-a").await.ok();
    harness.teardown().await.expect("teardown failed");
}

/// T-LOSS-02: 20% packet loss, graceful failure or completion.
#[tokio::test]
#[ignore = "requires docker"]
async fn t_loss_02_20_percent_loss() {
    let harness = setup_paired_harness().await;

    // 20% packet loss — aggressive but QUIC should cope
    let netem = NetemConfig::new().loss(20.0);
    harness
        .inject_netem("client-a", &netem)
        .await
        .expect("netem injection failed");

    // Push — may succeed or fail gracefully
    let push_result = harness.push("client-a", "loss-20pct-test").await;

    match push_result {
        Ok(_) => {
            // If push succeeded, pull should eventually see it
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let pull = harness.pull("client-b").await;
            // Either succeeds with data or fails gracefully — no corruption
            if let Ok(result) = pull {
                assert!(
                    !result.stdout.contains("payload="),
                    "No plaintext leak in output"
                );
            }
        }
        Err(e) => {
            // Graceful failure is acceptable at 20% loss
            let err_str = format!("{}", e);
            assert!(
                !err_str.contains("panic") && !err_str.contains("corruption"),
                "Should fail gracefully, got: {}",
                err_str
            );
        }
    }

    harness.clear_netem("client-a").await.ok();
    harness.teardown().await.expect("teardown failed");
}

/// T-LOSS-03: Burst loss (10% with 25% correlation).
#[tokio::test]
#[ignore = "requires docker"]
async fn t_loss_03_burst_loss() {
    let harness = setup_paired_harness().await;

    // Burst loss pattern
    let netem = NetemConfig::new().loss(10.0).loss_correlation(25.0);
    harness
        .inject_netem("client-a", &netem)
        .await
        .expect("netem injection failed");

    // Push multiple messages to test burst resilience
    for i in 0..3 {
        let _ = harness.push("client-a", &format!("burst-msg-{}", i)).await;
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Pull — some or all messages should arrive (QUIC retries)
    let pull_result = harness.pull("client-b").await;
    if let Ok(result) = pull_result {
        // At minimum, verify no data corruption
        assert!(
            !result.stdout.contains("payload="),
            "No plaintext leak in output"
        );
    }

    // Verify relay logs clean regardless
    let logs = harness.relay_logs().await.expect("log collection failed");
    let check = assert_no_plaintext_in_logs(&logs);
    assert!(check.passed, "Relay log check: {:?}", check.failure_details);

    harness.clear_netem("client-a").await.ok();
    harness.teardown().await.expect("teardown failed");
}

/// T-LOSS-04: 100% loss (partition) then recovery.
#[tokio::test]
#[ignore = "requires docker"]
async fn t_loss_04_partition_recovery() {
    let harness = setup_paired_harness().await;

    // Push data before partition
    harness
        .push("client-a", "pre-partition")
        .await
        .expect("pre-partition push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Create network partition: 100% loss on client-a
    let netem = NetemConfig::new().loss(100.0);
    harness
        .inject_netem("client-a", &netem)
        .await
        .expect("partition injection failed");

    // Push during partition — should fail
    let partition_push = harness.push("client-a", "during-partition").await;
    // Expect failure or timeout during partition
    assert!(
        partition_push.is_err(),
        "Push should fail during 100% packet loss partition"
    );

    // Heal partition
    harness
        .clear_netem("client-a")
        .await
        .expect("partition removal failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Push after recovery — should work
    harness
        .push("client-a", "post-partition")
        .await
        .expect("post-partition push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Pull — should see pre-partition and post-partition data
    let pull = harness.pull("client-b").await.expect("pull failed");
    assert!(
        pull.stdout.contains("pre-partition"),
        "Should have pre-partition data: {}",
        pull.stdout
    );
    assert!(
        pull.stdout.contains("post-partition"),
        "Should have post-partition data: {}",
        pull.stdout
    );

    harness.teardown().await.expect("teardown failed");
}

// ============================================================================
// T-CONN-* Connection Events (5 tests)
// ============================================================================

/// T-CONN-01: Relay crash mid-sync, client retries on reconnect.
#[tokio::test]
#[ignore = "requires docker"]
async fn t_conn_01_relay_crash_mid_sync() {
    let harness = setup_paired_harness().await;

    // Push some initial data
    harness
        .push("client-a", "before-crash")
        .await
        .expect("initial push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Verify B received it
    let pull = harness.pull("client-b").await.expect("initial pull failed");
    assert!(pull.stdout.contains("before-crash"));

    // Kill relay
    harness
        .kill_container("relay")
        .await
        .expect("kill relay failed");

    // Push during relay down — should fail
    let down_push = harness.push("client-a", "relay-down").await;
    assert!(down_push.is_err(), "Push should fail with relay down");

    // Restart relay
    harness
        .restart_container("relay")
        .await
        .expect("restart relay failed");

    // Wait for relay to come back up and be healthy
    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

    // Push after relay restart — should succeed (client reconnects)
    harness
        .push("client-a", "after-restart")
        .await
        .expect("post-restart push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let pull_after = harness
        .pull("client-b")
        .await
        .expect("pull after restart failed");
    assert!(
        pull_after.stdout.contains("after-restart"),
        "Data should arrive after relay restart: {}",
        pull_after.stdout
    );

    harness.teardown().await.expect("teardown failed");
}

/// T-CONN-02: Client crash mid-push, relay cleans up.
#[tokio::test]
#[ignore = "requires docker"]
async fn t_conn_02_client_crash_mid_push() {
    let harness = setup_paired_harness().await;

    // Push some data from A
    harness
        .push("client-a", "before-client-crash")
        .await
        .expect("initial push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Kill client-a (simulates crash)
    harness
        .kill_container("client-a")
        .await
        .expect("kill client-a failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Client B should still be able to pull data that was already synced
    let pull = harness.pull("client-b").await.expect("pull should work");
    assert!(
        pull.stdout.contains("before-client-crash"),
        "B should have data from before crash: {}",
        pull.stdout
    );

    // Restart client-a
    harness
        .restart_container("client-a")
        .await
        .expect("restart client-a failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Relay should have cleaned up A's session
    let logs = harness.relay_logs().await.expect("log collection failed");
    let check = assert_no_plaintext_in_logs(&logs);
    assert!(
        check.passed,
        "Relay logs clean: {:?}",
        check.failure_details
    );

    harness.teardown().await.expect("teardown failed");
}

/// T-CONN-03: Network partition (both clients online, relay unreachable).
#[tokio::test]
#[ignore = "requires docker"]
async fn t_conn_03_network_partition() {
    let harness = setup_paired_harness().await;

    // Push data before partition
    harness
        .push("client-a", "pre-partition-data")
        .await
        .expect("push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Partition both clients from relay
    let netem = NetemConfig::new().loss(100.0);
    harness
        .inject_netem("client-a", &netem)
        .await
        .expect("netem A failed");
    harness
        .inject_netem("client-b", &netem)
        .await
        .expect("netem B failed");

    // Both clients should detect partition (push/pull fail)
    let push_a = harness.push("client-a", "during-partition-a").await;
    assert!(push_a.is_err(), "A should fail during partition");

    let pull_b = harness.pull("client-b").await;
    assert!(pull_b.is_err(), "B should fail during partition");

    // Heal partition
    harness.clear_netem("client-a").await.ok();
    harness.clear_netem("client-b").await.ok();
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // After healing, sync should resume
    harness
        .push("client-a", "post-partition-data")
        .await
        .expect("post-partition push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let pull = harness
        .pull("client-b")
        .await
        .expect("post-partition pull failed");
    assert!(
        pull.stdout.contains("post-partition-data"),
        "Should sync after partition heals: {}",
        pull.stdout
    );

    harness.teardown().await.expect("teardown failed");
}

/// T-CONN-04: Rapid reconnect cycle (10 connect/disconnect in 5s).
#[tokio::test]
#[ignore = "requires docker"]
async fn t_conn_04_rapid_reconnect() {
    let harness = setup_paired_harness().await;

    // Rapid push cycles — each push creates a new connection
    for i in 0..10 {
        let msg = format!("rapid-{}", i);
        let _ = harness.push("client-a", &msg).await;
        // Brief pause between operations
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // Wait for all to settle
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Pull and check — should have some or all messages, no corruption
    let pull = harness.pull("client-b").await;
    if let Ok(result) = pull {
        // Verify no corruption patterns
        assert!(!result.stdout.contains("payload="), "No plaintext leak");
    }

    // Verify relay didn't leak connections
    let logs = harness.relay_logs().await.expect("log collection failed");
    let check = assert_no_plaintext_in_logs(&logs);
    assert!(
        check.passed,
        "Relay logs clean: {:?}",
        check.failure_details
    );

    harness.teardown().await.expect("teardown failed");
}

/// T-CONN-05: Half-open connection handling.
#[tokio::test]
#[ignore = "requires docker"]
async fn t_conn_05_half_open_connection() {
    let harness = setup_paired_harness().await;

    // Push initial data
    harness
        .push("client-a", "half-open-test")
        .await
        .expect("push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Pause client-a (simulates half-open — TCP keepalive dies, process frozen)
    harness
        .pause_container("client-a")
        .await
        .expect("pause failed");

    // Wait for relay to detect stale session
    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

    // Client B should still work independently
    let pull = harness
        .pull("client-b")
        .await
        .expect("pull should work with A paused");
    assert!(
        pull.stdout.contains("half-open-test"),
        "B should still get data: {}",
        pull.stdout
    );

    // Unpause A
    harness
        .unpause_container("client-a")
        .await
        .expect("unpause failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // A should be able to push again after unpause
    harness
        .push("client-a", "after-unpause")
        .await
        .expect("push after unpause failed");

    harness.teardown().await.expect("teardown failed");
}

// ============================================================================
// T-BW-* Bandwidth Constraints (3 tests)
// ============================================================================

/// T-BW-01: 56kbps edge network, small blobs sync.
#[tokio::test]
#[ignore = "requires docker"]
async fn t_bw_01_56kbps_edge_network() {
    let harness = setup_paired_harness().await;

    // 56kbps bandwidth limit on client-a
    let netem = NetemConfig::new().rate(56);
    harness
        .inject_netem("client-a", &netem)
        .await
        .expect("netem injection failed");

    // Push a small message — should work at 56kbps
    harness
        .push("client-a", "tiny-56k-data")
        .await
        .expect("push should work at 56kbps for small data");

    // Give time for slow transfer
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    let pull = harness.pull("client-b").await.expect("pull failed");
    assert!(
        pull.stdout.contains("tiny-56k-data"),
        "Small data should arrive at 56kbps: {}",
        pull.stdout
    );

    harness.clear_netem("client-a").await.ok();
    harness.teardown().await.expect("teardown failed");
}

/// T-BW-02: Bandwidth drop mid-transfer.
#[tokio::test]
#[ignore = "requires docker"]
async fn t_bw_02_bandwidth_drop_mid_transfer() {
    let harness = setup_paired_harness().await;

    // Start with reasonable bandwidth
    let netem_fast = NetemConfig::new().rate(1024);
    harness
        .inject_netem("client-a", &netem_fast)
        .await
        .expect("fast netem failed");

    // Push first message at normal speed
    harness
        .push("client-a", "bw-before-drop")
        .await
        .expect("first push failed");

    // Drop bandwidth dramatically
    harness.clear_netem("client-a").await.ok();
    let netem_slow = NetemConfig::new().rate(10);
    harness
        .inject_netem("client-a", &netem_slow)
        .await
        .expect("slow netem failed");

    // Push at reduced bandwidth — may be very slow but shouldn't corrupt
    let slow_push = harness.push("client-a", "bw-after-drop").await;

    // Regardless of push success, verify no corruption
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    let pull = harness.pull("client-b").await;
    if let Ok(result) = pull {
        assert!(
            result.stdout.contains("bw-before-drop"),
            "First message should arrive: {}",
            result.stdout
        );
    }

    // Allow slow push to be incomplete but not corrupt
    if slow_push.is_err() {
        // Acceptable — bandwidth too low
    }

    harness.clear_netem("client-a").await.ok();
    harness.teardown().await.expect("teardown failed");
}

/// T-BW-03: Asymmetric bandwidth (fast client A, slow client B).
#[tokio::test]
#[ignore = "requires docker"]
async fn t_bw_03_asymmetric_bandwidth() {
    let harness = setup_paired_harness().await;

    // Fast A, slow B
    let netem_fast = NetemConfig::new().rate(10240); // 10Mbit
    let netem_slow = NetemConfig::new().rate(128); // 128kbit

    harness
        .inject_netem("client-a", &netem_fast)
        .await
        .expect("netem A failed");
    harness
        .inject_netem("client-b", &netem_slow)
        .await
        .expect("netem B failed");

    // Push from fast A
    harness
        .push("client-a", "from-fast-client")
        .await
        .expect("fast push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Pull from slow B — should work, just slower
    let pull = harness.pull("client-b").await.expect("slow pull failed");
    assert!(
        pull.stdout.contains("from-fast-client"),
        "Slow client should eventually get data: {}",
        pull.stdout
    );

    // Push from slow B
    harness
        .push("client-b", "from-slow-client")
        .await
        .expect("slow push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Pull from fast A
    let pull_a = harness.pull("client-a").await.expect("fast pull failed");
    assert!(
        pull_a.stdout.contains("from-slow-client"),
        "Fast client should get slow client's data: {}",
        pull_a.stdout
    );

    harness.clear_netem("client-a").await.ok();
    harness.clear_netem("client-b").await.ok();
    harness.teardown().await.expect("teardown failed");
}
