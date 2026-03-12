use super::default_true;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Breakpoint rule: pause matching requests/responses for manual editing.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BreakpointRule {
    pub id: String,
    pub pattern: String,
    #[serde(default)]
    pub break_on_request: bool,
    #[serde(default)]
    pub break_on_response: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Action to take on a paused breakpoint.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum BreakpointAction {
    /// Forward the request/response as-is.
    #[serde(rename = "forward")]
    Forward,
    /// Modify and then forward.
    #[serde(rename = "modify_and_forward")]
    ModifyAndForward {
        #[serde(default)]
        headers: Option<HashMap<String, String>>,
        #[serde(default)]
        body: Option<String>,
        #[serde(default)]
        status: Option<u16>,
    },
    /// Drop the request (close connection).
    #[serde(rename = "drop")]
    Drop,
    /// Abort with an error response.
    #[serde(rename = "abort")]
    Abort,
}

/// Phase at which a breakpoint was hit.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum BreakpointPhase {
    #[serde(rename = "request")]
    Request,
    #[serde(rename = "response")]
    Response,
}

/// Data snapshot for a paused breakpoint.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BreakpointData {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub status: Option<u16>,
}

impl std::fmt::Display for BreakpointRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.enabled { "enabled" } else { "disabled" };
        let phases = match (self.break_on_request, self.break_on_response) {
            (true, true) => "req+res",
            (true, false) => "req",
            (false, true) => "res",
            (false, false) => "none",
        };
        write!(
            f,
            "[{}] {} (break on: {}) [{}]",
            self.id, self.pattern, phases, status
        )
    }
}
