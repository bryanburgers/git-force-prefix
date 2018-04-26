use std::io::{Error, ErrorKind};

pub struct Search {
    compare_len: usize,
    bytes: Vec<u8>,
    odd: Option<u8>,
}

impl Search {
    pub fn parse(s: &str) -> Result<Search, Error> {
        let mut i = 0;
        let bytes = s.as_bytes();
        let mut vec = Vec::new();
        while i < s.len() - 1 {
            let b1 = match bytes[i] {
                b'A'...b'F' => bytes[i] - b'A' + 10,
                b'a'...b'f' => bytes[i] - b'a' + 10,
                b'0'...b'9' => bytes[i] - b'0',
                _ => {
                    return Err(Error::new(ErrorKind::Other, "Digit was not a hex digit"));
                }
            };
            let b2 = match bytes[i + 1] {
                b'A'...b'F' => bytes[i + 1] - b'A' + 10,
                b'a'...b'f' => bytes[i + 1] - b'a' + 10,
                b'0'...b'9' => bytes[i + 1] - b'0',
                _ => {
                    return Err(Error::new(ErrorKind::Other, "Digit was not a hex digit"));
                }
            };
            let v = b1 << 4 | b2;
            vec.push(v);

            i += 2;
        }

        let odd = match s.len() % 2 {
            0 => None,
            1 => {
                let b = bytes[s.len() - 1];
                match b {
                    b'A'...b'F' => Some(b - b'A' + 10),
                    b'a'...b'f' => Some(b - b'a' + 10),
                    b'0'...b'9' => Some(b - b'0'),
                    _ => {
                        panic!("Invalid!");
                    }
                }
            }
            _ => unreachable!(),
        };

        // If an odd number of characters were specified, then we need to check the odd character in a
        // special way.
        Ok(Search {
            compare_len: vec.len(),
            bytes: vec,
            odd: odd,
        })
    }

    #[inline]
    pub fn test(&self, val: &[u8]) -> bool {
        if &val[0..self.compare_len] == &self.bytes[..] {
            match self.odd {
                Some(b) => val[self.compare_len] >> 4 == b,
                None => true,
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failed_parse() {
        let s = Search::parse("wtf");

        assert!(s.is_err());
    }

    #[test]
    fn test_succeeded_parse() {
        let s = Search::parse("0123456789abcdefABCDEF");

        assert!(s.is_ok());
    }

    #[test]
    fn test_success_even() {
        let s = Search::parse("0123").unwrap();

        assert!(s.test(&[0x01, 0x23, 0x45]));
    }

    #[test]
    fn test_failure_even() {
        let s = Search::parse("0123").unwrap();

        assert!(!s.test(&[0x01, 0x22, 0x45]));
    }

    #[test]
    fn test_success_odd() {
        let s = Search::parse("01234").unwrap();

        assert!(s.test(&[0x01, 0x23, 0x45]));
    }

    #[test]
    fn test_failure_odd() {
        let s = Search::parse("01234").unwrap();

        assert!(!s.test(&[0x01, 0x23, 0x55]));
    }
}
