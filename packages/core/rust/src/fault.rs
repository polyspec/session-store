//! The fault injection points of the library (SE-9.6, binding design G-5, X-1). Each constant names one place where a
//! failure that no input reaches is imitated, as `session_store.<id>`; the interface contract names the same points in
//! `faults`. The crates of the stores call the points of their places through this module. A point does nothing
//! without the feature `fault-test`, which only tests turn on.

use binding_core::fault::{point, Injected};

/// The random source of the operating system, in `random_hex` (SE-5.4).
#[doc(hidden)]
pub const RANDOM: &str = "session_store.random";
/// The point of the kind `internal` (X-1), at two invariant branches that no input reaches: the count of renewals of a
/// memory store, which passes u64 when it is armed as `broken` (SE-4.3), and the state of a record session that its
/// start left waiting, in the start of `get`, `set`, `remove` and `renew` of a record session (SE-7.2). The second is on
/// the path of every caller of a session of a request, so a contract that passes on the kinds of this library arms it
/// for a member of its own.
#[doc(hidden)]
pub const INTERNAL: &str = "session_store.internal";
/// The open of the session file, in `FileSessions::open` (SF-3, SF-9.2).
#[doc(hidden)]
pub const OPEN: &str = "session_store.open";
/// The exclusive creation of the file of a new session, in `close` of a file session (SF-4.4, SF-9.2): an errno
/// `EEXIST` of the fault is an identifier that another process took.
#[doc(hidden)]
pub const CREATE: &str = "session_store.create";
/// The read of the session directory, in `FileSessions::collect` (SF-5, SF-9.2).
#[doc(hidden)]
pub const READ_DIR: &str = "session_store.read_dir";
/// The array of a decoded record, which the root names, at the start of a record session that found a record (SE-6.3).
#[doc(hidden)]
pub const RECORD_ROOT: &str = "session_store.record_root";
/// The entry that the index of the values names, in `get` of a record session (SE-6.5).
#[doc(hidden)]
pub const RECORD_INDEX: &str = "session_store.record_index";
/// The value of a read element that is a reference, in `get` of a record session (SE-6.5).
#[doc(hidden)]
pub const VALUE_REFERENCE: &str = "session_store.value_reference";
/// The top value of the copy that `get` of a record session gives (SE-6.5).
#[doc(hidden)]
pub const VALUE_ROOT: &str = "session_store.value_root";
/// A new reference of a copy, made for a reference of the source (SE-6.5 to SE-6.7).
#[doc(hidden)]
pub const COPY_NEW_REFERENCE: &str = "session_store.copy_new_reference";
/// The array of the source that a copy fills from (SE-6.5 to SE-6.7).
#[doc(hidden)]
pub const COPY_ARRAY: &str = "session_store.copy_array";
/// An element placed into the array of a copy (SE-6.5 to SE-6.7).
#[doc(hidden)]
pub const COPY_ELEMENT: &str = "session_store.copy_element";
/// The object of the source that a copy fills from (SE-6.5 to SE-6.7).
#[doc(hidden)]
pub const COPY_OBJECT: &str = "session_store.copy_object";
/// A property placed into the object of a copy (SE-6.5 to SE-6.7).
#[doc(hidden)]
pub const COPY_PROPERTY: &str = "session_store.copy_property";
/// The reference of the source that a copy fills from (SE-6.5 to SE-6.7).
#[doc(hidden)]
pub const COPY_REFERENCE: &str = "session_store.copy_reference";
/// The value placed into a reference of a copy (SE-6.5 to SE-6.7).
#[doc(hidden)]
pub const COPY_SET_REFERENCE: &str = "session_store.copy_set_reference";

/// The `io` fault armed at the point `name`: `Some` with the errno of the fault, which may name none, or `None` when no
/// `io` fault is armed there. No point of the library takes `short` or `broken` as an I/O failure.
#[doc(hidden)]
#[must_use]
pub fn failed(name: &'static str) -> Option<Option<i32>> {
    match point!(name)? {
        Injected::Io { errno } => Some(errno),
        Injected::Short { .. } | Injected::Broken => None,
    }
}

/// True when the invariant of the point `name` is armed as broken.
#[doc(hidden)]
#[must_use]
pub fn broken(name: &'static str) -> bool {
    matches!(point!(name), Some(Injected::Broken))
}

/// The kind `internal` with `what` when the invariant of the point `name` is armed as broken: a point at an invariant
/// branch that gives `internal` and that no input reaches (SE-9.6, X-1).
pub(crate) fn invariant(name: &'static str, what: &'static str) -> Result<(), crate::Error> {
    if broken(name) {
        Err(crate::Error::Internal { what })
    } else {
        Ok(())
    }
}
