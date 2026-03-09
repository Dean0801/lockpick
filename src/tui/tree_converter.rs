use crate::tree::{DepTree, TreeNode};
use std::collections::HashMap;
use tui_tree_widget::TreeItem;

pub fn convert_to_tui_tree(dep_tree: &DepTree) -> Vec<TreeItem<'static, String>> {
    let mut duplicate_map = HashMap::new();
    count_duplicates(&dep_tree.roots, &mut duplicate_map);

    dep_tree
        .roots
        .iter()
        .enumerate()
        .map(|(i, root)| convert_node(root, &duplicate_map, &format!("{}", i)))
        .collect()
}

fn count_duplicates(nodes: &[TreeNode], map: &mut HashMap<String, usize>) {
    for node in nodes {
        *map.entry(node.name.clone()).or_insert(0) += 1;
        count_duplicates(&node.children, map);
    }
}

fn convert_node(
    node: &TreeNode,
    duplicate_map: &HashMap<String, usize>,
    path: &str,
) -> TreeItem<'static, String> {
    let is_duplicate = duplicate_map
        .get(&node.name)
        .is_some_and(|&count| count > 1);

    let label = if node.circular {
        format!("{} {} (circular)", node.name, node.version)
    } else if is_duplicate {
        format!("{} {} [DUP]", node.name, node.version)
    } else {
        format!("{} {}", node.name, node.version)
    };

    let children: Vec<TreeItem<String>> = node
        .children
        .iter()
        .enumerate()
        .map(|(i, child)| convert_node(child, duplicate_map, &format!("{}/{}", path, i)))
        .collect();

    TreeItem::new(path.to_string(), label, children).expect("Failed to create tree item")
}
