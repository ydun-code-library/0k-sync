//! Assertion helpers for chaos testing.
//!
//! These are pure functions that verify sync state correctness after chaos
//! scenarios complete. They take state as input and return pass/fail.

use std::collections::{HashMap, HashSet};

/// A simplified representation of a blob for testing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestBlob {
    /// Blob hash (content identifier)
    pub hash: String,
    /// Which client pushed this blob
    pub origin_client: String,
}

/// Client state for assertion checking.
#[derive(Debug, Clone, Default)]
pub struct ClientState {
    /// Client identifier
    pub client_id: String,
    /// Blobs present in this client's store
    pub blobs: HashSet<String>,
    /// Version vector (client_id -> last seen cursor)
    pub version_vector: HashMap<String, u64>,
}

/// Topology state for assertion checking.
#[derive(Debug, Clone, Default)]
pub struct TopologyState {
    /// All clients in the topology
    pub clients: Vec<ClientState>,
    /// All blobs that were pushed (for tracking)
    pub all_pushed_blobs: HashSet<String>,
}

/// Result of an assertion check.
#[derive(Debug, Clone)]
pub struct AssertionResult {
    /// Whether the assertion passed
    pub passed: bool,
    /// Description of what was checked
    pub description: String,
    /// Details on failure
    pub failure_details: Option<String>,
}

impl AssertionResult {
    /// Create a passing result.
    pub fn pass(description: &str) -> Self {
        Self {
            passed: true,
            description: description.into(),
            failure_details: None,
        }
    }

    /// Create a failing result.
    pub fn fail(description: &str, details: &str) -> Self {
        Self {
            passed: false,
            description: description.into(),
            failure_details: Some(details.into()),
        }
    }
}

/// Assert that a specific blob is present in a client's store.
///
/// # Arguments
/// * `client_state` - The client's current state
/// * `blob_hash` - The hash of the blob to check for
///
/// # Returns
/// Pass if blob is present, fail otherwise.
pub fn assert_blob_present(client_state: &ClientState, blob_hash: &str) -> AssertionResult {
    if client_state.blobs.contains(blob_hash) {
        AssertionResult::pass(&format!(
            "Blob {} present on client {}",
            blob_hash, client_state.client_id
        ))
    } else {
        AssertionResult::fail(
            &format!(
                "Blob {} should be present on client {}",
                blob_hash, client_state.client_id
            ),
            &format!(
                "Client has {} blobs but not the expected one",
                client_state.blobs.len()
            ),
        )
    }
}

/// Assert that no data was lost across the topology.
///
/// Every blob pushed by any client should be present on all paired clients
/// after sync quiescence.
///
/// # Arguments
/// * `topology_state` - The state of all clients
///
/// # Returns
/// Pass if all clients have all blobs, fail otherwise.
pub fn assert_no_data_loss(topology_state: &TopologyState) -> AssertionResult {
    let expected_blobs = &topology_state.all_pushed_blobs;

    for client in &topology_state.clients {
        for blob in expected_blobs {
            if !client.blobs.contains(blob) {
                return AssertionResult::fail(
                    "No data loss check",
                    &format!(
                        "Client {} is missing blob {}. Has {}/{} blobs.",
                        client.client_id,
                        blob,
                        client.blobs.len(),
                        expected_blobs.len()
                    ),
                );
            }
        }
    }

    AssertionResult::pass(&format!(
        "All {} clients have all {} blobs",
        topology_state.clients.len(),
        expected_blobs.len()
    ))
}

/// Assert that all clients have converged to identical version vectors.
///
/// This tests Invariant 4: after sync quiescence, all clients should have
/// the same view of what data exists.
///
/// # Arguments
/// * `topology_state` - The state of all clients
///
/// # Returns
/// Pass if all version vectors are identical, fail otherwise.
pub fn assert_version_vectors_converged(topology_state: &TopologyState) -> AssertionResult {
    if topology_state.clients.is_empty() {
        return AssertionResult::pass("No clients to check");
    }

    let reference = &topology_state.clients[0].version_vector;

    for (i, client) in topology_state.clients.iter().enumerate().skip(1) {
        if &client.version_vector != reference {
            return AssertionResult::fail(
                "Version vector convergence",
                &format!(
                    "Client {} version vector differs from client {}. \
                     Client 0: {:?}, Client {}: {:?}",
                    i, 0, reference, i, client.version_vector
                ),
            );
        }
    }

    AssertionResult::pass(&format!(
        "All {} clients have converged version vectors",
        topology_state.clients.len()
    ))
}

/// Assert that relay logs contain no plaintext data.
///
/// This tests Invariant 5: the relay should never log blob contents,
/// even in encrypted form that could be correlated.
///
/// # Arguments
/// * `relay_logs` - The relay's log output
///
/// # Returns
/// Pass if no sensitive data found, fail otherwise.
pub fn assert_no_plaintext_in_logs(relay_logs: &str) -> AssertionResult {
    // Patterns that should never appear in relay logs
    let forbidden_patterns = [
        // Actual blob content markers
        "payload=",
        "blob_content=",
        "plaintext=",
        "decrypted=",
        // Base64-encoded blobs (suspiciously long base64 strings)
        // Look for base64 strings > 100 chars that aren't known safe patterns
    ];

    for pattern in &forbidden_patterns {
        if relay_logs.contains(pattern) {
            return AssertionResult::fail(
                "No plaintext in logs",
                &format!("Found forbidden pattern '{}' in relay logs", pattern),
            );
        }
    }

    // Check for suspiciously long base64 strings (potential leaked data)
    for line in relay_logs.lines() {
        // Skip known safe log patterns
        if line.contains("device_id=") || line.contains("group_id=") {
            continue;
        }

        // Look for long base64-like strings that could be blob data
        for word in line.split_whitespace() {
            if word.len() > 100 && looks_like_base64(word) {
                return AssertionResult::fail(
                    "No plaintext in logs",
                    &format!(
                        "Found suspicious long base64 string in logs (len={})",
                        word.len()
                    ),
                );
            }
        }
    }

    AssertionResult::pass("No plaintext or suspicious data found in relay logs")
}

/// Check if a string looks like base64 data.
fn looks_like_base64(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client(id: &str, blobs: &[&str]) -> ClientState {
        ClientState {
            client_id: id.into(),
            blobs: blobs.iter().map(|s| s.to_string()).collect(),
            version_vector: HashMap::new(),
        }
    }

    #[test]
    fn test_blob_present_pass() {
        let client = make_client("A", &["hash1", "hash2"]);
        let result = assert_blob_present(&client, "hash1");
        assert!(result.passed);
    }

    #[test]
    fn test_blob_present_fail() {
        let client = make_client("A", &["hash1", "hash2"]);
        let result = assert_blob_present(&client, "hash3");
        assert!(!result.passed);
        assert!(result.failure_details.is_some());
    }

    #[test]
    fn test_no_data_loss_pass() {
        let topology = TopologyState {
            clients: vec![
                make_client("A", &["h1", "h2", "h3"]),
                make_client("B", &["h1", "h2", "h3"]),
            ],
            all_pushed_blobs: ["h1", "h2", "h3"].iter().map(|s| s.to_string()).collect(),
        };

        let result = assert_no_data_loss(&topology);
        assert!(result.passed);
    }

    #[test]
    fn test_no_data_loss_fail() {
        let topology = TopologyState {
            clients: vec![
                make_client("A", &["h1", "h2", "h3"]),
                make_client("B", &["h1", "h2"]), // Missing h3
            ],
            all_pushed_blobs: ["h1", "h2", "h3"].iter().map(|s| s.to_string()).collect(),
        };

        let result = assert_no_data_loss(&topology);
        assert!(!result.passed);
        assert!(result.failure_details.unwrap().contains("missing blob h3"));
    }

    #[test]
    fn test_version_vectors_converged_pass() {
        let mut vv = HashMap::new();
        vv.insert("A".into(), 5);
        vv.insert("B".into(), 3);

        let topology = TopologyState {
            clients: vec![
                ClientState {
                    client_id: "A".into(),
                    blobs: HashSet::new(),
                    version_vector: vv.clone(),
                },
                ClientState {
                    client_id: "B".into(),
                    blobs: HashSet::new(),
                    version_vector: vv.clone(),
                },
            ],
            all_pushed_blobs: HashSet::new(),
        };

        let result = assert_version_vectors_converged(&topology);
        assert!(result.passed);
    }

    #[test]
    fn test_version_vectors_converged_fail() {
        let mut vv1 = HashMap::new();
        vv1.insert("A".into(), 5);

        let mut vv2 = HashMap::new();
        vv2.insert("A".into(), 3); // Different!

        let topology = TopologyState {
            clients: vec![
                ClientState {
                    client_id: "A".into(),
                    blobs: HashSet::new(),
                    version_vector: vv1,
                },
                ClientState {
                    client_id: "B".into(),
                    blobs: HashSet::new(),
                    version_vector: vv2,
                },
            ],
            all_pushed_blobs: HashSet::new(),
        };

        let result = assert_version_vectors_converged(&topology);
        assert!(!result.passed);
    }

    #[test]
    fn test_no_plaintext_pass() {
        let logs = r#"
            2026-01-15T10:00:00Z INFO relay started
            2026-01-15T10:00:01Z INFO client connected device_id=abc123
            2026-01-15T10:00:02Z INFO blob received cursor=42
        "#;

        let result = assert_no_plaintext_in_logs(logs);
        assert!(result.passed);
    }

    #[test]
    fn test_no_plaintext_fail_payload() {
        let logs = "2026-01-15T10:00:00Z DEBUG payload=secretdata123";

        let result = assert_no_plaintext_in_logs(logs);
        assert!(!result.passed);
        assert!(result.failure_details.unwrap().contains("payload="));
    }

    #[test]
    fn test_no_plaintext_fail_long_base64() {
        let long_base64 = "a".repeat(150);
        let logs = format!("2026-01-15T10:00:00Z DEBUG suspicious {}", long_base64);

        let result = assert_no_plaintext_in_logs(&logs);
        assert!(!result.passed);
        assert!(result
            .failure_details
            .unwrap()
            .contains("suspicious long base64"));
    }

    #[test]
    fn test_looks_like_base64() {
        assert!(looks_like_base64("SGVsbG8gV29ybGQ="));
        assert!(looks_like_base64("abc123+/=="));
        assert!(!looks_like_base64("hello world!"));
        assert!(!looks_like_base64("has-dashes"));
    }
}
