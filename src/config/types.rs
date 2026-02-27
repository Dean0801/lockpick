use serde::Deserialize;

/// Project-level lockpick configuration (.lockpickrc.json / .lockpickrc.yaml)
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct LockpickConfig {
    /// Packages to ignore in unused detection
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Skip devDependencies
    #[serde(default)]
    pub skip_dev: bool,

    /// Language override (zh / en)
    #[serde(default)]
    pub lang: Option<String>,

    /// Output format override (terminal / json)
    #[serde(default)]
    pub format: Option<String>,

    /// Extra config file paths to scan (e.g. ["jest.config.ts"])
    #[serde(default)]
    pub extra_configs: Vec<String>,
}
