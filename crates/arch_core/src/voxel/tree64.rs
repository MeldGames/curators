use std::collections::VecDeque;
use std::fmt::{self, Display};

/// A 64-ary tree node with efficient memory layout
#[derive(Debug, Clone)]
pub struct Node<T> {
    pub data: T,
    children: Box<[Option<Box<Node<T>>>; 64]>,
    occupancy_mask: u64,
}

impl<T> Node<T> {
    /// Create a new node with the given data
    pub fn new(data: T) -> Self {
        Self { data, children: Box::new([None; 64]), child_count: 0 }
    }

    /// Add a child at the specified index (0-63)
    pub fn add_child(&mut self, index: usize, child: Node<T>) -> Result<(), &'static str> {
        if index >= 64 {
            return Err("Index out of bounds: must be 0-63");
        }

        self.occupancy_mask |= 1 << index;
        self.children[index] = Some(Box::new(child));
        Ok(())
    }

    /// Remove a child at the specified index
    pub fn remove_child(&mut self, index: usize) -> Option<Box<Node<T>>> {
        if index >= 64 {
            return None;
        }

        self.occupancy_mask &= !(1 << index);
        if let Some(child) = self.children[index].take() { Some(child) } else { None }
    }

    /// Get a reference to a child at the specified index
    pub fn get_child(&self, index: usize) -> Option<&Node<T>> {
        if index >= 64 { None } else { self.children[index].as_deref() }
    }

    /// Get a mutable reference to a child at the specified index
    pub fn get_child_mut(&mut self, index: usize) -> Option<&mut Node<T>> {
        if index >= 64 { None } else { self.children[index].as_deref_mut() }
    }

    /// Get the number of children
    pub fn child_count(&self) -> usize {
        self.child_count
    }

    /// Check if the node is a leaf (has no children)
    pub fn is_leaf(&self) -> bool {
        self.child_count == 0
    }

    /// Iterator over all non-None children with their indices
    pub fn children_with_indices(&self) -> impl Iterator<Item = (usize, &Node<T>)> {
        self.children
            .iter()
            .enumerate()
            .filter_map(|(i, child)| child.as_ref().map(|c| (i, c.as_ref())))
    }

    /// Iterator over all non-None children
    pub fn children(&self) -> impl Iterator<Item = &Node<T>> {
        self.children.iter().filter_map(|child| child.as_ref().map(|c| c.as_ref()))
    }

    /// Mutable iterator over all non-None children
    pub fn children_mut(&mut self) -> impl Iterator<Item = &mut Node<T>> {
        self.children.iter_mut().filter_map(|child| child.as_mut().map(|c| c.as_mut()))
    }
}

/// The main 64-ary tree structure
#[derive(Debug)]
pub struct Tree64<T> {
    root: Option<Box<Node<T>>>,
    size: usize,
}

impl<T> Tree64<T> {
    /// Create a new empty tree
    pub fn new() -> Self {
        Self { root: None, size: 0 }
    }

    /// Create a new tree with a root node
    pub fn with_root(data: T) -> Self {
        Self { root: Some(Box::new(Node::new(data))), size: 1 }
    }

    /// Grow by 1 layer
    pub fn grow() -> Self {
        self.root = Some()
    }

    /// Get a reference to the root node
    pub fn root(&self) -> Option<&Node<T>> {
        self.root.as_deref()
    }

    /// Get a mutable reference to the root node
    pub fn root_mut(&mut self) -> Option<&mut Node<T>> {
        self.root.as_deref_mut()
    }

    /// Set the root node
    pub fn set_root(&mut self, data: T) {
        self.root = Some(Box::new(Node::new(data)));
        self.size = 1;
    }

    /// Get the total number of nodes in the tree
    pub fn size(&self) -> usize {
        self.size
    }

    /// Check if the tree is empty
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Calculate the height of the tree
    pub fn height(&self) -> usize {
        fn height_helper<T>(node: Option<&Node<T>>) -> usize {
            match node {
                None => 0,
                Some(n) => {
                    let max_child_height =
                        n.children().map(|child| height_helper(Some(child))).max().unwrap_or(0);
                    1 + max_child_height
                },
            }
        }
        height_helper(self.root())
    }

    /// Depth-first search traversal (pre-order)
    pub fn dfs_preorder<F>(&self, mut visit: F)
    where
        F: FnMut(&T),
    {
        fn dfs_helper<T, F>(node: Option<&Node<T>>, visit: &mut F)
        where
            F: FnMut(&T),
        {
            if let Some(n) = node {
                visit(&n.data);
                for child in n.children() {
                    dfs_helper(Some(child), visit);
                }
            }
        }
        dfs_helper(self.root(), &mut visit);
    }

    /// Breadth-first search traversal
    pub fn bfs<F>(&self, mut visit: F)
    where
        F: FnMut(&T),
    {
        if let Some(root) = &self.root {
            let mut queue = VecDeque::new();
            queue.push_back(root.as_ref());

            while let Some(node) = queue.pop_front() {
                visit(&node.data);
                for child in node.children() {
                    queue.push_back(child);
                }
            }
        }
    }

    /// Find a node with the given predicate using DFS
    pub fn find<F>(&self, predicate: F) -> Option<&Node<T>>
    where
        F: Fn(&T) -> bool,
    {
        fn find_helper<T, F>(node: Option<&Node<T>>, predicate: &F) -> Option<&Node<T>>
        where
            F: Fn(&T) -> bool,
        {
            if let Some(n) = node {
                if predicate(&n.data) {
                    return Some(n);
                }
                for child in n.children() {
                    if let Some(found) = find_helper(Some(child), predicate) {
                        return Some(found);
                    }
                }
            }
            None
        }
        find_helper(self.root(), &predicate)
    }
}

impl<T: Default> Default for Tree64<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for Tree64<T> {
    fn clone(&self) -> Self {
        Self { root: self.root.clone() }
    }
}

impl<T: Display> Display for Tree64<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn print_node<T: Display>(
            f: &mut fmt::Formatter<'_>,
            node: Option<&Node<T>>,
            prefix: &str,
            is_last: bool,
        ) -> fmt::Result {
            if let Some(n) = node {
                writeln!(f, "{}{}", prefix, n.data)?;

                let children: Vec<_> = n.children_with_indices().collect();
                for (i, (idx, child)) in children.iter().enumerate() {
                    let is_last_child = i == children.len() - 1;
                    let new_prefix = if is_last {
                        format!("{}    ", prefix)
                    } else {
                        format!("{}│   ", prefix)
                    };
                    let child_prefix = if is_last_child { "└── " } else { "├── " };
                    write!(f, "{}{}[{}] ", new_prefix, child_prefix, idx)?;
                    print_node(f, Some(child), &new_prefix, is_last_child)?;
                }
            }
            Ok(())
        }

        if let Some(root) = &self.root {
            print_node(f, Some(root), "", true)
        } else {
            write!(f, "(empty tree)")
        }
    }
}
