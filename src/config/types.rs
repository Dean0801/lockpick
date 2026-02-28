use serde::Deserialize;

/// Output format for reports
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormatConfig {
    Terminal,
    Json,
    Markdown,
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

    /// CI threshold configuration
    #[serde(default)]
    pub thresholds: Option<Thresholds>,
}

fn default_neg_one() -> i32 {
    -1
}

/// CI threshold configuration for exit code strategy
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct Thresholds {
    /// Max critical vulns allowed (-1 = unlimited)
    #[serde(default = "default_neg_one")]
    pub max_critical: i32,
    /// Max high vulns allowed (-1 = unlimited)
    #[serde(default = "default_neg_one")]
    pub max_high: i32,
    /// Max unused deps allowed (-1 = unlimited)
    #[serde(default = "default_neg_one")]
    pub max_unused: i32,
    /// Max duplicate deps allowed (-1 = unlimited)
    #[serde(default = "default_neg_one")]
    pub max_duplicates: i32,
    /// Fail on license violations
    #[serde(default)]
    pub fail_on_license: bool,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            max_critical: -1,
            max_high: -1,
            max_unused: -1,
            max_duplicates: -1,
            fail_on_license: false,
        }
    }
}
