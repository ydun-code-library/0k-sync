//! Distributed chaos test harness — multi-machine orchestrator.
//!
//! Connects to 3 permanent relay instances on Beast (project `dist-chaos`).
//! Each test gets fresh client state (passphrase, data dirs) but shares
//! the same relays. Tests that kill/restart relays restore them before returning.
//!
//! ## Relay Lifecycle
//!
//! Relays are started ONCE and left running:
//! ```bash
//! ssh jimmyb@100.71.79.25 "cd ~/0k-sync && docker compose \
//!     -f tests/chaos/docker-compose.distributed.yml -p dist-chaos up -d --build --wait"
//! ```
//!
//! To stop them:
//! ```bash
//! ssh jimmyb@100.71.79.25 "cd ~/0k-sync && docker compose \
//!     -f tests/chaos/docker-compose.distributed.yml -p dist-chaos down -v"
//! ```

use thiserror::Error;

use super::config;
use super::ssh::SshError;
use crate::netem::NetemConfig;

/// Fixed Docker Compose project name for the permanent relays.
const COMPOSE_PROJECT: &str = "dist-chaos";

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

    /// Relays not running.
    #[error("relays not running — start them first: ssh jimmyb@100.71.79.25 \"cd ~/0k-sync && docker compose -f tests/chaos/docker-compose.distributed.yml -p dist-chaos up -d --build --wait\"")]
    RelaysNotRunning,
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
/// Connects to 3 permanent relays on Beast (`dist-chaos` project).
/// Each harness instance gets a unique passphrase and data directories
/// for test isolation, but shares the same relay infrastructure.
pub struct DistributedHarness {
    /// Random passphrase for this test's sync group.
    passphrase: String,
    /// Discovered Endpoint IDs for each relay (index 0-2).
    relay_endpoint_ids: Vec<String>,
    /// Local temp directory for Q's client state.
    q_data_dir: tempfile::TempDir,
    /// Guardian data directory for this test.
    guardian_data_dir: String,
    /// Unique test ID (for data dir isolation).
    test_id: String,
}

impl DistributedHarness {
    /// Connect to the permanent relays and create fresh client state.
    ///
    /// If any relays are down, restarts them before proceeding.
    pub async fn connect() -> Result<Self, DistributedError> {
        let test_id = uuid::Uuid::new_v4().as_simple().to_string()[..12].to_string();
        let passphrase = format!("dist-test-{}", &test_id);
        let q_data_dir = tempfile::tempdir()?;
        let guardian_data_dir = format!("{}/dist-{}", config::GUARDIAN_DATA_DIR, &test_id);

        // Check if all relays are healthy, restart if needed
        Self::ensure_relays_healthy().await?;

        // Discover Endpoint IDs from existing relay logs
        let mut relay_endpoint_ids = Vec::new();
        for i in 0..3 {
            let service = format!("relay-{}", i + 1);
            let id = Self::discover_endpoint_id(&service).await?;
            relay_endpoint_ids.push(id);
        }

        // Create Guardian data directory
        config::GUARDIAN
            .exec_ok(&format!("mkdir -p {}", guardian_data_dir))
            .await?;

        // Clear any stale chaos rules from prior test runs
        Self::clear_stale_chaos().await?;

        Ok(Self {
            passphrase,
            relay_endpoint_ids,
            q_data_dir,
            guardian_data_dir,
            test_id,
        })
    }

    /// Ensure all 3 relays are healthy. Restart any that are down.
    async fn ensure_relays_healthy() -> Result<(), DistributedError> {
        // Check health of each relay
        let mut any_down = false;
        for port in [8090, 8091, 8092] {
            let health = config::BEAST
                .exec(&format!("curl -sf http://localhost:{}/health", port))
                .await?;
            if !health.success() {
                any_down = true;
                break;
            }
        }

        // If any relay is down, restart all of them
        if any_down {
            let restart_cmd = format!(
                "cd ~/0k-sync/tests/chaos && docker compose -f docker-compose.distributed.yml -p {} restart",
                COMPOSE_PROJECT
            );
            config::BEAST.exec_ok(&restart_cmd).await.map_err(|e| {
                DistributedError::Compose(format!("relay restart failed: {}", e))
            })?;

            // Wait for relays to become healthy
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

            // Verify they're all healthy now
            for port in [8090, 8091, 8092] {
                let health = config::BEAST
                    .exec(&format!("curl -sf http://localhost:{}/health", port))
                    .await?;
                if !health.success() {
                    return Err(DistributedError::RelaysNotRunning);
                }
            }
        }

        Ok(())
    }

    /// Clear any stale chaos rules (iptables, tc netem) from prior test runs.
    /// Called on connect() to ensure clean state.
    async fn clear_stale_chaos() -> Result<(), DistributedError> {
        // Remove any DROP rules for our partition tests
        // Tests may partition: Guardian ↔ Beast, Q ↔ Beast
        // Use -D repeatedly until it fails (no more matching rules)
        let iptables_cmd = format!(
            "while sudo iptables -D INPUT -s {} -d {} -j DROP 2>/dev/null; do :; done; \
             while sudo iptables -D OUTPUT -s {} -d {} -j DROP 2>/dev/null; do :; done; \
             while sudo iptables -D INPUT -s {} -d {} -j DROP 2>/dev/null; do :; done; \
             while sudo iptables -D OUTPUT -s {} -d {} -j DROP 2>/dev/null; do :; done; \
             true",
            config::GUARDIAN_IP, config::BEAST_IP,
            config::BEAST_IP, config::GUARDIAN_IP,
            config::Q_IP, config::BEAST_IP,
            config::BEAST_IP, config::Q_IP
        );
        config::BEAST.exec(&iptables_cmd).await?;

        // Clear tc netem on relay containers
        for i in 1..=3 {
            let container = format!("{}-relay-{}-1", COMPOSE_PROJECT, i);
            let cmd = format!("docker exec {} tc qdisc del dev eth0 root 2>/dev/null || true", container);
            config::BEAST.exec(&cmd).await?;
        }

        // Clear tc netem on Guardian
        config::GUARDIAN
            .exec("sudo tc qdisc del dev eth0 root 2>/dev/null || true")
            .await?;

        Ok(())
    }

    /// Clean up client state only. Does NOT touch relays.
    pub async fn cleanup(&self) -> Result<(), DistributedError> {
        self.cleanup_client_state().await
    }

    /// Clean up client state on all machines. Safe to call even if state doesn't exist.
    /// This is called by cleanup() and also at the start of init_and_pair_all() to ensure
    /// idempotent initialization.
    async fn cleanup_client_state(&self) -> Result<(), DistributedError> {
        // Clean Guardian data for this test (ignore errors if dir doesn't exist)
        config::GUARDIAN
            .exec(&format!("rm -rf {} 2>/dev/null || true", self.guardian_data_dir))
            .await?;

        // Clean Beast client-beast container data for this test
        // (The container persists, but we can clean /data between tests)
        config::BEAST
            .exec(&format!(
                "docker exec {}-client-beast-1 sh -c 'rm -rf /data/* 2>/dev/null || true'",
                COMPOSE_PROJECT
            ))
            .await?;

        // Q data dir is a TempDir, but we should still clean it if it has stale data
        let q_data = self.q_data_dir.path();
        if q_data.exists() {
            for entry in std::fs::read_dir(q_data).into_iter().flatten() {
                if let Ok(entry) = entry {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }

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

    /// Get Q's data directory path.
    pub fn q_data_dir(&self) -> &std::path::Path {
        self.q_data_dir.path()
    }

    /// Get Guardian's data directory path.
    pub fn guardian_data_dir(&self) -> &str {
        &self.guardian_data_dir
    }

    /// Get the test ID.
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    // ========================================================================
    // Relay management (for tests that need to kill/restart)
    // ========================================================================

    /// Start all 3 relays on Beast (one-time setup).
    ///
    /// Run this once before a test session, not per test.
    pub async fn start_relays() -> Result<(), DistributedError> {
        let cmd = format!(
            "cd {} && docker compose -f {} -p {} up -d --build --wait",
            config::BEAST_REPO,
            config::DISTRIBUTED_COMPOSE,
            COMPOSE_PROJECT,
        );
        let result = config::BEAST.exec(&cmd).await?;
        if !result.success() {
            return Err(DistributedError::Compose(format!(
                "docker compose up failed: {}",
                result.stderr
            )));
        }
        Ok(())
    }

    /// Stop all relays on Beast (end of session cleanup).
    pub async fn stop_relays() -> Result<(), DistributedError> {
        let cmd = format!(
            "cd {} && docker compose -f {} -p {} down -v --remove-orphans",
            config::BEAST_REPO,
            config::DISTRIBUTED_COMPOSE,
            COMPOSE_PROJECT,
        );
        config::BEAST.exec(&cmd).await?;
        Ok(())
    }

    /// Discover a relay's Endpoint ID from its Docker logs on Beast.
    async fn discover_endpoint_id(service: &str) -> Result<String, DistributedError> {
        let cmd = format!(
            "docker compose -p {} logs {} 2>&1",
            COMPOSE_PROJECT, service
        );

        let deadline = tokio::time::Instant::now()
            + tokio::time::Duration::from_secs(config::ENDPOINT_ID_TIMEOUT_SECS);

        while tokio::time::Instant::now() < deadline {
            let result = config::BEAST.exec(&cmd).await?;
            if let Some(id) = extract_endpoint_id(&result.stdout) {
                return Ok(id);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

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
        let cmd = format!("curl -sf http://localhost:{}/health", port);
        let result = config::BEAST.exec(&cmd).await?;
        Ok(result.success())
    }

    /// Kill a specific relay container on Beast.
    pub async fn kill_relay(&self, index: usize) -> Result<(), DistributedError> {
        let service = format!("relay-{}", index + 1);
        let cmd = format!("docker compose -p {} kill {}", COMPOSE_PROJECT, service);
        config::BEAST.exec_ok(&cmd).await?;
        Ok(())
    }

    /// Restart a relay and return its new Endpoint ID.
    pub async fn restart_relay(&self, index: usize) -> Result<String, DistributedError> {
        let service = format!("relay-{}", index + 1);
        let cmd = format!(
            "cd {} && docker compose -f {} -p {} up -d --wait {}",
            config::BEAST_REPO,
            config::DISTRIBUTED_COMPOSE,
            COMPOSE_PROJECT,
            service,
        );
        let result = config::BEAST.exec(&cmd).await?;
        if !result.success() {
            return Err(DistributedError::Compose(format!(
                "relay restart failed: {}",
                result.stderr
            )));
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        Self::discover_endpoint_id(&service).await
    }

    /// Collect logs from all 3 relays.
    pub async fn all_relay_logs(&self) -> Result<Vec<String>, DistributedError> {
        let mut logs = Vec::new();
        for i in 1..=3 {
            let service = format!("relay-{}", i);
            let cmd = format!(
                "docker compose -p {} logs {} 2>&1",
                COMPOSE_PROJECT, service
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
        let check = config::GUARDIAN
            .exec(&format!("{} --version", config::GUARDIAN_CLI))
            .await?;

        if check.success() {
            return Ok(());
        }

        let build_cmd = format!(
            "export PATH=$HOME/.cargo/bin:$PATH && cd {} && \
             cargo install cross 2>/dev/null || true && \
             cross build --target aarch64-unknown-linux-gnu -p zerok-sync-cli --release",
            config::BEAST_REPO
        );
        config::BEAST.exec_ok(&build_cmd).await.map_err(|e| {
            DistributedError::GuardianBinary(format!("cross-compile failed: {}", e))
        })?;

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
        // Clean up any stale state first (makes this function idempotent)
        self.cleanup_client_state().await?;

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
        match machine {
            Machine::Q => self.run_cli_q(&["push", data]).await,
            Machine::Beast => self.run_cli_beast(&["push", data]).await,
            Machine::Guardian => self.run_cli_guardian(&["push", data]).await,
        }
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

        let output = tokio::process::Command::new(config::q_cli_path())
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
        let container = format!("{}-client-beast-1", COMPOSE_PROJECT);
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
    /// Uses native JSON manipulation for Q, Python for remote machines.
    async fn inject_multi_relay_config(&self, machine: Machine) -> Result<(), DistributedError> {
        let relay_ids = &self.relay_endpoint_ids;

        match machine {
            Machine::Q => {
                // For Q (local), use Rust JSON manipulation
                let group_json = self.q_data_dir.path().join("group.json");
                let content = tokio::fs::read_to_string(&group_json).await.map_err(|e| {
                    DistributedError::ClientError {
                        machine: "Q".into(),
                        detail: format!("read group.json failed: {}", e),
                    }
                })?;

                let mut json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                    DistributedError::ClientError {
                        machine: "Q".into(),
                        detail: format!("parse group.json failed: {}", e),
                    }
                })?;

                json["relay_addresses"] = serde_json::json!(relay_ids);

                let new_content = serde_json::to_string_pretty(&json).map_err(|e| {
                    DistributedError::ClientError {
                        machine: "Q".into(),
                        detail: format!("serialize group.json failed: {}", e),
                    }
                })?;

                tokio::fs::write(&group_json, new_content).await.map_err(|e| {
                    DistributedError::ClientError {
                        machine: "Q".into(),
                        detail: format!("write group.json failed: {}", e),
                    }
                })?;
            }
            Machine::Beast => {
                // For Beast (Docker container), use jq
                // Write JSON to temp file on Beast, docker cp into container
                let container = format!("{}-client-beast-1", COMPOSE_PROJECT);
                let json_array = serde_json::to_string(relay_ids).unwrap();

                // Step 1: Write relay array to temp file on Beast host
                let write_cmd = format!(
                    "cat > /tmp/relays.json << 'RELAYS_EOF'\n{}\nRELAYS_EOF",
                    json_array
                );
                config::BEAST.exec_ok(&write_cmd).await.map_err(|e| {
                    DistributedError::ClientError {
                        machine: "Beast".into(),
                        detail: format!("write relays.json failed: {}", e),
                    }
                })?;

                // Step 2: Copy into container
                let cp_cmd = format!("docker cp /tmp/relays.json {}:/tmp/relays.json", container);
                config::BEAST.exec_ok(&cp_cmd).await.map_err(|e| {
                    DistributedError::ClientError {
                        machine: "Beast".into(),
                        detail: format!("docker cp failed: {}", e),
                    }
                })?;

                // Step 3: Use jq with slurpfile
                let jq_cmd = format!(
                    "docker exec {} sh -c 'jq --slurpfile addrs /tmp/relays.json \".relay_addresses = \\$addrs[0]\" /data/group.json > /data/group.json.tmp && mv /data/group.json.tmp /data/group.json'",
                    container
                );
                config::BEAST.exec_ok(&jq_cmd).await.map_err(|e| {
                    DistributedError::ClientError {
                        machine: "Beast".into(),
                        detail: format!("jq inject failed: {}", e),
                    }
                })?;
            }
            Machine::Guardian => {
                // For Guardian (SSH), use Python via temp script file
                let json_array = serde_json::to_string(relay_ids).unwrap();
                let group_path = format!("{}/group.json", self.guardian_data_dir);

                // Write both the relay array and Python script via heredoc
                let script = format!(
                    r#"import json
relays = {}
with open('{}', 'r') as f:
    group = json.load(f)
group['relay_addresses'] = relays
with open('{}', 'w') as f:
    json.dump(group, f, indent=2)
"#,
                    json_array, group_path, group_path
                );

                let write_cmd = format!(
                    "cat > /tmp/inject_relays.py << 'SCRIPT_EOF'\n{}\nSCRIPT_EOF",
                    script
                );
                config::GUARDIAN.exec_ok(&write_cmd).await.map_err(|e| {
                    DistributedError::ClientError {
                        machine: "Guardian".into(),
                        detail: format!("write script failed: {}", e),
                    }
                })?;

                config::GUARDIAN.exec_ok("python3 /tmp/inject_relays.py").await.map_err(|e| {
                    DistributedError::ClientError {
                        machine: "Guardian".into(),
                        detail: format!("python inject failed: {}", e),
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
                let container = format!("{}-relay-{}-1", COMPOSE_PROJECT, index + 1);
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
                let container = format!("{}-relay-{}-1", COMPOSE_PROJECT, index + 1);
                let cmd = format!("docker exec {} tc {}", container, tc_args);
                config::BEAST.exec(&cmd).await?;
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

/// Extract the LATEST iroh Endpoint ID from log text.
/// Returns the last endpoint ID found, since relays may have been restarted.
fn extract_endpoint_id(text: &str) -> Option<String> {
    let mut last_id = None;
    for line in text.lines() {
        if let Some(pos) = line.find("Endpoint ID: ") {
            let after = &line[pos + 13..];
            let hex_str: String = after
                .chars()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            if hex_str.len() == 64 {
                last_id = Some(hex_str);
            }
        }
    }
    last_id
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
    // Relay connectivity (requires relays already running)
    // ====================================================================

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_connect_to_relays() {
        let harness = DistributedHarness::connect().await.expect("connect failed");

        assert_eq!(
            harness.relay_endpoint_ids().len(),
            3,
            "Expected 3 relay Endpoint IDs"
        );

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

        let unique: std::collections::HashSet<&str> =
            ids.iter().map(|s| s.as_str()).collect();
        assert_eq!(unique.len(), 3, "Endpoint IDs not unique: {:?}", ids);

        harness.cleanup().await.expect("cleanup failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_relay_health_checks() {
        let harness = DistributedHarness::connect().await.expect("connect failed");

        for i in 0..3 {
            let healthy = harness
                .relay_health(i)
                .await
                .expect("health check failed");
            assert!(healthy, "Relay {} not healthy", i);
        }

        harness.cleanup().await.expect("cleanup failed");
    }

    // ====================================================================
    // Guardian binary
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

        let result = config::GUARDIAN
            .exec_ok(&format!(
                "{} --data-dir {} init --name test-guardian",
                config::GUARDIAN_CLI, data_dir
            ))
            .await
            .expect("init failed");
        assert!(result.success(), "init failed on Guardian");

        config::GUARDIAN
            .exec(&format!("rm -rf {}", data_dir))
            .await
            .ok();
    }

    // ====================================================================
    // Client orchestration
    // ====================================================================

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_init_pair_q() {
        let harness = DistributedHarness::connect().await.expect("connect failed");

        let primary_relay = &harness.relay_endpoint_ids()[0].clone();
        harness.run_cli_q(&["init", "--name", "test-q"]).await.expect("init Q failed");
        harness
            .run_cli_q(&["pair", "--join", primary_relay, "--passphrase", harness.passphrase()])
            .await
            .expect("pair Q failed");

        harness.cleanup().await.expect("cleanup failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_init_pair_guardian() {
        let harness = DistributedHarness::connect().await.expect("connect failed");

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

        harness.cleanup().await.expect("cleanup failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_init_pair_beast_container() {
        let harness = DistributedHarness::connect().await.expect("connect failed");

        let primary_relay = &harness.relay_endpoint_ids()[0].clone();
        harness
            .run_cli_beast(&["init", "--name", "test-beast"])
            .await
            .expect("init Beast container failed");
        harness
            .run_cli_beast(&["pair", "--join", primary_relay, "--passphrase", harness.passphrase()])
            .await
            .expect("pair Beast container failed");

        harness.cleanup().await.expect("cleanup failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_push_pull_q_to_guardian() {
        let harness = DistributedHarness::connect().await.expect("connect failed");

        DistributedHarness::ensure_guardian_binary()
            .await
            .expect("ensure_guardian_binary failed");

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

        let msg = format!("dist-test-{}", uuid::Uuid::new_v4());
        harness.push(Machine::Q, &msg).await.expect("push from Q failed");

        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        let output = harness.pull(Machine::Guardian).await.expect("pull on Guardian failed");
        assert!(
            output.contains(&msg),
            "Guardian did not receive message. Output: {}",
            output
        );

        harness.cleanup().await.expect("cleanup failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_configure_multi_relay() {
        let harness = DistributedHarness::connect().await.expect("connect failed");

        DistributedHarness::ensure_guardian_binary()
            .await
            .expect("ensure_guardian_binary failed");

        harness.init_and_pair_all().await.expect("init_and_pair_all failed");

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

        harness.cleanup().await.expect("cleanup failed");
    }

    // ====================================================================
    // Chaos injection
    // ====================================================================

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_netem_relay_latency() {
        let harness = DistributedHarness::connect().await.expect("connect failed");

        let netem = NetemConfig::new().delay(200);
        harness
            .inject_netem(ChaosTarget::Relay(0), &netem)
            .await
            .expect("inject netem failed");

        let container = format!("{}-relay-1-1", COMPOSE_PROJECT);
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
        harness.cleanup().await.expect("cleanup failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn distributed_netem_guardian_loss() {
        let harness = DistributedHarness::connect().await.expect("connect failed");

        let netem = NetemConfig::new().loss(10.0);
        harness
            .inject_netem(ChaosTarget::Guardian, &netem)
            .await
            .expect("inject netem on Guardian failed");

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
        harness.cleanup().await.expect("cleanup failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed + passwordless sudo for iptables on Beast"]
    async fn distributed_partition_beast_guardian() {
        let harness = DistributedHarness::connect().await.expect("connect failed");

        harness
            .partition(config::GUARDIAN_IP, config::BEAST_IP)
            .await
            .expect("partition failed");

        let result = config::BEAST
            .exec_ok("sudo iptables -L INPUT -n")
            .await
            .expect("iptables list failed");
        assert!(
            result.stdout.contains(config::GUARDIAN_IP),
            "iptables rule not found for Guardian IP"
        );

        harness
            .heal_partition(config::GUARDIAN_IP, config::BEAST_IP)
            .await
            .expect("heal failed");

        harness.cleanup().await.expect("cleanup failed");
    }

    #[tokio::test]
    #[ignore = "requires distributed + passwordless sudo for iptables on Beast"]
    async fn distributed_heal_partition() {
        let harness = DistributedHarness::connect().await.expect("connect failed");

        harness
            .partition(config::GUARDIAN_IP, config::BEAST_IP)
            .await
            .expect("partition failed");
        harness
            .heal_partition(config::GUARDIAN_IP, config::BEAST_IP)
            .await
            .expect("heal failed");

        let result = config::BEAST
            .exec_ok("sudo iptables -L INPUT -n")
            .await
            .expect("iptables list failed");
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

        harness.cleanup().await.expect("cleanup failed");
    }
}
