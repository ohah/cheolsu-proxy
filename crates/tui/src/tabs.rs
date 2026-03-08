/// TUI tab (page) list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Network,
    WebSocket,
    InterceptRules,
    Script,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Network,
        Tab::WebSocket,
        Tab::InterceptRules,
        Tab::Script,
        Tab::Settings,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Network => "Network",
            Tab::WebSocket => "WebSocket",
            Tab::InterceptRules => "Rules",
            Tab::Script => "Script",
            Tab::Settings => "Settings",
        }
    }
}

cycle_enum!(Tab);
