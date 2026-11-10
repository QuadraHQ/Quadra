const HOSTS: [&str; 9] = [
    "client-telemetry.roblox.com",
    "ephemeralcounters.api.roblox.com",
    "metrics.roblox.com",
    "tracing.roblox.com",
    "ncs.roblox.com",
    "gold.roblox.com",
    "abtesting.roblox.com",
    "upload.crashes.roblox.com",
    "upload.crashes.rbxinfra.com",
];

pub const BLOCK_HOST_VARIABLE: &str = "ROSEMARY_BLOCK_HOST";

pub fn blocked_hosts() -> String {
    HOSTS.join(",")
}

#[cfg(test)]
mod tests {
    use super::blocked_hosts;

    #[test]
    fn lists_every_host_for_rosemary_without_spaces() {
        let hosts = blocked_hosts();
        assert_eq!(hosts.split(',').count(), 9);
        assert!(!hosts.contains(' '));
        assert!(hosts.starts_with("client-telemetry.roblox.com,"));
    }
}
