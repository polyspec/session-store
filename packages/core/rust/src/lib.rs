//! The session store interface (SE-2, SE-3), the memory store (SE-4), session identifiers (SE-5), the record of a
//! session (SE-6) and the record session of one request (SE-7). The crate does no I/O (SE-1.4) and knows no meaning of
//! the keys and values of a session (SE-1.2). The rules are in `docs/spec/sessions.md`.
#![forbid(unsafe_code)]

mod copy;
mod error;
mod identifier;
mod memory;
mod record;
mod session;
mod store;

pub use error::{Error, IoKind};
pub use identifier::{accept, new_identifier, random_hex};
pub use memory::MemoryStore;
pub use session::{Found, Outcome, RecordSession};
pub use store::{key_of, Store};

/// The failure of a store operation: a kind of this library or the exception of a store of another language, which goes
/// back unchanged (SE-3.2, binding design E-1c).
pub type Failure = binding_core::Failure<Error>;

#[cfg(test)]
mod tests {
    // B-6: a binding wraps these types, so their alignment is at most 16 bytes.
    const _: () = assert!(std::mem::align_of::<super::MemoryStore>() <= 16);
    const _: () = assert!(std::mem::align_of::<super::RecordSession>() <= 16);

    // B-5b: the types of the core are Send and Sync.
    const fn send_sync<T: Send + Sync>() {}
    const _: () = send_sync::<super::MemoryStore>();
    const _: () = send_sync::<super::RecordSession>();
    const _: () = send_sync::<super::Error>();
}
