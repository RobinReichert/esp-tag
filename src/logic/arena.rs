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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unwrap_print;

    #[test]
    fn test_alloc_and_get() {
        let mut arena: Arena<i32, 4> = Arena::new();

        let id = arena.alloc(42).expect("should allocate");
        assert!(id < 4);

        let val_ref = unwrap_print!(arena.get(id));
        assert_eq!(*val_ref.borrow(), 42);
    }

    #[test]
    fn test_remove() {
        let mut arena: Arena<String, 2> = Arena::new();

        let id1 = arena.alloc("hello".to_string()).expect("should allocate");
        let id2 = arena.alloc("world".to_string()).expect("should allocate");

        let val_cell = unwrap_print!(arena.remove(id1));
        assert_eq!(val_cell.into_inner(), "hello".to_string());

        assert!(arena.get(id1).is_err());

        let val_ref = unwrap_print!(arena.get(id2));
        assert_eq!(val_ref.borrow().as_str(), "world");
    }

    #[test]
    fn test_alloc_until_full() {
        let mut arena: Arena<i32, 2> = Arena::new();

        let id1 = arena.alloc(1);
        let id2 = arena.alloc(2);
        let id3 = arena.alloc(3);

        assert!(id1.is_some());
        assert!(id2.is_some());
        assert!(id3.is_none());
    }

    #[test]
    fn test_remove_invalid() {
        let mut arena: Arena<i32, 1> = Arena::new();

        let err = arena.remove(0).unwrap_err();
        match err {
            ArenaError::InvalidIndexError(id) => assert_eq!(id, 0),
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn test_get_empty_slot() {
        let arena: Arena<i32, 1> = Arena::new();

        let err = arena.get(0).unwrap_err();
        match err {
            ArenaError::SlotEmptyError(id) => assert_eq!(id, 0),
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    #[should_panic]
    fn test_get_out_of_bounds() {
        let arena: Arena<i32, 1> = Arena::new();
        let _ = arena.get(2).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_remove_out_of_bounds() {
        let mut arena: Arena<i32, 1> = Arena::new();
        let _ = arena.remove(2).unwrap();
    }
}
