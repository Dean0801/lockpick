use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

use oxc_ast::ast::{Argument, ClassBody, ClassElement, Expression, Statement};

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

static NODE_BUILTINS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "assert",
        "async_hooks",
        "buffer",
        "child_process",
        "cluster",
        "console",
        "constants",
        "crypto",
        "dgram",
        "diagnostics_channel",
        "dns",
        "domain",
        "events",
        "fs",
        "http",
        "http2",
        "https",
        "inspector",
        "module",
        "net",
        "os",
        "path",
        "perf_hooks",
        "process",
        "punycode",
        "querystring",
        "readline",
        "repl",
        "stream",
        "string_decoder",
        "sys",
        "timers",
        "tls",
        "trace_events",
        "tty",
        "url",
        "util",
        "v8",
        "vm",
        "wasi",
        "worker_threads",
        "zlib",
    ])
});

fn is_node_builtin(name: &str) -> bool {
    let base = name.split('/').next().unwrap_or(name);
    NODE_BUILTINS.contains(base)
}

/// Extract all imported package names from a single source file
pub fn extract_imports_from_source(source: &str, path: &Path) -> HashSet<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_default();

    let ret = Parser::new(&allocator, source, source_type).parse();

    let mut packages = HashSet::new();

    for stmt in &ret.program.body {
        extract_from_statement(stmt, &mut packages);
    }

    packages
}

/// Recurse into a block of statements
fn extract_from_body<'a>(stmts: &'a [Statement<'a>], packages: &mut HashSet<String>) {
    for stmt in stmts {
        extract_from_statement(stmt, packages);
    }
}

/// Extract imports from a single statement (used for top-level and nested bodies)
fn extract_from_statement<'a>(stmt: &'a Statement<'a>, packages: &mut HashSet<String>) {
    // Static import/export declarations
    if let Some(source_val) = extract_module_source(stmt)
        && let Some(pkg) = normalize_package_name(source_val)
    {
        packages.insert(pkg);
    }

    match stmt {
        // Variable declarations: const x = require('pkg') or const x = import('pkg')
        Statement::VariableDeclaration(decl) => {
            for declarator in &decl.declarations {
                if let Some(init) = &declarator.init {
                    extract_from_expression(init, packages);
                }
            }
        }

        // Bare expression statements: require('pkg') or import('pkg')
        Statement::ExpressionStatement(expr_stmt) => {
            extract_from_expression(&expr_stmt.expression, packages);
        }

        // Function declarations: recurse into body
        Statement::FunctionDeclaration(func) => {
            if let Some(body) = &func.body {
                extract_from_body(&body.statements, packages);
            }
        }

        // Block statement: { ... }
        Statement::BlockStatement(block) => {
            extract_from_body(&block.body, packages);
        }

        // If / else
        Statement::IfStatement(if_stmt) => {
            extract_from_expression(&if_stmt.test, packages);
            extract_from_statement(&if_stmt.consequent, packages);
            if let Some(alt) = &if_stmt.alternate {
                extract_from_statement(alt, packages);
            }
        }

        // Try / catch / finally
        Statement::TryStatement(try_stmt) => {
            extract_from_body(&try_stmt.block.body, packages);
            if let Some(handler) = &try_stmt.handler {
                extract_from_body(&handler.body.body, packages);
            }
            if let Some(finalizer) = &try_stmt.finalizer {
                extract_from_body(&finalizer.body, packages);
            }
        }

        // For statement
        Statement::ForStatement(for_stmt) => {
            extract_from_statement(&for_stmt.body, packages);
        }

        // For-in / for-of
        Statement::ForInStatement(stmt) => {
            extract_from_statement(&stmt.body, packages);
        }
        Statement::ForOfStatement(stmt) => {
            extract_from_statement(&stmt.body, packages);
        }

        // While / do-while
        Statement::WhileStatement(stmt) => {
            extract_from_statement(&stmt.body, packages);
        }
        Statement::DoWhileStatement(stmt) => {
            extract_from_statement(&stmt.body, packages);
        }

        // Switch statement
        Statement::SwitchStatement(switch) => {
            extract_from_expression(&switch.discriminant, packages);
            for case in &switch.cases {
                extract_from_body(&case.consequent, packages);
            }
        }

        // Return statement
        Statement::ReturnStatement(ret) => {
            if let Some(arg) = &ret.argument {
                extract_from_expression(arg, packages);
            }
        }

        // Class declaration (static blocks, method bodies)
        Statement::ClassDeclaration(class) => {
            extract_from_class_body(&class.body, packages);
        }

        _ => {}
    }
}

/// Extract imports from class body (methods, static blocks, property initializers)
fn extract_from_class_body<'a>(body: &'a ClassBody<'a>, packages: &mut HashSet<String>) {
    for elem in &body.body {
        match elem {
            ClassElement::MethodDefinition(method) => {
                if let Some(body) = &method.value.body {
                    extract_from_body(&body.statements, packages);
                }
            }
            ClassElement::StaticBlock(block) => {
                extract_from_body(&block.body, packages);
            }
            ClassElement::PropertyDefinition(prop) => {
                if let Some(value) = &prop.value {
                    extract_from_expression(value, packages);
                }
            }
            ClassElement::AccessorProperty(prop) => {
                if let Some(value) = &prop.value {
                    extract_from_expression(value, packages);
                }
            }
            _ => {}
        }
    }
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

/// Extract package name from require() or dynamic import() expressions
fn extract_from_expression<'a>(expr: &'a Expression<'a>, packages: &mut HashSet<String>) {
    match expr {
        // require('pkg') or require.resolve('pkg')
        Expression::CallExpression(call) => {
            // require('pkg')
            if let Expression::Identifier(id) = &call.callee
                && id.name == "require"
                && let Some(Argument::StringLiteral(lit)) = call.arguments.first()
                && let Some(pkg) = normalize_package_name(lit.value.as_str())
            {
                packages.insert(pkg);
            }
            // require.resolve('pkg')
            if let Expression::StaticMemberExpression(member) = &call.callee
                && member.property.name == "resolve"
                && let Expression::Identifier(id) = &member.object
                && id.name == "require"
                && let Some(Argument::StringLiteral(lit)) = call.arguments.first()
                && let Some(pkg) = normalize_package_name(lit.value.as_str())
            {
                packages.insert(pkg);
            }
            // Recurse into call arguments for nested require/import calls
            for arg in &call.arguments {
                match arg {
                    Argument::CallExpression(inner) => {
                        // Recurse by directly checking the inner call
                        if let Expression::Identifier(id) = &inner.callee
                            && id.name == "require"
                            && let Some(Argument::StringLiteral(lit)) = inner.arguments.first()
                            && let Some(pkg) = normalize_package_name(lit.value.as_str())
                        {
                            packages.insert(pkg);
                        }
                    }
                    Argument::ImportExpression(imp) => {
                        if let Expression::StringLiteral(lit) = &imp.source
                            && let Some(pkg) = normalize_package_name(lit.value.as_str())
                        {
                            packages.insert(pkg);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Dynamic import('pkg')
        Expression::ImportExpression(imp) => {
            if let Expression::StringLiteral(lit) = &imp.source
                && let Some(pkg) = normalize_package_name(lit.value.as_str())
            {
                packages.insert(pkg);
            }
        }

        // await expr
        Expression::AwaitExpression(await_expr) => {
            extract_from_expression(&await_expr.argument, packages);
        }

        // Conditional: cond ? require('a') : require('b')
        Expression::ConditionalExpression(cond) => {
            extract_from_expression(&cond.consequent, packages);
            extract_from_expression(&cond.alternate, packages);
        }

        // Logical: x || require('fallback')
        Expression::LogicalExpression(logic) => {
            extract_from_expression(&logic.left, packages);
            extract_from_expression(&logic.right, packages);
        }

        // Assignment: x = require('pkg')
        Expression::AssignmentExpression(assign) => {
            extract_from_expression(&assign.right, packages);
        }

        // Arrow function: () => { require('pkg') }
        Expression::ArrowFunctionExpression(arrow) => {
            extract_from_body(&arrow.body.statements, packages);
        }

        // Function expression: function() { require('pkg') }
        Expression::FunctionExpression(func) => {
            if let Some(body) = &func.body {
                extract_from_body(&body.statements, packages);
            }
        }

        // Class expression
        Expression::ClassExpression(class) => {
            extract_from_class_body(&class.body, packages);
        }

        // Sequence: (a, b, require('pkg'))
        Expression::SequenceExpression(seq) => {
            for expr in &seq.expressions {
                extract_from_expression(expr, packages);
            }
        }

        _ => {}
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

    #[test]
    fn test_extract_cjs_require() {
        let source = r#"const lodash = require('lodash');"#;
        let path = Path::new("test.js");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("lodash"));
    }

    #[test]
    fn test_extract_cjs_require_scoped() {
        let source = r#"const render = require('@testing-library/react');"#;
        let path = Path::new("test.js");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("@testing-library/react"));
    }

    #[test]
    fn test_extract_cjs_require_destructured() {
        let source = r#"const { useState } = require('react');"#;
        let path = Path::new("test.js");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("react"));
    }

    #[test]
    fn test_extract_dynamic_import() {
        let source = r#"const mod = import('lodash');"#;
        let path = Path::new("test.js");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("lodash"));
    }

    #[test]
    fn test_extract_dynamic_import_in_function() {
        let source = r#"
            async function load() {
                const { default: React } = await import('react');
            }
        "#;
        let path = Path::new("test.ts");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("react"));
    }

    #[test]
    fn test_extract_require_in_if_statement() {
        let source = r#"
            if (process.env.NODE_ENV === 'test') {
                const mock = require('jest-mock');
            }
        "#;
        let path = Path::new("test.js");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("jest-mock"));
    }

    #[test]
    fn test_extract_require_in_try_catch() {
        let source = r#"
            try {
                const pkg = require('optional-dep');
            } catch (e) {
                const fallback = require('fallback-dep');
            }
        "#;
        let path = Path::new("test.js");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("optional-dep"));
        assert!(imports.contains("fallback-dep"));
    }

    #[test]
    fn test_extract_require_in_arrow_function() {
        let source = r#"
            const loader = () => {
                const lib = require('lazy-lib');
            };
        "#;
        let path = Path::new("test.js");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("lazy-lib"));
    }

    #[test]
    fn test_extract_require_in_class_method() {
        let source = r#"
            class MyService {
                init() {
                    const db = require('pg');
                }
            }
        "#;
        let path = Path::new("test.js");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("pg"));
    }

    #[test]
    fn test_extract_conditional_require() {
        let source = r#"
            const lib = condition ? require('lib-a') : require('lib-b');
        "#;
        let path = Path::new("test.js");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("lib-a"));
        assert!(imports.contains("lib-b"));
    }

    #[test]
    fn test_extract_require_resolve() {
        let source = r#"
            const resolved = require.resolve('some-pkg');
        "#;
        let path = Path::new("test.js");
        let imports = extract_imports_from_source(source, path);
        assert!(imports.contains("some-pkg"));
    }
}
