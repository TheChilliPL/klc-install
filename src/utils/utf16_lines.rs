use std::{
    fmt::Display,
    io::{self, BufRead},
};

use widestring::{Utf16Str, Utf16String};

use super::{AsU16Slice, StringExt};

#[derive(Debug)]
pub enum ReadUtf16LineError {
    Io(io::Error),
    Utf16(widestring::error::Utf16Error),
}

impl Display for ReadUtf16LineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadUtf16LineError::Io(e) => write!(f, "IO error: {}", e),
            ReadUtf16LineError::Utf16(e) => write!(f, "UTF-16 error: {}", e),
        }
    }
}

pub trait ReadUtf16Line {
    fn read_utf16_line(&mut self) -> Result<Utf16String, ReadUtf16LineError>;
    fn utf16_lines(self) -> Utf16Lines<Self>
    where
        Self: Sized;
}

impl<T: BufRead> ReadUtf16Line for T {
    fn read_utf16_line(&mut self) -> Result<Utf16String, ReadUtf16LineError> {
        let mut buf: Vec<u8> = Vec::new();
        let mut read = 0usize;
        'l: loop {
            let (done, used) = 'block: {
                let available = match self.fill_buf() {
                    Ok(n) => n,
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue 'l,
                    Err(e) => return Err(ReadUtf16LineError::Io(e)),
                };

                if available.is_empty() {
                    break 'block (true, 0);
                }

                // Index of the second byte
                let mut i = 1 - (read % 2);
                loop {
                    if i >= available.len() {
                        buf.extend_from_slice(available);
                        break (false, available.len());
                    }
                    let first_byte = if i == 0 {
                        *buf.last().unwrap()
                    } else {
                        available[i - 1]
                    };
                    let second_byte = available[i];

                    if first_byte == b'\n' && second_byte == b'\0' {
                        buf.extend_from_slice(&available[..=i]);
                        break (true, i + 1);
                    }

                    i += 2;
                }
            };
            self.consume(used);
            read += used;
            if done || used == 0 {
                break;
            }
        }

        Ok(Utf16Str::from_slice(buf.as_u16_slice())
            .map_err(|e| ReadUtf16LineError::Utf16(e))?
            .to_owned())
    }

    fn utf16_lines(self) -> Utf16Lines<Self> {
        Utf16Lines { reader: self }
    }
}

pub struct Utf16Lines<R> {
    reader: R,
}

impl<R: BufRead> Iterator for Utf16Lines<R> {
    type Item = Result<String, ReadUtf16LineError>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.reader.read_utf16_line();
        match line {
            Ok(line) => {
                if line.is_empty() {
                    None
                } else {
                    let mut string = line.to_string();
                    string.remove_prefix("\u{feff}");
                    string.remove_suffix("\n");
                    string.remove_suffix("\r");
                    Some(Ok(string))
                }
            }
            Err(e) => Some(Err(e)),
        }
    }
}
