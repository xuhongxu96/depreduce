use std::collections::HashMap;

pub type INode = usize;

#[derive(PartialEq, Debug, Clone, Default)]
pub struct NormalNodeProps {
    children: HashMap<String, INode>,
}

#[derive(PartialEq, Debug, Clone, Default)]
pub struct SymlinkNodeProps {
    target: INode,
}

#[derive(PartialEq, Debug, Clone)]
pub enum NodeProps {
    Normal(NormalNodeProps),
    Symlink(SymlinkNodeProps),
}

#[derive(PartialEq, Debug, Clone)]
pub struct Node {
    inode: INode,
    name: String,
    parent: INode,
    props: NodeProps,
}

#[derive(PartialEq, Debug, Clone)]
pub struct VFS {
    nodes: Vec<Option<Node>>,
}

impl VFS {
    pub fn new() -> Self {
        VFS {
            nodes: vec![Some(Node {
                inode: 0, // Root inode
                name: "/".to_string(),
                parent: 0, // Root's parent is itself
                props: NodeProps::Normal(NormalNodeProps {
                    children: HashMap::new(),
                }),
            })],
        }
    }

    pub fn get_children(&self, inode: INode) -> Option<&HashMap<String, INode>> {
        self.get_node_at(inode).and_then(|node| match &node.props {
            NodeProps::Normal(props) => Some(&props.children),
            NodeProps::Symlink(props) => self.get_children(props.target),
        })
    }

    pub fn get_children_mut(&mut self, mut inode: INode) -> Option<&mut HashMap<String, INode>> {
        loop {
            let node_props = &self.get_node_at(inode)?.props;
            match node_props {
                NodeProps::Normal(_) => break,
                NodeProps::Symlink(props) => {
                    inode = props.target;
                }
            }
        }
        match &mut self.get_node_mut_at(inode)?.props {
            NodeProps::Normal(props) => Some(&mut props.children),
            NodeProps::Symlink(_) => None, // This should not happen due to the loop above
        }
    }

    pub fn get_inode_by_path(&self, path: &str) -> Option<INode> {
        let mut current_inode = 0;
        for part in path.split('/') {
            if part.is_empty() {
                continue;
            }

            current_inode = *self.get_children(current_inode)?.get(part)?;
        }

        Some(current_inode)
    }

    pub fn get_node_by_path(&self, path: &str) -> Option<&Node> {
        let inode = self.get_inode_by_path(path)?;
        self.get_node_at(inode)
    }

    pub fn get_node_at(&self, inode: INode) -> Option<&Node> {
        self.nodes.get(inode)?.as_ref()
    }

    pub fn get_node_mut_at(&mut self, inode: INode) -> Option<&mut Node> {
        self.nodes.get_mut(inode)?.as_mut()
    }

    pub fn create_node(&mut self, parent_inode: INode, name: &str, props: NodeProps) -> INode {
        let inode = self.nodes.len();
        self.get_children_mut(parent_inode)
            .unwrap()
            .insert(name.to_string(), inode);
        self.nodes.push(Some(Node {
            inode,
            name: name.to_string(),
            parent: parent_inode,
            props,
        }));

        inode
    }

    pub fn create_node_recursively(&mut self, path: &str) -> INode {
        let mut current_inode = 0;

        for part in path.split('/').filter(|s| !s.is_empty()) {
            if let Some(child_inode) = self.get_children(current_inode).unwrap().get(part) {
                current_inode = *child_inode;
            } else {
                current_inode = self.create_node(
                    current_inode,
                    part,
                    NodeProps::Normal(NormalNodeProps {
                        children: HashMap::new(),
                    }),
                );
            }
        }

        current_inode
    }

    pub fn create_symlink(&mut self, path: &str, target: &str) -> INode {
        let parent_path = &path[..path.rfind('/').unwrap_or(0)];
        let parent_inode = self.create_node_recursively(parent_path);
        self.create_node(
            parent_inode,
            &path[parent_path.len() + 1..],
            NodeProps::Symlink(SymlinkNodeProps {
                target: self.get_inode_by_path(target).unwrap(),
            }),
        )
    }

    pub fn remove_node(&mut self, path: &str) -> Result<INode, &str> {
        if let Some(inode) = self.get_inode_by_path(path) {
            match &self.get_node_at(inode).unwrap().props {
                NodeProps::Normal(normal_node_props) => {
                    if !normal_node_props.children.is_empty() {
                        return Err("Node has children");
                    }
                }
                NodeProps::Symlink(_) => {}
            }

            let node = self.nodes.get_mut(inode).unwrap().take().unwrap();

            self.get_children_mut(node.parent)
                .unwrap()
                .remove(&node.name);

            return Ok(inode);
        }

        Err("Path does not exist")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfs() {
        let vfs = VFS::new();
        assert_eq!(vfs.get_inode_by_path("/"), Some(0));
        assert_eq!(vfs.get_inode_by_path("/nonexistent"), None);

        let mut vfs = VFS::new();
        let inode = vfs.create_node_recursively("/a/b/c");
        assert_eq!(vfs.get_inode_by_path("/a/b/c"), Some(inode));
        assert_eq!(vfs.get_node_by_path("/a/b/c").unwrap().name, "c");
        assert_eq!(
            vfs.get_node_by_path("/a/b/c").unwrap().parent,
            vfs.get_inode_by_path("/a/b").unwrap()
        );
        assert_eq!(vfs.get_node_by_path("/a/b").unwrap().name, "b");
        assert_eq!(
            vfs.get_node_by_path("/a/b").unwrap().parent,
            vfs.get_inode_by_path("/a").unwrap()
        );
        assert_eq!(vfs.get_node_by_path("/a").unwrap().name, "a");
        assert_eq!(vfs.get_node_by_path("/a/b/c/d"), None); // Should not exist
    }

    #[test]
    fn test_symlink() {
        let mut vfs = VFS::new();
        let target_inode = vfs.create_node_recursively("/target");
        let file_inode = vfs.create_node_recursively("/target/a/a");
        let symlink_inode = vfs.create_symlink("/link", "/target");
        assert_eq!(vfs.get_inode_by_path("/link"), Some(symlink_inode));
        assert_eq!(vfs.get_node_by_path("/link").unwrap().name, "link");
        assert_eq!(
            vfs.get_node_by_path("/link").unwrap().parent,
            vfs.get_inode_by_path("/").unwrap()
        );
        assert!(matches!(
            &vfs.get_node_by_path("/link").unwrap().props,
            NodeProps::Symlink(props) if props.target == target_inode
        ));

        assert_eq!(vfs.get_inode_by_path("/link/a/a"), Some(file_inode));
        assert_eq!(vfs.get_node_by_path("/link/a/a").unwrap().name, "a");
        assert_eq!(
            vfs.get_node_by_path("/link/a/a").unwrap().parent,
            vfs.get_inode_by_path("/link/a").unwrap()
        );
        assert!(matches!(
            &vfs.get_node_by_path("/link/a/a").unwrap().props,
            NodeProps::Normal(props) if props.children.is_empty()
        ));
    }

    #[test]
    fn test_remove_node() {
        let mut vfs = VFS::new();
        let inode_c = vfs.create_node_recursively("/a/b/c");

        assert_eq!(vfs.remove_node("/a/b"), Err("Node has children"));

        assert_eq!(vfs.get_inode_by_path("/a/b/c"), Some(inode_c));
        assert_eq!(vfs.remove_node("/a/b/c"), Ok(inode_c));
        assert_eq!(vfs.get_inode_by_path("/a/b/c"), None);
        assert_eq!(vfs.remove_node("/a/b/c"), Err("Path does not exist"));

        let inode_b = vfs.get_inode_by_path("/a/b").unwrap();
        assert_eq!(vfs.get_children(inode_b).unwrap().len(), 0); // No children left after removal
        assert_eq!(vfs.remove_node("/a/b"), Ok(inode_b));
        assert_eq!(vfs.get_inode_by_path("/a/b"), None);

        let inode_a = vfs.get_inode_by_path("/a").unwrap();
        assert_eq!(vfs.get_children(inode_a).unwrap().len(), 0); // No children left after removal
    }
}
