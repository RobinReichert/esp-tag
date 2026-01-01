use heapless::Vec;
use crate::logic::error::{CodecError, CursorError};

pub struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], CursorError> {
        if self.pos + n > self.buf.len() {
            return Err(CursorError::BufferUnderflowError);
        }
        let slice = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    pub fn remaining(&self) -> &'a [u8] {
        &self.buf[self.pos..]
    }
}

pub trait WireCodec<const N: usize>: Sized {
    fn encode(&self, out: &mut Vec<u8, N>) -> Result<(), CodecError>;
    fn decode(cursor: &mut Cursor<'_>) -> Result<Self, CodecError>;
}

impl<T, const N: usize> WireCodec<N> for Option<T>
where
    T: WireCodec<N>,
{
    fn encode(&self, out: &mut Vec<u8, N>) -> Result<(), CodecError> {
        match self {
            None => {
                out.push(0).map_err(|e| CodecError::BufferOverflowError(e))?;
            }
            Some(value) => {
                out.push(1).map_err(|e| CodecError::BufferOverflowError(e))?;
                value.encode(out)?;
            }
        }
        Ok(())
    }

    fn decode(cursor: &mut Cursor<'_>) -> Result<Self, CodecError> {
        let flag = cursor
            .take(1)
            .map_err(|e| CodecError::CursorReadError(e))?[0];

        match flag {
            0 => Ok(None),
            1 => Ok(Some(T::decode(cursor)?)),
            x => Err(CodecError::InvalidOptionFlagError(x)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unwrap_print;

    #[test]
    fn test_cursor_take_success() {
        let data = &[1, 2, 3, 4, 5];
        let mut cursor = Cursor::new(data);

        let slice = unwrap_print!(cursor.take(3));
        assert_eq!(slice, &[1, 2, 3]);

        assert_eq!(cursor.remaining(), &[4, 5]);

        let slice2 = unwrap_print!(cursor.take(2));
        assert_eq!(slice2, &[4, 5]);

        assert!(cursor.remaining().is_empty());
    }

    #[test]
    fn test_cursor_take_underflow() {
        let data = &[1, 2];
        let mut cursor = Cursor::new(data);

        let err = cursor.take(3).unwrap_err();
        assert!(matches!(err, CursorError::BufferUnderflowError));

        assert_eq!(cursor.remaining(), &[1, 2]);

        let slice = unwrap_print!(cursor.take(2));
        assert_eq!(slice, &[1, 2]);
    }

    #[test]
    fn test_cursor_remaining_returns_correct_slice() {
        let data = &[10, 20, 30, 40];
        let mut cursor = Cursor::new(data);

        assert_eq!(cursor.remaining(), &[10, 20, 30, 40]);

        unwrap_print!(cursor.take(2));

        assert_eq!(cursor.remaining(), &[30, 40]);

        unwrap_print!(cursor.take(2));

        assert_eq!(cursor.remaining(), &[]);
    }
}
