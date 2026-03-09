use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

/// Map CLI commands found in package.json scripts to their package names.
static CLI_TO_PACKAGE: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
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
        ("turbo", "turbo"),
        ("turbo-run", "@vben/turbo-run"),
    ])
});

/// Commands that are runners/shells and should be skipped when extracting the real tool.
const SKIP_COMMANDS: &[&str] = &[
    "node", "npx", "npm", "yarn", "pnpm", "run", "exec", "env", "sh", "bash", "echo", "cd", "rm",
    "cp", "mkdir", "cat", "exit", "true", "false", "test",
];

/// Extract the first meaningful command token from a single command segment.
///
/// Skips env var assignments (`KEY=val`), flags (`--foo`), and runner commands.
fn extract_command_from_segment(segment: &str) -> Option<String> {
    for token in segment.split_whitespace() {
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

/// Split a script value by chain operators (`&&`, `||`, `;`, `|`)
/// and extract meaningful commands from each segment.
fn extract_commands(script_value: &str) -> Vec<String> {
    // Replace operators with a common delimiter for splitting
    let normalized = script_value
        .replace("&&", "\n")
        .replace("||", "\n")
        .replace([';', '|'], "\n");

    normalized
        .lines()
        .filter_map(|seg| extract_command_from_segment(seg.trim()))
        .collect()
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

    if let Some(scripts) = json.get("scripts").and_then(|s| s.as_object()) {
        for script_value in scripts.values() {
            if let Some(val) = script_value.as_str() {
                for cmd in extract_commands(val) {
                    if let Some(&pkg) = CLI_TO_PACKAGE.get(cmd.as_str()) {
                        deps.insert(pkg.to_string());
                    }
                }
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
        assert_eq!(
            extract_command_from_segment("eslint ."),
            Some("eslint".to_string())
        );
    }

    #[test]
    fn test_extract_command_skip_npx() {
        assert_eq!(
            extract_command_from_segment("npx eslint ."),
            Some("eslint".to_string())
        );
    }

    #[test]
    fn test_extract_command_skip_node() {
        assert_eq!(
            extract_command_from_segment("node dist/index.js"),
            Some("dist/index.js".to_string())
        );
    }

    #[test]
    fn test_extract_command_with_env_var() {
        assert_eq!(
            extract_command_from_segment("cross-env NODE_ENV=production node server.js"),
            Some("cross-env".to_string())
        );
    }

    #[test]
    fn test_extract_command_npm_run() {
        assert_eq!(
            extract_command_from_segment("npm run build"),
            Some("build".to_string())
        );
    }

    #[test]
    fn test_extract_command_empty() {
        assert_eq!(extract_command_from_segment(""), None);
    }

    #[test]
    fn test_extract_commands_chained() {
        let cmds = extract_commands("eslint . && prettier --write .");
        assert_eq!(cmds, vec!["eslint", "prettier"]);
    }

    #[test]
    fn test_extract_commands_semicolon() {
        let cmds = extract_commands("tsc; eslint .");
        assert_eq!(cmds, vec!["tsc", "eslint"]);
    }

    #[test]
    fn test_extract_commands_or_chain() {
        let cmds = extract_commands("eslint . || true");
        assert_eq!(cmds, vec!["eslint"]);
    }

    #[test]
    fn test_extract_commands_pipe() {
        let cmds = extract_commands("jest --json | tsc");
        assert_eq!(cmds, vec!["jest", "tsc"]);
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
