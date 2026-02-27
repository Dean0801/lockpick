use std::collections::HashSet;
use std::path::Path;

use super::imports::extract_imports_from_source;

/// Known JS/TS config file patterns to scan in project root
const JS_CONFIG_FILES: &[&str] = &[
    "eslint.config.js",
    "eslint.config.mjs",
    "eslint.config.cjs",
    "eslint.config.ts",
    "tailwind.config.js",
    "tailwind.config.ts",
    "vite.config.js",
    "vite.config.ts",
    "vite.config.mjs",
    "next.config.js",
    "next.config.mjs",
    "next.config.ts",
    "webpack.config.js",
    "webpack.config.ts",
    "babel.config.js",
    "babel.config.cjs",
    "babel.config.mjs",
    "postcss.config.js",
    "postcss.config.cjs",
    "postcss.config.mjs",
    "jest.config.js",
    "jest.config.ts",
    "vitest.config.js",
    "vitest.config.ts",
    "vitest.config.mts",
    "rollup.config.js",
    "rollup.config.ts",
    "rollup.config.mjs",
    "tsup.config.ts",
    "tsup.config.js",
];

/// Scan JS/TS config files in project root, return imported package names
pub fn extract_js_config_deps(project_root: &Path) -> HashSet<String> {
    let mut deps = HashSet::new();

    for filename in JS_CONFIG_FILES {
        let path = project_root.join(filename);
        if path.exists()
            && let Ok(source) = std::fs::read_to_string(&path)
        {
            let imports = extract_imports_from_source(&source, &path);
            deps.extend(imports);
        }
    }

    deps
}

/// Scan extra config files specified by user in .lockpickrc
pub fn extract_extra_config_deps(project_root: &Path, extra_configs: &[String]) -> HashSet<String> {
    let mut deps = HashSet::new();

    for filename in extra_configs {
        let path = project_root.join(filename);
        if path.exists()
            && let Ok(source) = std::fs::read_to_string(&path)
        {
            let imports = extract_imports_from_source(&source, &path);
            deps.extend(imports);
        }
    }

    deps
}

/// Known JSON/YAML config files and their extraction rules
struct JsonConfigRule {
    filenames: &'static [&'static str],
    fields: &'static [&'static str],
    prefixes: &'static [&'static str],
}

const JSON_CONFIG_RULES: &[JsonConfigRule] = &[
    JsonConfigRule {
        filenames: &[".eslintrc.json", ".eslintrc.yml", ".eslintrc.yaml"],
        fields: &["extends", "plugins"],
        prefixes: &["eslint-plugin-", "eslint-config-"],
    },
    JsonConfigRule {
        filenames: &[".babelrc", ".babelrc.json"],
        fields: &["presets", "plugins"],
        prefixes: &[
            "babel-plugin-",
            "babel-preset-",
            "@babel/plugin-",
            "@babel/preset-",
        ],
    },
    JsonConfigRule {
        filenames: &[".postcssrc", ".postcssrc.json"],
        fields: &["plugins"],
        prefixes: &[],
    },
];

fn expand_plugin_name(short_name: &str, prefixes: &[&str]) -> Vec<String> {
    if short_name.starts_with('@') || prefixes.iter().any(|p| short_name.starts_with(p)) {
        return vec![short_name.to_string()];
    }
    if prefixes.is_empty() {
        return vec![short_name.to_string()];
    }
    let mut candidates = Vec::new();
    for prefix in prefixes {
        candidates.push(format!("{prefix}{short_name}"));
    }
    candidates.push(short_name.to_string());
    candidates
}

fn extract_json_string_values(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| match v {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Array(inner) => {
                    inner.first().and_then(|f| f.as_str().map(String::from))
                }
                _ => None,
            })
            .collect(),
        serde_json::Value::Object(obj) => obj.keys().cloned().collect(),
        _ => vec![],
    }
}

pub fn extract_json_config_deps(project_root: &Path) -> HashSet<String> {
    let mut deps = HashSet::new();

    for rule in JSON_CONFIG_RULES {
        for filename in rule.filenames {
            let path = project_root.join(filename);
            if !path.exists() {
                continue;
            }
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let json: serde_json::Value =
                if filename.ends_with(".yml") || filename.ends_with(".yaml") {
                    match serde_yaml::from_str(&content) {
                        Ok(v) => v,
                        Err(_) => continue,
                    }
                } else {
                    match serde_json::from_str(&content) {
                        Ok(v) => v,
                        Err(_) => continue,
                    }
                };

            for field in rule.fields {
                if let Some(value) = json.get(*field) {
                    let names = extract_json_string_values(value);
                    for name in names {
                        let expanded = expand_plugin_name(&name, rule.prefixes);
                        deps.extend(expanded);
                    }
                }
            }
        }
    }

    deps
}

pub fn extract_config_deps(project_root: &Path) -> HashSet<String> {
    let mut deps = extract_js_config_deps(project_root);
    deps.extend(extract_json_config_deps(project_root));
    deps
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_extract_js_config_deps_eslint() {
        let dir = std::env::temp_dir().join("lockpick_test_js_config_eslint");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join("eslint.config.js"),
            r#"
import eslintPluginReact from 'eslint-plugin-react';
import tseslint from 'typescript-eslint';
export default [
    eslintPluginReact.configs.recommended,
    ...tseslint.configs.recommended,
];
"#,
        )
        .unwrap();

        let deps = extract_js_config_deps(&dir);
        assert!(deps.contains("eslint-plugin-react"));
        assert!(deps.contains("typescript-eslint"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extract_js_config_deps_vite() {
        let dir = std::env::temp_dir().join("lockpick_test_js_config_vite");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join("vite.config.ts"),
            r#"
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
export default defineConfig({ plugins: [react()] });
"#,
        )
        .unwrap();

        let deps = extract_js_config_deps(&dir);
        assert!(deps.contains("vite"));
        assert!(deps.contains("@vitejs/plugin-react"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extract_js_config_deps_cjs() {
        let dir = std::env::temp_dir().join("lockpick_test_js_config_cjs");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join("postcss.config.cjs"),
            r#"
const tailwindcss = require('tailwindcss');
const autoprefixer = require('autoprefixer');
module.exports = { plugins: [tailwindcss, autoprefixer] };
"#,
        )
        .unwrap();

        let deps = extract_js_config_deps(&dir);
        assert!(deps.contains("tailwindcss"));
        assert!(deps.contains("autoprefixer"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extract_js_config_deps_no_configs() {
        let dir = std::env::temp_dir().join("lockpick_test_js_config_none");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let deps = extract_js_config_deps(&dir);
        assert!(deps.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extract_extra_config_deps() {
        let dir = std::env::temp_dir().join("lockpick_test_extra_config");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join("jest.config.ts"),
            r#"
import type { Config } from 'jest';
import { pathsToModuleNameMapper } from 'ts-jest';
const config: Config = { preset: 'ts-jest' };
export default config;
"#,
        )
        .unwrap();

        let extras = vec!["jest.config.ts".to_string()];
        let deps = extract_extra_config_deps(&dir, &extras);
        assert!(deps.contains("jest"));
        assert!(deps.contains("ts-jest"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extract_json_config_eslintrc() {
        let dir = std::env::temp_dir().join("lockpick_test_json_eslintrc");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join(".eslintrc.json"),
            r#"{
                "extends": ["airbnb", "plugin:react/recommended"],
                "plugins": ["react", "import"]
            }"#,
        )
        .unwrap();

        let deps = extract_json_config_deps(&dir);
        assert!(deps.contains("eslint-plugin-react"));
        assert!(deps.contains("eslint-plugin-import"));
        assert!(deps.contains("eslint-config-airbnb"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extract_json_config_babelrc() {
        let dir = std::env::temp_dir().join("lockpick_test_json_babelrc");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join(".babelrc"),
            r#"{
                "presets": ["@babel/preset-env", ["@babel/preset-react", {"runtime": "automatic"}]],
                "plugins": ["transform-runtime"]
            }"#,
        )
        .unwrap();

        let deps = extract_json_config_deps(&dir);
        assert!(deps.contains("@babel/preset-env"));
        assert!(deps.contains("@babel/preset-react"));
        assert!(deps.contains("babel-plugin-transform-runtime"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extract_json_config_postcssrc() {
        let dir = std::env::temp_dir().join("lockpick_test_json_postcssrc");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join(".postcssrc.json"),
            r#"{
                "plugins": {"autoprefixer": {}, "tailwindcss": {}}
            }"#,
        )
        .unwrap();

        let deps = extract_json_config_deps(&dir);
        assert!(deps.contains("autoprefixer"));
        assert!(deps.contains("tailwindcss"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extract_json_config_yaml_eslintrc() {
        let dir = std::env::temp_dir().join("lockpick_test_yaml_eslintrc");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join(".eslintrc.yaml"),
            "extends:\n  - airbnb\nplugins:\n  - react\n",
        )
        .unwrap();

        let deps = extract_json_config_deps(&dir);
        assert!(deps.contains("eslint-config-airbnb"));
        assert!(deps.contains("eslint-plugin-react"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_expand_plugin_name_scoped() {
        let result = expand_plugin_name("@typescript-eslint/eslint-plugin", &["eslint-plugin-"]);
        assert_eq!(result, vec!["@typescript-eslint/eslint-plugin".to_string()]);
    }

    #[test]
    fn test_expand_plugin_name_already_prefixed() {
        let result = expand_plugin_name("eslint-plugin-react", &["eslint-plugin-"]);
        assert_eq!(result, vec!["eslint-plugin-react".to_string()]);
    }

    #[test]
    fn test_extract_config_deps_combined() {
        let dir = std::env::temp_dir().join("lockpick_test_config_combined");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join("vite.config.ts"),
            r#"
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
export default defineConfig({ plugins: [react()] });
"#,
        )
        .unwrap();

        fs::write(dir.join(".eslintrc.json"), r#"{"plugins": ["react"]}"#).unwrap();

        let deps = extract_config_deps(&dir);
        assert!(deps.contains("@vitejs/plugin-react"));
        assert!(deps.contains("eslint-plugin-react"));

        let _ = fs::remove_dir_all(&dir);
    }
}
