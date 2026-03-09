use std::collections::HashMap;
use tui_tree_widget::TreeState as TuiTreeState;

pub struct TreeState {
    pub tui_state: TuiTreeState<String>,
    pub search_query: String,
    pub duplicate_packages: HashMap<String, usize>,
}

impl Default for TreeState {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeState {
    pub fn new() -> Self {
        let mut tui_state = TuiTreeState::default();
        tui_state.select_first();
        tui_state.open(vec![]);

        Self {
            tui_state,
            search_query: String::new(),
            duplicate_packages: HashMap::new(),
        }
    }
}
