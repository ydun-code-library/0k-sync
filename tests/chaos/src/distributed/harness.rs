//! Distributed chaos test harness — multi-machine orchestrator.
//!
//! Manages 3 relay instances on Beast, clients on Q/Beast/Guardian,
//! and chaos injection across the Tailscale mesh.

use thiserror::Error;

use super::config;
use super::ssh::SshError;
use crate::netem::NetemConfig;

/// Errors from distributed harness operations.
#[derive(Debug, Error)]
pub enum DistributedError {
    /// SSH execution failed.
    #[error("ssh error: {0}")]
    Ssh(#[from] SshError),

    /// Docker Compose operation failed.
    #[error("compose error: {0}")]
    Compose(String),

    /// Relay endpoint ID not found in logs.
    #[error("endpoint ID not found for relay {index}")]
    EndpointIdNotFound {
        /// Relay index (0-2).
        index: usize,
    },

    /// Timeout waiting for relay startup.
    #[error("timeout waiting for relay {index} startup")]
    RelayTimeout {
        /// Relay index (0-2).
        index: usize,
    },

    /// Guardian binary not available.
    #[error("guardian binary not available: {0}")]
    GuardianBinary(String),

    /// General I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Client operation failed.
    #[error("client error on {machine}: {detail}")]
    ClientError {
        /// Which machine.
        machine: String,
        /// Error detail.
        detail: String,
    },
}

/// Which machine to target for client operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Machine {
    /// Q — local Mac Mini (test orchestrator).
    Q,
    /// Beast — Docker container on Beast.
    Beast,
    /// Guardian — Raspberry Pi via SSH.
    Guardian,
}

impl std::fmt::Display for Machine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Machine::Q => write!(f, "Q"),
            Machine::Beast => write!(f, "Beast"),
            Machine::Guardian => write!(f, "Guardian"),
        }
    }
}

/// Target for chaos injection.
#[derive(Debug, Clone, Copy)]
pub enum ChaosTarget {
    /// tc netem inside a relay container on Beast.
    Relay(usize),
    /// tc netem on Guardian's network interface.
    Guardian,
}

/// Multi-machine distributed chaos test orchestrator.
///
/// Manages:
/// - 3 relay instances on Beast (Docker Compose)
/// - Client on Q (local process)
/// - Client on Beast (Docker container)
/// - Client on Guardian (SSH + cross-compiled binary)
/// - Chaos injection (tc netem, iptables)
pub struct DistributedHarness {
    /// Random passphrase for this test's sync group.
    passphrase: String,
    /// Discovered Endpoint IDs for each relay (index 0-2).
    relay_endpoint_ids: Vec<String>,
    /// Unique Docker Compose project name on Beast.
    compose_project: String,
    /// Local temp directory for Q's client state.
    q_data_dir: tempfile::TempDir,
    /// Per-machine data directories (for cleanup tracking).
    guardian_data_dir: String,
}

impl DistributedHarness {
    /// Start 3 relays on Beast and discover their Endpoint IDs.
    pub async fn setup() -> Result<Self, DistributedError> {
        let compose_project = format!("dist-{}", &uuid::Uuid::new_v4().as_simple().to_string()[..12]);
        let passphrase = format!("dist-test-{}", &compose_project[5..]);
        let q_data_dir = tempfile::tempdir()?;
        let guardian_data_dir = format!("{}/{}", config::GUARDIAN_DATA_DIR, &compose_project);

        // Ensure Beast repo is up to date
        config::BEAST
            .exec_ok(&format!("cd {} && git pull --ff-only", config::BEAST_REPO))
            .await
            .map_err(|e| DistributedError::Compose(format!("git pull on Beast failed: {}", e)))?;

        // Start 3 relays on Beast via docker compose
        let compose_cmd = format!(
            "cd {} && docker compose -f {} -p {} up -d --build --wait",
            config::BEAST_REPO,
            config::DISTRIBUTED_COMPOSE,
            compose_project,
        );
        let result = config::BEAST.exec(&compose_cmd).await?;
        if !result.success() {
            return Err(DistributedError::Compose(format!(
                "docker compose up failed: {}",
                result.stderr
            )));
        }

        // Discover Endpoint IDs from each relay's logs
        let mut relay_endpoint_ids = Vec::new();
        for i in 0..3 {
            let service = format!("relay-{}", i + 1);
            let id = Self::discover_endpoint_id_on_beast(&compose_project, &service).await?;
            relay_endpoint_ids.push(id);
        }

        // Create Guardian data directory
        config::GUARDIAN
            .exec_ok(&format!("mkdir -p {}", guardian_data_dir))
            .await?;

        Ok(Self {
            passphrase,
            relay_endpoint_ids,
            compose_project,
            q_data_dir,
            guardian_data_dir,
        })
    }

    /// Tear down all relays and clean up client state everywhere.
    pub async fn teardown(&self) -> Result<(), DistributedError> {
        // Stop relays on Beast
        let compose_cmd = format!(
            "cd {} && docker compose -f {} -p {} down -v --remove-orphans",
            config::BEAST_REPO,
            config::DISTRIBUTED_COMPOSE,
            self.compose_project,
        );
        config::BEAST.exec(&compose_cmd).await?;

        // Clean Guardian data
        config::GUARDIAN
            .exec(&format!("rm -rf {}", self.guardian_data_dir))
            .await?;

        // Q data dir cleaned up by TempDir drop
        Ok(())
    }

    /// Get the test passphrase.
    pub fn passphrase(&self) -> &str {
        &self.passphrase
    }

    /// Get all relay Endpoint IDs.
    pub fn relay_endpoint_ids(&self) -> &[String] {
        &self.relay_endpoint_ids
    }

    /// Get the compose project name.
    pub fn compose_project(&self) -> &str {
        &self.compose_project
    }

    /// Get Q's data directory path.
    pub fn q_data_dir(&self) -> &std::path::Path {
        self.q_data_dir.path()
    }

    /// Get Guardian's data directory path.
    pub fn guardian_data_dir(&self) -> &str {
        &self.guardian_data_dir
    }

    // ========================================================================
    // Relay management
    // ========================================================================

    /// Discover a relay's Endpoint ID from its Docker logs on Beast.
    async fn discover_endpoint_id_on_beast(
        project: &str,
        service: &str,
    ) -> Result<String, DistributedError> {
        let cmd = format!(
            "docker compose -p {} logs {} 2>&1",
            project, service
        );

        // Retry for up to ENDPOINT_ID_TIMEOUT_SECS
        let deadline = tokio::time::Instant::now()
            + tokio::time::Duration::from_secs(config::ENDPOINT_ID_TIMEOUT_SECS);

        while tokio::time::Instant::now() < deadline {
            let result = config::BEAST.exec(&cmd).await?;
            if let Some(id) = extract_endpoint_id(&result.stdout) {
                return Ok(id);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        // Parse index from service name (e.g., "relay-1" -> 0)
        let index = service
            .strip_prefix("relay-")
            .and_then(|s| s.parse::<usize>().ok())
            .map(|n| n.saturating_sub(1))
            .unwrap_or(0);

        Err(DistributedError::EndpointIdNotFound { index })
    }

    /// Check health of a specific relay via its HTTP endpoint.
    pub async fn relay_health(&self, index: usize) -> Result<bool, DistributedError> {
        let port = config::RELAY_HTTP_PORTS[index];
        let cmd = format!(
            "curl -sf http://localhost:{}/health",
            port
        );
        let result = config::BEAST.exec(&cmd).await?;
        Ok(result.success())
    }

    /// Kill a specific relay container on Beast.
    pub async fn kill_relay(&self, index: usize) -> Result<(), DistributedError> {
        let service = format!("relay-{}", index + 1);
        let cmd = format!(
            "docker compose -p {} kill {}",
            self.compose_project, service
        );
        config::BEAST.exec_ok(&cmd).await?;
        Ok(())
    }

    /// Restart a relay and return its new Endpoint ID.
    pub async fn restart_relay(&self, index: usize) -> Result<String, DistributedError> {
        let service = format!("relay-{}", index + 1);
        let cmd = format!(
            "docker compose -f {} -p {} up -d --wait {}",
            config::DISTRIBUTED_COMPOSE,
            self.compose_project,
            service,
        );
        let result = config::BEAST.exec(&format!("cd {} && {}", config::BEAST_REPO, cmd)).await?;
        if !result.success() {
            return Err(DistributedError::Compose(format!(
                "relay restart failed: {}",
                result.stderr
            )));
        }

        // Wait for new Endpoint ID
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        Self::discover_endpoint_id_on_beast(&self.compose_project, &service).await
    }

    /// Collect logs from all 3 relays.
    pub async fn all_relay_logs(&self) -> Result<Vec<String>, DistributedError> {
        let mut logs = Vec::new();
        for i in 1..=3 {
            let service = format!("relay-{}", i);
            let cmd = format!(
                "docker compose -p {} logs {} 2>&1",
                self.compose_project, service
            );
            let result = config::BEAST.exec(&cmd).await?;
            logs.push(result.stdout);
        }
        Ok(logs)
    }

    // ========================================================================
    // Guardian binary management
    // ========================================================================

    /// Ensure the ARM binary exists on Guardian.
    ///
    /// Checks if the binary exists. If not, cross-compiles on Beast and SCPs.
    pub async fn ensure_guardian_binary() -> Result<(), DistributedError> {
        // Check if binary exists on Guardian
        let check = config::GUARDIAN
            .exec(&format!("{} --version", config::GUARDIAN_CLI))
            .await?;

        if check.success() {
            return Ok(());
        }

        // Cross-compile on Beast
        let build_cmd = format!(
            "export PATH=$HOME/.cargo/bin:$PATH && cd {} && \
             cargo install cross 2>/dev/null || true && \
             cross build --target aarch64-unknown-linux-gnu -p zerok-sync-cli --release",
            config::BEAST_REPO
        );
        config::BEAST.exec_ok(&build_cmd).await.map_err(|e| {
            DistributedError::GuardianBinary(format!("cross-compile failed: {}", e))
        })?;

        // SCP from Beast to Guardian
        let scp_cmd = format!(
            "scp {}/target/aarch64-unknown-linux-gnu/release/sync-cli {}@{}:{}",
            config::BEAST_REPO,
            config::GUARDIAN.user,
            config::GUARDIAN.host,
            config::GUARDIAN_CLI,
        );
        config::BEAST.exec_ok(&scp_cmd).await.map_err(|e| {
            DistributedError::GuardianBinary(format!("SCP to Guardian failed: {}", e))
        })?;

        // Make executable
        config::GUARDIAN
            .exec_ok(&format!("chmod +x {}", config::GUARDIAN_CLI))
            .await?;

        Ok(())
    }

    // ========================================================================
    // Client orchestration
    // ========================================================================

    /// Initialize and pair all 3 clients with the primary relay, then
    /// inject secondary relay addresses.
    pub async fn init_and_pair_all(&self) -> Result<(), DistributedError> {
        let primary_relay = &self.relay_endpoint_ids[0];

        // Init + pair on Q (local)
        self.run_cli_q(&["init", "--name", "client-q"]).await?;
        self.run_cli_q(&["pair", "--join", primary_relay, "--passphrase", &self.passphrase])
            .await?;

        // Init + pair on Beast (docker exec in client-beast container)
        self.run_cli_beast(&["init", "--name", "client-beast"]).await?;
        self.run_cli_beast(&["pair", "--join", primary_relay, "--passphrase", &self.passphrase])
            .await?;

        // Init + pair on Guardian (SSH)
        self.run_cli_guardian(&["init", "--name", "client-guardian"]).await?;
        self.run_cli_guardian(&["pair", "--join", primary_relay, "--passphrase", &self.passphrase])
            .await?;

        // Inject secondary relay addresses into all group configs
        if self.relay_endpoint_ids.len() > 1 {
            self.inject_multi_relay_config(Machine::Q).await?;
            self.inject_multi_relay_config(Machine::Beast).await?;
            self.inject_multi_relay_config(Machine::Guardian).await?;
        }

        Ok(())
    }

    /// Push data from a specific machine.
    pub async fn push(&self, machine: Machine, data: &str) -> Result<String, DistributedError> {
        let result = match machine {
            Machine::Q => self.run_cli_q(&["push", data]).await?,
            Machine::Beast => self.run_cli_beast(&["push", data]).await?,
            Machine::Guardian => self.run_cli_guardian(&["push", data]).await?,
        };
        Ok(result)
    }

    /// Pull data from a specific machine.
    pub async fn pull(&self, machine: Machine) -> Result<String, DistributedError> {
        match machine {
            Machine::Q => self.run_cli_q(&["pull", "--after-cursor", "0"]).await,
            Machine::Beast => self.run_cli_beast(&["pull", "--after-cursor", "0"]).await,
            Machine::Guardian => self.run_cli_guardian(&["pull", "--after-cursor", "0"]).await,
        }
    }

    /// Pull data from a specific machine after a cursor.
    pub async fn pull_after(&self, machine: Machine, cursor: u64) -> Result<String, DistributedError> {
        let cursor_str = cursor.to_string();
        match machine {
            Machine::Q => self.run_cli_q(&["pull", "--after-cursor", &cursor_str]).await,
            Machine::Beast => self.run_cli_beast(&["pull", "--after-cursor", &cursor_str]).await,
            Machine::Guardian => self.run_cli_guardian(&["pull", "--after-cursor", &cursor_str]).await,
        }
    }

    // ========================================================================
    // CLI execution per machine
    // ========================================================================

    /// Run sync-cli on Q (local process).
    async fn run_cli_q(&self, args: &[&str]) -> Result<String, DistributedError> {
        let data_dir = self.q_data_dir.path().to_str().unwrap_or("/tmp/q-data");
        let mut cmd_args = vec!["--data-dir", data_dir];
        cmd_args.extend_from_slice(args);

        let output = tokio::process::Command::new("sync-cli")
            .args(&cmd_args)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DistributedError::ClientError {
                machine: "Q".into(),
                detail: format!("exit={}, stderr={}", output.status.code().unwrap_or(-1), stderr),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run sync-cli in the client-beast Docker container on Beast.
    async fn run_cli_beast(&self, args: &[&str]) -> Result<String, DistributedError> {
        let container = format!("{}-client-beast-1", self.compose_project);
        let mut cmd_parts = vec![
            "docker", "exec", &container,
            "sync-cli", "--data-dir", "/data",
        ];
        let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        for arg in &args_owned {
            cmd_parts.push(arg);
        }
        let cmd = cmd_parts.join(" ");
        let result = config::BEAST.exec_ok(&cmd).await.map_err(|e| {
            DistributedError::ClientError {
                machine: "Beast".into(),
                detail: e.to_string(),
            }
        })?;
        Ok(result.stdout)
    }

    /// Run sync-cli on Guardian via SSH.
    async fn run_cli_guardian(&self, args: &[&str]) -> Result<String, DistributedError> {
        let mut cmd = format!(
            "{} --data-dir {}",
            config::GUARDIAN_CLI,
            self.guardian_data_dir,
        );
        for arg in args {
            cmd.push(' ');
            // Quote args that might contain spaces
            if arg.contains(' ') {
                cmd.push('"');
                cmd.push_str(arg);
                cmd.push('"');
            } else {
                cmd.push_str(arg);
            }
        }
        let result = config::GUARDIAN.exec_ok(&cmd).await.map_err(|e| {
            DistributedError::ClientError {
                machine: "Guardian".into(),
                detail: e.to_string(),
            }
        })?;
        Ok(result.stdout)
    }

    // ========================================================================
    // Multi-relay config injection
    // ========================================================================

    /// Inject all relay Endpoint IDs into a client's group.json.
    ///
    /// After `pair --join <relay1>`, group.json has only relay-1's address.
    /// This patches it to include all 3 relays for failover testing.
    async fn inject_multi_relay_config(&self, machine: Machine) -> Result<(), DistributedError> {
        let all_ids: Vec<&str> = self.relay_endpoint_ids.iter().map(|s| s.as_str()).collect();
        // Build JSON array string
        let json_array = format!(
            "[{}]",
            all_ids
                .iter()
                .map(|id| format!("\"{}\"", id))
                .collect::<Vec<_>>()
                .join(",")
        );

        // Use Python-style sed to replace the relay_addresses array
        // The group.json has: "relay_node_ids":["<id1>"]  or  "relay_addresses":["<id1>"]
        // We need to replace it with all relay IDs
        let sed_cmd = format!(
            r#"sed -i 's/"relay_node_ids":\[[^]]*\]/"relay_node_ids":{}/g' "#,
            json_array,
        );

        match machine {
            Machine::Q => {
                let group_json = self.q_data_dir.path().join("group.json");
                let path = group_json.to_str().unwrap_or("");
                // macOS sed requires '' after -i
                let mac_sed = format!(
                    r#"sed -i '' 's/"relay_node_ids":\[[^]]*\]/"relay_node_ids":{}/g' "{}""#,
                    json_array, path,
                );
                let output = tokio::process::Command::new("sh")
                    .args(["-c", &mac_sed])
                    .output()
                    .await?;
                if !output.status.success() {
                    return Err(DistributedError::ClientError {
                        machine: "Q".into(),
                        detail: format!("sed failed: {}", String::from_utf8_lossy(&output.stderr)),
                    });
                }
            }
            Machine::Beast => {
                let container = format!("{}-client-beast-1", self.compose_project);
                let cmd = format!(
                    "docker exec {} sh -c '{}\"/data/group.json\"'",
                    container, sed_cmd
                );
                config::BEAST.exec_ok(&cmd).await.map_err(|e| {
                    DistributedError::ClientError {
                        machine: "Beast".into(),
                        detail: format!("sed failed: {}", e),
                    }
                })?;
            }
            Machine::Guardian => {
                let cmd = format!(
                    "{}\"{}/group.json\"",
                    sed_cmd, self.guardian_data_dir,
                );
                config::GUARDIAN.exec_ok(&cmd).await.map_err(|e| {
                    DistributedError::ClientError {
                        machine: "Guardian".into(),
                        detail: format!("sed failed: {}", e),
                    }
                })?;
            }
        }

        Ok(())
    }

    // ========================================================================
    // Chaos injection
    // ========================================================================

    /// Inject tc netem on a relay container or Guardian's interface.
    pub async fn inject_netem(
        &self,
        target: ChaosTarget,
        netem: &NetemConfig,
    ) -> Result<(), DistributedError> {
        let tc_args = netem.to_tc_add_args().join(" ");

        match target {
            ChaosTarget::Relay(index) => {
                let container = format!("{}-relay-{}-1", self.compose_project, index + 1);
                let cmd = format!("docker exec {} tc {}", container, tc_args);
                config::BEAST.exec_ok(&cmd).await?;
            }
            ChaosTarget::Guardian => {
                let cmd = format!("sudo tc {}", tc_args);
                config::GUARDIAN.exec_ok(&cmd).await?;
            }
        }

        Ok(())
    }

    /// Clear tc netem rules from a target.
    pub async fn clear_netem(&self, target: ChaosTarget) -> Result<(), DistributedError> {
        let netem = NetemConfig::new();
        let tc_args = netem.to_tc_del_args().join(" ");

        match target {
            ChaosTarget::Relay(index) => {
                let container = format!("{}-relay-{}-1", self.compose_project, index + 1);
                let cmd = format!("docker exec {} tc {}", container, tc_args);
                config::BEAST.exec(&cmd).await?; // Don't fail if no qdisc
            }
            ChaosTarget::Guardian => {
                let cmd = format!("sudo tc {}", tc_args);
                config::GUARDIAN.exec(&cmd).await?;
            }
        }

        Ok(())
    }

    /// Block traffic between two Tailscale IPs on Beast (iptables).
    pub async fn partition(&self, from_ip: &str, to_ip: &str) -> Result<(), DistributedError> {
        let cmd = format!(
            "sudo iptables -I INPUT -s {} -d {} -j DROP && \
             sudo iptables -I OUTPUT -s {} -d {} -j DROP",
            from_ip, to_ip, to_ip, from_ip
        );
        config::BEAST.exec_ok(&cmd).await?;
        Ok(())
    }

    /// Remove an iptables partition rule on Beast.
    pub async fn heal_partition(&self, from_ip: &str, to_ip: &str) -> Result<(), DistributedError> {
        let cmd = format!(
            "sudo iptables -D INPUT -s {} -d {} -j DROP 2>/dev/null; \
             sudo iptables -D OUTPUT -s {} -d {} -j DROP 2>/dev/null; true",
            from_ip, to_ip, to_ip, from_ip
        );
        config::BEAST.exec(&cmd).await?;
        Ok(())
    }
}

/// Extract an iroh Endpoint ID from a log line.
///
/// Looks for `Endpoint ID: <64-char hex>` pattern. Reuses the same
/// regex logic as `crate::harness::extract_endpoint_id`.
fn extract_endpoint_id(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(pos) = line.find("Endpoint ID: ") {
            let after = &line[pos + 13..];
            let hex_str: String = after
                .chars()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            if hex_str.len() == 64 {
                return Some(hex_str);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_endpoint_id_from_docker_logs() {
        let logs = "relay-1  | 2026-02-07T10:00:00Z INFO Endpoint ID: aa8e9a9115685ffab95d24c40714db6fae3e046b9eb197ccc1b04cb46a014444\nrelay-1  | 2026-02-07T10:00:01Z INFO listening";
        let id = extract_endpoint_id(logs);
        assert_eq!(
            id,
            Some("aa8e9a9115685ffab95d24c40714db6fae3e046b9eb197ccc1b04cb46a014444".to_string())
        );
    }

    #[test]
    fn extract_endpoint_id_not_found() {
        let logs = "relay-1  | 2026-02-07T10:00:00Z INFO starting up\nrelay-1  | ready";
        assert_eq!(extract_endpoint_id(logs), None);
    }

    #[test]
    fn machine_display() {
        assert_eq!(Machine::Q.to_string(), "Q");
        assert_eq!(Machine::Beast.to_string(), "Beast");
        assert_eq!(Machine::Guardian.to_string(), "Guardian");
    }

    // ====================================================================
    // Sprint 2: Remote relay management
    // ====================================================================

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_start_3_relays() {
        let harness = DistributedHarness::setup().await.expect("setup failed");

        // All 3 relays should be running
        assert_eq!(
            harness.relay_endpoint_ids().len(),
            3,
            "Expected 3 relay Endpoint IDs"
        );

        harness.teardown().await.expect("teardown failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_discover_endpoint_ids() {
        let harness = DistributedHarness::setup().await.expect("setup failed");

        // All 3 IDs should be distinct 64-char hex
        let ids = harness.relay_endpoint_ids();
        for (i, id) in ids.iter().enumerate() {
            assert_eq!(id.len(), 64, "Relay {} Endpoint ID wrong length: {}", i, id);
            assert!(
                id.chars().all(|c| c.is_ascii_hexdigit()),
                "Relay {} Endpoint ID not hex: {}",
                i,
                id
            );
        }

        // All distinct
        let unique: std::collections::HashSet<&str> =
            ids.iter().map(|s| s.as_str()).collect();
        assert_eq!(unique.len(), 3, "Endpoint IDs not unique: {:?}", ids);

        harness.teardown().await.expect("teardown failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_relay_health_checks() {
        let harness = DistributedHarness::setup().await.expect("setup failed");

        for i in 0..3 {
            let healthy = harness
                .relay_health(i)
                .await
                .expect("health check failed");
            assert!(healthy, "Relay {} not healthy", i);
        }

        harness.teardown().await.expect("teardown failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_teardown_cleans_up() {
        let harness = DistributedHarness::setup().await.expect("setup failed");
        let project = harness.compose_project().to_string();

        harness.teardown().await.expect("teardown failed");

        // Verify containers are gone
        let result = config::BEAST
            .exec(&format!("docker compose -p {} ps -q", project))
            .await
            .expect("ps check failed");
        assert!(
            result.stdout.trim().is_empty(),
            "Containers still running after teardown: {}",
            result.stdout
        );
    }

    // ====================================================================
    // Sprint 3: Guardian binary
    // ====================================================================

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_guardian_binary_exists() {
        DistributedHarness::ensure_guardian_binary()
            .await
            .expect("ensure_guardian_binary failed");

        let result = config::GUARDIAN
            .exec(&format!("test -x {}", config::GUARDIAN_CLI))
            .await
            .expect("SSH failed");
        assert!(result.success(), "Guardian binary not executable");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_guardian_cli_version() {
        DistributedHarness::ensure_guardian_binary()
            .await
            .expect("ensure_guardian_binary failed");

        let result = config::GUARDIAN
            .exec_ok(&format!("{} --version", config::GUARDIAN_CLI))
            .await
            .expect("version check failed");
        assert!(
            !result.stdout.trim().is_empty(),
            "Expected version output, got empty"
        );
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_guardian_init() {
        DistributedHarness::ensure_guardian_binary()
            .await
            .expect("ensure_guardian_binary failed");

        let data_dir = format!("{}/init-test-{}", config::GUARDIAN_DATA_DIR, uuid::Uuid::new_v4());

        // Init on Guardian
        let result = config::GUARDIAN
            .exec_ok(&format!(
                "{} --data-dir {} init --name test-guardian",
                config::GUARDIAN_CLI, data_dir
            ))
            .await
            .expect("init failed");
        assert!(result.success(), "init failed on Guardian");

        // Clean up
        config::GUARDIAN
            .exec(&format!("rm -rf {}", data_dir))
            .await
            .ok();
    }

    // ====================================================================
    // Sprint 4: Client orchestration
    // ====================================================================

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_init_pair_q() {
        let harness = DistributedHarness::setup().await.expect("setup failed");

        // Init and pair on Q only
        let primary_relay = &harness.relay_endpoint_ids()[0].clone();
        harness.run_cli_q(&["init", "--name", "test-q"]).await.expect("init Q failed");
        harness
            .run_cli_q(&["pair", "--join", primary_relay, "--passphrase", harness.passphrase()])
            .await
            .expect("pair Q failed");

        harness.teardown().await.expect("teardown failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_init_pair_guardian() {
        let harness = DistributedHarness::setup().await.expect("setup failed");

        DistributedHarness::ensure_guardian_binary()
            .await
            .expect("ensure_guardian_binary failed");

        let primary_relay = &harness.relay_endpoint_ids()[0].clone();
        harness
            .run_cli_guardian(&["init", "--name", "test-guardian"])
            .await
            .expect("init Guardian failed");
        harness
            .run_cli_guardian(&["pair", "--join", primary_relay, "--passphrase", harness.passphrase()])
            .await
            .expect("pair Guardian failed");

        harness.teardown().await.expect("teardown failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_init_pair_beast_container() {
        let harness = DistributedHarness::setup().await.expect("setup failed");

        let primary_relay = &harness.relay_endpoint_ids()[0].clone();
        harness
            .run_cli_beast(&["init", "--name", "test-beast"])
            .await
            .expect("init Beast container failed");
        harness
            .run_cli_beast(&["pair", "--join", primary_relay, "--passphrase", harness.passphrase()])
            .await
            .expect("pair Beast container failed");

        harness.teardown().await.expect("teardown failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_push_pull_q_to_guardian() {
        let harness = DistributedHarness::setup().await.expect("setup failed");

        DistributedHarness::ensure_guardian_binary()
            .await
            .expect("ensure_guardian_binary failed");

        // Init and pair Q + Guardian
        let primary_relay = &harness.relay_endpoint_ids()[0].clone();
        harness.run_cli_q(&["init", "--name", "test-q"]).await.unwrap();
        harness
            .run_cli_q(&["pair", "--join", primary_relay, "--passphrase", harness.passphrase()])
            .await
            .unwrap();
        harness
            .run_cli_guardian(&["init", "--name", "test-guardian"])
            .await
            .unwrap();
        harness
            .run_cli_guardian(&["pair", "--join", primary_relay, "--passphrase", harness.passphrase()])
            .await
            .unwrap();

        // Push from Q
        let msg = format!("dist-test-{}", uuid::Uuid::new_v4());
        harness.push(Machine::Q, &msg).await.expect("push from Q failed");

        // Wait for relay propagation
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Pull on Guardian
        let output = harness.pull(Machine::Guardian).await.expect("pull on Guardian failed");
        assert!(
            output.contains(&msg),
            "Guardian did not receive message. Output: {}",
            output
        );

        harness.teardown().await.expect("teardown failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_configure_multi_relay() {
        let harness = DistributedHarness::setup().await.expect("setup failed");

        DistributedHarness::ensure_guardian_binary()
            .await
            .expect("ensure_guardian_binary failed");

        harness.init_and_pair_all().await.expect("init_and_pair_all failed");

        // Verify Q's group.json has all 3 relay IDs
        let group_json_path = harness.q_data_dir().join("group.json");
        let content = tokio::fs::read_to_string(&group_json_path)
            .await
            .expect("read Q group.json");
        for id in harness.relay_endpoint_ids() {
            assert!(
                content.contains(id),
                "Q's group.json missing relay ID {}. Content: {}",
                id,
                content
            );
        }

        harness.teardown().await.expect("teardown failed");
    }

    // ====================================================================
    // Sprint 5: Chaos injection
    // ====================================================================

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_netem_relay_latency() {
        let harness = DistributedHarness::setup().await.expect("setup failed");

        let netem = NetemConfig::new().delay(200);
        harness
            .inject_netem(ChaosTarget::Relay(0), &netem)
            .await
            .expect("inject netem failed");

        // Verify qdisc was applied
        let container = format!("{}-relay-1-1", harness.compose_project());
        let result = config::BEAST
            .exec_ok(&format!("docker exec {} tc qdisc show dev eth0", container))
            .await
            .expect("tc show failed");
        assert!(
            result.stdout.contains("netem"),
            "netem not applied. Output: {}",
            result.stdout
        );

        harness.clear_netem(ChaosTarget::Relay(0)).await.ok();
        harness.teardown().await.expect("teardown failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_netem_guardian_loss() {
        let harness = DistributedHarness::setup().await.expect("setup failed");

        let netem = NetemConfig::new().loss(10.0);
        harness
            .inject_netem(ChaosTarget::Guardian, &netem)
            .await
            .expect("inject netem on Guardian failed");

        // Verify — tc show on Guardian
        let result = config::GUARDIAN
            .exec_ok("sudo tc qdisc show dev eth0")
            .await
            .expect("tc show on Guardian failed");
        assert!(
            result.stdout.contains("netem"),
            "netem not applied on Guardian. Output: {}",
            result.stdout
        );

        harness.clear_netem(ChaosTarget::Guardian).await.ok();
        harness.teardown().await.expect("teardown failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_partition_beast_guardian() {
        let harness = DistributedHarness::setup().await.expect("setup failed");

        // Partition
        harness
            .partition(config::GUARDIAN_IP, config::BEAST_IP)
            .await
            .expect("partition failed");

        // Verify iptables rule exists
        let result = config::BEAST
            .exec_ok("sudo iptables -L INPUT -n")
            .await
            .expect("iptables list failed");
        assert!(
            result.stdout.contains(config::GUARDIAN_IP),
            "iptables rule not found for Guardian IP"
        );

        // Heal
        harness
            .heal_partition(config::GUARDIAN_IP, config::BEAST_IP)
            .await
            .expect("heal failed");

        harness.teardown().await.expect("teardown failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_heal_partition() {
        let harness = DistributedHarness::setup().await.expect("setup failed");

        // Create and immediately heal
        harness
            .partition(config::GUARDIAN_IP, config::BEAST_IP)
            .await
            .expect("partition failed");
        harness
            .heal_partition(config::GUARDIAN_IP, config::BEAST_IP)
            .await
            .expect("heal failed");

        // Verify rule is gone — Guardian IP should not be in iptables DROP rules
        let result = config::BEAST
            .exec_ok("sudo iptables -L INPUT -n")
            .await
            .expect("iptables list failed");
        // After heal, the DROP rule for Guardian should be removed
        // (Note: there may be other rules, but our specific rule should be gone)
        let drop_lines: Vec<&str> = result
            .stdout
            .lines()
            .filter(|l| l.contains("DROP") && l.contains(config::GUARDIAN_IP))
            .collect();
        assert!(
            drop_lines.is_empty(),
            "iptables DROP rule still present after heal: {:?}",
            drop_lines
        );

        harness.teardown().await.expect("teardown failed");
    }
}
