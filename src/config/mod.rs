pub mod types;

use std::path::Path;
use types::LockpickConfig;

/// Load .lockpickrc.json or .lockpickrc.yaml from project root.
/// Returns Default if no config file found.
pub fn load_config(project_root: &Path) -> LockpickConfig {
    let json_path = project_root.join(".lockpickrc.json");
    if json_path.exists()
        && let Ok(content) = std::fs::read_to_string(&json_path)
        && let Ok(config) = serde_json::from_str::<LockpickConfig>(&content)
    {
        return config;
    }

    let yaml_path = project_root.join(".lockpickrc.yaml");
    if yaml_path.exists()
        && let Ok(content) = std::fs::read_to_string(&yaml_path)
        && let Ok(config) = serde_yaml::from_str::<LockpickConfig>(&content)
    {
        return config;
    }

    LockpickConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_json_config() {
        let dir = std::env::temp_dir().join("lockpick_test_rc_json");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join(".lockpickrc.json"),
            r#"{
                "ignore": ["husky", "lint-staged"],
                "skip_dev": true,
                "lang": "zh",
                "extra_configs": ["jest.config.ts"]
            }"#,
        )
        .unwrap();

        let config = load_config(&dir);
        assert_eq!(config.ignore, vec!["husky", "lint-staged"]);
        assert!(config.skip_dev);
        assert_eq!(config.lang, Some("zh".to_string()));
        assert_eq!(config.extra_configs, vec!["jest.config.ts"]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_yaml_config() {
        let dir = std::env::temp_dir().join("lockpick_test_rc_yaml");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join(".lockpickrc.yaml"),
            "ignore:\n  - husky\n  - lint-staged\nskip_dev: false\nlang: en\n",
        )
        .unwrap();

        let config = load_config(&dir);
        assert_eq!(config.ignore, vec!["husky", "lint-staged"]);
        assert!(!config.skip_dev);
        assert_eq!(config.lang, Some("en".to_string()));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_json_takes_priority_over_yaml() {
        let dir = std::env::temp_dir().join("lockpick_test_rc_priority");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join(".lockpickrc.json"), r#"{"ignore": ["from-json"]}"#).unwrap();
        fs::write(dir.join(".lockpickrc.yaml"), "ignore:\n  - from-yaml\n").unwrap();

        let config = load_config(&dir);
        assert_eq!(config.ignore, vec!["from-json"]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_no_config_returns_default() {
        let dir = std::env::temp_dir().join("lockpick_test_rc_none");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let config = load_config(&dir);
        assert_eq!(config, LockpickConfig::default());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_invalid_json_returns_default() {
        let dir = std::env::temp_dir().join("lockpick_test_rc_invalid");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join(".lockpickrc.json"), "not valid json{{{").unwrap();

        let config = load_config(&dir);
        assert_eq!(config, LockpickConfig::default());

        let _ = fs::remove_dir_all(&dir);
    }
}
