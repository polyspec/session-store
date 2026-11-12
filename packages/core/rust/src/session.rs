//! The record session of one request (SE-7).

use polyspec_php_serialization::PhpValue;

use crate::error::Error;
use crate::identifier::new_identifier;
use crate::record::Values;
use crate::store::{key_of, Store};
use crate::Failure;

/// An existing session that a store found: its identifier and its record (SE-7.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub id: String,
    pub record: Vec<u8>,
}

/// What the store of a finished record session writes (SE-7.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The session never started: nothing is written.
    Unused,
    /// The record of the session is the record that was read: the bytes stay, and the session lives on.
    Unchanged { id: String },
    /// The record to write under the identifier; `new` when the request created the identifier, and `removes` the
    /// found identifier that a renewal leaves (SE-7.3).
    Write { id: String, record: Vec<u8>, new: bool, removes: Option<String> },
}

#[derive(Debug, Clone)]
enum State {
    Waiting,
    Started { values: Values, id: String, new: bool },
    Failed(Error),
}

/// The store of one request of a store that keeps records (SE-7.1). It does no I/O.
#[derive(Debug, Clone)]
pub struct RecordSession {
    found: Option<Found>,
    state: State,
}

impl RecordSession {
    /// The record session of a request whose store found an existing session, or none (SE-7.1).
    #[must_use]
    pub const fn new(found: Option<Found>) -> Self {
        Self { found, state: State::Waiting }
    }

    /// The identifier of the existing session that the store found.
    #[must_use]
    pub fn found(&self) -> Option<&str> {
        self.found.as_ref().map(|found| found.id.as_str())
    }

    /// Starts the session at its first use (SE-7.2).
    fn start(&mut self) -> Result<(&mut Values, &mut String, &mut bool), Error> {
        if let State::Waiting = self.state {
            self.state = match &self.found {
                Some(found) => match Values::decode(&found.id, &found.record) {
                    Ok(values) => State::Started { values, id: found.id.clone(), new: false },
                    Err(error) => State::Failed(error),
                },
                None => State::Started { values: Values::empty(), id: new_identifier()?, new: true },
            };
        }
        match &mut self.state {
            State::Started { values, id, new } => Ok((values, id, new)),
            State::Failed(error) => Err(error.clone()),
            State::Waiting => Err(Error::Internal { what: "a started session waits" }),
        }
    }

    /// Ends the record session and gives what its store writes (SE-7.4, SE-7.5).
    pub fn finish(self) -> Result<Outcome, Error> {
        let (values, id, new) = match self.state {
            State::Waiting | State::Failed(_) => return Ok(Outcome::Unused),
            State::Started { values, id, new } => (values, id, new),
        };
        let record = values.encode()?;
        let found = self.found;
        if !new && found.as_ref().is_some_and(|found| found.record == record) {
            return Ok(Outcome::Unchanged { id });
        }
        let removes = match found {
            Some(found) if new => Some(found.id),
            Some(_) | None => None,
        };
        Ok(Outcome::Write { id, record, new, removes })
    }
}

impl Store for RecordSession {
    fn get(&mut self, key: &str) -> Result<Option<PhpValue>, Failure> {
        let (values, _, _) = self.start().map_err(Failure::Kind)?;
        values.get(&key_of(key)).map_err(Failure::Kind)
    }

    fn set(&mut self, key: &str, value: &PhpValue) -> Result<(), Failure> {
        let (values, _, _) = self.start().map_err(Failure::Kind)?;
        values.set(key_of(key), value);
        Ok(())
    }

    fn remove(&mut self, key: &str) -> Result<(), Failure> {
        let (values, _, _) = self.start().map_err(Failure::Kind)?;
        values.remove(&key_of(key));
        Ok(())
    }

    /// Gives the session a new identifier (SE-7.3).
    fn renew(&mut self) -> Result<(), Failure> {
        let (_, id, new) = self.start().map_err(Failure::Kind)?;
        *id = new_identifier().map_err(Failure::Kind)?;
        *new = true;
        Ok(())
    }
}
