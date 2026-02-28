use super::{DepTree, TreeNode};
use crate::DepType;
use crate::i18n::I18n;

#[derive(Debug, Clone)]
pub enum TreeFormat {
    Terminal,
    Dot,
    Json,
    Mermaid,
}

/// Render dependency tree in the specified format.
pub fn render(tree: &DepTree, format: &TreeFormat, _i18n: &I18n) -> String {
    match format {
        TreeFormat::Terminal => render_terminal(tree),
        TreeFormat::Dot => render_dot(tree),
        TreeFormat::Json => render_json(tree),
        TreeFormat::Mermaid => render_mermaid(tree),
    }
}

fn render_terminal(tree: &DepTree) -> String {
    let mut out = String::new();
    for (i, root) in tree.roots.iter().enumerate() {
        let is_last = i == tree.roots.len() - 1;
        let suffix = match root.dep_type {
            DepType::Dev => " (dev)",
            DepType::Prod => "",
        };
        let label = if root.circular {
            format!("{}@{} (circular){suffix}", root.name, root.version)
        } else {
            format!("{}@{}{suffix}", root.name, root.version)
        };
        let prefix = if is_last { "└── " } else { "├── " };
        out.push_str(&format!("{prefix}{label}\n"));
        let child_prefix = if is_last { "    " } else { "│   " };
        render_children(&mut out, &root.children, child_prefix);
    }
    out
}

fn render_children(out: &mut String, children: &[TreeNode], prefix: &str) {
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let label = if child.circular {
            format!("{}@{} (circular)", child.name, child.version)
        } else {
            format!("{}@{}", child.name, child.version)
        };
        out.push_str(&format!("{prefix}{connector}{label}\n"));
        let next_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
        render_children(out, &child.children, &next_prefix);
    }
}

fn render_dot(tree: &DepTree) -> String {
    let mut out = String::from("digraph dependencies {\n  rankdir=LR;\n");
    for root in &tree.roots {
        dot_edges(&mut out, root);
    }
    out.push_str("}\n");
    out
}

fn dot_edges(out: &mut String, node: &TreeNode) {
    let from = format!("{}@{}", node.name, node.version);
    for child in &node.children {
        if !child.circular {
            let to = format!("{}@{}", child.name, child.version);
            out.push_str(&format!("  \"{}\" -> \"{}\";\n", from, to));
            dot_edges(out, child);
        }
    }
}

fn render_json(tree: &DepTree) -> String {
    serde_json::to_string_pretty(tree).unwrap_or_else(|_| "{}".to_string())
}

fn render_mermaid(tree: &DepTree) -> String {
    let mut out = String::from("graph LR\n");
    for root in &tree.roots {
        mermaid_edges(&mut out, root);
    }
    out
}

fn sanitize_id(s: &str) -> String {
    s.replace(['@', '/', '.'], "_")
}

fn mermaid_edges(out: &mut String, node: &TreeNode) {
    let from_label = format!("{}@{}", node.name, node.version);
    let from_id = sanitize_id(&from_label);
    for child in &node.children {
        if !child.circular {
            let to_label = format!("{}@{}", child.name, child.version);
            let to_id = sanitize_id(&to_label);
            out.push_str(&format!(
                "  {from_id}[\"{from_label}\"] --> {to_id}[\"{to_label}\"]\n"
            ));
            mermaid_edges(out, child);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::DepTree;
    use crate::tree::tests::make_test_graph;

    #[test]
    fn test_terminal_render() {
        let graph = make_test_graph();
        let tree = DepTree::from_graph(&graph);
        let i18n = crate::i18n::I18n::detect(Some("en"));
        let out = render(&tree, &TreeFormat::Terminal, &i18n);
        assert!(out.contains("react@18.2.0"));
        assert!(out.contains("└── ") || out.contains("├── "));
        assert!(out.contains("js-tokens@4.0.0"));
    }

    #[test]
    fn test_dot_render() {
        let graph = make_test_graph();
        let tree = DepTree::from_graph(&graph);
        let i18n = crate::i18n::I18n::detect(Some("en"));
        let out = render(&tree, &TreeFormat::Dot, &i18n);
        assert!(out.starts_with("digraph dependencies {"));
        assert!(out.contains("\"react@18.2.0\" -> \"loose-envify@1.4.0\""));
        assert!(out.contains("\"loose-envify@1.4.0\" -> \"js-tokens@4.0.0\""));
    }

    #[test]
    fn test_json_render() {
        let graph = make_test_graph();
        let tree = DepTree::from_graph(&graph);
        let i18n = crate::i18n::I18n::detect(Some("en"));
        let out = render(&tree, &TreeFormat::Json, &i18n);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(parsed["roots"].is_array());
    }

    #[test]
    fn test_mermaid_render() {
        let graph = make_test_graph();
        let tree = DepTree::from_graph(&graph);
        let i18n = crate::i18n::I18n::detect(Some("en"));
        let out = render(&tree, &TreeFormat::Mermaid, &i18n);
        assert!(out.starts_with("graph LR"));
        assert!(out.contains("-->"));
    }

    #[test]
    fn test_sanitize_id() {
        assert_eq!(sanitize_id("react@18.2.0"), "react_18_2_0");
        assert_eq!(sanitize_id("@scope/pkg@1.0.0"), "_scope_pkg_1_0_0");
    }
}
