use crate::logic::error::ArenaError;
use core::{cell::RefCell, option::Option, result::Result};
use heapless::Vec;

pub type SlotId = u8;

pub struct Arena<T, const N: usize> {
    free: Vec<SlotId, N>,
    slots: [Option<RefCell<T>>; N],
}

impl<T, const N: usize> Arena<T, N> {
    pub fn new() -> Self {
        Arena {
            free: (0..N as SlotId).collect(),
            slots: [(); N].map(|_| None),
        }
    }

    pub fn alloc(&mut self, val: T) -> Option<SlotId> {
        let id = self.free.pop()?;
        self.slots[id as usize] = Some(RefCell::new(val));
        Some(id)
    }

    pub fn remove(&mut self, id: SlotId) -> Result<RefCell<T>, ArenaError> {
        let val = self.slots[id as usize]
            .take()
            .ok_or(ArenaError::InvalidIndexError(id))?;
        self.free.push(id).ok();
        Ok(val)
    }

    pub fn get(&self, id: SlotId) -> Result<&RefCell<T>, ArenaError> {
        self.slots[id as usize]
            .as_ref()
            .ok_or(ArenaError::SlotEmptyError(id))
    }
}
