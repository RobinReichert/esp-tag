use crate::logic::{
    error::{ArenaError, TreeError},
    node::Node,
};
use core::fmt::{self, Display, Formatter};
use core::{cell::RefCell, option::Option, result::Result};
use heapless::Vec;

pub type SlotId = u8;

const MAX_LEAFS: usize = 256;
const MAX_CHILD_LEAFS: usize = 8;
const MAX_PREFIX: usize = 256;

pub struct Tree {
    leafs: Arena<Leaf, MAX_LEAFS>,
    root_id: SlotId,
}

impl Tree {
    pub fn new(own: Node) -> Result<Self, TreeError> {
        let root = Leaf::new(own);
        let mut leafs = Arena::new();
        let root_id = leafs.alloc(root).ok_or(TreeError::LeafAllocationError)?;
        Ok(Tree {
            leafs,
            root_id: root_id,
        })
    }

    pub fn upsert_edge(&mut self, from: Node, to: Node) -> Result<(), TreeError> {
        let leaf_id = match self.remove_node_helper(to, self.root_id) {
            Some(id) => id,
            None => self
                .leafs
                .alloc(Leaf::new(to))
                .ok_or(TreeError::LeafAllocationError)?,
        };
        self.insert_node_helper(from, self.root_id, leaf_id)
            .ok_or(TreeError::NodeNotFoundError)
    }

    fn remove_node_helper(&self, address: Node, current_id: SlotId) -> Option<SlotId> {
        let mut current = self.leafs.get(current_id).ok()?.borrow_mut();
        if let Some(pos) = current.nexts.iter().enumerate().find_map(|(idx, next_id)| {
            let next = self.leafs.get(*next_id).ok()?.borrow();
            if next.node == address {
                Some(idx)
            } else {
                None
            }
        }) {
            return Some(current.nexts.remove(pos));
        }
        if let Some(res) = current
            .nexts
            .iter()
            .find_map(|next_id| self.remove_node_helper(address, *next_id))
        {
            return Some(res);
        }
        None
    }

    fn insert_node_helper(
        &self,
        parent_address: Node,
        current_id: SlotId,
        leaf_id: SlotId,
    ) -> Option<()> {
        let mut current = self.leafs.get(current_id).ok()?.borrow_mut();
        if current.node == parent_address {
            current.nexts.push(leaf_id).ok()?;
            return Some(());
        }
        if let Some(res) = current
            .nexts
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
        if current.node == destination {
            return Ok(current.node);
        }
        if let Some(&ret_id) = current
            .nexts
            .iter()
            .find(|&&next_id| self.next_hop_helper(destination, next_id).is_ok())
        {
            let ret = self
                .leafs
                .get(ret_id)
                .map_err(|e| TreeError::LeafNotFoundError(e))?
                .borrow();
            return Ok(ret.node);
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
            .nexts
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
        write!(f, "{}\n", current.node)?;

        for (idx, next_id) in current.nexts.iter().enumerate() {
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
            let last = idx == current.nexts.len() - 1;
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

struct Leaf {
    nexts: Vec<SlotId, MAX_CHILD_LEAFS>,
    node: Node,
}

impl Leaf {
    fn new(node: Node) -> Self {
        Leaf {
            nexts: Vec::new(),
            node,
        }
    }
}

struct Arena<T, const N: usize> {
    free: Vec<SlotId, N>,
    slots: [Option<RefCell<T>>; N],
}

impl<T, const N: usize> Arena<T, N> {
    fn new() -> Self {
        Arena {
            free: (0..N as SlotId).collect(),
            slots: [(); N].map(|_| None),
        }
    }

    fn alloc(&mut self, val: T) -> Option<SlotId> {
        let id = self.free.pop()?;
        self.slots[id as usize] = Some(RefCell::new(val));
        Some(id)
    }

    fn remove(&mut self, id: SlotId) -> Result<RefCell<T>, ArenaError> {
        let val = self.slots[id as usize]
            .take()
            .ok_or(ArenaError::InvalidIndexError(id))?;
        self.free.push(id).ok();
        Ok(val)
    }

    fn get(&self, id: SlotId) -> Result<&RefCell<T>, ArenaError> {
        self.slots[id as usize]
            .as_ref()
            .ok_or(ArenaError::SlotEmptyError(id))
    }
}
