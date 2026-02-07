//! Distributed chaos test scenarios.
//!
//! Multi-machine tests across the Tailscale mesh:
//! - Q (Mac Mini), Beast (Linux server), Guardian (Raspberry Pi)
//! - 3 relay instances on Beast, real QUIC over Tailscale
//!
//! All tests: `#[ignore = "requires distributed"]`
//!
//! Run: `cargo test -p chaos-tests distributed -- --ignored`
//!
//! ## Scenario Categories
//!
//! | Prefix | Category | Count |
//! |--------|----------|-------|
//! | MR_* | Multi-relay failover | 4 |
//! | CM_* | Cross-machine sync | 4 |
//! | EDGE_* | Edge device (Guardian) | 4 |
//! | NET_* / CONV_* | Network partition & convergence | 4 |

// Helpers only used by #[ignore] tests — suppress dead_code warnings
#![allow(dead_code, unused_imports)]

use crate::assertions::assert_no_plaintext_in_logs;
use crate::distributed::config;
use crate::distributed::harness::{ChaosTarget, DistributedHarness, Machine};
use crate::netem::NetemConfig;

/// Set up a fully paired 3-machine harness with Guardian binary ready.
async fn setup_full_harness() -> DistributedHarness {
    DistributedHarness::ensure_guardian_binary()
        .await
        .expect("ensure_guardian_binary failed");

    let harness = DistributedHarness::connect().await.expect("connect to relays failed");
    harness.init_and_pair_all().await.expect("init_and_pair_all failed");
    harness
}

/// Wait for relay propagation.
async fn settle() {
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
}

/// Extended wait for chaos recovery scenarios.
async fn settle_long() {
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
}

// ========================================================================
// Sprint 6: Multi-Relay Failover (MR_01 - MR_04)
// ========================================================================

/// MR_01: Kill relay-1, verify Q reconnects to relay-2, push/pull still works.
#[tokio::test]
#[ignore = "requires distributed"]
async fn mr_01_relay_crash_failover() {
    let harness = setup_full_harness().await;

    // Kill relay-1
    harness.kill_relay(0).await.expect("kill relay-1 failed");
    settle().await;

    // Push from Q — should fail over to relay-2 or relay-3
    let msg = format!("mr01-failover-{}", uuid::Uuid::new_v4());
    harness
        .push(Machine::Q, &msg)
        .await
        .expect("push after relay-1 kill should succeed via failover");

    settle().await;

    // Pull on Guardian — should get the message via surviving relay
    let output = harness
        .pull(Machine::Guardian)
        .await
        .expect("pull on Guardian failed");
    assert!(
        output.contains(&msg),
        "Guardian should receive message after relay-1 crash. Output: {}",
        output
    );

    // Verify zero-knowledge
    let logs = harness.all_relay_logs().await.expect("log collection failed");
    for log in &logs {
        let result = assert_no_plaintext_in_logs(log);
        assert!(result.passed, "Relay log leak: {:?}", result.failure_details);
    }

    harness.cleanup().await.expect("cleanup failed");
}

/// MR_02: Push from Q, verify data reachable from all 3 relays.
#[tokio::test]
#[ignore = "requires distributed"]
async fn mr_02_fan_out_all_relays() {
    let harness = setup_full_harness().await;

    // Push from Q — fan-out should replicate to all 3 relays
    let msg = format!("mr02-fanout-{}", uuid::Uuid::new_v4());
    harness
        .push(Machine::Q, &msg)
        .await
        .expect("push from Q failed");

    settle().await;

    // Pull from Beast container — gets data from one of the relays
    let beast_output = harness
        .pull(Machine::Beast)
        .await
        .expect("pull on Beast failed");
    assert!(
        beast_output.contains(&msg),
        "Beast should receive fan-out message. Output: {}",
        beast_output
    );

    // Pull from Guardian
    let guardian_output = harness
        .pull(Machine::Guardian)
        .await
        .expect("pull on Guardian failed");
    assert!(
        guardian_output.contains(&msg),
        "Guardian should receive fan-out message. Output: {}",
        guardian_output
    );

    harness.cleanup().await.expect("cleanup failed");
}

/// MR_03: Restart relay-1, verify new Endpoint ID, reconfigure, push/pull works.
#[tokio::test]
#[ignore = "requires distributed"]
async fn mr_03_relay_restart_new_endpoint() {
    let harness = setup_full_harness().await;

    let old_id = harness.relay_endpoint_ids()[0].clone();

    // Restart relay-1
    let new_id = harness
        .restart_relay(0)
        .await
        .expect("restart relay-1 failed");

    // New ID should differ (new ephemeral keys)
    assert_ne!(
        old_id, new_id,
        "Restarted relay should have new Endpoint ID"
    );
    assert_eq!(new_id.len(), 64, "New Endpoint ID wrong length");

    // Push/pull should still work via relay-2/relay-3
    let msg = format!("mr03-restart-{}", uuid::Uuid::new_v4());
    harness
        .push(Machine::Q, &msg)
        .await
        .expect("push after restart failed");

    settle().await;

    let output = harness
        .pull(Machine::Guardian)
        .await
        .expect("pull after restart failed");
    assert!(
        output.contains(&msg),
        "Guardian should get message after relay restart. Output: {}",
        output
    );

    harness.cleanup().await.expect("cleanup failed");
}

/// MR_04: Kill all 3 relays, verify error, restart 1, verify recovery.
#[tokio::test]
#[ignore = "requires distributed"]
async fn mr_04_all_relays_down() {
    let harness = setup_full_harness().await;

    // Kill all relays
    for i in 0..3 {
        harness.kill_relay(i).await.expect(&format!("kill relay-{} failed", i + 1));
    }
    settle().await;

    // Push should fail (all relays down)
    let result = harness.push(Machine::Q, "should-fail").await;
    assert!(
        result.is_err(),
        "Push should fail when all relays are down"
    );

    // Restart relay-2
    harness
        .restart_relay(1)
        .await
        .expect("restart relay-2 failed");
    settle_long().await;

    // Push should now succeed
    let msg = format!("mr04-recovery-{}", uuid::Uuid::new_v4());
    harness
        .push(Machine::Q, &msg)
        .await
        .expect("push after relay recovery should succeed");

    settle().await;

    let output = harness
        .pull(Machine::Guardian)
        .await
        .expect("pull after recovery failed");
    assert!(
        output.contains(&msg),
        "Guardian should get message after relay recovery. Output: {}",
        output
    );

    harness.cleanup().await.expect("cleanup failed");
}

// ========================================================================
// Sprint 7: Cross-Machine Sync (CM_01 - CM_04)
// ========================================================================

/// CM_01: Q pushes 10 messages, Guardian pulls all 10.
#[tokio::test]
#[ignore = "requires distributed"]
async fn cm_01_q_push_guardian_pull() {
    let harness = setup_full_harness().await;

    let messages: Vec<String> = (0..10)
        .map(|i| format!("cm01-msg{}-{}", i, uuid::Uuid::new_v4()))
        .collect();

    for msg in &messages {
        harness.push(Machine::Q, msg).await.expect("push failed");
    }

    settle_long().await;

    let output = harness
        .pull(Machine::Guardian)
        .await
        .expect("pull on Guardian failed");
    for msg in &messages {
        assert!(
            output.contains(msg),
            "Guardian missing message: {}. Output: {}",
            msg,
            output
        );
    }

    harness.cleanup().await.expect("cleanup failed");
}

/// CM_02: Q pushes 5, Guardian pushes 5, both see all 10.
#[tokio::test]
#[ignore = "requires distributed"]
async fn cm_02_bidirectional_sync() {
    let harness = setup_full_harness().await;

    let q_messages: Vec<String> = (0..5)
        .map(|i| format!("cm02-q{}-{}", i, uuid::Uuid::new_v4()))
        .collect();
    let g_messages: Vec<String> = (0..5)
        .map(|i| format!("cm02-g{}-{}", i, uuid::Uuid::new_v4()))
        .collect();

    for msg in &q_messages {
        harness.push(Machine::Q, msg).await.expect("Q push failed");
    }
    for msg in &g_messages {
        harness.push(Machine::Guardian, msg).await.expect("Guardian push failed");
    }

    settle_long().await;

    let q_output = harness.pull(Machine::Q).await.expect("Q pull failed");
    let g_output = harness.pull(Machine::Guardian).await.expect("Guardian pull failed");

    let all_messages: Vec<&String> = q_messages.iter().chain(g_messages.iter()).collect();
    for msg in &all_messages {
        assert!(q_output.contains(msg.as_str()), "Q missing: {}", msg);
        assert!(g_output.contains(msg.as_str()), "Guardian missing: {}", msg);
    }

    harness.cleanup().await.expect("cleanup failed");
}

/// CM_03: Q, Beast container, Guardian all push, all see all messages.
#[tokio::test]
#[ignore = "requires distributed"]
async fn cm_03_three_way_sync() {
    let harness = setup_full_harness().await;

    let q_msg = format!("cm03-q-{}", uuid::Uuid::new_v4());
    let b_msg = format!("cm03-b-{}", uuid::Uuid::new_v4());
    let g_msg = format!("cm03-g-{}", uuid::Uuid::new_v4());

    harness.push(Machine::Q, &q_msg).await.expect("Q push failed");
    harness.push(Machine::Beast, &b_msg).await.expect("Beast push failed");
    harness.push(Machine::Guardian, &g_msg).await.expect("Guardian push failed");

    settle_long().await;

    for machine in [Machine::Q, Machine::Beast, Machine::Guardian] {
        let output = harness.pull(machine).await.expect(&format!("{} pull failed", machine));
        assert!(output.contains(&q_msg), "{} missing Q msg", machine);
        assert!(output.contains(&b_msg), "{} missing Beast msg", machine);
        assert!(output.contains(&g_msg), "{} missing Guardian msg", machine);
    }

    // Verify zero-knowledge across all relays
    let logs = harness.all_relay_logs().await.expect("log collection failed");
    for log in &logs {
        let result = assert_no_plaintext_in_logs(log);
        assert!(result.passed, "Relay log leak: {:?}", result.failure_details);
    }

    harness.cleanup().await.expect("cleanup failed");
}

/// CM_04: Q pushes continuously, Guardian pulls continuously, no data loss.
#[tokio::test]
#[ignore = "requires distributed"]
async fn cm_04_concurrent_push_pull() {
    let harness = setup_full_harness().await;

    let messages: Vec<String> = (0..20)
        .map(|i| format!("cm04-{}-{}", i, uuid::Uuid::new_v4()))
        .collect();

    // Push all messages with short delays
    for msg in &messages {
        harness.push(Machine::Q, msg).await.expect("push failed");
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    settle_long().await;

    // Pull all on Guardian
    let output = harness
        .pull(Machine::Guardian)
        .await
        .expect("pull failed");

    let mut found = 0;
    for msg in &messages {
        if output.contains(msg.as_str()) {
            found += 1;
        }
    }
    assert_eq!(
        found,
        messages.len(),
        "Guardian received {}/{} messages",
        found,
        messages.len()
    );

    harness.cleanup().await.expect("cleanup failed");
}

// ========================================================================
// Sprint 8: Edge Device — Guardian (EDGE_01 - EDGE_04)
// ========================================================================

/// EDGE_01: 500ms + 100ms jitter on Guardian, push/pull still works.
#[tokio::test]
#[ignore = "requires distributed"]
async fn edge_01_guardian_high_latency() {
    let harness = setup_full_harness().await;

    // Inject high latency on Guardian
    let netem = NetemConfig::new().delay(500).jitter(100);
    harness
        .inject_netem(ChaosTarget::Guardian, &netem)
        .await
        .expect("inject netem on Guardian failed");

    // Push from Q
    let msg = format!("edge01-latency-{}", uuid::Uuid::new_v4());
    harness
        .push(Machine::Q, &msg)
        .await
        .expect("push from Q failed");

    // Longer settle for high latency
    settle_long().await;

    // Pull on Guardian (will be slow but should work)
    let output = harness
        .pull(Machine::Guardian)
        .await
        .expect("pull on Guardian with high latency failed");
    assert!(
        output.contains(&msg),
        "Guardian should receive message despite 500ms latency. Output: {}",
        output
    );

    harness.clear_netem(ChaosTarget::Guardian).await.ok();
    harness.cleanup().await.expect("cleanup failed");
}

/// EDGE_02: 128kbps bandwidth limit on Guardian, small messages still sync.
#[tokio::test]
#[ignore = "requires distributed"]
async fn edge_02_guardian_bandwidth_limit() {
    let harness = setup_full_harness().await;

    // Inject bandwidth limit on Guardian
    let netem = NetemConfig::new().rate(128);
    harness
        .inject_netem(ChaosTarget::Guardian, &netem)
        .await
        .expect("inject netem on Guardian failed");

    // Push small messages from Q
    let messages: Vec<String> = (0..3)
        .map(|i| format!("edge02-bw{}-{}", i, uuid::Uuid::new_v4()))
        .collect();

    for msg in &messages {
        harness.push(Machine::Q, msg).await.expect("push failed");
    }

    settle_long().await;

    // Pull on Guardian — slow but should work for small messages
    let output = harness
        .pull(Machine::Guardian)
        .await
        .expect("pull on bandwidth-limited Guardian failed");
    for msg in &messages {
        assert!(
            output.contains(msg),
            "Guardian missing message under bandwidth limit: {}",
            msg
        );
    }

    harness.clear_netem(ChaosTarget::Guardian).await.ok();
    harness.cleanup().await.expect("cleanup failed");
}

/// EDGE_03: Block Guardian, Q pushes 10 messages, unblock, Guardian catches up.
#[tokio::test]
#[ignore = "requires distributed"]
async fn edge_03_guardian_partition_recovery() {
    let harness = setup_full_harness().await;

    // Partition Guardian from Beast (where relays run)
    harness
        .partition(config::GUARDIAN_IP, config::BEAST_IP)
        .await
        .expect("partition failed");

    // Q pushes 10 messages while Guardian is partitioned
    let messages: Vec<String> = (0..10)
        .map(|i| format!("edge03-part{}-{}", i, uuid::Uuid::new_v4()))
        .collect();
    for msg in &messages {
        harness.push(Machine::Q, msg).await.expect("push failed");
    }

    settle().await;

    // Heal partition
    harness
        .heal_partition(config::GUARDIAN_IP, config::BEAST_IP)
        .await
        .expect("heal failed");

    // Wait for Guardian to catch up
    settle_long().await;

    // Guardian should now have all messages
    let output = harness
        .pull(Machine::Guardian)
        .await
        .expect("pull on Guardian after partition heal failed");
    for msg in &messages {
        assert!(
            output.contains(msg),
            "Guardian missing message after partition heal: {}",
            msg
        );
    }

    harness.cleanup().await.expect("cleanup failed");
}

/// EDGE_04: 200ms on relay-1, Guardian pushes, Q pulls — bidirectional under chaos.
#[tokio::test]
#[ignore = "requires distributed"]
async fn edge_04_guardian_slow_relay_fast_client() {
    let harness = setup_full_harness().await;

    // Inject latency on relay-1
    let netem = NetemConfig::new().delay(200);
    harness
        .inject_netem(ChaosTarget::Relay(0), &netem)
        .await
        .expect("inject netem on relay-1 failed");

    // Guardian pushes
    let msg_g = format!("edge04-g-{}", uuid::Uuid::new_v4());
    harness
        .push(Machine::Guardian, &msg_g)
        .await
        .expect("Guardian push failed");

    // Q pushes
    let msg_q = format!("edge04-q-{}", uuid::Uuid::new_v4());
    harness
        .push(Machine::Q, &msg_q)
        .await
        .expect("Q push failed");

    settle_long().await;

    // Both should see each other's messages
    let q_output = harness.pull(Machine::Q).await.expect("Q pull failed");
    let g_output = harness.pull(Machine::Guardian).await.expect("Guardian pull failed");

    assert!(q_output.contains(&msg_g), "Q missing Guardian message");
    assert!(g_output.contains(&msg_q), "Guardian missing Q message");

    harness.clear_netem(ChaosTarget::Relay(0)).await.ok();
    harness.cleanup().await.expect("cleanup failed");
}

// ========================================================================
// Sprint 9: Network Partition & Convergence (NET_01 - NET_03, CONV_01)
// ========================================================================

/// NET_01: Block Q↔Beast, verify error, heal, verify recovery.
#[tokio::test]
#[ignore = "requires distributed"]
async fn net_01_partition_q_beast() {
    let harness = setup_full_harness().await;

    // Partition Q from Beast
    harness
        .partition(config::Q_IP, config::BEAST_IP)
        .await
        .expect("partition failed");

    settle().await;

    // Push from Q should fail (can't reach relays on Beast)
    let result = harness.push(Machine::Q, "should-fail-partition").await;
    assert!(
        result.is_err(),
        "Push from Q should fail when partitioned from Beast"
    );

    // Heal
    harness
        .heal_partition(config::Q_IP, config::BEAST_IP)
        .await
        .expect("heal failed");

    settle_long().await;

    // Push should now work
    let msg = format!("net01-recovery-{}", uuid::Uuid::new_v4());
    harness
        .push(Machine::Q, &msg)
        .await
        .expect("push after partition heal failed");

    settle().await;

    let output = harness
        .pull(Machine::Guardian)
        .await
        .expect("Guardian pull after heal failed");
    assert!(
        output.contains(&msg),
        "Guardian should get message after Q↔Beast partition healed. Output: {}",
        output
    );

    harness.cleanup().await.expect("cleanup failed");
}

/// NET_02: Block relay-1 from Guardian only, Guardian fails over to relay-2.
#[tokio::test]
#[ignore = "requires distributed"]
async fn net_02_selective_relay_partition() {
    let harness = setup_full_harness().await;

    // Inject 100% loss on relay-1 (simulates partition for relay-1 only)
    let netem = NetemConfig::new().loss(100.0);
    harness
        .inject_netem(ChaosTarget::Relay(0), &netem)
        .await
        .expect("inject netem on relay-1 failed");

    settle().await;

    // Push from Q — should use relay-2 or relay-3
    let msg = format!("net02-selective-{}", uuid::Uuid::new_v4());
    harness
        .push(Machine::Q, &msg)
        .await
        .expect("push should succeed via relay-2/3");

    settle_long().await;

    // Guardian should get the message via relay-2 or relay-3
    let output = harness
        .pull(Machine::Guardian)
        .await
        .expect("Guardian pull failed");
    assert!(
        output.contains(&msg),
        "Guardian should get message via surviving relays. Output: {}",
        output
    );

    harness.clear_netem(ChaosTarget::Relay(0)).await.ok();
    harness.cleanup().await.expect("cleanup failed");
}

/// NET_03: Relay-1: 200ms latency, Relay-2: 10% loss, Relay-3: clean — verify convergence.
#[tokio::test]
#[ignore = "requires distributed"]
async fn net_03_asymmetric_chaos() {
    let harness = setup_full_harness().await;

    // Asymmetric chaos across relays
    harness
        .inject_netem(ChaosTarget::Relay(0), &NetemConfig::new().delay(200))
        .await
        .expect("inject relay-1 latency failed");
    harness
        .inject_netem(ChaosTarget::Relay(1), &NetemConfig::new().loss(10.0))
        .await
        .expect("inject relay-2 loss failed");
    // Relay-3: clean

    // Push from all 3 machines
    let q_msg = format!("net03-q-{}", uuid::Uuid::new_v4());
    let b_msg = format!("net03-b-{}", uuid::Uuid::new_v4());
    let g_msg = format!("net03-g-{}", uuid::Uuid::new_v4());

    harness.push(Machine::Q, &q_msg).await.expect("Q push failed");
    harness.push(Machine::Beast, &b_msg).await.expect("Beast push failed");
    harness.push(Machine::Guardian, &g_msg).await.expect("Guardian push failed");

    // Extended settle for lossy/laggy relays
    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

    // All machines should eventually converge
    for machine in [Machine::Q, Machine::Beast, Machine::Guardian] {
        let output = harness.pull(machine).await.expect(&format!("{} pull failed", machine));
        assert!(output.contains(&q_msg), "{} missing Q msg", machine);
        assert!(output.contains(&b_msg), "{} missing Beast msg", machine);
        assert!(output.contains(&g_msg), "{} missing Guardian msg", machine);
    }

    // Clean up chaos
    harness.clear_netem(ChaosTarget::Relay(0)).await.ok();
    harness.clear_netem(ChaosTarget::Relay(1)).await.ok();
    harness.cleanup().await.expect("cleanup failed");
}

/// CONV_01: Kill relay-1, partition Guardian, Q pushes. Restart relay-1,
/// heal Guardian. All converge.
#[tokio::test]
#[ignore = "requires distributed"]
async fn conv_01_convergence_after_multi_failure() {
    let harness = setup_full_harness().await;

    // Kill relay-1
    harness.kill_relay(0).await.expect("kill relay-1 failed");

    // Partition Guardian
    harness
        .partition(config::GUARDIAN_IP, config::BEAST_IP)
        .await
        .expect("partition Guardian failed");

    settle().await;

    // Q pushes (should reach relay-2 or relay-3)
    let messages: Vec<String> = (0..5)
        .map(|i| format!("conv01-{}-{}", i, uuid::Uuid::new_v4()))
        .collect();
    for msg in &messages {
        harness.push(Machine::Q, msg).await.expect("Q push failed");
    }

    settle().await;

    // Restart relay-1
    harness.restart_relay(0).await.expect("restart relay-1 failed");

    // Heal Guardian partition
    harness
        .heal_partition(config::GUARDIAN_IP, config::BEAST_IP)
        .await
        .expect("heal Guardian failed");

    // Extended settle for convergence
    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

    // All machines should have all messages
    for machine in [Machine::Q, Machine::Beast, Machine::Guardian] {
        let output = harness.pull(machine).await.expect(&format!("{} pull failed", machine));
        for msg in &messages {
            assert!(
                output.contains(msg),
                "{} missing message {} after convergence. Output: {}",
                machine,
                msg,
                output
            );
        }
    }

    // Verify zero-knowledge
    let logs = harness.all_relay_logs().await.expect("log collection failed");
    for log in &logs {
        let result = assert_no_plaintext_in_logs(log);
        assert!(result.passed, "Relay log leak: {:?}", result.failure_details);
    }

    harness.cleanup().await.expect("cleanup failed");
}
