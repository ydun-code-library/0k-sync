//! `tc netem` command builder for network chaos injection.
//!
//! Builds `tc qdisc` commands for injecting latency, packet loss,
//! and bandwidth constraints via Linux traffic control.
//! Requires `iproute2` in the container and `NET_ADMIN` capability.

/// Network emulation configuration for `tc netem`.
#[derive(Debug, Clone, Default)]
pub struct NetemConfig {
    /// Fixed delay in milliseconds.
    pub delay_ms: Option<u64>,
    /// Jitter in milliseconds (requires delay_ms).
    pub jitter_ms: Option<u64>,
    /// Packet loss percentage (0.0–100.0).
    pub loss_percent: Option<f32>,
    /// Loss correlation percentage (0.0–100.0) for burst loss patterns.
    pub loss_correlation: Option<f32>,
    /// Bandwidth limit in kbit/s.
    pub rate_kbit: Option<u64>,
    /// Network interface to apply rules to.
    pub interface: String,
}

impl NetemConfig {
    /// Create a new NetemConfig for the default `eth0` interface.
    pub fn new() -> Self {
        Self {
            interface: "eth0".into(),
            ..Default::default()
        }
    }

    /// Set the network interface.
    pub fn interface(mut self, iface: &str) -> Self {
        self.interface = iface.into();
        self
    }

    /// Add fixed latency.
    pub fn delay(mut self, ms: u64) -> Self {
        self.delay_ms = Some(ms);
        self
    }

    /// Add jitter (requires delay).
    pub fn jitter(mut self, ms: u64) -> Self {
        self.jitter_ms = Some(ms);
        self
    }

    /// Add packet loss.
    pub fn loss(mut self, percent: f32) -> Self {
        self.loss_percent = Some(percent);
        self
    }

    /// Add loss correlation for burst patterns.
    pub fn loss_correlation(mut self, percent: f32) -> Self {
        self.loss_correlation = Some(percent);
        self
    }

    /// Add bandwidth limit.
    pub fn rate(mut self, kbit: u64) -> Self {
        self.rate_kbit = Some(kbit);
        self
    }

    /// Build the `tc qdisc add` command arguments.
    ///
    /// Returns args for: `tc qdisc add dev <iface> root netem <params>`
    pub fn to_tc_add_args(&self) -> Vec<String> {
        let mut args = vec![
            "qdisc".into(),
            "add".into(),
            "dev".into(),
            self.interface.clone(),
            "root".into(),
            "netem".into(),
        ];

        if let Some(delay) = self.delay_ms {
            args.push("delay".into());
            args.push(format!("{}ms", delay));

            if let Some(jitter) = self.jitter_ms {
                args.push(format!("{}ms", jitter));
            }
        }

        if let Some(loss) = self.loss_percent {
            args.push("loss".into());
            args.push(format!("{:.1}%", loss));

            if let Some(corr) = self.loss_correlation {
                args.push(format!("{:.1}%", corr));
            }
        }

        if let Some(rate) = self.rate_kbit {
            args.push("rate".into());
            args.push(format!("{}kbit", rate));
        }

        args
    }

    /// Build the `tc qdisc del` command arguments for clearing rules.
    ///
    /// Returns args for: `tc qdisc del dev <iface> root`
    pub fn to_tc_del_args(&self) -> Vec<String> {
        vec![
            "qdisc".into(),
            "del".into(),
            "dev".into(),
            self.interface.clone(),
            "root".into(),
        ]
    }

    /// Build the full `tc` command string (for logging/debugging).
    pub fn to_tc_command(&self) -> String {
        let args = self.to_tc_add_args();
        format!("tc {}", args.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netem_latency_builds_tc_args() {
        let config = NetemConfig::new().delay(200);
        let args = config.to_tc_add_args();

        assert_eq!(
            args,
            vec!["qdisc", "add", "dev", "eth0", "root", "netem", "delay", "200ms"]
        );
    }

    #[test]
    fn netem_latency_with_jitter_builds_tc_args() {
        let config = NetemConfig::new().delay(200).jitter(150);
        let args = config.to_tc_add_args();

        assert_eq!(
            args,
            vec!["qdisc", "add", "dev", "eth0", "root", "netem", "delay", "200ms", "150ms"]
        );
    }

    #[test]
    fn netem_loss_builds_tc_args() {
        let config = NetemConfig::new().loss(5.0);
        let args = config.to_tc_add_args();

        assert_eq!(
            args,
            vec!["qdisc", "add", "dev", "eth0", "root", "netem", "loss", "5.0%"]
        );
    }

    #[test]
    fn netem_burst_loss_builds_tc_args() {
        let config = NetemConfig::new().loss(10.0).loss_correlation(25.0);
        let args = config.to_tc_add_args();

        assert_eq!(
            args,
            vec!["qdisc", "add", "dev", "eth0", "root", "netem", "loss", "10.0%", "25.0%"]
        );
    }

    #[test]
    fn netem_rate_builds_tc_args() {
        let config = NetemConfig::new().rate(56);
        let args = config.to_tc_add_args();

        assert_eq!(
            args,
            vec!["qdisc", "add", "dev", "eth0", "root", "netem", "rate", "56kbit"]
        );
    }

    #[test]
    fn netem_combined_builds_tc_args() {
        let config = NetemConfig::new()
            .delay(100)
            .jitter(20)
            .loss(5.0)
            .rate(1024);
        let args = config.to_tc_add_args();

        assert_eq!(
            args,
            vec![
                "qdisc", "add", "dev", "eth0", "root", "netem", "delay", "100ms", "20ms", "loss",
                "5.0%", "rate", "1024kbit"
            ]
        );
    }

    #[test]
    fn netem_del_args() {
        let config = NetemConfig::new();
        let args = config.to_tc_del_args();

        assert_eq!(args, vec!["qdisc", "del", "dev", "eth0", "root"]);
    }

    #[test]
    fn netem_custom_interface() {
        let config = NetemConfig::new().interface("eth1").delay(50);
        let args = config.to_tc_add_args();

        assert!(args.contains(&"eth1".to_string()));
        assert!(!args.contains(&"eth0".to_string()));
    }

    #[test]
    fn netem_command_string() {
        let config = NetemConfig::new().delay(200).loss(5.0);
        let cmd = config.to_tc_command();

        assert_eq!(
            cmd,
            "tc qdisc add dev eth0 root netem delay 200ms loss 5.0%"
        );
    }

    #[test]
    fn netem_100_percent_loss_partition() {
        let config = NetemConfig::new().loss(100.0);
        let args = config.to_tc_add_args();

        assert_eq!(
            args,
            vec!["qdisc", "add", "dev", "eth0", "root", "netem", "loss", "100.0%"]
        );
    }
}
