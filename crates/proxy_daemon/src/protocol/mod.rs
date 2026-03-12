mod auth;
mod breakpoint_types;
mod command;
mod host_mapping;
mod intercept;
mod ssl;

pub(crate) fn default_true() -> bool {
    true
}

pub use auth::ProxyAuthConfig;
pub use breakpoint_types::{BreakpointAction, BreakpointData, BreakpointPhase, BreakpointRule};
pub use command::{ClientCommand, DaemonMessage, ProxyLockInfo};
pub use host_mapping::HostMapping;
pub use intercept::{InterceptAction, InterceptRule, RewriteTarget, ServerReplayEntry};
pub use ssl::{
    CertificateInfo, ClientCertConfig, DomainClientCertConfig, RequestClientCertConfig,
    SslProxyingEntry, SslProxyingMode, TlsPassthroughEntry,
};

#[cfg(test)]
mod tests_auth;
#[cfg(test)]
mod tests_breakpoint;
#[cfg(test)]
mod tests_command;
#[cfg(test)]
mod tests_daemon_message;
#[cfg(test)]
mod tests_host_mapping;
#[cfg(test)]
mod tests_ssl;
