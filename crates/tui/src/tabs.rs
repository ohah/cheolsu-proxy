/// TUI tab (page) list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Network,
    WebSocket,
    InterceptRules,
    Script,
    Settings,
    Breakpoint,
    Analytics,
    Logs,
}

impl Tab {
    pub const ALL: [Tab; 8] = [
        Tab::Network,
        Tab::WebSocket,
        Tab::InterceptRules,
        Tab::Script,
        Tab::Breakpoint,
        Tab::Analytics,
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
            Tab::Analytics => "Analytics",
            Tab::Settings => "Settings",
            Tab::Logs => "Logs",
        }
    }
}

cycle_enum!(Tab);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_contains_all_variants() {
        assert_eq!(Tab::ALL.len(), 8);
    }

    #[test]
    fn next_wraps_around() {
        assert_eq!(Tab::Network.next(), Tab::WebSocket);
        assert_eq!(Tab::Logs.next(), Tab::Network);
    }

    #[test]
    fn prev_wraps_around() {
        assert_eq!(Tab::Network.prev(), Tab::Logs);
        assert_eq!(Tab::WebSocket.prev(), Tab::Network);
    }

    #[test]
    fn full_next_cycle_returns_to_start() {
        let mut tab = Tab::Network;
        for _ in 0..Tab::ALL.len() {
            tab = tab.next();
        }
        assert_eq!(tab, Tab::Network);
    }

    #[test]
    fn full_prev_cycle_returns_to_start() {
        let mut tab = Tab::Network;
        for _ in 0..Tab::ALL.len() {
            tab = tab.prev();
        }
        assert_eq!(tab, Tab::Network);
    }

    #[test]
    fn all_titles_not_empty() {
        for tab in Tab::ALL {
            assert!(!tab.title().is_empty(), "Tab {:?} has empty title", tab);
        }
    }

    #[test]
    fn title_mapping() {
        assert_eq!(Tab::Network.title(), "Network");
        assert_eq!(Tab::WebSocket.title(), "WebSocket");
        assert_eq!(Tab::InterceptRules.title(), "Rules");
        assert_eq!(Tab::Script.title(), "Script");
        assert_eq!(Tab::Breakpoint.title(), "Breakpoint");
        assert_eq!(Tab::Analytics.title(), "Analytics");
        assert_eq!(Tab::Settings.title(), "Settings");
        assert_eq!(Tab::Logs.title(), "Logs");
    }
}
