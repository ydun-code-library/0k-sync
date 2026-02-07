//! Container chaos operations via bollard Docker API.
//!
//! Replaces the Pumba binary dependency with direct bollard calls.
//! All container lifecycle operations (kill, pause, stop, restart)
//! are now handled through `ChaosHarness` methods.
//!
//! This module retains the `ContainerAction` and `PumbaConfig` types
//! for backward compatibility with existing test code.

use thiserror::Error;

/// Errors that can occur during container chaos operations.
#[derive(Debug, Error)]
pub enum PumbaError {
    /// Command execution failed
    #[error("command failed: {0}")]
    CommandFailed(String),

    /// Container not found
    #[error("container not found: {0}")]
    ContainerNotFound(String),

    /// Invalid configuration
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

/// Action to perform on a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerAction {
    /// Kill the container
    Kill,
    /// Pause the container
    Pause,
    /// Stop the container
    Stop,
    /// Remove the container
    Remove,
}

impl ContainerAction {
    /// Get the action as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ContainerAction::Kill => "kill",
            ContainerAction::Pause => "pause",
            ContainerAction::Stop => "stop",
            ContainerAction::Remove => "rm",
        }
    }
}

/// Configuration for a container chaos action.
///
/// Use `ChaosHarness::kill_container()`, `ChaosHarness::pause_container()`,
/// etc. instead of building these manually.
#[derive(Debug, Clone)]
pub struct PumbaConfig {
    /// Container name or regex pattern
    pub container: String,
    /// Action to perform
    pub action: ContainerAction,
    /// Duration for pause action (seconds)
    pub duration_secs: Option<u64>,
    /// Signal for kill action
    pub signal: Option<String>,
    /// Dry run (don't actually execute)
    pub dry_run: bool,
}

impl PumbaConfig {
    /// Create a new config for killing a container.
    pub fn kill(container: &str) -> Self {
        Self {
            container: container.into(),
            action: ContainerAction::Kill,
            duration_secs: None,
            signal: Some("SIGKILL".into()),
            dry_run: false,
        }
    }

    /// Create a new config for pausing a container.
    pub fn pause(container: &str, duration_secs: u64) -> Self {
        Self {
            container: container.into(),
            action: ContainerAction::Pause,
            duration_secs: Some(duration_secs),
            signal: None,
            dry_run: false,
        }
    }

    /// Create a new config for stopping a container.
    pub fn stop(container: &str) -> Self {
        Self {
            container: container.into(),
            action: ContainerAction::Stop,
            duration_secs: None,
            signal: None,
            dry_run: false,
        }
    }

    /// Set dry run mode.
    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Set the kill signal.
    pub fn signal(mut self, signal: &str) -> Self {
        self.signal = Some(signal.into());
        self
    }
}

/// Pumba command builder (legacy — prefer ChaosHarness methods).
pub struct PumbaCommand {
    config: PumbaConfig,
}

impl PumbaCommand {
    /// Create a new Pumba command.
    pub fn new(config: PumbaConfig) -> Self {
        Self { config }
    }

    /// Build the command arguments.
    pub fn build_args(&self) -> Vec<String> {
        let mut args = vec![];

        if self.config.dry_run {
            args.push("--dry-run".into());
        }

        // Action
        args.push(self.config.action.as_str().into());

        // Action-specific options
        match self.config.action {
            ContainerAction::Kill => {
                if let Some(signal) = &self.config.signal {
                    args.push("--signal".into());
                    args.push(signal.clone());
                }
            }
            ContainerAction::Pause => {
                if let Some(duration) = self.config.duration_secs {
                    args.push("--duration".into());
                    args.push(format!("{}s", duration));
                }
            }
            _ => {}
        }

        // Container target
        args.push(self.config.container.clone());

        args
    }

    /// Build the full command string.
    pub fn build_command(&self) -> String {
        let args = self.build_args();
        format!("pumba {}", args.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kill_command() {
        let config = PumbaConfig::kill("relay-1");
        let cmd = PumbaCommand::new(config);
        let command = cmd.build_command();

        assert!(command.contains("kill"));
        assert!(command.contains("--signal SIGKILL"));
        assert!(command.contains("relay-1"));
    }

    #[test]
    fn pause_command() {
        let config = PumbaConfig::pause("client-a", 30);
        let cmd = PumbaCommand::new(config);
        let command = cmd.build_command();

        assert!(command.contains("pause"));
        assert!(command.contains("--duration 30s"));
        assert!(command.contains("client-a"));
    }

    #[test]
    fn stop_command() {
        let config = PumbaConfig::stop("relay-1");
        let cmd = PumbaCommand::new(config);
        let command = cmd.build_command();

        assert!(command.contains("stop"));
        assert!(command.contains("relay-1"));
    }

    #[test]
    fn dry_run_flag() {
        let config = PumbaConfig::kill("test").dry_run(true);
        let cmd = PumbaCommand::new(config);
        let args = cmd.build_args();

        assert!(args.contains(&"--dry-run".to_string()));
    }

    #[test]
    fn custom_signal() {
        let config = PumbaConfig::kill("test").signal("SIGTERM");
        let cmd = PumbaCommand::new(config);
        let command = cmd.build_command();

        assert!(command.contains("--signal SIGTERM"));
    }

    #[test]
    fn container_action_strings() {
        assert_eq!(ContainerAction::Kill.as_str(), "kill");
        assert_eq!(ContainerAction::Pause.as_str(), "pause");
        assert_eq!(ContainerAction::Stop.as_str(), "stop");
        assert_eq!(ContainerAction::Remove.as_str(), "rm");
    }
}
