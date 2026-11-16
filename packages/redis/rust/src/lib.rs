//! The session store of Redis (SR-1 to SR-9): each session is the key `<prefix><id>` with the lock `<prefix><id>_LOCK`
//! of the session handler of phpredis. The protocol ([`protocol`]) is a state machine without I/O; the connection
//! ([`RedisSessions`]) runs it with redis-rs on the tokio runtime of the process. The rules are in
//! `docs/spec/sessions.md`.
#![forbid(unsafe_code)]

mod connection;
pub mod protocol;
mod trust;
mod turns;

pub use connection::{RedisSession, RedisSessions, Setting};
pub use protocol::{Command, Options, Reply, Session, StoreOptions};

#[cfg(test)]
mod tests {
    // B-6 and B-5b: a binding wraps these types.
    const _: () = assert!(std::mem::align_of::<super::RedisSessions>() <= 16);
    const _: () = assert!(std::mem::align_of::<super::RedisSession>() <= 16);
    const fn send_sync<T: Send + Sync>() {}
    const _: () = send_sync::<super::RedisSessions>();
    const _: () = send_sync::<super::RedisSession>();
}
