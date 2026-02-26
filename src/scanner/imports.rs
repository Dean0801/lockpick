use std::collections::HashSet;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

use oxc_ast::ast::Statement;

/// Normalize an import source to a package name
/// "@scope/pkg/utils" → "@scope/pkg"
/// "lodash/get" → "lodash"
/// "./local" → None (skip relative)
/// "node:fs" → None (skip builtin)
pub fn normalize_package_name(source: &str) -> Option<String> {
    // Skip relative imports
    if source.starts_with('.') {
        return None;
    }

    // Skip Node.js builtin modules
    if source.starts_with("node:") {
        return None;
    }

    // Skip known Node.js builtins without prefix
    if is_node_builtin(source) {
        return None;
    }

    // Scoped package: @scope/pkg/sub → @scope/pkg
    if source.starts_with('@') {
        let parts: Vec<&str> = source.splitn(3, '/').collect();
        if parts.len() >= 2 {
            return Some(format!("{}/{}", parts[0], parts[1]));
        }
        return Some(source.to_string());
    }

    // Regular package: lodash/get → lodash
    let name = source.split('/').next().unwrap_or(source);
    Some(name.to_string())
}

fn is_node_builtin(name: &str) -> bool {
    const BUILTINS: &[&str] = &[
        "assert",
        "buffer",
        "child_process",
        "cluster",
        "crypto",
        "dgram",
        "dns",
        "events",
        "fs",
        "http",
        "http2",
        "https",
        "net",
        "os",
        "path",
        "perf_hooks",
        "process",
        "querystring",
        "readline",
        "stream",
        "string_decoder",
        "timers",
        "tls",
        "tty",
        "url",
        "util",
        "v8",
        "vm",
        "worker_threads",
        "zlib",
    ];
    let base = name.split('/').next().unwrap_or(name);
    BUILTINS.contains(&base)
}

/// Extract all imported package names from a single source file
pub fn extract_imports_from_source(source: &str, path: &Path) -> HashSet<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_default();

    let ret = Parser::new(&allocator, source, source_type).parse();

    let mut packages = HashSet::new();

    // Extract from static import/export declarations
    for stmt in &ret.program.body {
        if let Some(source_val) = extract_module_source(stmt)
            && let Some(pkg) = normalize_package_name(source_val)
        {
            packages.insert(pkg);
        }
    }

    packages
}

/// Extract module source string from import/export statements
fn extract_module_source<'a>(stmt: &'a Statement<'a>) -> Option<&'a str> {
    match stmt {
        Statement::ImportDeclaration(decl) => Some(decl.source.value.as_str()),
        Statement::ExportNamedDeclaration(decl) => decl.source.as_ref().map(|s| s.value.as_str()),
        Statement::ExportAllDeclaration(decl) => Some(decl.source.value.as_str()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_normalize_scoped_package() {
        assert_eq!(
            normalize_package_name("@scope/pkg/utils"),
            Some("@scope/pkg".to_string())
        );
    }

    #[test]
    fn test_normalize_regular_package() {
        assert_eq!(
            normalize_package_name("lodash/get"),
            Some("lodash".to_string())
        );
    }

    #[test]
    fn test_normalize_skip_relative() {
        assert_eq!(normalize_package_name("./local"), None);
        assert_eq!(normalize_package_name("../parent"), None);
    }

    #[test]
    fn test_normalize_skip_builtin() {
        assert_eq!(normalize_package_name("node:fs"), None);
        assert_eq!(normalize_package_name("path"), None);
    }

    #[test]
    fn test_extract_esm_import() {
        let source = r#"import React from 'react';"#;
        let path = Path::new("test.ts");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("react"));
    }

    #[test]
    fn test_extract_named_import() {
        let source = r#"import { useState } from 'react';"#;
        let path = Path::new("test.ts");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("react"));
    }

    #[test]
    fn test_extract_side_effect_import() {
        let source = r#"import 'polyfill';"#;
        let path = Path::new("test.js");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("polyfill"));
    }

    #[test]
    fn test_extract_scoped_import() {
        let source = r#"import { render } from '@testing-library/react';"#;
        let path = Path::new("test.ts");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("@testing-library/react"));
    }

    #[test]
    fn test_skip_relative_import() {
        let source = r#"import { foo } from './utils';"#;
        let path = Path::new("test.ts");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.is_empty());
    }
}
