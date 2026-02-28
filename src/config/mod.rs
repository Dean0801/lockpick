pub mod types;

use std::path::Path;

use crate::error::LockpickError;
use types::LockpickConfig;

/// Load .lockpickrc.json or .lockpickrc.yaml from project root.
/// Returns Default if no config file found.
/// Returns Error if file exists but cannot be parsed.
pub fn load_config(project_root: &Path) -> Result<LockpickConfig, LockpickError> {
    let json_path = project_root.join(".lockpickrc.json");
    if json_path.exists() {
        let content = std::fs::read_to_string(&json_path).map_err(LockpickError::Io)?;
        let config = serde_json::from_str::<LockpickConfig>(&content)
            .map_err(|e| LockpickError::Config(format!("Invalid .lockpickrc.json: {e}")))?;
        return Ok(config);
    }

    let yaml_path = project_root.join(".lockpickrc.yaml");
    if yaml_path.exists() {
        let content = std::fs::read_to_string(&yaml_path).map_err(LockpickError::Io)?;
        let config = serde_yml::from_str::<LockpickConfig>(&content)
            .map_err(|e| LockpickError::Config(format!("Invalid .lockpickrc.yaml: {e}")))?;
        return Ok(config);
    }

    Ok(LockpickConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_json_config() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join(".lockpickrc.json"),
            r#"{
                "ignore": ["husky", "lint-staged"],
                "skip_dev": true,
                "lang": "zh",
                "extra_configs": ["jest.config.ts"]
            }"#,
        )
        .unwrap();

        let config = load_config(dir.path()).unwrap();
        assert_eq!(config.ignore, vec!["husky", "lint-staged"]);
        assert!(config.skip_dev);
        assert_eq!(config.lang, Some("zh".to_string()));
        assert_eq!(config.extra_configs, vec!["jest.config.ts"]);
    }

    #[test]
    fn test_load_yaml_config() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join(".lockpickrc.yaml"),
            "ignore:\n  - husky\n  - lint-staged\nskip_dev: false\nlang: en\n",
        )
        .unwrap();

        let config = load_config(dir.path()).unwrap();
        assert_eq!(config.ignore, vec!["husky", "lint-staged"]);
        assert!(!config.skip_dev);
        assert_eq!(config.lang, Some("en".to_string()));
    }

    #[test]
    fn test_json_takes_priority_over_yaml() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join(".lockpickrc.json"),
            r#"{"ignore": ["from-json"]}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join(".lockpickrc.yaml"),
            "ignore:\n  - from-yaml\n",
        )
        .unwrap();

        let config = load_config(dir.path()).unwrap();
        assert_eq!(config.ignore, vec!["from-json"]);
    }

    #[test]
    fn test_no_config_returns_default() {
        let dir = tempfile::tempdir().unwrap();

        let config = load_config(dir.path()).unwrap();
        assert_eq!(config, LockpickConfig::default());
    }

    #[test]
    fn test_invalid_json_returns_error() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(dir.path().join(".lockpickrc.json"), "not valid json{{{").unwrap();

        let result = load_config(dir.path());
        assert!(result.is_err());
    }
}
