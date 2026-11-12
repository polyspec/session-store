//! The errors of the store of files (SF-9).

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
    /// The kind of an `errno` (SF-9.2).
    #[must_use]
    pub fn of(errno: Option<i32>) -> Self {
        use rustix::io::Errno;
        let Some(raw) = errno else {
            return Self::Io;
        };
        let errno = Errno::from_raw_os_error(raw);
        if errno == Errno::NOENT {
            Self::NotFound
        } else if errno == Errno::ACCESS || errno == Errno::PERM {
            Self::PermissionDenied
        } else if errno == Errno::XDEV {
            Self::CrossDevice
        } else if errno == Errno::NOSPC || errno == Errno::DQUOT {
            Self::StorageFull
        } else {
            Self::Io
        }
    }

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

/// A failure of the store of files. A caller tells failures apart by the kind ([`bindgen_core::Kinds`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The directory is no absolute path of an existing directory (SF-1.2).
    InvalidDirectory { path: PathBuf, detail: String },
    /// The lifetime is 0 (SF-1.3).
    InvalidLifetime,
    /// The caller gave up waiting (SF-3.8).
    Cancelled,
    /// A file already has the new identifier (SF-4.4).
    IdentifierTaken { id: String },
    /// An operation of the file system failed (SF-9.2).
    Io { kind: IoKind, operation: &'static str, path: PathBuf, errno: Option<i32>, detail: String },
    /// A failure of the core keeps its kind (SE-9.4).
    Core(polyspec_sessions_core::Error),
}

impl Error {
    /// The error of an operation of the file system on a path.
    pub(crate) fn io(operation: &'static str, path: impl Into<PathBuf>, error: &std::io::Error) -> Self {
        let errno = error.raw_os_error();
        Self::Io { kind: IoKind::of(errno), operation, path: path.into(), errno, detail: error.to_string() }
    }

    /// The error of a call of rustix on a path.
    pub(crate) fn errno(operation: &'static str, path: impl Into<PathBuf>, errno: rustix::io::Errno) -> Self {
        Self::io(operation, path, &std::io::Error::from(errno))
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
            Self::InvalidDirectory { .. } => "invalid_directory",
            Self::InvalidLifetime => "invalid_lifetime",
            Self::Cancelled => "cancelled",
            Self::IdentifierTaken { .. } => "identifier_taken",
            Self::Io { kind, .. } => kind.kind(),
            Self::Core(error) => error.kind(),
        }
    }

    // SF-9.3: the errors of the call that makes the store have no status; every other fails the request (SE-9.3).
    fn status(&self) -> Option<u16> {
        match self {
            Self::InvalidDirectory { .. } | Self::InvalidLifetime => None,
            Self::Cancelled | Self::IdentifierTaken { .. } | Self::Io { .. } => Some(500),
            Self::Core(error) => error.status(),
        }
    }

    fn errno(&self) -> Option<i32> {
        match self {
            Self::Io { errno, .. } => *errno,
            Self::InvalidDirectory { .. }
            | Self::InvalidLifetime
            | Self::Cancelled
            | Self::IdentifierTaken { .. }
            | Self::Core(_) => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDirectory { path, detail } => write!(
                formatter,
                "the session directory {} is no absolute path of an existing directory: {detail}",
                path.display()
            ),
            Self::InvalidLifetime => formatter.write_str("the session lifetime is no positive number of seconds"),
            Self::Cancelled => formatter.write_str("the caller gave up waiting for the session"),
            Self::IdentifierTaken { id } => write!(formatter, "a session file already has the new identifier {id}"),
            Self::Io { operation, path, detail, .. } => write!(formatter, "{operation} {}: {detail}", path.display()),
            Self::Core(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Core(error) => Some(error),
            Self::InvalidDirectory { .. }
            | Self::InvalidLifetime
            | Self::Cancelled
            | Self::IdentifierTaken { .. }
            | Self::Io { .. } => None,
        }
    }
}
