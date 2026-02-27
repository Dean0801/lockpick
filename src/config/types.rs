use serde::Deserialize;

/// Output format for reports
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormatConfig {
    Terminal,
    Json,
}

/// License policy configuration
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
pub struct LicensePolicy {
    /// Allowlist: only these licenses are permitted (when non-empty)
    #[serde(default)]
    pub allow: Vec<String>,
    /// Denylist: these licenses are forbidden
    #[serde(default)]
    pub deny: Vec<String>,
}

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
    pub format: Option<OutputFormatConfig>,

    /// Extra config file paths to scan (e.g. ["jest.config.ts"])
    #[serde(default)]
    pub extra_configs: Vec<String>,

    /// License policy (allow/deny lists)
    #[serde(default)]
    pub license: Option<LicensePolicy>,

    /// OSV cache TTL in seconds (default: 86400 = 24h)
    #[serde(default)]
    pub cache_ttl: Option<u64>,
}
