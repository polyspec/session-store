//! The failures of the file system as errors of the library (SF-9.2).

use std::path::PathBuf;

use polyspec_sessions_core::{Error, IoKind};

/// The kind of an `errno` (SF-9.2).
fn kind_of(errno: Option<i32>) -> IoKind {
    use rustix::io::Errno;
    let Some(raw) = errno else {
        return IoKind::Io;
    };
    let errno = Errno::from_raw_os_error(raw);
    if errno == Errno::NOENT {
        IoKind::NotFound
    } else if errno == Errno::ACCESS || errno == Errno::PERM {
        IoKind::PermissionDenied
    } else if errno == Errno::XDEV {
        IoKind::CrossDevice
    } else if errno == Errno::NOSPC || errno == Errno::DQUOT {
        IoKind::StorageFull
    } else {
        IoKind::Io
    }
}

/// The error of an operation of the file system on a path.
pub(crate) fn io(operation: &'static str, path: impl Into<PathBuf>, error: &std::io::Error) -> Error {
    let errno = error.raw_os_error();
    Error::Io { kind: kind_of(errno), operation, path: path.into(), errno, detail: error.to_string() }
}

/// The error of a call of rustix on a path.
pub(crate) fn errno(operation: &'static str, path: impl Into<PathBuf>, errno: rustix::io::Errno) -> Error {
    io(operation, path, &std::io::Error::from(errno))
}
