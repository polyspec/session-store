//! The store interface (SE-2, SE-3).

use polyspec_php_serialize::{Key, PhpValue};

use crate::Failure;

/// The store of the values of one session for one request (SE-2). A store of another language implements it through
/// its binding (SE-3.1); an exception of that store is [`bindgen_core::Failure::Raised`] and goes back to the caller
/// unchanged (SE-3.2).
pub trait Store {
    /// The value that a key holds, or `None` when it holds none (SE-2.1). The caller owns the value (SE-2.4).
    fn get(&mut self, key: &str) -> Result<Option<PhpValue>, Failure>;
    /// Makes the key hold a copy of the value: a key that holds a value keeps its place, a new key goes last (SE-2.2).
    fn set(&mut self, key: &str, value: &PhpValue) -> Result<(), Failure>;
    /// Makes the key hold no value; a key that holds none changes nothing (SE-2.3).
    fn remove(&mut self, key: &str) -> Result<(), Failure>;
    /// Moves the values to a new session identifier (SE-7.3); the memory store counts it (SE-4.3).
    fn renew(&mut self) -> Result<(), Failure>;
}

/// The PHP array key that a key names: a text in the canonical form of a 64-bit integer names that integer (SE-2.5,
/// RQ-004).
#[must_use]
pub fn key_of(key: &str) -> Key {
    Key::of_string(key.as_bytes().to_vec())
}
