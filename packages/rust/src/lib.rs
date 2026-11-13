//! The session stores of polyspec as one library (SE-1.3): the store interface, identifiers and the record of
//! `polyspec-session-store-core`, the store of files of `polyspec-session-store-file` (feature `file`) and the store of Redis of
//! `polyspec-session-store-redis` (feature `redis`). The interface contract of the bindings, `interface.json`, names the items
//! of this crate. The rules are in `docs/spec/sessions.md`.
#![forbid(unsafe_code)]

/// The fault injection points of the library (SE-9.6), which the interface contract names in `faults`.
#[doc(hidden)]
pub use polyspec_session_store_core::fault;
pub use polyspec_session_store_core::{
    accept, key_of, new_identifier, random_hex, Error, Failure, Found, IoKind, MemoryStore, Outcome, RecordSession,
    Store,
};
#[cfg(feature = "file")]
pub use polyspec_session_store_file::{FileSession, FileSessions, DEFAULT_LIFETIME};
#[cfg(feature = "redis")]
pub use polyspec_session_store_redis::{protocol, Options, RedisSession, RedisSessions, Setting, StoreOptions};
