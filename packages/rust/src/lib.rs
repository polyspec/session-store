//! The session stores of polyspec as one library (SE-1.3): the store interface, identifiers and the record of
//! `polyspec-sessions-core`, the store of files of `polyspec-sessions-file` (feature `file`) and the store of Redis of
//! `polyspec-sessions-redis` (feature `redis`). The interface contract of the bindings, `interface.json`, names the items
//! of this crate. The rules are in `docs/spec/sessions.md`.
#![forbid(unsafe_code)]

pub use polyspec_sessions_core::{
    accept, key_of, new_identifier, random_hex, Error, Failure, Found, IoKind, MemoryStore, Outcome, RecordSession,
    Store,
};
#[cfg(feature = "file")]
pub use polyspec_sessions_file::{FileSession, FileSessions, DEFAULT_LIFETIME};
#[cfg(feature = "redis")]
pub use polyspec_sessions_redis::{protocol, Options, RedisSession, RedisSessions, Setting, StoreOptions};
