
use crate::modules::wire::{WireCodec, Cursor};
use core::cmp::PartialEq;
use core::fmt;

use heapless::Vec;

pub const SIZE: usize = 6;


#[derive(Copy, Clone, Debug)]
pub struct Node {
    pub mac: [u8; 6]
}

impl Node {

    pub fn new(mac: [u8; 6]) -> Self {
        return Node { mac }
    }

}

impl WireCodec for Node {
    const SIZE: usize = 6;

    fn encode(&self, out: &mut Vec<u8, 256>) -> Result<(), &'static str> {
        out.extend_from_slice(&self.mac).map_err(|_| "Buffer full")
    }

    fn decode(cursor: &mut Cursor<'_>) -> Result<Self, &'static str> {
        let bytes = cursor.take(6)?;
        let mut mac = [0u8; 6];
        mac.copy_from_slice(bytes);
        Ok(Node { mac })
    }
}

impl PartialEq for Node {

    fn eq(&self, other: &Self) -> bool {
    self.mac.iter()
            .zip(other.mac.iter())
            .all(|(a, b)| a == b )
    }

}

impl fmt::Display for Node{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, byte) in self.mac.iter().enumerate() {
            if i != 0 {
                write!(f, ":")?;
            }
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}
