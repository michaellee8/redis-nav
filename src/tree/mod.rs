use crate::redis_client::RedisType;

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub full_key: Option<String>,
    pub node_type: NodeType,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    pub loaded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Folder,
    Key(RedisType),
}

impl TreeNode {
    pub fn new_folder(name: String) -> Self {
        Self {
            name,
            full_key: None,
            node_type: NodeType::Folder,
            children: Vec::new(),
            expanded: false,
            loaded: true,
        }
    }

    pub fn new_key(name: String, full_key: String, redis_type: RedisType) -> Self {
        Self {
            name,
            full_key: Some(full_key),
            node_type: NodeType::Key(redis_type),
            children: Vec::new(),
            expanded: false,
            loaded: true,
        }
    }

    pub fn is_folder(&self) -> bool {
        matches!(self.node_type, NodeType::Folder)
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }
}

pub struct TreeBuilder {
    delimiters: Vec<char>,
}

impl TreeBuilder {
    pub fn new(delimiters: Vec<char>) -> Self {
        Self { delimiters }
    }

    pub fn build(&self, keys: &[(String, RedisType)]) -> Vec<TreeNode> {
        let mut root_children: Vec<TreeNode> = Vec::new();

        for (key, redis_type) in keys {
            self.insert_key(&mut root_children, key, *redis_type);
        }

        self.sort_nodes(&mut root_children);
        root_children
    }

    fn insert_key(&self, nodes: &mut Vec<TreeNode>, key: &str, redis_type: RedisType) {
        let parts = self.split_key(key);

        if parts.is_empty() {
            return;
        }

        self.insert_parts(nodes, &parts, key, redis_type);
    }

    fn insert_parts(
        &self,
        nodes: &mut Vec<TreeNode>,
        parts: &[&str],
        full_key: &str,
        redis_type: RedisType,
    ) {
        if parts.is_empty() {
            return;
        }

        let name = parts[0];
        let remaining = &parts[1..];

        // Find or create node
        let node_idx = nodes.iter().position(|n| n.name == name);

        if remaining.is_empty() {
            // This is a leaf node (actual key)
            if let Some(idx) = node_idx {
                // Convert folder to key if needed, or update
                if nodes[idx].is_folder() {
                    // Keep as folder but mark it also has a key
                    nodes[idx].full_key = Some(full_key.to_string());
                    nodes[idx].node_type = NodeType::Key(redis_type);
                }
            } else {
                nodes.push(TreeNode::new_key(
                    name.to_string(),
                    full_key.to_string(),
                    redis_type,
                ));
            }
        } else {
            // This is an intermediate node (folder)
            let idx = if let Some(idx) = node_idx {
                idx
            } else {
                nodes.push(TreeNode::new_folder(name.to_string()));
                nodes.len() - 1
            };

            self.insert_parts(&mut nodes[idx].children, remaining, full_key, redis_type);
        }
    }

    fn split_key<'a>(&self, key: &'a str) -> Vec<&'a str> {
        let mut parts = Vec::new();
        let mut start = 0;

        for (i, c) in key.char_indices() {
            if self.delimiters.contains(&c) {
                if i > start {
                    parts.push(&key[start..i]);
                }
                start = i + c.len_utf8();
            }
        }

        if start < key.len() {
            parts.push(&key[start..]);
        }

        parts
    }

    fn sort_nodes(&self, nodes: &mut Vec<TreeNode>) {
        nodes.sort_by(|a, b| {
            // Folders first, then by name
            match (&a.node_type, &b.node_type) {
                (NodeType::Folder, NodeType::Key(_)) => std::cmp::Ordering::Less,
                (NodeType::Key(_), NodeType::Folder) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        for node in nodes {
            self.sort_nodes(&mut node.children);
        }
    }
}
