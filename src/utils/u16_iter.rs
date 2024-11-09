use std::iter::FusedIterator;

pub struct U16Iter<I> {
    pub(crate) iter: I,
}

impl<I: Iterator<Item = u8>> U16Iter<I> {
    pub(crate) fn new(iter: impl IntoIterator<Item = u8, IntoIter = I>) -> U16Iter<I> {
        U16Iter::_new(iter.into_iter())
    }

    pub(crate) fn _new(iter: I) -> U16Iter<I> {
        U16Iter { iter }
    }
}

impl<R: Iterator<Item = u8>> Iterator for U16Iter<R> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        let low = self.iter.next()?;
        let high = self.iter.next()?;
        Some(u16::from_le_bytes([low, high]))
    }
}

pub trait IntoU16Iter<T> {
    fn into_u16_iter(self) -> U16Iter<T>;
}

impl<T: Iterator<Item = u8>> IntoU16Iter<T> for T {
    fn into_u16_iter(self) -> U16Iter<T> {
        U16Iter::new(self)
    }
}

impl<R: FusedIterator<Item = u8>> FusedIterator for U16Iter<R> {}
