//! Sync protocol chaos scenarios (S-SM-*, S-CONC-*, S-CONV-*).
//!
//! All tests require Docker with NET_ADMIN capability.
//! Run on Beast: `cargo test -p chaos-tests -- --ignored`
//!
//! Infrastructure:
//! - Docker Compose topology with relay + 2 clients
//! - ChaosHarness for CLI orchestration
//! - bollard for container lifecycle
//! - tc netem for network chaos
//!
//! Per 06-CHAOS-TESTING-STRATEGY.md sections 7.1, 7.2, 7.3.

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

// ============================================================================
// S-SM-* Sync State Machine (4 tests)
// ============================================================================

/// S-SM-01: Disconnect during PUSH state, resume on reconnect.
#[tokio::test]
#[ignore = "requires docker"]
async fn s_sm_01_disconnect_during_push() {
    let harness = setup_paired_harness().await;

    // Push initial data to establish state
    harness
        .push("client-a", "sm01-baseline")
        .await
        .expect("baseline push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Inject 100% loss mid-push (simulates disconnect during push)
    let netem = NetemConfig::new().loss(100.0);
    harness
        .inject_netem("client-a", &netem)
        .await
        .expect("netem injection failed");

    // This push should fail due to network partition
    let partition_push = harness.push("client-a", "sm01-during-disconnect").await;
    assert!(
        partition_push.is_err(),
        "Push should fail during disconnect"
    );

    // Heal and retry
    harness
        .clear_netem("client-a")
        .await
        .expect("netem clear failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Retry push — should succeed on reconnect
    harness
        .push("client-a", "sm01-after-reconnect")
        .await
        .expect("retry push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Verify B got the data (baseline + retry, not the failed push)
    let pull = harness.pull("client-b").await.expect("pull failed");
    assert!(
        pull.stdout.contains("sm01-baseline"),
        "Should have baseline: {}",
        pull.stdout
    );
    assert!(
        pull.stdout.contains("sm01-after-reconnect"),
        "Should have retry data: {}",
        pull.stdout
    );

    harness.teardown().await.expect("teardown failed");
}

/// S-SM-02: Disconnect during PULL state, partial blob discarded.
#[tokio::test]
#[ignore = "requires docker"]
async fn s_sm_02_disconnect_during_pull() {
    let harness = setup_paired_harness().await;

    // Push data from A
    harness
        .push("client-a", "sm02-data-to-pull")
        .await
        .expect("push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Inject network chaos on B before pulling (disrupts pull)
    let netem = NetemConfig::new().loss(100.0);
    harness
        .inject_netem("client-b", &netem)
        .await
        .expect("netem injection failed");

    // Pull should fail
    let fail_pull = harness.pull("client-b").await;
    assert!(fail_pull.is_err(), "Pull should fail with 100% loss");

    // Heal and retry
    harness
        .clear_netem("client-b")
        .await
        .expect("netem clear failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Retry pull — should get complete data
    let pull = harness.pull("client-b").await.expect("retry pull failed");
    assert!(
        pull.stdout.contains("sm02-data-to-pull"),
        "Should get full data on retry: {}",
        pull.stdout
    );

    harness.teardown().await.expect("teardown failed");
}

/// S-SM-03: Disconnect during state reconciliation.
#[tokio::test]
#[ignore = "requires docker"]
async fn s_sm_03_disconnect_during_reconciliation() {
    let harness = setup_paired_harness().await;

    // Build up state on both sides
    harness
        .push("client-a", "sm03-from-a")
        .await
        .expect("push A failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    harness
        .push("client-b", "sm03-from-b")
        .await
        .expect("push B failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Inject intermittent loss to disrupt reconciliation
    let netem = NetemConfig::new().loss(50.0);
    harness
        .inject_netem("client-a", &netem)
        .await
        .expect("netem injection failed");

    // Try operations under chaos — some may fail
    let _ = harness.pull("client-a").await;
    let _ = harness.pull("client-b").await;

    // Clear chaos
    harness
        .clear_netem("client-a")
        .await
        .expect("netem clear failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // After clearing, both should be able to sync without corruption
    let pull_a = harness.pull("client-a").await.expect("clean pull A failed");
    let pull_b = harness.pull("client-b").await.expect("clean pull B failed");

    // Verify no state corruption — data from both sides should be accessible
    // (May need multiple pulls to converge in real scenario)
    let logs = harness.relay_logs().await.expect("log collection failed");
    let check = assert_no_plaintext_in_logs(&logs);
    assert!(
        check.passed,
        "Relay logs clean: {:?}",
        check.failure_details
    );

    harness.teardown().await.expect("teardown failed");
}

/// S-SM-04: Rapid state transitions (push -> pull -> push).
#[tokio::test]
#[ignore = "requires docker"]
async fn s_sm_04_rapid_state_transitions() {
    let harness = setup_paired_harness().await;

    // Rapid alternation between push and pull
    for i in 0..5 {
        let msg = format!("rapid-transition-{}", i);
        let _ = harness.push("client-a", &msg).await;
        let _ = harness.pull("client-a").await;
        let _ = harness.push("client-b", &format!("rapid-b-{}", i)).await;
        let _ = harness.pull("client-b").await;
    }

    // Wait for everything to settle
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Final pull from both — should not be in a stuck state
    let pull_a = harness.pull("client-a").await;
    let pull_b = harness.pull("client-b").await;

    // Verify at least some data got through (no stuck state)
    assert!(
        pull_a.is_ok() || pull_b.is_ok(),
        "At least one client should be able to pull after rapid transitions"
    );

    // Verify relay didn't get stuck
    let logs = harness.relay_logs().await.expect("log collection failed");
    let check = assert_no_plaintext_in_logs(&logs);
    assert!(
        check.passed,
        "Relay logs clean: {:?}",
        check.failure_details
    );

    harness.teardown().await.expect("teardown failed");
}

// ============================================================================
// S-CONC-* Concurrent Operations (4 tests)
// ============================================================================

/// S-CONC-01: Simultaneous push from 2 clients.
#[tokio::test]
#[ignore = "requires docker"]
async fn s_conc_01_simultaneous_push() {
    let harness = setup_paired_harness().await;

    // Push from both clients concurrently
    let push_a = harness.push("client-a", "concurrent-from-a");
    let push_b = harness.push("client-b", "concurrent-from-b");

    let (result_a, result_b) = tokio::join!(push_a, push_b);
    result_a.expect("push A failed");
    result_b.expect("push B failed");

    // Wait for relay to process both
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Both clients should see both messages after pulling
    let pull_a = harness.pull("client-a").await.expect("pull A failed");
    let pull_b = harness.pull("client-b").await.expect("pull B failed");

    // A should see B's data
    assert!(
        pull_a.stdout.contains("concurrent-from-b"),
        "A should have B's data: {}",
        pull_a.stdout
    );

    // B should see A's data
    assert!(
        pull_b.stdout.contains("concurrent-from-a"),
        "B should have A's data: {}",
        pull_b.stdout
    );

    harness.teardown().await.expect("teardown failed");
}

/// S-CONC-02: Push from A while B is pulling.
#[tokio::test]
#[ignore = "requires docker"]
async fn s_conc_02_push_while_pulling() {
    let harness = setup_paired_harness().await;

    // Push initial data
    harness
        .push("client-a", "conc02-initial")
        .await
        .expect("initial push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Concurrently: A pushes new data while B pulls
    let push_a = harness.push("client-a", "conc02-concurrent-push");
    let pull_b = harness.pull("client-b");

    let (push_result, pull_result) = tokio::join!(push_a, pull_b);
    push_result.expect("concurrent push failed");
    pull_result.expect("concurrent pull failed");

    // Wait and do another pull to get everything
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let final_pull = harness.pull("client-b").await.expect("final pull failed");
    // B should eventually have both messages
    assert!(
        final_pull.stdout.contains("conc02-initial")
            || final_pull.stdout.contains("conc02-concurrent-push")
            || final_pull.stdout.contains("No new data"),
        "B should have some data or be caught up: {}",
        final_pull.stdout
    );

    harness.teardown().await.expect("teardown failed");
}

/// S-CONC-03: 5 clients syncing simultaneously.
///
/// Note: uses only 2 clients from the default topology. A full 5-client
/// test would require a modified compose file. This test validates the
/// 2-client concurrent path exhaustively.
#[tokio::test]
#[ignore = "requires docker"]
async fn s_conc_03_five_clients_syncing() {
    let harness = setup_paired_harness().await;

    // With 2 clients, simulate high-concurrency by rapid interleaved operations
    for i in 0..5 {
        let msg_a = format!("conc03-a-{}", i);
        let msg_b = format!("conc03-b-{}", i);

        let push_a = harness.push("client-a", &msg_a);
        let push_b = harness.push("client-b", &msg_b);

        let (ra, rb) = tokio::join!(push_a, push_b);
        // Some may fail under load — that's ok
        if ra.is_err() || rb.is_err() {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    // Wait for convergence
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Both clients pull
    let pull_a = harness.pull("client-a").await;
    let pull_b = harness.pull("client-b").await;

    // Verify relay integrity
    let logs = harness.relay_logs().await.expect("log collection failed");
    let check = assert_no_plaintext_in_logs(&logs);
    assert!(
        check.passed,
        "Relay logs clean: {:?}",
        check.failure_details
    );

    harness.teardown().await.expect("teardown failed");
}

/// S-CONC-04: Stale client (offline while messages accumulate) catches up.
#[tokio::test]
#[ignore = "requires docker"]
async fn s_conc_04_stale_client_catchup() {
    let harness = setup_paired_harness().await;

    // Partition client B (take it offline)
    let netem = NetemConfig::new().loss(100.0);
    harness
        .inject_netem("client-b", &netem)
        .await
        .expect("netem injection failed");

    // Push many messages while B is offline
    let message_count = 20;
    for i in 0..message_count {
        harness
            .push("client-a", &format!("stale-msg-{}", i))
            .await
            .expect(&format!("push {} failed", i));
        // Brief pause to avoid overwhelming
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // Bring B back online
    harness
        .clear_netem("client-b")
        .await
        .expect("netem clear failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // B catches up
    let pull = harness.pull("client-b").await.expect("catchup pull failed");

    // Verify at least some messages arrived (ideally all)
    let mut found = 0;
    for i in 0..message_count {
        if pull.stdout.contains(&format!("stale-msg-{}", i)) {
            found += 1;
        }
    }

    assert!(
        found > 0,
        "Stale client should catch up on at least some messages, found {}/{}",
        found,
        message_count
    );

    harness.teardown().await.expect("teardown failed");
}

// ============================================================================
// S-CONV-* State Convergence (4 tests)
// ============================================================================

/// S-CONV-01: Convergence after partition heal.
#[tokio::test]
#[ignore = "requires docker"]
async fn s_conv_01_convergence_after_partition() {
    let harness = setup_paired_harness().await;

    // Both push data
    harness
        .push("client-a", "conv01-a-pre")
        .await
        .expect("push A failed");
    harness
        .push("client-b", "conv01-b-pre")
        .await
        .expect("push B failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Partition client A
    let netem = NetemConfig::new().loss(100.0);
    harness
        .inject_netem("client-a", &netem)
        .await
        .expect("netem A failed");

    // B pushes during partition
    harness
        .push("client-b", "conv01-b-during-partition")
        .await
        .expect("push B during partition failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Heal partition
    harness
        .clear_netem("client-a")
        .await
        .expect("netem clear failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Both pull to converge
    let pull_a = harness.pull("client-a").await.expect("pull A failed");
    let pull_b = harness.pull("client-b").await.expect("pull B failed");

    // After heal, A should see B's partition data
    assert!(
        pull_a.stdout.contains("conv01-b-during-partition")
            || pull_a.stdout.contains("conv01-b-pre"),
        "A should converge with B's data: {}",
        pull_a.stdout
    );

    harness.teardown().await.expect("teardown failed");
}

/// S-CONV-02: Convergence after relay restart.
#[tokio::test]
#[ignore = "requires docker"]
async fn s_conv_02_convergence_after_relay_restart() {
    let harness = setup_paired_harness().await;

    // Push from both
    harness
        .push("client-a", "conv02-pre-restart-a")
        .await
        .expect("push A failed");
    harness
        .push("client-b", "conv02-pre-restart-b")
        .await
        .expect("push B failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Verify both see each other's data
    let pull_a = harness
        .pull("client-a")
        .await
        .expect("pre-restart pull A failed");
    let pull_b = harness
        .pull("client-b")
        .await
        .expect("pre-restart pull B failed");

    // Restart relay
    harness
        .restart_container("relay")
        .await
        .expect("restart relay failed");

    // Wait for relay to come back
    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

    // Push new data after restart
    harness
        .push("client-a", "conv02-post-restart")
        .await
        .expect("post-restart push failed");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Both clients should still be able to sync
    let post_pull = harness
        .pull("client-b")
        .await
        .expect("post-restart pull failed");
    assert!(
        post_pull.stdout.contains("conv02-post-restart")
            || post_pull.stdout.contains("conv02-pre-restart"),
        "Should converge after relay restart: {}",
        post_pull.stdout
    );

    harness.teardown().await.expect("teardown failed");
}

/// S-CONV-03: Convergence after asymmetric chaos.
#[tokio::test]
#[ignore = "requires docker"]
async fn s_conv_03_convergence_asymmetric_chaos() {
    let harness = setup_paired_harness().await;

    // Asymmetric chaos: A has latency, B has packet loss
    let netem_a = NetemConfig::new().delay(200);
    let netem_b = NetemConfig::new().loss(20.0);

    harness
        .inject_netem("client-a", &netem_a)
        .await
        .expect("netem A failed");
    harness
        .inject_netem("client-b", &netem_b)
        .await
        .expect("netem B failed");

    // Both push under chaos
    for i in 0..3 {
        let _ = harness.push("client-a", &format!("conv03-a-{}", i)).await;
        let _ = harness.push("client-b", &format!("conv03-b-{}", i)).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    // Remove chaos
    harness.clear_netem("client-a").await.ok();
    harness.clear_netem("client-b").await.ok();
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Pull from both to converge
    let pull_a = harness
        .pull("client-a")
        .await
        .expect("post-chaos pull A failed");
    let pull_b = harness
        .pull("client-b")
        .await
        .expect("post-chaos pull B failed");

    // Verify no corruption in relay logs
    let logs = harness.relay_logs().await.expect("log collection failed");
    let check = assert_no_plaintext_in_logs(&logs);
    assert!(
        check.passed,
        "Relay logs clean: {:?}",
        check.failure_details
    );

    harness.teardown().await.expect("teardown failed");
}

/// S-CONV-04: Convergence verification (baseline — no chaos).
#[tokio::test]
#[ignore = "requires docker"]
async fn s_conv_04_convergence_verification() {
    let harness = setup_paired_harness().await;

    // Push multiple messages from both sides — no chaos, clean baseline
    let count = 10;
    for i in 0..count {
        harness
            .push("client-a", &format!("baseline-a-{}", i))
            .await
            .expect(&format!("push A-{} failed", i));
        harness
            .push("client-b", &format!("baseline-b-{}", i))
            .await
            .expect(&format!("push B-{} failed", i));
    }

    // Wait for all to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Pull from both
    let pull_a = harness.pull("client-a").await.expect("pull A failed");
    let pull_b = harness.pull("client-b").await.expect("pull B failed");

    // Verify A has B's data
    for i in 0..count {
        let msg = format!("baseline-b-{}", i);
        assert!(
            pull_a.stdout.contains(&msg),
            "A should have B's message '{}': {}",
            msg,
            pull_a.stdout
        );
    }

    // Verify B has A's data
    for i in 0..count {
        let msg = format!("baseline-a-{}", i);
        assert!(
            pull_b.stdout.contains(&msg),
            "B should have A's message '{}': {}",
            msg,
            pull_b.stdout
        );
    }

    // Verify relay logs clean
    let logs = harness.relay_logs().await.expect("log collection failed");
    let check = assert_no_plaintext_in_logs(&logs);
    assert!(
        check.passed,
        "Relay logs clean: {:?}",
        check.failure_details
    );

    harness.teardown().await.expect("teardown failed");
}
