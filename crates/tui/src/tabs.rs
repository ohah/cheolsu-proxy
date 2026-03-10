/// TUI tab (page) list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Network,
    WebSocket,
    InterceptRules,
    Script,
    Settings,
    Breakpoint,
    Logs,
}

impl Tab {
    pub const ALL: [Tab; 7] = [
        Tab::Network,
        Tab::WebSocket,
        Tab::InterceptRules,
        Tab::Script,
        Tab::Breakpoint,
        Tab::Settings,
        Tab::Logs,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Network => "Network",
            Tab::WebSocket => "WebSocket",
            Tab::InterceptRules => "Rules",
            Tab::Script => "Script",
            Tab::Breakpoint => "Breakpoint",
            Tab::Settings => "Settings",
            Tab::Logs => "Logs",
        }
    }
}

cycle_enum!(Tab);
