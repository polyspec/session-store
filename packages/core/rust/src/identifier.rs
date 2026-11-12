//! Session identifiers (SE-5).

use crate::error::Error;

/// The least and the greatest length of an identifier (SE-5.1).
const SHORTEST: usize = 22;
const LONGEST: usize = 128;

/// Gives the identifier of the bytes of a session cookie value, or `None` when the bytes are not an identifier, the
/// only reason of the absence (SE-5.1, SE-5.2).
#[must_use]
pub fn accept(cookie: &[u8]) -> Option<String> {
    let valid = (SHORTEST..=LONGEST).contains(&cookie.len())
        && cookie.iter().all(|byte| byte.is_ascii_alphanumeric() || *byte == b',' || *byte == b'-');
    if !valid {
        return None;
    }
    // Every byte is ASCII, so each byte is its character.
    Some(cookie.iter().map(|byte| char::from(*byte)).collect())
}

/// A new identifier: 32 bytes of the random source of the operating system as 64 lowercase hexadecimal digits (SE-5.3).
pub fn new_identifier() -> Result<String, Error> {
    random_hex(32)
}

/// `count` bytes of the random source of the operating system as lowercase hexadecimal digits; a failure of the
/// source is the kind `random` and no other source replaces it (SE-5.4).
pub fn random_hex(count: usize) -> Result<String, Error> {
    let mut bytes = vec![0_u8; count];
    getrandom::fill(&mut bytes).map_err(|error| Error::Random { detail: error.to_string() })?;
    let mut text = String::with_capacity(count * 2);
    for byte in bytes {
        text.push(DIGITS[usize::from(byte >> 4)]);
        text.push(DIGITS[usize::from(byte & 0x0f)]);
    }
    Ok(text)
}

const DIGITS: [char; 16] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'];

#[cfg(test)]
mod tests {
    use super::{accept, new_identifier};

    #[test]
    fn a_new_identifier_is_64_lowercase_hexadecimal_digits_and_accepted() {
        // SE-5.3
        let first = new_identifier();
        let second = new_identifier();
        let (Ok(first), Ok(second)) = (first, second) else {
            panic!("the random source failed");
        };
        assert_eq!(first.len(), 64);
        assert!(first.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        assert_ne!(first, second);
        assert_eq!(accept(first.as_bytes()), Some(first));
    }

    #[test]
    fn every_byte_value_is_judged() {
        // SE-5.1: each byte value at one place of an identifier of 22 characters.
        for byte in 0..=u8::MAX {
            let mut cookie = vec![b'a'; 22];
            cookie[10] = byte;
            let expected = byte.is_ascii_alphanumeric() || byte == b',' || byte == b'-';
            assert_eq!(accept(&cookie).is_some(), expected, "byte {byte:#04x}");
        }
    }
}
