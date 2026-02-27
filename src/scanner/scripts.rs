use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Map CLI commands found in package.json scripts to their package names.
fn cli_to_package_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("tsc", "typescript"),
        ("tsup", "tsup"),
        ("tsx", "tsx"),
        ("vitest", "vitest"),
        ("jest", "jest"),
        ("mocha", "mocha"),
        ("eslint", "eslint"),
        ("prettier", "prettier"),
        ("webpack", "webpack"),
        ("rollup", "rollup"),
        ("esbuild", "esbuild"),
        ("vite", "vite"),
        ("next", "next"),
        ("nuxt", "nuxt"),
        ("tailwindcss", "tailwindcss"),
        ("postcss", "postcss"),
        ("nodemon", "nodemon"),
        ("ts-node", "ts-node"),
        ("cross-env", "cross-env"),
        ("rimraf", "rimraf"),
        ("concurrently", "concurrently"),
        ("lint-staged", "lint-staged"),
        ("husky", "husky"),
        ("commitlint", "commitlint"),
        ("typedoc", "typedoc"),
        ("tsc-alias", "tsc-alias"),
    ])
}

/// Commands that are runners/shells and should be skipped when extracting the real tool.
const SKIP_COMMANDS: &[&str] = &[
    "node", "npx", "npm", "yarn", "pnpm", "run", "exec", "env", "sh", "bash", "echo", "cd",
    "rm", "cp", "mkdir", "cat", "exit", "true", "false", "test",
];

/// Extract the first meaningful command token from a script value.
///
/// Skips env var assignments (`KEY=val`), flags (`--foo`), and runner commands.
fn extract_command(script_value: &str) -> Option<String> {
    for token in script_value.split_whitespace() {
        // Skip env var assignments like NODE_ENV=production
        if token.contains('=') && !token.starts_with('-') {
            continue;
        }
        // Skip flags
        if token.starts_with('-') {
            continue;
        }
        // Skip runner / shell commands
        if SKIP_COMMANDS.contains(&token) {
            continue;
        }
        return Some(token.to_string());
    }
    None
}

/// Read package.json scripts and return the set of package names referenced by those scripts.
pub fn extract_script_deps(project_root: &Path) -> HashSet<String> {
    let mut deps = HashSet::new();
    let pkg_path = project_root.join("package.json");

    let content = match std::fs::read_to_string(&pkg_path) {
        Ok(c) => c,
        Err(_) => return deps,
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return deps,
    };

    let map = cli_to_package_map();

    if let Some(scripts) = json.get("scripts").and_then(|s| s.as_object()) {
        for script_value in scripts.values() {
            if let Some(val) = script_value.as_str()
                && let Some(cmd) = extract_command(val)
                && let Some(&pkg) = map.get(cmd.as_str())
            {
                deps.insert(pkg.to_string());
            }
        }
    }

    deps
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_extract_command_simple() {
        assert_eq!(extract_command("eslint ."), Some("eslint".to_string()));
    }

    #[test]
    fn test_extract_command_skip_npx() {
        assert_eq!(extract_command("npx eslint ."), Some("eslint".to_string()));
    }

    #[test]
    fn test_extract_command_skip_node() {
        assert_eq!(
            extract_command("node dist/index.js"),
            Some("dist/index.js".to_string())
        );
    }

    #[test]
    fn test_extract_command_with_env_var() {
        assert_eq!(
            extract_command("cross-env NODE_ENV=production node server.js"),
            Some("cross-env".to_string())
        );
    }

    #[test]
    fn test_extract_command_npm_run() {
        assert_eq!(
            extract_command("npm run build"),
            Some("build".to_string())
        );
    }

    #[test]
    fn test_extract_command_empty() {
        assert_eq!(extract_command(""), None);
    }

    #[test]
    fn test_extract_script_deps_basic() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = serde_json::json!({
            "scripts": {
                "build": "tsc",
                "lint": "eslint src/",
                "test": "vitest run",
                "dev": "nodemon src/index.ts",
                "format": "prettier --write ."
            }
        });
        fs::write(dir.path().join("package.json"), pkg.to_string()).unwrap();

        let deps = extract_script_deps(dir.path());
        assert!(deps.contains("typescript"));
        assert!(deps.contains("eslint"));
        assert!(deps.contains("vitest"));
        assert!(deps.contains("nodemon"));
        assert!(deps.contains("prettier"));
    }

    #[test]
    fn test_extract_script_deps_skip_runners() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = serde_json::json!({
            "scripts": {
                "start": "node dist/index.js",
                "prepare": "npx husky install"
            }
        });
        fs::write(dir.path().join("package.json"), pkg.to_string()).unwrap();

        let deps = extract_script_deps(dir.path());
        assert!(!deps.contains("node"));
        assert!(!deps.contains("npx"));
        assert!(deps.contains("husky"));
    }

    #[test]
    fn test_extract_script_deps_no_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let deps = extract_script_deps(dir.path());
        assert!(deps.is_empty());
    }
}
