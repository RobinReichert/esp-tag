use heapless::Vec;

pub struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], &'static str> {
        if self.pos + n > self.buf.len() {
            return Err("Buffer underflow");
        }
        let slice = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    pub fn remaining(&self) -> &'a [u8] {
        &self.buf[self.pos..]
    }
}

pub trait WireCodec: Sized {
    const SIZE: usize;

    fn encode(&self, out: &mut Vec<u8, 256>) -> Result<(), &'static str>;
    fn decode(cursor: &mut Cursor<'_>) -> Result<Self, &'static str>;
}
