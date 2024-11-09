use std::{cmp::min, ops::RangeBounds};

use super::RangeBoundsExt;

pub trait StringExt {
    fn remove_byte_range(&mut self, range: impl RangeBounds<usize>);
    fn remove_prefix(&mut self, prefix: &str) -> bool;
    fn remove_suffix(&mut self, suffix: &str) -> bool;
}

impl StringExt for String {
    fn remove_byte_range(&mut self, range: impl RangeBounds<usize>) {
        let range = range.into_range(self.len());

        if range.is_empty() {
            return;
        }

        let (start, end) = (range.start, range.end);
        let amount = range.len();

        let old_len = self.len();
        let new_len = old_len - amount;

        unsafe {
            let bytes = self.as_bytes_mut();

            for i in start..new_len {
                bytes[i] = bytes[i + amount];
            }

            self.truncate(new_len);
            self.shrink_to_fit();
        }
    }

    fn remove_prefix(&mut self, prefix: &str) -> bool {
        if self.starts_with(prefix) {
            self.remove_byte_range(..prefix.len());
            true
        } else {
            false
        }
    }

    fn remove_suffix(&mut self, suffix: &str) -> bool {
        if self.ends_with(suffix) {
            self.remove_byte_range(self.len() - suffix.len()..);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod test {
    use crate::utils::StringExt;

    #[test]
    fn test_remove_byte_range() {
        let mut string = "Hello, world!".to_string();
        string.remove_byte_range(5..12);
        assert_eq!(string, "Hello!");
    }

    #[test]
    fn test_remove_prefix() {
        let mut string = "Hello, world!".to_string();
        assert!(!string.remove_prefix("world!"));
        assert_eq!(string, "Hello, world!");
        assert!(string.remove_prefix("Hello, "));
        assert_eq!(string, "world!");
    }

    #[test]
    fn test_remove_suffix() {
        let mut string = "Hello, world!".to_string();
        assert!(!string.remove_suffix("Hello, "));
        assert_eq!(string, "Hello, world!");
        assert!(string.remove_suffix(", world!"));
        assert_eq!(string, "Hello");
    }

    #[test]
    fn test_utf16() {
        let str = "\u{feff}KBD\tmultilin\t\"Multilingual\"\r\n";
        let mut string = str.to_string();
        assert!(string.remove_prefix("\u{feff}"));
        assert_eq!(string, "KBD\tmultilin\t\"Multilingual\"\r\n");
        assert!(string.remove_suffix("\n"));
        assert_eq!(string, "KBD\tmultilin\t\"Multilingual\"\r");
        assert!(string.remove_suffix("\r"));
        assert_eq!(string, "KBD\tmultilin\t\"Multilingual\"");
    }
}
