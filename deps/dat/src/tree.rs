use crate::error::DatError;

use std::collections::VecDeque;
use std::mem::take;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{
    Arc,
    RwLock
};

type NodePtrType = Arc<RwLock<Node>>;

unsafe impl Send for Node {}
unsafe impl Sync for Node {}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    Compressed{ size: usize },
    #[default]
    Uncompressed,
}

#[derive(Debug, Default, Clone)]
pub struct FileEntry {
    pub offset: usize,
    pub size: usize,
    pub state: FileState,
}


//Tree Node
//Keep children sorted to enable binary search
#[derive(Debug, Clone)]
pub struct Node {
    //Name of the contained object
    pub name: String,

    //Full path from root (including file name)
    pub path: String,

    //Type of object
    pub node_type: NodeType,
}

#[derive(Debug, Clone)]
pub enum NodeType {
    Directory{ children: Vec<NodePtrType> },
    File{ entry: FileEntry },

}

impl Node {
    fn new(name: &str, path: &str, node_type: NodeType) -> Self {
        Node {
            name: name.into(),
            path: path.into(),
            node_type,
        }
    }

    fn new_ptr(node: Node) -> NodePtrType {
        Arc::new(RwLock::new(node))
    }
    
    pub fn new_file(name: &str, path: &str, entry: FileEntry) -> Node {
        let node_type = NodeType::File { entry };
        Node::new(name, path, node_type)
    }

    pub fn new_dir(name: &str, path: &str, children: Vec<Node>) -> Node {
        let mut children: Vec<_> = children.into_iter()
            .map(|n| Node::new_ptr(n))
            .collect();
        children.sort_by(|a, b| {
            let a_lock = a.read().unwrap();
            let b_lock = b.read().unwrap();

            let a = a_lock.get_name();
            let b = b_lock.get_name();

            a.partial_cmp(b).unwrap()
        });

        let node_type = NodeType::Directory { children };
        let node = Node::new(name, path, node_type);

        node
    }

    pub fn new_dir_unsorted(name: &str, path: &str, children: Vec<Node>) -> Node {
        let children: Vec<_> = children.into_iter()
            .map(|n| Node::new_ptr(n))
            .collect();

        let node_type = NodeType::Directory { children };
        let node = Node::new(name, path, node_type);

        node
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub fn get_file_entry(&self) -> Option<&FileEntry> {
        match &self.node_type {
            NodeType::File { entry } => Some(&entry),
            _ => None,
        }
    }

    pub fn get_dir_children(&self) -> Option<&Vec<NodePtrType>> {
        match &self.node_type {
            NodeType::Directory { children } => Some(&children),
            _ => None,
        }
    }

    fn sort_children(children: &mut Vec<NodePtrType>) {
        children.sort_by(|a, b| {
            let a_lock = a.read().unwrap();
            let b_lock = b.read().unwrap();

            let a = a_lock.get_name();
            let b = b_lock.get_name();

            a.partial_cmp(b).unwrap()
        });
    }

    pub fn add_child(&mut self, node: Node) -> Result<NodePtrType, DatError> {
        match &mut self.node_type {
            NodeType::File { entry: _ } => Err(DatError::TreeNodeError),
            NodeType::Directory { children } => {
                let child = Node::new_ptr(node);
                children.push(child.clone());
                Self::sort_children(children);
                Ok(child)
            }
        }
    }

    pub fn add_child_unsorted(&mut self, node: Node) -> Result<NodePtrType, DatError> {
        match &mut self.node_type {
            NodeType::File { entry: _ } => Err(DatError::TreeNodeError),
            NodeType::Directory { children } => {
                let child = Node::new_ptr(node);
                children.push(child.clone());
                //Self::sort_children(children);
                Ok(child)
            }
        }
    }

    pub fn add_children(&mut self, nodes: Vec<Node>) -> Result<(), DatError> {
        match &mut self.node_type {
            NodeType::File { entry: _ } => Err(DatError::TreeNodeError),
            NodeType::Directory { children } => {
                for n in nodes {
                    let n = Node::new_ptr(n);
                    children.push(n);
                }

                Self::sort_children(children);
                Ok(())
            }
        }
    }

    pub fn find_child(&self, name: &str) -> Option<&NodePtrType> {
        match &self.node_type {
            NodeType::File { entry: _ } => None,
            NodeType::Directory { children } => {
                let index = children.binary_search_by(|probe| {
                    let lock = probe.read().unwrap();

                    let pn = lock.get_name();
                    pn.partial_cmp(name).unwrap()
                }).ok()?;

                Some(&children[index])
            }
        }
    }

    pub fn find_child_linear(&self, name: &str) -> Option<&NodePtrType> {
        match &self.node_type {
            NodeType::File { entry: _ } => None,
            NodeType::Directory { children } => {
                for c in children {
                    let lock = c.read().unwrap();
                    let node_name = lock.get_name();

                    if node_name == name {
                        return Some(c);
                    } else {
                        continue;
                    }
                }
                None
            }
        }
    }

    pub fn find_child_mut(&mut self, name: &str) -> Option<NodePtrType> {
        let c = self.find_child(name)?;
        let c2 = c.clone();
        Some(c2)
    }

    pub fn is_file(&self) -> bool {
        matches!(self.node_type, NodeType::File { entry: _ })
    }

    pub fn is_dir(&self) -> bool {
        matches!(self.node_type, NodeType::Directory { children: _ })
    }

}

pub struct FileTree {
    root: NodePtrType
}

impl FileTree {
    pub fn new() -> Self {
        let root = Node::new_ptr(
            Node::new_dir(".", "", Vec::new()) 
        );
        Self{ root }
    }

    pub fn create(nodes: Vec<Node>) -> Result<Self, DatError> {
        let this = Self::new();

        {
            let mut lock = this.root.write().unwrap();
            lock.add_children(nodes)?;
        };

        Ok(this)
    }

    fn get_path_parts(path: &str) -> VecDeque<String> {
        let p = path.replace("\\", "/");
        let mut parts: VecDeque<_> = p
            .split('/')
            .map(|s| s.to_string())
            .collect();


        if parts.len() > 0 && parts[0] == "." {
            parts.pop_front();
        }

        parts
    }

    pub fn get(&self, path: &str) -> Option<NodePtrType> {
        if path == "." || path == "./" {
            return Some(self.root.clone())
        }
        let parts = Self::get_path_parts(path);

        let mut node = self.root.clone();
        for p in parts {
            node = {
                let lock = node.read().unwrap();
                let n = lock.find_child(&p);
                n?.clone()
            };
        }

        Some(node)
    }

    
    pub fn insert(&mut self, path: &str, entry: FileEntry) -> Result<NodePtrType, DatError> {
        let parts = Self::get_path_parts(path);
        let parts_count = parts.len();

        let mut node_path = ".".to_string();

        let mut node = self.root.clone();
        for (i, p) in parts.into_iter().enumerate() {
            node_path += &format!("/{}", p);

            let n = {
                let lock = node.read().unwrap();
                let n = lock.find_child(&p);

                if let Some(n) = n {
                    //if node exists
                    //do nothing
                    n.clone()
                } else {
                    let last_elem = i == parts_count - 1;
                    let new_node = if last_elem {
                        Node::new_file(&p, &node_path, entry.clone())
                    } else {
                        Node::new_dir(&p, &node_path, Vec::new())
                    };

                    //drop lock to avoid deadlock :)
                    drop(lock);
                    node.write().unwrap().add_child(new_node)?
                }
            };

            node = n.clone();
        }

        Ok(node)
    }

    pub fn insert_unsorted(&mut self, path: &str, entry: FileEntry) -> Result<NodePtrType, DatError> {
        let parts = Self::get_path_parts(path);
        let parts_count = parts.len();

        let mut node_path = ".".to_string();

        let mut node = self.root.clone();
        for (i, p) in parts.into_iter().enumerate() {
            node_path += &format!("/{}", p);

            let n = {
                let lock = node.read().unwrap();
                let n = lock.find_child_linear(&p);

                if let Some(n) = n {
                    //if node exists
                    //do nothing
                    n.clone()
                } else {
                    let last_elem = i == parts_count - 1;
                    let new_node = if last_elem {
                        Node::new_file(&p, &node_path, entry.clone())
                    } else {
                        Node::new_dir_unsorted(&p, &node_path, Vec::new())
                    };

                    //drop lock to avoid deadlock :)
                    drop(lock);
                    node.write().unwrap().add_child_unsorted(new_node)?
                }
            };

            node = n.clone();
        }

        Ok(node)
    }

    pub fn sort(&mut self) -> Result<(), DatError> {
        sort_nodes(self.root.clone())
    }
}

fn sort_nodes(node: NodePtrType) -> Result<(), DatError> {
    let mut lock = node.try_write().map_err(|_| DatError::TreeError)?;
    
    match &mut lock.node_type {
        NodeType::Directory { children } => {
            Node::sort_children(children);
            for n in children {
                sort_nodes(n.clone())?;
            }
        },
        _ => {  }
    };

    Ok(())
}

impl<'a> IntoIterator for &'a FileTree {
    type Item = Node;
    type IntoIter = FileTreeIterator;

    fn into_iter(self) -> Self::IntoIter {
        FileTreeIterator {
            nodes: get_file_nodes(self.root.clone()),
        }
    }
}

#[derive(Default)]
pub struct FileTreeIterator {
    nodes: VecDeque<Node>,
}

fn get_file_nodes(root: NodePtrType) -> VecDeque<Node> {
    let mut out = VecDeque::<Node>::new();
    let lock = root.read().unwrap();

    match &lock.node_type {
        NodeType::File { entry: _ } => { out.push_back(lock.clone()) },
        NodeType::Directory { children } => {
            for n in children {
                let nodes = get_file_nodes(n.clone());
                for node in nodes {
                    out.push_back(node);
                }
            }
        },
    };

    out
}

impl Iterator for FileTreeIterator {
    type Item = Node;
    fn next(&mut self) -> Option<Self::Item> {
        self.nodes.pop_front()
    }
}
