use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utils::{from_json_lines, to_json_lines};

pub type NodeIndex = usize;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct NormalNodeProps {
    pub children: HashMap<String, NodeIndex>,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct SymlinkNodeProps {
    pub target: NodeIndex,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum NodeProps {
    Normal(NormalNodeProps),
    Symlink(SymlinkNodeProps),
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub index: NodeIndex,
    pub name: String,
    pub parent: NodeIndex,
    pub props: NodeProps,
}

#[derive(PartialEq, Debug, Clone)]
pub struct VFS {
    nodes: Vec<Node>,
}

impl Default for VFS {
    fn default() -> Self {
        VFS::new()
    }
}

impl VFS {
    pub fn to_json_lines(&self) -> String {
        to_json_lines(&self.nodes)
    }

    pub fn from_json_lines(json: &str) -> Self {
        VFS {
            nodes: from_json_lines::<Node>(json).collect(),
        }
    }

    pub fn new() -> Self {
        VFS {
            nodes: vec![Node {
                index: 0, // Root index
                name: "/".to_string(),
                parent: 0, // Root's parent is itself
                props: NodeProps::Normal(NormalNodeProps {
                    children: HashMap::new(),
                }),
            }],
        }
    }

    pub fn get_children(&self, index: NodeIndex) -> Option<&HashMap<String, NodeIndex>> {
        self.get_node_at(index).and_then(|node| match &node.props {
            NodeProps::Normal(props) => Some(&props.children),
            NodeProps::Symlink(props) => self.get_children(props.target),
        })
    }

    pub fn get_children_mut(
        &mut self,
        mut index: NodeIndex,
    ) -> Option<&mut HashMap<String, NodeIndex>> {
        loop {
            let node_props = &self.get_node_at(index)?.props;
            match node_props {
                NodeProps::Normal(_) => break,
                NodeProps::Symlink(props) => {
                    index = props.target;
                }
            }
        }
        match &mut self.get_node_mut_at(index)?.props {
            NodeProps::Normal(props) => Some(&mut props.children),
            NodeProps::Symlink(_) => None, // This should not happen due to the loop above
        }
    }

    pub fn get_index_by_path(&self, path: &str) -> Option<NodeIndex> {
        let mut current_index = 0;
        for part in path.split('/') {
            if part.is_empty() {
                continue;
            }

            current_index = *self.get_children(current_index)?.get(part)?;
        }

        Some(current_index)
    }

    pub fn get_node_by_path(&self, path: &str) -> Option<&Node> {
        let index = self.get_index_by_path(path)?;
        self.get_node_at(index)
    }

    pub fn get_node_at(&self, index: NodeIndex) -> Option<&Node> {
        self.nodes.get(index)
    }

    pub fn get_node_mut_at(&mut self, index: NodeIndex) -> Option<&mut Node> {
        self.nodes.get_mut(index)
    }

    pub fn create_node(
        &mut self,
        parent_index: NodeIndex,
        name: &str,
        props: NodeProps,
    ) -> Result<NodeIndex, String> {
        let new_index = self.nodes.len();
        let real_index = *self
            .get_children_mut(parent_index)
            .unwrap()
            .entry(name.to_string())
            .or_insert(new_index);

        if real_index == new_index {
            self.nodes.push(Node {
                index: new_index,
                name: name.to_string(),
                parent: parent_index,
                props,
            });
        } else {
            match props {
                NodeProps::Symlink(_) => {
                    return Err(format!(
                        "Cannot create a symlink '{}' with the same name as an existing node",
                        name
                    ));
                }
                _ => {}
            }
        }

        Ok(real_index)
    }

    pub fn create_node_recursively(&mut self, path: &str) -> NodeIndex {
        let mut current_index = 0;

        for part in path.split('/').filter(|s| !s.is_empty()) {
            if let Some(child_index) = self.get_children(current_index).unwrap().get(part) {
                current_index = *child_index;
            } else {
                current_index = self
                    .create_node(
                        current_index,
                        part,
                        NodeProps::Normal(NormalNodeProps {
                            children: HashMap::new(),
                        }),
                    )
                    .unwrap();
            }
        }

        current_index
    }

    pub fn create_symlink(&mut self, path: &str, target: &str) -> Result<NodeIndex, String> {
        let parent_path = &path[..path.rfind('/').unwrap_or(0)];
        let parent_index = self.create_node_recursively(parent_path);
        self.create_node(
            parent_index,
            &path[parent_path.len() + 1..],
            NodeProps::Symlink(SymlinkNodeProps {
                target: self
                    .get_index_by_path(target)
                    .ok_or_else(|| format!("Target '{}' does not exist", target))?,
            }),
        )
    }

    pub fn remove_index(&mut self, index: NodeIndex) -> Result<NodeIndex, String> {
        match &self.get_node_at(index).unwrap().props {
            NodeProps::Normal(normal_node_props) => {
                if !normal_node_props.children.is_empty() {
                    return Err(format!(
                        "Node '{}' has children",
                        self.get_node_at(index).unwrap().name
                    ));
                }
            }
            NodeProps::Symlink(_) => {}
        }

        let (parent, name) = {
            let node = self.get_node_at(index).unwrap();
            (node.parent, node.name.clone())
        };

        self.get_children_mut(parent).unwrap().remove(&name);

        return Ok(index);
    }

    pub fn remove_node(&mut self, path: &str) -> Result<NodeIndex, String> {
        if let Some(index) = self.get_index_by_path(path) {
            self.remove_index(index)
        } else {
            Err(format!("Path '{}' does not exist", path))
        }
    }

    pub fn remove_index_recursively(&mut self, index: NodeIndex) -> Result<NodeIndex, String> {
        // If the index has children, remove its content recursively first
        let children = if let Some(children) = self.get_children(index) {
            children.clone()
        } else {
            HashMap::new()
        };

        for (_, child_index) in children {
            self.remove_index_recursively(child_index)?;
        }

        self.remove_index(index)
    }

    pub fn remove_node_recursively(&mut self, path: &str) -> Result<NodeIndex, String> {
        if let Some(index) = self.get_index_by_path(path) {
            self.remove_index_recursively(index)
        } else {
            Err(format!("Path '{}' does not exist", path))
        }
    }

    pub fn to_path(&self, index: NodeIndex) -> Result<String, String> {
        let mut path = String::new();
        let mut current_index = index;

        while current_index != 0 {
            if let Some(node) = self.get_node_at(current_index) {
                if !path.is_empty() {
                    path.insert_str(0, "/");
                }
                path.insert_str(0, &node.name);
                current_index = node.parent;
            } else {
                return Err(format!("INode {} not found", current_index));
            }
        }

        Ok(if path.is_empty() {
            "/".to_string() // Root path
        } else {
            path.insert(0, '/');
            path
        })
    }

    pub fn resolve_link_path(&self, index: NodeIndex) -> Result<String, String> {
        let mut path = String::new();
        let mut current_index = index;

        while current_index != 0 {
            if let Some(node) = self.get_node_at(current_index) {
                match &node.props {
                    NodeProps::Symlink(props) => {
                        current_index = props.target; // Follow symlink
                        continue;
                    }
                    NodeProps::Normal(_) => {}
                }

                if !path.is_empty() {
                    path.insert_str(0, "/");
                }
                path.insert_str(0, &node.name);
                current_index = node.parent;
            } else {
                return Err(format!("INode {} not found", current_index));
            }
        }

        Ok(if path.is_empty() {
            "/".to_string() // Root path
        } else {
            path.insert(0, '/');
            path
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfs() {
        let vfs = VFS::new();
        assert_eq!(vfs.get_index_by_path("/"), Some(0));
        assert_eq!(vfs.get_index_by_path("/nonexistent"), None);

        let mut vfs = VFS::new();
        let index = vfs.create_node_recursively("/a/b/c");
        assert_eq!(vfs.get_index_by_path("/a/b/c"), Some(index));
        assert_eq!(vfs.get_node_by_path("/a/b/c").unwrap().name, "c");
        assert_eq!(
            vfs.get_node_by_path("/a/b/c").unwrap().parent,
            vfs.get_index_by_path("/a/b").unwrap()
        );
        assert_eq!(vfs.get_node_by_path("/a/b").unwrap().name, "b");
        assert_eq!(
            vfs.get_node_by_path("/a/b").unwrap().parent,
            vfs.get_index_by_path("/a").unwrap()
        );
        assert_eq!(vfs.get_node_by_path("/a").unwrap().name, "a");
        assert_eq!(vfs.get_node_by_path("/a/b/c/d"), None); // Should not exist
    }

    #[test]
    fn test_symlink() {
        let mut vfs = VFS::new();
        let target_index = vfs.create_node_recursively("/target");
        let file_index = vfs.create_node_recursively("/target/a/a");
        let symlink_index = vfs.create_symlink("/link", "/target").unwrap();
        assert_eq!(vfs.get_index_by_path("/link"), Some(symlink_index));
        assert_eq!(vfs.get_node_by_path("/link").unwrap().name, "link");
        assert_eq!(
            vfs.get_node_by_path("/link").unwrap().parent,
            vfs.get_index_by_path("/").unwrap()
        );
        assert!(matches!(
            &vfs.get_node_by_path("/link").unwrap().props,
            NodeProps::Symlink(props) if props.target == target_index
        ));

        assert_eq!(vfs.get_index_by_path("/link/a/a"), Some(file_index));
        assert_eq!(vfs.get_node_by_path("/link/a/a").unwrap().name, "a");
        assert_eq!(
            vfs.get_node_by_path("/link/a/a").unwrap().parent,
            vfs.get_index_by_path("/link/a").unwrap()
        );
        assert!(matches!(
            &vfs.get_node_by_path("/link/a/a").unwrap().props,
            NodeProps::Normal(props) if props.children.is_empty()
        ));
    }

    #[test]
    fn test_remove_node() {
        let mut vfs = VFS::new();
        let index_c = vfs.create_node_recursively("/a/b/c");

        assert_eq!(
            vfs.remove_node("/a/b"),
            Err("Node 'b' has children".to_string())
        );

        assert_eq!(vfs.get_index_by_path("/a/b/c"), Some(index_c));
        assert_eq!(vfs.remove_node("/a/b/c").unwrap(), index_c);
        assert_eq!(vfs.get_index_by_path("/a/b/c"), None);
        assert_eq!(
            vfs.remove_node("/a/b/c"),
            Err("Path '/a/b/c' does not exist".to_string())
        );

        let index_b = vfs.get_index_by_path("/a/b").unwrap();
        assert_eq!(vfs.get_children(index_b).unwrap().len(), 0); // No children left after removal
        assert_eq!(vfs.remove_node("/a/b").unwrap(), index_b);
        assert_eq!(vfs.get_index_by_path("/a/b"), None);

        let index_a = vfs.get_index_by_path("/a").unwrap();
        assert_eq!(vfs.get_children(index_a).unwrap().len(), 0); // No children left after removal
    }

    #[test]
    fn test_remove_node_recursively() {
        let mut vfs = VFS::new();
        vfs.create_node_recursively("/a/b/c");
        vfs.create_node_recursively("/a/b/d");
        vfs.create_node_recursively("/a/e/f");
        vfs.create_node_recursively("/x/y/z");
        vfs.create_node_recursively("/x/y/a/b/c");
        vfs.create_node_recursively("/x/y/a/b/d");

        // remove
        vfs.remove_node_recursively("/a/b").unwrap();
        assert_eq!(vfs.get_index_by_path("/a/b"), None);
        assert_eq!(vfs.get_index_by_path("/a/b/c"), None);
        assert_eq!(vfs.get_index_by_path("/a/b/d"), None);
        assert!(vfs.get_index_by_path("/a/e/f").is_some()); // Still exists
        assert!(vfs.get_index_by_path("/x/y/z").is_some()); // Still exists
        assert!(vfs.get_index_by_path("/x/y/a/b/c").is_some()); // Still exists
        assert!(vfs.get_index_by_path("/x/y/a/b/d").is_some()); // Still exists
        assert!(vfs.get_index_by_path("/a/e/f").is_some()); // Still exists
    }

    #[test]
    fn test_to_path() {
        let mut vfs = VFS::new();
        let _ = vfs.create_node_recursively("/a/b/c");
        let _ = vfs.create_node_recursively("/a/b/d");
        let _ = vfs.create_node_recursively("/x/y/z");

        vfs.remove_node_recursively("/a/b").unwrap();
        assert_eq!(vfs.get_index_by_path("/a/b"), None);
        assert_eq!(vfs.get_index_by_path("/a/b/c"), None);
        assert_eq!(vfs.get_index_by_path("/a/b/d"), None);
    }

    #[test]
    fn test_resolve_link_path() {
        let mut vfs = VFS::new();
        let _ = vfs.create_node_recursively("/target");
        let symlink_index = vfs.create_symlink("/link", "/target").unwrap();
        let a_index = vfs.create_node_recursively("/link/a");
        assert_eq!(vfs.resolve_link_path(symlink_index).unwrap(), "/target");
        assert_eq!(vfs.resolve_link_path(a_index).unwrap(), "/target/a");
        assert_eq!(vfs.resolve_link_path(0).unwrap(), "/");
    }
}
