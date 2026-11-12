//! The memory store (SE-4).

use std::collections::HashMap;

use polyspec_php_serialization::{Key, PhpValue};

use crate::error::Error;
use crate::store::{key_of, Store};
use crate::Failure;

/// The values of one session in the memory of one process (SE-4.1).
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    entries: Vec<(Key, PhpValue)>,
    index: HashMap<Key, usize>,
    renewals: u64,
}

impl MemoryStore {
    /// A store without values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How many times the session was renewed (SE-4.3).
    #[must_use]
    pub const fn renewals(&self) -> u64 {
        self.renewals
    }

    /// The value of a key (SE-2.1); `get` of [`Store`] gives the same value.
    #[must_use]
    pub fn value(&self, key: &str) -> Option<PhpValue> {
        let position = *self.index.get(&key_of(key))?;
        self.entries.get(position).map(|entry| entry.1.clone())
    }

    /// Sets a key (SE-2.2).
    pub fn put(&mut self, key: &str, value: &PhpValue) {
        let key = key_of(key);
        if let Some(entry) = self.index.get(&key).and_then(|position| self.entries.get_mut(*position)) {
            entry.1 = value.clone();
            return;
        }
        self.index.insert(key.clone(), self.entries.len());
        self.entries.push((key, value.clone()));
    }

    /// Removes a key; a key that holds no value changes nothing (SE-2.3).
    pub fn delete(&mut self, key: &str) {
        let Some(position) = self.index.remove(&key_of(key)) else {
            return;
        };
        if position < self.entries.len() {
            self.entries.remove(position);
        }
        for later in self.index.values_mut() {
            if *later > position {
                *later -= 1;
            }
        }
    }

    /// The keys in their order.
    pub fn keys(&self) -> impl ExactSizeIterator<Item = &Key> {
        self.entries.iter().map(|entry| &entry.0)
    }
}

impl Store for MemoryStore {
    fn get(&mut self, key: &str) -> Result<Option<PhpValue>, Failure> {
        Ok(self.value(key))
    }

    fn set(&mut self, key: &str, value: &PhpValue) -> Result<(), Failure> {
        self.put(key, value);
        Ok(())
    }

    fn remove(&mut self, key: &str) -> Result<(), Failure> {
        self.delete(key);
        Ok(())
    }

    fn renew(&mut self) -> Result<(), Failure> {
        self.renewals = self
            .renewals
            .checked_add(1)
            .ok_or(binding_core::Failure::Kind(Error::Internal { what: "the count of renewals passes u64" }))?;
        Ok(())
    }
}
