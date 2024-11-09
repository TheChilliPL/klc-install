use std::{
    cmp::{max, min},
    ops::{Add, Bound, Range, RangeBounds},
};

pub trait RangeBoundsExt<U> {
    fn is_empty(&self) -> bool
    where
        U: PartialOrd + Default;

    fn into_range(self, len: U) -> Range<U>
    where
        U: Ord + Default + Add<Output = U> + Copy,
        u8: Into<U>;
}

impl<U, T: RangeBounds<U>> RangeBoundsExt<U> for T {
    fn is_empty(&self) -> bool
    where
        U: PartialOrd + Default,
    {
        match (self.start_bound(), self.end_bound()) {
            (Bound::Included(start), Bound::Included(end)) => start > end,
            (Bound::Included(start), Bound::Excluded(end)) => start >= end,
            (Bound::Excluded(start), Bound::Included(end)) => start >= end,
            (Bound::Excluded(start), Bound::Excluded(end)) => start >= end,
            (Bound::Unbounded, Bound::Included(end)) => end < &U::default(),
            (Bound::Unbounded, Bound::Excluded(end)) => end <= &U::default(),
            (_, Bound::Unbounded) => false,
        }
    }

    fn into_range(self, len: U) -> Range<U>
    where
        U: Ord + Default + Add<Output = U> + Copy,
        u8: Into<U>,
    {
        if len <= U::default() {
            return U::default()..U::default();
        }

        let start = match self.start_bound() {
            Bound::Included(&start) => max(start, U::default()),
            Bound::Excluded(&start) => max(start + 1.into(), U::default()),
            Bound::Unbounded => U::default(),
        };

        let end = match self.end_bound() {
            Bound::Included(&end) => min(end + 1.into(), len),
            Bound::Excluded(&end) => min(end, len),
            Bound::Unbounded => len,
        };

        start..end
    }
}

#[cfg(test)]
mod test {
    use std::ops::RangeFull;

    use super::*;

    #[test]
    fn test_is_empty() {
        let full = ..;
        assert!(!<RangeFull as RangeBoundsExt<usize>>::is_empty(&full));
        assert!((..0).is_empty());
        assert!((1..0).is_empty());
        assert!((1..1).is_empty());
        assert!(!(1..2).is_empty());
        assert!(!(-10..).is_empty());
    }

    #[test]
    fn test_into_range() {
        assert_eq!(0..10, (..).into_range(10));
        assert_eq!(0..7, (..7).into_range(10));
        assert_eq!(0..8, (..=7).into_range(10));
        assert_eq!(3..10, (3..).into_range(10));
        assert_eq!(3..7, (3..7).into_range(10));
        assert_eq!(3..8, (3..=7).into_range(10));
    }
}
