//! The session store of files (SF-1 to SF-9): each session is the file `sess_<id>` of one directory, whose bytes are the
//! record of `polyspec-sessions-core` (SE-6) and which a request holds under `flock(LOCK_EX)` from its opening to its
//! closing. The rules are in `docs/spec/sessions.md`.
#![forbid(unsafe_code)]

mod error;
mod store;
mod turns;

pub use store::{FileSession, FileSessions, DEFAULT_LIFETIME};

#[cfg(test)]
mod tests {
    // B-6 and B-5b: a binding wraps these types.
    const _: () = assert!(std::mem::align_of::<super::FileSessions>() <= 16);
    const _: () = assert!(std::mem::align_of::<super::FileSession>() <= 16);
    const fn send_sync<T: Send + Sync>() {}
    const _: () = send_sync::<super::FileSessions>();
    const _: () = send_sync::<super::FileSession>();
}
