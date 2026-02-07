//! SSH execution primitives for distributed chaos testing.
//!
//! Uses `tokio::process::Command` to shell out to `ssh` and `scp`.
//! Tailscale handles authentication — SSH keys must be pre-configured.

use std::path::Path;
use thiserror::Error;

/// Errors from SSH operations.
#[derive(Debug, Error)]
pub enum SshError {
    /// SSH command failed to execute (process spawn error).
    #[error("ssh spawn error: {0}")]
    Spawn(#[from] std::io::Error),

    /// SSH command returned non-zero exit code.
    #[error("ssh command failed on {host}: exit={exit_code}, stderr={stderr}")]
    CommandFailed {
        /// Target host.
        host: String,
        /// Exit code.
        exit_code: i32,
        /// Standard error output.
        stderr: String,
    },

    /// SCP transfer failed.
    #[error("scp failed: {0}")]
    ScpFailed(String),
}

/// Result of executing a command via SSH.
#[derive(Debug, Clone)]
pub struct SshResult {
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Exit code (0 = success).
    pub exit_code: i32,
}

impl SshResult {
    /// Returns true if the command succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// SSH target machine on the Tailscale mesh.
#[derive(Debug, Clone, Copy)]
pub struct SshTarget {
    /// Tailscale IP address.
    pub host: &'static str,
    /// SSH username.
    pub user: &'static str,
}

impl SshTarget {
    /// Execute a command on the remote machine via SSH.
    ///
    /// Returns the raw result including exit code, stdout, and stderr.
    /// Does NOT fail on non-zero exit — use `exec_ok` for that.
    pub async fn exec(&self, cmd: &str) -> Result<SshResult, SshError> {
        let output = tokio::process::Command::new("ssh")
            .args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "ConnectTimeout=30",
                "-o",
                "BatchMode=yes",
                &format!("{}@{}", self.user, self.host),
                cmd,
            ])
            .output()
            .await?;

        Ok(SshResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    /// Execute a command on the remote machine, failing on non-zero exit.
    pub async fn exec_ok(&self, cmd: &str) -> Result<SshResult, SshError> {
        let result = self.exec(cmd).await?;
        if !result.success() {
            return Err(SshError::CommandFailed {
                host: self.host.to_string(),
                exit_code: result.exit_code,
                stderr: result.stderr.clone(),
            });
        }
        Ok(result)
    }

    /// Copy a local file to the remote machine via SCP.
    pub async fn scp_to(&self, local: &Path, remote: &str) -> Result<(), SshError> {
        let output = tokio::process::Command::new("scp")
            .args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "ConnectTimeout=30",
                "-o",
                "BatchMode=yes",
                local.to_str().unwrap_or(""),
                &format!("{}@{}:{}", self.user, self.host, remote),
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SshError::ScpFailed(format!(
                "scp to {}@{}:{} failed: {}",
                self.user, self.host, remote, stderr
            )));
        }

        Ok(())
    }

    /// Copy a file from the remote machine to local via SCP.
    pub async fn scp_from(&self, remote: &str, local: &Path) -> Result<(), SshError> {
        let output = tokio::process::Command::new("scp")
            .args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "ConnectTimeout=30",
                "-o",
                "BatchMode=yes",
                &format!("{}@{}:{}", self.user, self.host, remote),
                local.to_str().unwrap_or(""),
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SshError::ScpFailed(format!(
                "scp from {}@{}:{} failed: {}",
                self.user, self.host, remote, stderr
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::distributed::config::{BEAST, GUARDIAN};

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn ssh_exec_beast_whoami() {
        let result = BEAST.exec_ok("whoami").await.expect("SSH to Beast failed");
        assert_eq!(result.stdout.trim(), "jimmyb");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn ssh_exec_beast_docker_version() {
        let result = BEAST
            .exec_ok("docker --version")
            .await
            .expect("Docker check on Beast failed");
        assert!(
            result.stdout.contains("Docker version"),
            "Expected Docker version string, got: {}",
            result.stdout
        );
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn ssh_exec_guardian_whoami() {
        let result = GUARDIAN
            .exec_ok("whoami")
            .await
            .expect("SSH to Guardian failed");
        assert_eq!(result.stdout.trim(), "jamesb");
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn ssh_exec_nonexistent_command() {
        let result = BEAST.exec("this-command-does-not-exist-xyz").await;
        match result {
            Ok(r) => assert_ne!(r.exit_code, 0, "Expected non-zero exit for bad command"),
            Err(_) => {} // Also acceptable — SSH itself might fail
        }
    }

    #[tokio::test]
    #[ignore = "requires distributed"]
    async fn ssh_scp_round_trip() {
        // Create a temp file locally
        let tmp_dir = tempfile::tempdir().expect("temp dir");
        let local_file = tmp_dir.path().join("scp-test.txt");
        let content = format!("scp-round-trip-{}", uuid::Uuid::new_v4());
        std::fs::write(&local_file, &content).expect("write local file");

        // SCP to Beast
        let remote_path = "/tmp/0k-sync-scp-test.txt";
        BEAST
            .scp_to(&local_file, remote_path)
            .await
            .expect("SCP to Beast failed");

        // Read back via SSH
        let result = BEAST
            .exec_ok(&format!("cat {}", remote_path))
            .await
            .expect("cat on Beast failed");
        assert_eq!(result.stdout.trim(), content);

        // Clean up remote
        BEAST
            .exec(&format!("rm -f {}", remote_path))
            .await
            .ok();
    }
}
