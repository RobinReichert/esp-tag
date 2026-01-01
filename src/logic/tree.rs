use crate::logic::{
    arena::{Arena, SlotId},
    error::TreeError,
    node::Node,
};
use core::fmt::{self, Display, Formatter};
use core::{option::Option, result::Result};
use heapless::Vec;

const MAX_LEAFS: usize = 32;
const MAX_CHILD_LEAFS: usize = 8;
const MAX_PREFIX: usize = 32;

pub struct Tree {
    leafs: Arena<Leaf, MAX_LEAFS>,
    root_id: SlotId,
}

impl Tree {
    pub fn new() -> Result<Self, TreeError> {
        let root = Leaf::new_own();
        let mut leafs = Arena::new();
        let root_id = leafs.alloc(root).ok_or(TreeError::LeafAllocationError)?;
        Ok(Tree {
            leafs,
            root_id: root_id,
        })
    }

    pub fn upsert_edge(&mut self, from: Option<Node>, to: Node) -> Result<(), TreeError> {
        let leaf_id = match self.remove_node_helper(to, self.root_id) {
            Some(id) => id,
            None => self
                .leafs
                .alloc(Leaf::new_foreign(to))
                .ok_or(TreeError::LeafAllocationError)?,
        };
        self.insert_node_helper(from, self.root_id, leaf_id)
            .ok_or(TreeError::NodeNotFoundError)
    }

    fn remove_node_helper(&self, address: Node, current_id: SlotId) -> Option<SlotId> {
        let mut current = self.leafs.get(current_id).ok()?.borrow_mut();
        if let Some(pos) = current.get_nexts().iter().enumerate().find_map(|(idx, next_id)| {
            let next = self.leafs.get(*next_id).ok()?.borrow();
            match *next {
                Leaf::Own { nexts: _ } => None, 
                Leaf::Foreign { nexts: _, node } if node == address => Some(idx),
                Leaf::Foreign { nexts: _, node: _ } => None,
            }
        }) {
            return Some(current.get_nexts_mut().remove(pos));
        }
        if let Some(res) = current
            .get_nexts()
            .iter()
            .find_map(|next_id| self.remove_node_helper(address, *next_id))
        {
            return Some(res);
        }
        None
    }

    fn insert_node_helper(
        &self,
        parent_address: Option<Node>,
        current_id: SlotId,
        leaf_id: SlotId,
    ) -> Option<()> {
        let mut current = self.leafs.get(current_id).ok()?.borrow_mut();
        if current.get_node() == parent_address {
            current.get_nexts_mut().push(leaf_id).ok()?;
            return Some(());
        }
        if let Some(res) = current
            .get_nexts()
            .iter()
            .find_map(|next_id| self.insert_node_helper(parent_address, *next_id, leaf_id))
        {
            return Some(res);
        }
        None
    }

    pub fn next_hop(&self, destination: Node) -> Result<Node, TreeError> {
        self.next_hop_helper(destination, self.root_id)
    }

    fn next_hop_helper(&self, destination: Node, current_id: SlotId) -> Result<Node, TreeError> {
        let current = self
            .leafs
            .get(current_id)
            .map_err(|e| TreeError::LeafNotFoundError(e))?
            .borrow();
        match *current {
            Leaf::Own { nexts: _ } => (),
            Leaf::Foreign { nexts: _, node } if node == destination => return Ok(node),
            Leaf::Foreign { nexts: _, node: _ } => (),
        }
        if let Some(&ret_id) = current
            .get_nexts()
            .iter()
            .find(|&&next_id| self.next_hop_helper(destination, next_id).is_ok())
        {
            let ret = self
                .leafs
                .get(ret_id)
                .map_err(|e| TreeError::LeafNotFoundError(e))?
                .borrow();
            return Ok(ret.get_node().ok_or(TreeError::RootIsDestinationError)?);
        }
        Err(TreeError::NodeNotFoundError)
    }

    pub fn height(&self) -> usize {
        self.height_helper(self.root_id)
    }

    fn height_helper(&self, current_id: SlotId) -> usize {
        let current = match self.leafs.get(current_id).ok() {
            Some(cell) => cell.borrow(),
            None => return 0,
        };
        current
            .get_nexts()
            .iter()
            .map(|next_id| self.height_helper(*next_id))
            .max()
            .unwrap_or(0)
            + 1
    }

    fn fmt_leaf(
        &self,
        f: &mut Formatter<'_>,
        id: SlotId,
        prefixs: &[Option<Prefix>],
    ) -> fmt::Result {
        let mut depth = 0;
        for (d, prefix) in prefixs.iter().enumerate() {
            if let Some(p) = prefix {
                write!(f, "{}", p)?;
            } else {
                depth = d + 1;
                break;
            }
        }
        let current = self.leafs.get(id).map_err(|_| fmt::Error)?.borrow();
        match *current {
            Leaf::Own { nexts: _ } => write!(f, "self\n")?,
            Leaf::Foreign { nexts: _, node } => write!(f, "{}\n", node)?,
        }

        for (idx, next_id) in current.get_nexts().iter().enumerate() {
            let mut new_prefixs = [None; MAX_PREFIX];
            new_prefixs[..prefixs.len()].copy_from_slice(prefixs);
            for d in 0..depth {
                if let Some(prefix) = prefixs[d] {
                    new_prefixs[d] = Some(match prefix {
                        Prefix::Ellbow => Prefix::Space,
                        Prefix::Tee => Prefix::Pipe,
                        other => other,
                    });
                }
            }
            let last = idx == current.get_nexts().len() - 1;
            new_prefixs[depth - 1] = Some(if last { Prefix::Ellbow } else { Prefix::Tee });
            self.fmt_leaf(f, *next_id, &new_prefixs)?;
        }

        Ok(())
    }
}

impl Display for Tree {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let prefixs: [Option<Prefix>; MAX_PREFIX] = [(); MAX_PREFIX].map(|_| None);
        self.fmt_leaf(f, self.root_id, &prefixs)
    }
}

#[derive(Clone, Copy)]
enum Prefix {
    Space,
    Pipe,
    Tee,
    Ellbow,
}

impl Display for Prefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Prefix::Space => write!(f, "   "),
            Prefix::Pipe => write!(f, "│  "),
            Prefix::Tee => write!(f, "├──"),
            Prefix::Ellbow => write!(f, "└──"),
        }
    }
}

enum Leaf {
    Own {nexts: Vec<SlotId, MAX_CHILD_LEAFS>},
    Foreign {nexts: Vec<SlotId, MAX_CHILD_LEAFS>, node: Node},
}

impl Leaf {
    fn new_own() -> Self {
        Self::Own {
            nexts: Vec::new(),
        }
    }
    fn new_foreign(node: Node) -> Self {
        Self::Foreign {
            nexts: Vec::new(),
            node,
        }
    }

    fn get_nexts(&self) -> &Vec<SlotId, MAX_CHILD_LEAFS> {
        match self {
            Self::Own { nexts } => nexts,
            Self::Foreign { nexts, node: _ } => nexts,
        }
    }

    fn get_nexts_mut(&mut self) -> &mut Vec<SlotId, MAX_CHILD_LEAFS> {
        match self {
            Self::Own { nexts } => nexts,
            Self::Foreign { nexts, node: _ } => nexts,
        }
    }

    fn get_node(&self) -> Option<Node> {
        match self {
            Self::Own { nexts: _ } => None,
            Self::Foreign { nexts: _, node } => Some(node.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unwrap_print;

    fn n(mac_last_byte: u8) -> Node {
        let mut mac = [0u8; 6];
        mac[5] = mac_last_byte;
        Node::new(mac)
    }

    #[test]
    fn tree_creation() {
        let tree = unwrap_print!(Tree::new());

        assert_eq!(tree.height(), 1);
    }

    #[test]
    fn insert_single_edge() {
        let mut tree = unwrap_print!(Tree::new());

        unwrap_print!(tree.upsert_edge(None, n(1)));

        assert_eq!(tree.height(), 2);
        assert_eq!(unwrap_print!(tree.next_hop(n(1))), n(1));
    }

    #[test]
    fn insert_multiple_children() {
        let mut tree = unwrap_print!(Tree::new());

        unwrap_print!(tree.upsert_edge(None, n(1)));
        unwrap_print!(tree.upsert_edge(None, n(2)));
        unwrap_print!(tree.upsert_edge(None, n(3)));

        assert_eq!(tree.height(), 2);
        assert_eq!(unwrap_print!(tree.next_hop(n(1))), n(1));
        assert_eq!(unwrap_print!(tree.next_hop(n(2))), n(2));
        assert_eq!(unwrap_print!(tree.next_hop(n(3))), n(3));
    }

    #[test]
    fn insert_deep_tree() {
        let mut tree = unwrap_print!(Tree::new());

        unwrap_print!(tree.upsert_edge(None, n(1)));
        unwrap_print!(tree.upsert_edge(Some(n(1)), n(2)));
        unwrap_print!(tree.upsert_edge(Some(n(2)), n(3)));
        unwrap_print!(tree.upsert_edge(Some(n(3)), n(4)));

        assert_eq!(tree.height(), 5);

        assert_eq!(unwrap_print!(tree.next_hop(n(4))), n(1));
        assert_eq!(unwrap_print!(tree.next_hop(n(3))), n(1));
        assert_eq!(unwrap_print!(tree.next_hop(n(2))), n(1));
    }

    #[test]
    fn reparent_node_with_upsert() {
        let mut tree = unwrap_print!(Tree::new());

        unwrap_print!(tree.upsert_edge(None, n(1)));
        unwrap_print!(tree.upsert_edge(Some(n(1)), n(2)));

        unwrap_print!(tree.upsert_edge(None, n(2)));

        assert_eq!(tree.height(), 2);
        assert_eq!(unwrap_print!(tree.next_hop(n(2))), n(2));
    }

    #[test]
    fn next_hop_unknown_node() {
        let tree = unwrap_print!(Tree::new());

        let err = tree.next_hop(n(42)).unwrap_err();
        assert!(matches!(err, TreeError::NodeNotFoundError));
    }

    #[test]
    fn insert_under_unknown_parent() {
        let mut tree = unwrap_print!(Tree::new());

        let err = tree.upsert_edge(Some(n(99)), n(1)).unwrap_err();
        assert!(matches!(err, TreeError::NodeNotFoundError));
    }

    #[test]
    fn height_with_branching() {
        let mut tree = unwrap_print!(Tree::new());

        unwrap_print!(tree.upsert_edge(None, n(1)));
        unwrap_print!(tree.upsert_edge(Some(n(1)), n(2)));

        unwrap_print!(tree.upsert_edge(None, n(3)));
        unwrap_print!(tree.upsert_edge(Some(n(3)), n(4)));
        unwrap_print!(tree.upsert_edge(Some(n(4)), n(5)));

        assert_eq!(tree.height(), 4);
    }
}
