use crate::AnalysisResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Scanning,
    Results,
    Settings,
    Tree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    FullScan,
    Unused,
    Audit,
    Fix,
    Tree,
    Outdated,
    SupplyChain,
    Settings,
    Exit,
}

impl MenuItem {
    pub fn all() -> Vec<Self> {
        vec![
            Self::FullScan,
            Self::Unused,
            Self::Audit,
            Self::Fix,
            Self::Tree,
            Self::Outdated,
            Self::SupplyChain,
            Self::Settings,
            Self::Exit,
        ]
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::FullScan => "🔍",
            Self::Unused => "📦",
            Self::Audit => "🛡️",
            Self::Fix => "🔧",
            Self::Tree => "🌲",
            Self::Outdated => "📊",
            Self::SupplyChain => "🔗",
            Self::Settings => "⚙️",
            Self::Exit => "❌",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::FullScan => "Full Scan",
            Self::Unused => "Unused Deps",
            Self::Audit => "Security Audit",
            Self::Fix => "Auto Fix",
            Self::Tree => "Dependency Tree",
            Self::Outdated => "Outdated Check",
            Self::SupplyChain => "Supply Chain",
            Self::Settings => "Settings",
            Self::Exit => "Exit",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::FullScan => "Analyze all dependencies",
            Self::Unused => "Find unused packages",
            Self::Audit => "Check vulnerabilities",
            Self::Fix => "Remove unused deps",
            Self::Tree => "Visualize relationships",
            Self::Outdated => "Find outdated packages",
            Self::SupplyChain => "Security analysis",
            Self::Settings => "Configure options",
            Self::Exit => "",
        }
    }
}

pub struct App {
    pub screen: Screen,
    pub selected_menu: usize,
    pub should_quit: bool,
    pub result: Option<AnalysisResult>,
    pub scan_status: String,
    pub scan_progress: u16,
    pub tree_state: Option<super::tree_state::TreeState>,
    pub tree_data: Option<crate::tree::DepTree>,
    pub results_selected: usize,
    pub results_expanded: std::collections::HashSet<usize>,
    pub results_scroll: usize,
    pub results_auto_scroll: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Menu,
            selected_menu: 0,
            should_quit: false,
            result: None,
            scan_status: String::new(),
            scan_progress: 0,
            tree_state: None,
            tree_data: None,
            results_selected: 0,
            results_expanded: std::collections::HashSet::new(),
            results_scroll: 0,
            results_auto_scroll: true,
        }
    }

    pub fn next_menu(&mut self) {
        let items = MenuItem::all();
        self.selected_menu = (self.selected_menu + 1) % items.len();
    }

    pub fn prev_menu(&mut self) {
        let items = MenuItem::all();
        self.selected_menu = if self.selected_menu == 0 {
            items.len() - 1
        } else {
            self.selected_menu - 1
        };
    }

    pub fn selected_item(&self) -> MenuItem {
        MenuItem::all()[self.selected_menu]
    }
}
