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
pub fn extract_extra_config_deps(
    project_root: &Path,
    extra_configs: &[String],
) -> HashSet<String> {
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
}
