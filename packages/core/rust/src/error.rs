//! The errors of the core (SE-9): every failure has a kind of the table of SE-9.2.

use std::fmt;

/// A failure of the core. A caller tells failures apart by its kind ([`bindgen_core::Kinds`]), never by the message,
/// apart from the message that SE-6.4 states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The random source of the operating system failed (SE-5.4).
    Random { detail: String },
    /// The record of a session cannot be decoded (SE-6.3); the source keeps its kind and offset.
    Record { id: String, source: polyspec_php_serialize::Error },
    /// The record of a session holds no array (SE-6.4).
    RecordNotArray { id: String },
    /// The values of a session have no record that php-serialize writes, such as a value nested deeper than its limit
    /// (SE-6.1); the source keeps its kind.
    RecordWrite { source: polyspec_php_serialize::Error },
    /// An invariant of the implementation fails; it is a defect.
    Internal { what: &'static str },
}

impl Error {
    /// The offset in the record of a failure of decoding, counted in bytes from 0 (SE-6.3, RQ-010), or `None`.
    #[must_use]
    pub const fn offset(&self) -> Option<u64> {
        match self {
            Self::Record { source, .. } | Self::RecordWrite { source } => source.offset(),
            Self::Random { .. } | Self::RecordNotArray { .. } | Self::Internal { .. } => None,
        }
    }
}

impl bindgen_core::Kinds for Error {
    fn kind(&self) -> &'static str {
        match self {
            Self::Random { .. } => "random",
            Self::Record { .. } => "record",
            Self::RecordNotArray { .. } => "record_not_array",
            Self::RecordWrite { .. } => "record_write",
            Self::Internal { .. } => "internal",
        }
    }

    // SE-9.3: a session that the server cannot read or write fails the request as an error of the server.
    fn status(&self) -> Option<u16> {
        Some(500)
    }

    fn errno(&self) -> Option<i32> {
        None
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
            Self::Internal { what } => write!(formatter, "internal error: {what}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Record { source, .. } | Self::RecordWrite { source } => Some(source),
            Self::Random { .. } | Self::RecordNotArray { .. } | Self::Internal { .. } => None,
        }
    }
}

/// The error of a broken invariant in place of an error of php-serialize that a valid value cannot give.
pub(crate) fn internal(what: &'static str) -> impl Fn(polyspec_php_serialize::Error) -> Error {
    move |_source| Error::Internal { what }
}
