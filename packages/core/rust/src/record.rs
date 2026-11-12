//! The record of a session (SE-6): the serialize form of one PHP array from the keys of the session to their values.

use std::collections::HashMap;

use polyspec_php_serialize::{serialize, unserialize, Key, Node, PhpValue};

use crate::copy::{value_of, Copier};
use crate::error::{internal, Error};

/// What a key of the record holds: the node of the record as it was read, or a value that the request set.
#[derive(Debug, Clone)]
enum Slot {
    Read(Node),
    Set(PhpValue),
}

/// The values of a session, read from its record and changed by the request.
#[derive(Debug, Clone)]
pub(crate) struct Values {
    /// The value that was read; its nodes are those of [`Slot::Read`].
    read: PhpValue,
    entries: Vec<(Key, Slot)>,
    index: HashMap<Key, usize>,
}

impl Values {
    /// The values of a session without values (SE-6.2).
    pub(crate) fn empty() -> Self {
        Self { read: PhpValue::new(), entries: Vec::new(), index: HashMap::new() }
    }

    /// Decodes the record of the session `id` (SE-6.1 to SE-6.4).
    pub(crate) fn decode(id: &str, record: &[u8]) -> Result<Self, Error> {
        if record.is_empty() {
            return Ok(Self::empty());
        }
        let read = unserialize(record).map_err(|source| Error::Record { id: id.to_owned(), source })?;
        let Node::Array(array) = read.root() else {
            return Err(Error::RecordNotArray { id: id.to_owned() });
        };
        let mut entries = Vec::new();
        let mut index = HashMap::new();
        for (key, node) in read.array(*array).map_err(internal("the root names no array"))?.iter() {
            index.insert(key.clone(), entries.len());
            entries.push((key.clone(), Slot::Read(node.clone())));
        }
        Ok(Self { read, entries, index })
    }

    /// The value of a key, or `None` (SE-2.1, SE-6.5).
    pub(crate) fn get(&self, key: &Key) -> Result<Option<PhpValue>, Error> {
        let Some(position) = self.index.get(key) else {
            return Ok(None);
        };
        match self.entries.get(*position) {
            Some((_, Slot::Read(node))) => value_of(&self.read, node).map(Some),
            Some((_, Slot::Set(value))) => Ok(Some(value.clone())),
            None => Err(Error::Internal { what: "an index names no entry" }),
        }
    }

    /// Sets a key in its place, or last for a new key (SE-2.2).
    pub(crate) fn set(&mut self, key: Key, value: &PhpValue) {
        if let Some(entry) = self.index.get(&key).and_then(|position| self.entries.get_mut(*position)) {
            entry.1 = Slot::Set(value.clone());
            return;
        }
        self.index.insert(key.clone(), self.entries.len());
        self.entries.push((key, Slot::Set(value.clone())));
    }

    /// Removes a key; a key that holds no value changes nothing (SE-2.3).
    pub(crate) fn remove(&mut self, key: &Key) {
        let Some(position) = self.index.remove(key) else {
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

    /// The record of the values (SE-6.1, SE-6.6, SE-6.7). The keys that the request did not set are copied by one
    /// copier, so the objects and references that they share stay shared; each set value is copied by its own.
    pub(crate) fn encode(&self) -> Result<Vec<u8>, Error> {
        let mut record = PhpValue::new();
        let array = record.new_array();
        record.set_root(Node::Array(array)).map_err(internal("a new array is refused as the top value"))?;
        let mut read = Copier::new(&self.read);
        for (key, slot) in &self.entries {
            let node = match slot {
                Slot::Read(node) => read.copy(&mut record, node)?,
                Slot::Set(value) => Copier::new(value).copy(&mut record, value.root())?,
            };
            record.set_element(array, key.clone(), node).map_err(internal("a copied value is refused"))?;
        }
        serialize(&record).map_err(|source| Error::RecordWrite { source })
    }
}

#[cfg(test)]
mod tests {
    use bindgen_core::Kinds;
    use polyspec_php_serialize::{Key, Node, PhpValue};

    use super::Values;

    /// A value of `depth` nested arrays.
    fn nested(depth: usize) -> PhpValue {
        let mut value = PhpValue::new();
        let mut outer = value.new_array();
        assert_eq!(value.set_root(Node::Array(outer)), Ok(()));
        for _ in 1..depth {
            let inner = value.new_array();
            assert_eq!(value.set_element(outer, Key::Int(0), Node::Array(inner)), Ok(()));
            outer = inner;
        }
        value
    }

    #[test]
    fn a_deep_value_is_copied_without_recursion() {
        // SE-6.5, SE-6.7: 4000 levels are read back through the copier.
        let mut values = Values::empty();
        values.set(Key::Int(1), &nested(4000));
        let Ok(record) = values.encode() else {
            panic!("a value of 4000 levels has a record");
        };
        let Ok(read) = Values::decode("e", &record) else {
            panic!("the record is read");
        };
        assert!(matches!(read.get(&Key::Int(1)), Ok(Some(_))));
    }

    #[test]
    fn a_value_deeper_than_php_reads_has_no_record() {
        // SE-6.1: the record adds one level to the value, and php-serialize writes no value deeper than its limit.
        let mut values = Values::empty();
        values.set(Key::Int(1), &nested(5000));
        match values.encode() {
            Err(error) => assert_eq!(error.kind(), "record_write"),
            Ok(_) => panic!("a value of 5000 levels has no record"),
        }
    }
}
