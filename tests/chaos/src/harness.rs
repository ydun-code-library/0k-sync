//! Chaos test harness — high-level orchestrator for Docker-based chaos tests.
//!
//! Manages the lifecycle of a Docker Compose topology and provides methods
//! for executing CLI commands inside containers, injecting network chaos,
//! and collecting test state.

use bollard::container::{LogsOptions, RestartContainerOptions};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::Docker;
use futures_util::StreamExt;
use std::path::PathBuf;
use thiserror::Error;

use crate::netem::NetemConfig;

/// Errors that can occur during chaos harness operations.
#[derive(Debug, Error)]
pub enum HarnessError {
    /// Docker API error.
    #[error("docker error: {0}")]
    Docker(#[from] bollard::errors::Error),

    /// Command execution failed inside container.
    #[error("exec failed in {container}: exit={exit_code}, stderr={stderr}")]
    ExecFailed {
        /// Container name.
        container: String,
        /// Exit code from command.
        exit_code: i64,
        /// Standard error output.
        stderr: String,
    },

    /// Docker Compose CLI error.
    #[error("compose error: {0}")]
    Compose(String),

    /// Timeout waiting for relay endpoint ID.
    #[error("timeout waiting for relay endpoint ID")]
    EndpointIdTimeout,

    /// Could not parse endpoint ID from relay logs.
    #[error("could not parse endpoint ID from relay logs")]
    EndpointIdNotFound,

    /// General I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result of executing a command inside a container.
#[derive(Debug, Clone)]
pub struct ExecResult {
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Exit code (0 = success).
    pub exit_code: i64,
}

impl ExecResult {
    /// Returns true if the command succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// High-level chaos test orchestrator.
///
/// Manages a Docker Compose topology with unique project name for test isolation.
/// Provides methods for:
/// - Starting/stopping the topology
/// - Discovering the relay's iroh Endpoint ID
/// - Executing CLI commands inside containers (init, pair, push, pull)
/// - Injecting/clearing network chaos via `tc netem`
/// - Container lifecycle operations (kill, restart, pause/unpause)
/// - Collecting relay logs for assertion
pub struct ChaosHarness {
    /// Unique project name for Docker Compose isolation.
    project_name: String,
    /// Path to docker-compose.chaos.yml.
    compose_file: PathBuf,
    /// bollard Docker client.
    docker: Docker,
    /// Passphrase for this test's sync group.
    passphrase: String,
    /// Relay's iroh Endpoint ID (discovered from logs after startup).
    relay_endpoint_id: Option<String>,
}

impl ChaosHarness {
    /// Create a new harness with a unique project name.
    ///
    /// The compose_file path should be absolute or relative to cwd.
    pub fn new(compose_file: PathBuf) -> Result<Self, HarnessError> {
        let docker = Docker::connect_with_local_defaults()?;
        let project_name = format!("chaos-{}", uuid::Uuid::new_v4().as_simple());
        let passphrase = format!("chaos-test-{}", &project_name[6..22]);

        Ok(Self {
            project_name,
            compose_file,
            docker,
            passphrase,
            relay_endpoint_id: None,
        })
    }

    /// Get the unique project name.
    pub fn project_name(&self) -> &str {
        &self.project_name
    }

    /// Get the test passphrase.
    pub fn passphrase(&self) -> &str {
        &self.passphrase
    }

    /// Get the relay endpoint ID (available after setup).
    pub fn relay_endpoint_id(&self) -> Option<&str> {
        self.relay_endpoint_id.as_deref()
    }

    // ========================================================================
    // Lifecycle
    // ========================================================================

    /// Start the Docker Compose topology and discover the relay Endpoint ID.
    ///
    /// Runs `docker compose up -d --build --wait` with the unique project name,
    /// then streams relay logs to extract the Endpoint ID.
    pub async fn setup(&mut self) -> Result<(), HarnessError> {
        // docker compose up
        let output = tokio::process::Command::new("docker")
            .args([
                "compose",
                "-f",
                self.compose_file
                    .to_str()
                    .unwrap_or("docker-compose.chaos.yml"),
                "-p",
                &self.project_name,
                "up",
                "-d",
                "--build",
                "--wait",
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HarnessError::Compose(format!(
                "docker compose up failed: {}",
                stderr
            )));
        }

        // Discover relay endpoint ID from logs
        self.relay_endpoint_id = Some(self.discover_relay_endpoint_id().await?);

        Ok(())
    }

    /// Tear down the Docker Compose topology, removing containers and volumes.
    pub async fn teardown(&self) -> Result<(), HarnessError> {
        let output = tokio::process::Command::new("docker")
            .args([
                "compose",
                "-f",
                self.compose_file
                    .to_str()
                    .unwrap_or("docker-compose.chaos.yml"),
                "-p",
                &self.project_name,
                "down",
                "-v",
                "--remove-orphans",
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HarnessError::Compose(format!(
                "docker compose down failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Discover the relay's iroh Endpoint ID from its container logs.
    ///
    /// Streams logs from the relay container and looks for
    /// `Endpoint ID: <64-char hex>`.
    async fn discover_relay_endpoint_id(&self) -> Result<String, HarnessError> {
        let container_name = self.container_name("relay");

        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            follow: true,
            ..Default::default()
        };

        let mut log_stream = self.docker.logs(&container_name, Some(options));

        let timeout = tokio::time::Duration::from_secs(60);
        let deadline = tokio::time::Instant::now() + timeout;

        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout_at(deadline, log_stream.next()).await {
                Ok(Some(Ok(log_output))) => {
                    let line = log_output.to_string();
                    if let Some(id) = extract_endpoint_id(&line) {
                        return Ok(id);
                    }
                }
                Ok(Some(Err(e))) => return Err(HarnessError::Docker(e)),
                Ok(None) => break, // Stream ended
                Err(_) => return Err(HarnessError::EndpointIdTimeout),
            }
        }

        Err(HarnessError::EndpointIdNotFound)
    }

    // ========================================================================
    // Container exec
    // ========================================================================

    /// Execute a command inside a container and return the result.
    pub async fn exec_in_container(
        &self,
        service: &str,
        cmd: Vec<&str>,
    ) -> Result<ExecResult, HarnessError> {
        let container_name = self.container_name(service);

        let exec_options = CreateExecOptions {
            cmd: Some(cmd.iter().map(|s| s.to_string()).collect()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(&container_name, exec_options)
            .await?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        if let StartExecResults::Attached { mut output, .. } =
            self.docker.start_exec(&exec.id, None).await?
        {
            while let Some(Ok(msg)) = output.next().await {
                match msg {
                    bollard::container::LogOutput::StdOut { message } => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
        }

        // Get exit code
        let inspect = self.docker.inspect_exec(&exec.id).await?;
        let exit_code = inspect.exit_code.unwrap_or(-1);

        Ok(ExecResult {
            stdout,
            stderr,
            exit_code,
        })
    }

    /// Execute a command inside a container, returning error on non-zero exit.
    pub async fn exec_ok(&self, service: &str, cmd: Vec<&str>) -> Result<ExecResult, HarnessError> {
        let result = self.exec_in_container(service, cmd).await?;
        if !result.success() {
            return Err(HarnessError::ExecFailed {
                container: service.to_string(),
                exit_code: result.exit_code,
                stderr: result.stderr.clone(),
            });
        }
        Ok(result)
    }

    // ========================================================================
    // CLI orchestration
    // ========================================================================

    /// Initialize a client device inside its container.
    pub async fn init_client(&self, service: &str, name: &str) -> Result<ExecResult, HarnessError> {
        self.exec_ok(
            service,
            vec!["sync-cli", "--data-dir", "/data", "init", "--name", name],
        )
        .await
    }

    /// Create a sync group on a client (pair --create).
    pub async fn pair_create(&self, service: &str) -> Result<ExecResult, HarnessError> {
        self.exec_ok(
            service,
            vec![
                "sync-cli",
                "--data-dir",
                "/data",
                "pair",
                "--create",
                "--passphrase",
                &self.passphrase,
            ],
        )
        .await
    }

    /// Join a sync group on a client (pair --join with relay endpoint ID).
    pub async fn pair_join(&self, service: &str) -> Result<ExecResult, HarnessError> {
        let endpoint_id = self
            .relay_endpoint_id
            .as_deref()
            .ok_or(HarnessError::EndpointIdNotFound)?;

        self.exec_ok(
            service,
            vec![
                "sync-cli",
                "--data-dir",
                "/data",
                "pair",
                "--join",
                endpoint_id,
                "--passphrase",
                &self.passphrase,
            ],
        )
        .await
    }

    /// Initialize and pair both clients (both join with relay endpoint ID).
    ///
    /// Both clients use `pair --join <endpoint_id>` with the same passphrase
    /// so they derive identical group credentials AND store the real relay
    /// address. Using `pair --create` would store a placeholder relay address,
    /// causing push/pull to fail.
    pub async fn init_and_pair(&self) -> Result<(), HarnessError> {
        // Init both clients
        self.init_client("client-a", "client-a").await?;
        self.init_client("client-b", "client-b").await?;

        // Both clients join with relay endpoint ID (same passphrase = same group)
        self.pair_join("client-a").await?;
        self.pair_join("client-b").await?;

        Ok(())
    }

    /// Push data from a client.
    pub async fn push(&self, service: &str, message: &str) -> Result<ExecResult, HarnessError> {
        self.exec_ok(
            service,
            vec!["sync-cli", "--data-dir", "/data", "push", message],
        )
        .await
    }

    /// Pull data on a client.
    pub async fn pull(&self, service: &str) -> Result<ExecResult, HarnessError> {
        self.exec_ok(
            service,
            vec![
                "sync-cli",
                "--data-dir",
                "/data",
                "pull",
                "--after-cursor",
                "0",
            ],
        )
        .await
    }

    /// Pull data on a client after a specific cursor.
    pub async fn pull_after(&self, service: &str, cursor: u64) -> Result<ExecResult, HarnessError> {
        let cursor_str = cursor.to_string();
        self.exec_ok(
            service,
            vec![
                "sync-cli",
                "--data-dir",
                "/data",
                "pull",
                "--after-cursor",
                &cursor_str,
            ],
        )
        .await
    }

    // ========================================================================
    // Network chaos (tc netem)
    // ========================================================================

    /// Inject network chaos into a container using `tc netem`.
    pub async fn inject_netem(
        &self,
        service: &str,
        config: &NetemConfig,
    ) -> Result<ExecResult, HarnessError> {
        let mut cmd = vec!["tc"];
        let args = config.to_tc_add_args();
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        cmd.extend(arg_refs);

        self.exec_ok(service, cmd).await
    }

    /// Clear network chaos from a container.
    pub async fn clear_netem(&self, service: &str) -> Result<ExecResult, HarnessError> {
        let config = NetemConfig::new();
        let mut cmd = vec!["tc"];
        let args = config.to_tc_del_args();
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        cmd.extend(arg_refs);

        // Clearing may fail if no qdisc exists — that's ok
        self.exec_in_container(service, cmd).await
    }

    // ========================================================================
    // Container lifecycle (replaces Pumba)
    // ========================================================================

    /// Kill a container with SIGKILL.
    pub async fn kill_container(&self, service: &str) -> Result<(), HarnessError> {
        let name = self.container_name(service);
        self.docker.kill_container::<String>(&name, None).await?;
        Ok(())
    }

    /// Restart a container.
    pub async fn restart_container(&self, service: &str) -> Result<(), HarnessError> {
        let name = self.container_name(service);
        self.docker
            .restart_container(&name, Some(RestartContainerOptions { t: 10 }))
            .await?;
        Ok(())
    }

    /// Pause a container (freeze all processes).
    pub async fn pause_container(&self, service: &str) -> Result<(), HarnessError> {
        let name = self.container_name(service);
        self.docker.pause_container(&name).await?;
        Ok(())
    }

    /// Unpause a paused container.
    pub async fn unpause_container(&self, service: &str) -> Result<(), HarnessError> {
        let name = self.container_name(service);
        self.docker.unpause_container(&name).await?;
        Ok(())
    }

    // ========================================================================
    // Log collection
    // ========================================================================

    /// Collect all relay logs.
    pub async fn relay_logs(&self) -> Result<String, HarnessError> {
        self.container_logs("relay").await
    }

    /// Collect all logs from a specific service container.
    pub async fn container_logs(&self, service: &str) -> Result<String, HarnessError> {
        let container_name = self.container_name(service);

        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            follow: false,
            ..Default::default()
        };

        let mut log_stream = self.docker.logs(&container_name, Some(options));
        let mut logs = String::new();

        while let Some(Ok(log_output)) = log_stream.next().await {
            logs.push_str(&log_output.to_string());
        }

        Ok(logs)
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    /// Build the full container name from project name and service.
    ///
    /// Docker Compose naming: `<project>-<service>-1`
    fn container_name(&self, service: &str) -> String {
        format!("{}-{}-1", self.project_name, service)
    }
}

/// Extract the iroh Endpoint ID from a log line.
///
/// Looks for `Endpoint ID: <64-char hex>` pattern.
fn extract_endpoint_id(line: &str) -> Option<String> {
    // Look for "Endpoint ID: " followed by a 64-char hex string
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
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_endpoint_id_from_log_line() {
        let line = "2026-02-07T10:00:00Z INFO Endpoint ID: aa8e9a9115685ffab95d24c40714db6fae3e046b9eb197ccc1b04cb46a014444";
        let id = extract_endpoint_id(line);
        assert_eq!(
            id,
            Some("aa8e9a9115685ffab95d24c40714db6fae3e046b9eb197ccc1b04cb46a014444".to_string())
        );
    }

    #[test]
    fn extract_endpoint_id_from_println() {
        let line = "Endpoint ID: aa8e9a9115685ffab95d24c40714db6fae3e046b9eb197ccc1b04cb46a014444";
        let id = extract_endpoint_id(line);
        assert_eq!(
            id,
            Some("aa8e9a9115685ffab95d24c40714db6fae3e046b9eb197ccc1b04cb46a014444".to_string())
        );
    }

    #[test]
    fn extract_endpoint_id_too_short() {
        let line = "Endpoint ID: aa8e9a91";
        let id = extract_endpoint_id(line);
        assert_eq!(id, None);
    }

    #[test]
    fn extract_endpoint_id_no_match() {
        let line = "2026-02-07 INFO relay started on port 8080";
        let id = extract_endpoint_id(line);
        assert_eq!(id, None);
    }

    #[test]
    fn container_name_format() {
        // Verify the naming convention
        let project = "chaos-abc123";
        let service = "relay";
        let expected = "chaos-abc123-relay-1";
        assert_eq!(format!("{}-{}-1", project, service), expected);
    }

    #[test]
    fn harness_unique_project_names() {
        let h1 = ChaosHarness::new(PathBuf::from("test.yml")).unwrap();
        let h2 = ChaosHarness::new(PathBuf::from("test.yml")).unwrap();

        assert_ne!(h1.project_name(), h2.project_name());
        assert!(h1.project_name().starts_with("chaos-"));
        assert!(h2.project_name().starts_with("chaos-"));
    }

    #[test]
    fn harness_passphrase_generated() {
        let h = ChaosHarness::new(PathBuf::from("test.yml")).unwrap();
        assert!(h.passphrase().starts_with("chaos-test-"));
        assert!(h.passphrase().len() >= 20);
    }

    #[test]
    fn harness_endpoint_id_none_before_setup() {
        let h = ChaosHarness::new(PathBuf::from("test.yml")).unwrap();
        assert!(h.relay_endpoint_id().is_none());
    }
}
