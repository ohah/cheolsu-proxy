use serde::{Deserialize, Serialize};

/// Host mapping entry for DNS spoofing / remote host mapping.
/// Maps requests from source host to a different target host/IP,
/// allowing testing against staging/dev servers without modifying hosts file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HostMapping {
    pub id: String,
    /// Source host pattern, supports wildcards (e.g., "*.api.example.com")
    pub source_host: String,
    /// Source port filter (None = any port)
    pub source_port: Option<u16>,
    /// Target host (IP address or domain name)
    pub target_host: String,
    /// Target port (None = keep original port)
    pub target_port: Option<u16>,
    pub enabled: bool,
}

impl std::fmt::Display for HostMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.enabled { "enabled" } else { "disabled" };
        let src_port = self
            .source_port
            .map(|p| format!(":{}", p))
            .unwrap_or_default();
        let tgt_port = self
            .target_port
            .map(|p| format!(":{}", p))
            .unwrap_or_default();
        write!(
            f,
            "[{}] {}{} -> {}{} [{}]",
            self.id, self.source_host, src_port, self.target_host, tgt_port, status
        )
    }
}
