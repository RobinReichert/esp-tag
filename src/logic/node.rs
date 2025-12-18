use crate::logic::{
    error::CodecError,
    message::MESSAGE_SIZE,
    wire::{Cursor, WireCodec},
};
use core::cmp::PartialEq;
use core::fmt;

use heapless::Vec;

#[derive(Copy, Clone, Debug)]
pub struct Node {
    pub mac: [u8; 6],
}

impl Node {
    pub fn new(mac: [u8; 6]) -> Self {
        return Node { mac };
    }
}

impl WireCodec<MESSAGE_SIZE> for Node {
    const SIZE: usize = 6;

    fn encode(&self, out: &mut Vec<u8, MESSAGE_SIZE>) -> Result<(), CodecError> {
        out.extend_from_slice(&self.mac)
            .map_err(|e| CodecError::BufferCapacityError(e))
    }

    fn decode(cursor: &mut Cursor<'_>) -> Result<Self, CodecError> {
        let bytes = cursor.take(6).map_err(|e| CodecError::CursorReadError(e))?;
        let mut mac = [0u8; 6];
        mac.copy_from_slice(bytes);
        Ok(Node { mac })
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.mac.iter().zip(other.mac.iter()).all(|(a, b)| a == b)
    }
}

impl fmt::Display for Node {
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
