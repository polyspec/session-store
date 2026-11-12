//! The errors of the library (SE-9): one enum holds every failure of the core, the store of files and the store of
//! Redis, so that one kind interface and one table of classes hold them all (SE-9.4).

use std::fmt;
use std::path::PathBuf;

/// The kind of a failure of the file system, by its `errno` (SF-9.2, plan F-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoKind {
    NotFound,
    PermissionDenied,
    CrossDevice,
    StorageFull,
    Io,
}

impl IoKind {
    const fn kind(self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::PermissionDenied => "permission_denied",
            Self::CrossDevice => "cross_device",
            Self::StorageFull => "storage_full",
            Self::Io => "io",
        }
    }
}

/// A failure of the library. A caller tells failures apart by its kind ([`binding_core::Kinds`]), never by the
/// message, apart from the message that SE-6.4 states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The random source of the operating system failed (SE-5.4).
    Random { detail: String },
    /// The record of a session cannot be decoded (SE-6.3); the source keeps its kind and offset.
    Record { id: String, source: polyspec_php_serialization::Error },
    /// The record of a session holds no array (SE-6.4).
    RecordNotArray { id: String },
    /// The values of a session have no record that php-serialization writes (SE-6.8); the source keeps its kind.
    RecordWrite { source: polyspec_php_serialization::Error },
    /// The directory of the store of files is no absolute path of an existing directory (SF-1.2).
    InvalidDirectory { path: PathBuf, detail: String },
    /// The lifetime of a store is 0 (SF-1.3, SR-1.2).
    InvalidLifetime,
    /// The caller gave up waiting (SF-3.8).
    Cancelled,
    /// A file already has the new identifier (SF-4.4).
    IdentifierTaken { id: String },
    /// An operation of the file system failed (SF-9.2).
    Io { kind: IoKind, operation: &'static str, path: PathBuf, errno: Option<i32>, detail: String },
    /// The lock expiry of the store of Redis is 0, or its milliseconds pass u64 (SR-1.2).
    InvalidLockExpire,
    /// An address is no URL of Redis, or a list of addresses is empty (SR-6.1).
    InvalidSetting { detail: String },
    /// The lock stayed taken after every try (SR-3.3).
    LockTimeout { key: String },
    /// The lock expired before the write (SR-4.3).
    LockLost { key: String },
    /// A reply of another type than the command gives (SR-5.1).
    Reply { command: &'static str, found: &'static str },
    /// An error reply of Redis (SR-5.2).
    Redis { command: &'static str, message: String },
    /// The connection cannot be made or failed during a command (SR-6.4); the detail names no address or user.
    Connection { detail: String },
    /// An invariant of the implementation fails; it is a defect.
    Internal { what: &'static str },
}

impl Error {
    /// The offset in the record of a failure of decoding, counted in bytes from 0 (SE-9.2, RQ-010), or `None`.
    #[must_use]
    pub const fn offset(&self) -> Option<u64> {
        match self {
            Self::Record { source, .. } | Self::RecordWrite { source } => source.offset(),
            Self::Random { .. }
            | Self::RecordNotArray { .. }
            | Self::InvalidDirectory { .. }
            | Self::InvalidLifetime
            | Self::Cancelled
            | Self::IdentifierTaken { .. }
            | Self::Io { .. }
            | Self::InvalidLockExpire
            | Self::InvalidSetting { .. }
            | Self::LockTimeout { .. }
            | Self::LockLost { .. }
            | Self::Reply { .. }
            | Self::Redis { .. }
            | Self::Connection { .. }
            | Self::Internal { .. } => None,
        }
    }
}

impl binding_core::Kinds for Error {
    fn kind(&self) -> &'static str {
        match self {
            Self::Random { .. } => "random",
            Self::Record { .. } => "record",
            Self::RecordNotArray { .. } => "record_not_array",
            Self::RecordWrite { .. } => "record_write",
            Self::InvalidDirectory { .. } => "invalid_directory",
            Self::InvalidLifetime => "invalid_lifetime",
            Self::Cancelled => "cancelled",
            Self::IdentifierTaken { .. } => "identifier_taken",
            Self::Io { kind, .. } => kind.kind(),
            Self::InvalidLockExpire => "invalid_lock_expire",
            Self::InvalidSetting { .. } => "invalid_setting",
            Self::LockTimeout { .. } => "lock_timeout",
            Self::LockLost { .. } => "lock_lost",
            Self::Reply { .. } => "reply",
            Self::Redis { .. } => "redis",
            Self::Connection { .. } => "connection",
            Self::Internal { .. } => "internal",
        }
    }

    // SE-9.3, SF-9.3: the errors of the call that makes a store have no status; every other fails the request.
    fn status(&self) -> Option<u16> {
        match self {
            Self::InvalidDirectory { .. }
            | Self::InvalidLifetime
            | Self::InvalidLockExpire
            | Self::InvalidSetting { .. } => None,
            Self::Random { .. }
            | Self::Record { .. }
            | Self::RecordNotArray { .. }
            | Self::RecordWrite { .. }
            | Self::Cancelled
            | Self::IdentifierTaken { .. }
            | Self::Io { .. }
            | Self::LockTimeout { .. }
            | Self::LockLost { .. }
            | Self::Reply { .. }
            | Self::Redis { .. }
            | Self::Connection { .. }
            | Self::Internal { .. } => Some(500),
        }
    }

    fn errno(&self) -> Option<i32> {
        match self {
            Self::Io { errno, .. } => *errno,
            Self::Random { .. }
            | Self::Record { .. }
            | Self::RecordNotArray { .. }
            | Self::RecordWrite { .. }
            | Self::InvalidDirectory { .. }
            | Self::InvalidLifetime
            | Self::Cancelled
            | Self::IdentifierTaken { .. }
            | Self::InvalidLockExpire
            | Self::InvalidSetting { .. }
            | Self::LockTimeout { .. }
            | Self::LockLost { .. }
            | Self::Reply { .. }
            | Self::Redis { .. }
            | Self::Connection { .. }
            | Self::Internal { .. } => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Random { detail } => write!(formatter, "the random source of the operating system failed: {detail}"),
            Self::Record { id, source } => write!(formatter, "the record of the session {id} cannot be read: {source}"),
            // SE-6.4: the message is a rule.
            Self::RecordNotArray { id } => write!(formatter, "the session of {id} holds no array of values"),
            Self::RecordWrite { source } => write!(formatter, "the values of the session have no record: {source}"),
            Self::InvalidDirectory { path, detail } => write!(
                formatter,
                "the session directory {} is no absolute path of an existing directory: {detail}",
                path.display()
            ),
            Self::InvalidLifetime => formatter.write_str("the session lifetime is no positive number of seconds"),
            Self::Cancelled => formatter.write_str("the caller gave up waiting for the session"),
            Self::IdentifierTaken { id } => write!(formatter, "a session file already has the new identifier {id}"),
            Self::Io { operation, path, detail, .. } => write!(formatter, "{operation} {}: {detail}", path.display()),
            Self::InvalidLockExpire => formatter.write_str("the lock expiry is no positive number of seconds"),
            Self::InvalidSetting { detail } => write!(formatter, "the connection setting is not valid: {detail}"),
            Self::LockTimeout { key } => write!(formatter, "the lock of the session {key} was not free"),
            Self::LockLost { key } => write!(formatter, "the lock of the session {key} expired"),
            Self::Reply { command, found } => write!(formatter, "the reply of {command} is {found}"),
            Self::Redis { command, message } => write!(formatter, "Redis refused {command}: {message}"),
            Self::Connection { detail } => write!(formatter, "the connection to Redis failed: {detail}"),
            Self::Internal { what } => write!(formatter, "internal error: {what}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Record { source, .. } | Self::RecordWrite { source } => Some(source),
            Self::Random { .. }
            | Self::RecordNotArray { .. }
            | Self::InvalidDirectory { .. }
            | Self::InvalidLifetime
            | Self::Cancelled
            | Self::IdentifierTaken { .. }
            | Self::Io { .. }
            | Self::InvalidLockExpire
            | Self::InvalidSetting { .. }
            | Self::LockTimeout { .. }
            | Self::LockLost { .. }
            | Self::Reply { .. }
            | Self::Redis { .. }
            | Self::Connection { .. }
            | Self::Internal { .. } => None,
        }
    }
}

/// The error of a broken invariant in place of an error of php-serialization that a valid value cannot give.
pub(crate) fn internal(what: &'static str) -> impl Fn(polyspec_php_serialization::Error) -> Error {
    move |_source| Error::Internal { what }
}
