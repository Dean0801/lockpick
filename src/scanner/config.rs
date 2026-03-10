use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::imports::extract_imports_from_source;
use crate::utils::strip_jsonc_comments;

/// Known JS/TS config file patterns to scan in project root
const JS_CONFIG_FILES: &[&str] = &[
    "eslint.config.js",
    "eslint.config.mjs",
    "eslint.config.cjs",
    "eslint.config.ts",
    "tailwind.config.js",
    "tailwind.config.ts",
    "tailwind.config.mjs",
    "tailwind.config.cjs",
    "vite.config.js",
    "vite.config.ts",
    "vite.config.mjs",
    "next.config.js",
    "next.config.mjs",
    "next.config.ts",
    "next.config.cjs",
    "webpack.config.js",
    "webpack.config.ts",
    "babel.config.js",
    "babel.config.cjs",
    "babel.config.mjs",
    "babel.config.ts",
    "postcss.config.js",
    "postcss.config.cjs",
    "postcss.config.mjs",
    "postcss.config.ts",
    "jest.config.js",
    "jest.config.ts",
    "jest.config.mjs",
    "jest.config.cjs",
    "vitest.config.js",
    "vitest.config.ts",
    "vitest.config.mts",
    "vitest.config.mjs",
    "vitest.config.cjs",
    "rollup.config.js",
    "rollup.config.ts",
    "rollup.config.mjs",
    "tsup.config.ts",
    "tsup.config.js",
    "stylelint.config.js",
    "stylelint.config.mjs",
    "stylelint.config.cjs",
];

/// Scan JS/TS config files in project root, return imported package names
pub fn extract_js_config_deps(project_root: &Path) -> HashSet<String> {
    let mut deps = HashSet::new();

    for filename in JS_CONFIG_FILES {
        let path = project_root.join(filename);
        if path.exists()
            && let Ok(source) = std::fs::read_to_string(&path)
        {
            // First extract regular imports
            let imports = extract_imports_from_source(&source, &path);
            deps.extend(imports);

            // For stylelint config files, also extract extends field
            if filename.starts_with("stylelint.config")
                && let Some(filename_str) = path.file_name().and_then(|n| n.to_str())
            {
                let extends = extract_extends_from_js_config(&source, filename_str);
                deps.extend(extends);
            }
        }
    }

    deps
}

/// Extract extends field from JS config files (stylelint, eslint, etc.)
/// Handles patterns like: extends: ['@vben/stylelint-config'] or extends: "config-name"
fn extract_extends_from_js_config(source: &str, _filename: &str) -> HashSet<String> {
    let mut deps = HashSet::new();

    // Simple regex-like pattern matching for extends field
    // Matches: extends: ['pkg1', 'pkg2'] or extends: ["pkg1", "pkg2"] or extends: "pkg" or extends: 'pkg'
    let extends_patterns = [
        // extends: ['pkg1', 'pkg2'] or extends: ["pkg1", "pkg2"]
        r"extends\s*:\s*\[([^\]]+)\]",
        // extends: "pkg"
        r"extends\s*:\s*\x22([^\x22]+)\x22",
        // extends: 'pkg'
        r"extends\s*:\s*'([^']+)'",
    ];

    for pattern in &extends_patterns {
        let regex = regex::Regex::new(pattern).unwrap();
        for cap in regex.captures_iter(source) {
            if let Some(matched) = cap.get(1) {
                let content = matched.as_str();
                // Extract package names from array or single string
                for line in content.split(',') {
                    let pkg = line.trim().trim_matches('"').trim_matches('\'');
                    if !pkg.is_empty() && !pkg.starts_with('.') {
                        // Extract package name (handle @scope/name/subpath -> @scope/name)
                        let pkg_name = extract_package_name_from_extends(pkg);
                        deps.insert(pkg_name);
                    }
                }
            }
        }
    }

    deps
}

/// Extract package name from extends value
/// Handles: @scope/name/subpath -> @scope/name, package-name/config -> package-name
fn extract_package_name_from_extends(extends: &str) -> String {
    if extends.starts_with('@') {
        // Scoped package: @scope/name/subpath -> @scope/name
        let parts: Vec<&str> = extends.split('/').collect();
        if parts.len() >= 2 {
            format!("{}/{}", parts[0], parts[1])
        } else {
            extends.to_string()
        }
    } else {
        // Regular package: package-name/config -> package-name
        extends.split('/').next().unwrap_or(extends).to_string()
    }
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
    // tsconfig.json extends field for shared configs like @vben/tsconfig
    JsonConfigRule {
        filenames: &["tsconfig.json"],
        fields: &["extends"],
        prefixes: &[],
    },
    // stylelint extends field for shared configs like @vben/stylelint-config
    JsonConfigRule {
        filenames: &[".stylelintrc", ".stylelintrc.json", ".stylelintrc.yml", ".stylelintrc.yaml"],
        fields: &["extends", "plugins"],
        prefixes: &["stylelint-"],
    },
];

fn expand_plugin_name(short_name: &str, prefixes: &[&str]) -> Vec<String> {
    // Strip "plugin:X/config" format -> extract "X"
    let name = if let Some(stripped) = short_name.strip_prefix("plugin:") {
        stripped.split('/').next().unwrap_or(stripped)
    } else {
        short_name
    };

    // For scoped packages like @vben/tsconfig/web-app.json, extract just @vben/tsconfig
    let name = if name.starts_with('@') {
        // Count slashes: @scope/name has 1, @scope/name/subpath has 2+
        let parts: Vec<&str> = name.split('/').collect();
        if parts.len() >= 2 {
            format!("{}/{}", parts[0], parts[1])
        } else {
            name.to_string()
        }
    } else {
        name.to_string()
    };

    if name.starts_with('@') || prefixes.iter().any(|p| name.starts_with(p)) {
        return vec![name.to_string()];
    }
    if prefixes.is_empty() {
        return vec![name.to_string()];
    }
    let mut candidates = Vec::new();
    for prefix in prefixes {
        candidates.push(format!("{prefix}{name}"));
    }
    candidates.push(name.to_string());
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

/// Directories to exclude when scanning for config files
const CONFIG_EXCLUDE_DIRS: &[&str] = &["node_modules", "dist", "build", ".git", ".next", "coverage", ".turbo"];

/// Recursively find all config files matching the given filenames
fn find_config_files(root: &Path, filenames: &[&str]) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();

    let walker = walkdir::WalkDir::new(root).into_iter().filter_entry(|entry| {
        if entry.file_type().is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            return !CONFIG_EXCLUDE_DIRS.contains(&name);
        }
        true
    });

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.file_type().is_file() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if filenames.contains(&name) {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        files.push((path.to_path_buf(), content));
                    }
                }
            }
        }
    }

    files
}

/// Config files that should be scanned recursively in monorepo sub-packages
const RECURSIVE_CONFIG_FILES: &[&str] = &["tsconfig.json", ".stylelintrc", ".stylelintrc.json", ".stylelintrc.yml", ".stylelintrc.yaml"];

pub fn extract_json_config_deps(project_root: &Path) -> HashSet<String> {
    let mut deps = HashSet::new();

    for rule in JSON_CONFIG_RULES {
        // Collect all filenames for this rule
        let filenames: Vec<&str> = rule.filenames.iter().copied().collect();

        // For tsconfig.json and stylelint configs, scan recursively; for others, only check root
        let should_scan_recursively = rule.filenames.iter().any(|f| RECURSIVE_CONFIG_FILES.contains(f));
        let files: Vec<(PathBuf, String)> = if should_scan_recursively {
            find_config_files(project_root, &filenames)
        } else {
            // For other config files, only check project root
            filenames
                .iter()
                .filter_map(|&f| {
                    let path = project_root.join(f);
                    if path.exists() {
                        std::fs::read_to_string(&path).ok().map(|c| (path, c))
                    } else {
                        None
                    }
                })
                .collect()
        };

        for (path, content) in files {
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let json: serde_json::Value =
                if filename.ends_with(".yml") || filename.ends_with(".yaml") {
                    match serde_yml::from_str(&content) {
                        Ok(v) => v,
                        Err(_) => continue,
                    }
                } else {
                    // Strip JSONC comments before parsing
                    let stripped = strip_jsonc_comments(&content);
                    match serde_json::from_str(&stripped) {
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
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join("eslint.config.js"),
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

        let deps = extract_js_config_deps(dir.path());
        assert!(deps.contains("eslint-plugin-react"));
        assert!(deps.contains("typescript-eslint"));
    }

    #[test]
    fn test_extract_js_config_deps_vite() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join("vite.config.ts"),
            r#"
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
export default defineConfig({ plugins: [react()] });
"#,
        )
        .unwrap();

        let deps = extract_js_config_deps(dir.path());
        assert!(deps.contains("vite"));
        assert!(deps.contains("@vitejs/plugin-react"));
    }

    #[test]
    fn test_extract_js_config_deps_cjs() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join("postcss.config.cjs"),
            r#"
const tailwindcss = require('tailwindcss');
const autoprefixer = require('autoprefixer');
module.exports = { plugins: [tailwindcss, autoprefixer] };
"#,
        )
        .unwrap();

        let deps = extract_js_config_deps(dir.path());
        assert!(deps.contains("tailwindcss"));
        assert!(deps.contains("autoprefixer"));
    }

    #[test]
    fn test_extract_js_config_deps_no_configs() {
        let dir = tempfile::tempdir().unwrap();

        let deps = extract_js_config_deps(dir.path());
        assert!(deps.is_empty());
    }

    #[test]
    fn test_extract_extra_config_deps() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join("jest.config.ts"),
            r#"
import type { Config } from 'jest';
import { pathsToModuleNameMapper } from 'ts-jest';
const config: Config = { preset: 'ts-jest' };
export default config;
"#,
        )
        .unwrap();

        let extras = vec!["jest.config.ts".to_string()];
        let deps = extract_extra_config_deps(dir.path(), &extras);
        assert!(deps.contains("jest"));
        assert!(deps.contains("ts-jest"));
    }

    #[test]
    fn test_extract_json_config_eslintrc() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join(".eslintrc.json"),
            r#"{
                "extends": ["airbnb", "plugin:react/recommended"],
                "plugins": ["react", "import"]
            }"#,
        )
        .unwrap();

        let deps = extract_json_config_deps(dir.path());
        assert!(deps.contains("eslint-plugin-react"));
        assert!(deps.contains("eslint-plugin-import"));
        assert!(deps.contains("eslint-config-airbnb"));
    }

    #[test]
    fn test_extract_json_config_babelrc() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join(".babelrc"),
            r#"{
                "presets": ["@babel/preset-env", ["@babel/preset-react", {"runtime": "automatic"}]],
                "plugins": ["transform-runtime"]
            }"#,
        )
        .unwrap();

        let deps = extract_json_config_deps(dir.path());
        assert!(deps.contains("@babel/preset-env"));
        assert!(deps.contains("@babel/preset-react"));
        assert!(deps.contains("babel-plugin-transform-runtime"));
    }

    #[test]
    fn test_extract_json_config_postcssrc() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join(".postcssrc.json"),
            r#"{
                "plugins": {"autoprefixer": {}, "tailwindcss": {}}
            }"#,
        )
        .unwrap();

        let deps = extract_json_config_deps(dir.path());
        assert!(deps.contains("autoprefixer"));
        assert!(deps.contains("tailwindcss"));
    }

    #[test]
    fn test_extract_json_config_yaml_eslintrc() {
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join(".eslintrc.yaml"),
            "extends:\n  - airbnb\nplugins:\n  - react\n",
        )
        .unwrap();

        let deps = extract_json_config_deps(dir.path());
        assert!(deps.contains("eslint-config-airbnb"));
        assert!(deps.contains("eslint-plugin-react"));
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
        let dir = tempfile::tempdir().unwrap();

        fs::write(
            dir.path().join("vite.config.ts"),
            r#"
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
export default defineConfig({ plugins: [react()] });
"#,
        )
        .unwrap();

        fs::write(
            dir.path().join(".eslintrc.json"),
            r#"{"plugins": ["react"]}"#,
        )
        .unwrap();

        let deps = extract_config_deps(dir.path());
        assert!(deps.contains("@vitejs/plugin-react"));
        assert!(deps.contains("eslint-plugin-react"));
    }

    #[test]
    fn test_strip_jsonc_comments() {
        let input = r#"{
            // This is a line comment
            "plugins": ["react"], /* block comment */
            "extends": ["airbnb"]
        }"#;
        let stripped = strip_jsonc_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(parsed["plugins"][0].as_str(), Some("react"));
        assert_eq!(parsed["extends"][0].as_str(), Some("airbnb"));
    }

    #[test]
    fn test_extract_tsconfig_extends_recursive() {
        let dir = tempfile::tempdir().unwrap();

        // Root tsconfig.json
        fs::write(
            dir.path().join("tsconfig.json"),
            r#"{
                "extends": "@org/root-tsconfig"
            }"#,
        )
        .unwrap();

        // Sub-package tsconfig.json
        let sub_pkg = dir.path().join("packages/web");
        fs::create_dir_all(&sub_pkg).unwrap();
        fs::write(
            sub_pkg.join("tsconfig.json"),
            r#"{
                "extends": "@vben/tsconfig/web-app.json"
            }"#,
        )
        .unwrap();

        let deps = extract_json_config_deps(dir.path());
        assert!(deps.contains("@org/root-tsconfig"));
        assert!(deps.contains("@vben/tsconfig"));
    }

    #[test]
    fn test_extract_stylelint_extends_recursive() {
        let dir = tempfile::tempdir().unwrap();

        // Root .stylelintrc.json
        fs::write(
            dir.path().join(".stylelintrc.json"),
            r#"{
                "extends": "stylelint-config-standard"
            }"#,
        )
        .unwrap();

        // Sub-package .stylelintrc.json
        let sub_pkg = dir.path().join("packages/web");
        fs::create_dir_all(&sub_pkg).unwrap();
        fs::write(
            sub_pkg.join(".stylelintrc.json"),
            r#"{
                "extends": "@vben/stylelint-config"
            }"#,
        )
        .unwrap();

        let deps = extract_json_config_deps(dir.path());
        assert!(deps.contains("stylelint-config-standard"));
        assert!(deps.contains("@vben/stylelint-config"));
    }

    #[test]
    fn test_extract_stylelint_js_config_extends() {
        let dir = tempfile::tempdir().unwrap();

        // stylelint.config.mjs with extends
        fs::write(
            dir.path().join("stylelint.config.mjs"),
            r#"export default {
  extends: ['@vben/stylelint-config'],
  root: true,
};"#,
        )
        .unwrap();

        let deps = extract_js_config_deps(dir.path());
        assert!(deps.contains("@vben/stylelint-config"));
    }

    #[test]
    fn test_extract_stylelint_js_config_extends_double_quotes() {
        let dir = tempfile::tempdir().unwrap();

        // stylelint.config.mjs with double quotes
        fs::write(
            dir.path().join("stylelint.config.mjs"),
            r#"export default {
  extends: ["@vben/stylelint-config"],
  root: true,
};"#,
        )
        .unwrap();

        let deps = extract_js_config_deps(dir.path());
        assert!(deps.contains("@vben/stylelint-config"));
    }

    #[test]
    fn test_extract_stylelint_js_config_extends_single_string() {
        let dir = tempfile::tempdir().unwrap();

        // stylelint.config.mjs with single string (not array)
        fs::write(
            dir.path().join("stylelint.config.mjs"),
            r#"export default {
  extends: "@vben/stylelint-config",
  root: true,
};"#,
        )
        .unwrap();

        let deps = extract_js_config_deps(dir.path());
        assert!(deps.contains("@vben/stylelint-config"));
    }
}
