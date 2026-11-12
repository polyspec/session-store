//! The errors of the store of Redis (SR-9).

use std::fmt;

/// A failure of the store of Redis. A caller tells failures apart by the kind ([`bindgen_core::Kinds`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The lifetime is 0 (SR-1.2).
    InvalidLifetime,
    /// The lock expiry is 0, or its milliseconds pass u64 (SR-1.2).
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
    /// A failure of the core keeps its kind (SE-9.4).
    Core(polyspec_sessions_core::Error),
}

impl Error {
    /// The offset of a failure of the record (SE-9.2), or `None`.
    #[must_use]
    pub const fn offset(&self) -> Option<u64> {
        match self {
            Self::Core(error) => error.offset(),
            Self::InvalidLifetime
            | Self::InvalidLockExpire
            | Self::InvalidSetting { .. }
            | Self::LockTimeout { .. }
            | Self::LockLost { .. }
            | Self::Reply { .. }
            | Self::Redis { .. }
            | Self::Connection { .. } => None,
        }
    }
}

impl From<polyspec_sessions_core::Error> for Error {
    fn from(error: polyspec_sessions_core::Error) -> Self {
        Self::Core(error)
    }
}

impl bindgen_core::Kinds for Error {
    fn kind(&self) -> &'static str {
        match self {
            Self::InvalidLifetime => "invalid_lifetime",
            Self::InvalidLockExpire => "invalid_lock_expire",
            Self::InvalidSetting { .. } => "invalid_setting",
            Self::LockTimeout { .. } => "lock_timeout",
            Self::LockLost { .. } => "lock_lost",
            Self::Reply { .. } => "reply",
            Self::Redis { .. } => "redis",
            Self::Connection { .. } => "connection",
            Self::Core(error) => error.kind(),
        }
    }

    fn status(&self) -> Option<u16> {
        match self {
            Self::InvalidLifetime | Self::InvalidLockExpire | Self::InvalidSetting { .. } => None,
            Self::LockTimeout { .. }
            | Self::LockLost { .. }
            | Self::Reply { .. }
            | Self::Redis { .. }
            | Self::Connection { .. } => Some(500),
            Self::Core(error) => error.status(),
        }
    }

    fn errno(&self) -> Option<i32> {
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLifetime => formatter.write_str("the session lifetime is no positive number of seconds"),
            Self::InvalidLockExpire => formatter.write_str("the lock expiry is no positive number of seconds"),
            Self::InvalidSetting { detail } => write!(formatter, "the connection setting is not valid: {detail}"),
            Self::LockTimeout { key } => write!(formatter, "the lock of the session {key} was not free"),
            Self::LockLost { key } => write!(formatter, "the lock of the session {key} expired"),
            Self::Reply { command, found } => write!(formatter, "the reply of {command} is {found}"),
            Self::Redis { command, message } => write!(formatter, "Redis refused {command}: {message}"),
            Self::Connection { detail } => write!(formatter, "the connection to Redis failed: {detail}"),
            Self::Core(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Core(error) => Some(error),
            Self::InvalidLifetime
            | Self::InvalidLockExpire
            | Self::InvalidSetting { .. }
            | Self::LockTimeout { .. }
            | Self::LockLost { .. }
            | Self::Reply { .. }
            | Self::Redis { .. }
            | Self::Connection { .. } => None,
        }
    }
}
