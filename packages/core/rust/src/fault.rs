//! The fault injection points of the library (SE-9.6, binding design G-5, X-1). Each constant names one place where a
//! failure that no input reaches is imitated, as `session_store.<id>`; the interface contract names the same points in
//! `faults`. The crates of the stores call the points of their places through this module. A point does nothing
//! without the feature `fault-test`, which only tests turn on.

use binding_core::fault::{point, Injected};

/// The random source of the operating system, in `random_hex` (SE-5.4).
#[doc(hidden)]
pub const RANDOM: &str = "session_store.random";
/// The count of renewals of a memory store: armed as `broken`, it passes u64 (SE-4.3, X-1).
#[doc(hidden)]
pub const INTERNAL: &str = "session_store.internal";
/// The open of the session file, in `FileSessions::open` (SF-3, SF-9.2).
#[doc(hidden)]
pub const OPEN: &str = "session_store.open";
/// The read of the session directory, in `FileSessions::collect` (SF-5, SF-9.2).
#[doc(hidden)]
pub const READ_DIR: &str = "session_store.read_dir";
/// The `io` fault armed at the point `name`: `Some` with the errno of the fault, which may name none, or `None` when no
/// `io` fault is armed there. No point of the library takes `short` or `broken` as an I/O failure.
#[doc(hidden)]
#[must_use]
pub fn failed(name: &'static str) -> Option<Option<i32>> {
    match point(name)? {
        Injected::Io { errno } => Some(errno),
        Injected::Short { .. } | Injected::Broken => None,
    }
}

/// True when the invariant of the point `name` is armed as broken.
#[doc(hidden)]
#[must_use]
pub fn broken(name: &'static str) -> bool {
    matches!(point(name), Some(Injected::Broken))
}
