/// TUI 탭 (페이지) 목록
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Network,
    WebSocket,
    InterceptRules,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 4] = [
        Tab::Network,
        Tab::WebSocket,
        Tab::InterceptRules,
        Tab::Settings,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Network => "Network",
            Tab::WebSocket => "WebSocket",
            Tab::InterceptRules => "Rules",
            Tab::Settings => "Settings",
        }
    }

    pub fn next(&self) -> Tab {
        let idx = Tab::ALL.iter().position(|t| t == self).unwrap_or(0);
        Tab::ALL[(idx + 1) % Tab::ALL.len()]
    }

    pub fn prev(&self) -> Tab {
        let idx = Tab::ALL.iter().position(|t| t == self).unwrap_or(0);
        if idx == 0 {
            Tab::ALL[Tab::ALL.len() - 1]
        } else {
            Tab::ALL[idx - 1]
        }
    }
}
