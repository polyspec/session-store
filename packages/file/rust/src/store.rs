//! The store of files and the session of one request (SF-1 to SF-5).

use std::fs::File;
use std::io::Read;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use binding_core::{Cancel, Io, ProcessWait};
use polyspec_php_serialization::PhpValue;
use polyspec_session_store_core::{accept, Failure, Found, Outcome, RecordSession, Store};
use rustix::fs::{FlockOperation, Mode, OFlags, Stat, Timespec, Timestamps};
use rustix::io::Errno;

use polyspec_session_store_core::Error;

use crate::error::{errno as errno_error, io as io_error};
use crate::turns::{Turn, Turns};

/// The lifetime of a session when the caller states none: `session.gc_maxlifetime` of PHP (SF-1.3).
pub const DEFAULT_LIFETIME: u64 = 1440;

/// The prefix of the name of a session file (SF-2.1).
const PREFIX: &str = "sess_";

/// The flags of every open of a session file (SF-2.4).
const OPEN: OFlags = OFlags::RDWR.union(OFlags::NOFOLLOW).union(OFlags::CLOEXEC);

#[derive(Debug)]
struct Shared {
    directory: PathBuf,
    lifetime: u64,
    turns: Arc<Turns>,
}

impl Shared {
    fn file(&self, id: &str) -> PathBuf {
        self.directory.join(format!("{PREFIX}{id}"))
    }

    /// Whether a file last modified at `stat` has ended (SF-2.3): more than the lifetime ago.
    fn ended(&self, stat: &Stat) -> bool {
        let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(now) => i128::from(now.as_secs()),
            // A clock before 1970 makes every file younger than now.
            Err(_) => return false,
        };
        now - i128::from(stat.st_mtime) > i128::from(self.lifetime)
    }
}

/// The store of files of one directory (SF-1).
#[derive(Debug, Clone)]
pub struct FileSessions {
    shared: Arc<Shared>,
}

/// The held file of a session: its descriptor, which holds the lock, and the turn of the request (SF-3.7).
#[derive(Debug)]
struct Held {
    file: File,
    #[allow(dead_code, reason = "SF-3.2: the turn ends when the held file is dropped")]
    turn: Turn,
}

/// The session of one request of the store of files: the record session (SE-7) and the lock that it holds.
#[derive(Debug)]
pub struct FileSession {
    shared: Arc<Shared>,
    record: RecordSession,
    held: Option<Held>,
}

/// Takes `flock` with an operation, again after an interruption by a signal.
fn flock(file: &File, operation: FlockOperation) -> Result<(), Errno> {
    loop {
        match rustix::fs::flock(file, operation) {
            Err(Errno::INTR) => {}
            other => return other,
        }
    }
}

/// Whether the path names the file that was locked (SF-3.5).
fn same_file(locked: &Stat, path: &Path) -> Result<bool, Error> {
    match rustix::fs::lstat(path) {
        Ok(current) => Ok(current.st_dev == locked.st_dev && current.st_ino == locked.st_ino),
        Err(Errno::NOENT) => Ok(false),
        Err(errno) => Err(errno_error("lstat", path, errno)),
    }
}

impl FileSessions {
    /// The store of an absolute existing directory with a lifetime in seconds, 1440 when it is `None` (SF-1.2, SF-1.3).
    pub fn new(_io: Io, directory: &Path, lifetime: Option<u64>) -> Result<Self, Error> {
        let invalid =
            |detail: &str| Error::InvalidDirectory { path: directory.to_path_buf(), detail: detail.to_owned() };
        if !directory.is_absolute() {
            return Err(invalid("the path is not absolute"));
        }
        match std::fs::metadata(directory) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => return Err(invalid("the path names no directory")),
            Err(error) => return Err(invalid(&error.to_string())),
        }
        let lifetime = match lifetime {
            None => DEFAULT_LIFETIME,
            Some(0) => return Err(Error::InvalidLifetime),
            Some(seconds) => seconds,
        };
        Ok(Self {
            shared: Arc::new(Shared {
                directory: directory.to_path_buf(),
                lifetime,
                turns: Arc::new(Turns::default()),
            }),
        })
    }

    /// The lifetime in seconds (SF-1.3).
    #[must_use]
    pub fn lifetime(&self) -> u64 {
        self.shared.lifetime
    }

    /// The directory (SF-1.2).
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.shared.directory
    }

    fn without_session(&self) -> FileSession {
        FileSession { shared: Arc::clone(&self.shared), record: RecordSession::new(None), held: None }
    }

    /// Opens the session of a request from the bytes of its session cookie (SF-3). It waits for the earlier requests
    /// of the session in this process and for the lock of the file, which another process may hold.
    pub fn open(
        &self,
        _wait: ProcessWait,
        cookie: Option<&[u8]>,
        cancel: Option<&Cancel>,
    ) -> Result<FileSession, Error> {
        let Some(id) = cookie.and_then(accept) else {
            return Ok(self.without_session());
        };
        let cancelled = || cancel.is_some_and(Cancel::is_cancelled);
        let turn = self.shared.turns.enter(&id, cancel)?;
        let path = self.shared.file(&id);
        let (file, locked) = loop {
            let file = match rustix::fs::open(&path, OPEN, Mode::empty()) {
                Ok(descriptor) => File::from(descriptor),
                Err(Errno::NOENT) => return Ok(self.without_session()),
                Err(errno) => return Err(errno_error("open", &path, errno)),
            };
            flock(&file, FlockOperation::LockExclusive).map_err(|errno| errno_error("flock", &path, errno))?;
            if cancelled() {
                return Err(Error::Cancelled);
            }
            let locked = rustix::fs::fstat(&file).map_err(|errno| errno_error("fstat", &path, errno))?;
            if same_file(&locked, &path)? {
                break (file, locked);
            }
        };
        if self.shared.ended(&locked) {
            return Ok(self.without_session());
        }
        let mut record = Vec::new();
        (&file).read_to_end(&mut record).map_err(|error| io_error("read", &path, &error))?;
        Ok(FileSession {
            shared: Arc::clone(&self.shared),
            record: RecordSession::new(Some(Found { id, record })),
            held: Some(Held { file, turn }),
        })
    }

    /// Removes the files of ended sessions whose lock no request holds, and gives how many it removed (SF-5).
    pub fn collect(&self, _io: Io) -> Result<u64, Error> {
        let directory = &self.shared.directory;
        let entries = std::fs::read_dir(directory).map_err(|error| io_error("read_dir", directory, &error))?;
        let mut removed: u64 = 0;
        for entry in entries {
            let entry = entry.map_err(|error| io_error("read_dir", directory, &error))?;
            let name = entry.file_name();
            let Some(id) =
                name.to_str().and_then(|name| name.strip_prefix(PREFIX)).and_then(|id| accept(id.as_bytes()))
            else {
                continue;
            };
            let path = self.shared.file(&id);
            let file = match rustix::fs::open(&path, OPEN, Mode::empty()) {
                Ok(descriptor) => File::from(descriptor),
                Err(Errno::NOENT) => continue,
                Err(errno) => return Err(errno_error("open", &path, errno)),
            };
            match flock(&file, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => {}
                Err(Errno::WOULDBLOCK) => continue,
                Err(errno) => return Err(errno_error("flock", &path, errno)),
            }
            let locked = rustix::fs::fstat(&file).map_err(|errno| errno_error("fstat", &path, errno))?;
            if !self.shared.ended(&locked) || !same_file(&locked, &path)? {
                continue;
            }
            rustix::fs::unlink(&path).map_err(|errno| errno_error("unlink", &path, errno))?;
            removed =
                removed.checked_add(1).ok_or(Error::Internal { what: "the count of removed sessions passes u64" })?;
        }
        Ok(removed)
    }
}

impl FileSession {
    /// The identifier of the existing session that the request found, or `None`.
    #[must_use]
    pub fn found(&self) -> Option<&str> {
        self.record.found()
    }

    /// Writes what the session gives (SF-4), releases the lock and the turn, also when a write fails (SF-4.5), and gives
    /// the new identifier, or `None`.
    pub fn close(self, _io: Io) -> Result<Option<String>, Error> {
        let Self { shared, record, held } = self;
        match record.finish()? {
            Outcome::Unused => Ok(None),
            Outcome::Unchanged { id } => {
                let Some(held) = held else {
                    return Err(Error::Internal { what: "an unchanged session holds no file" });
                };
                let now = Timespec { tv_sec: 0, tv_nsec: rustix::fs::UTIME_NOW };
                rustix::fs::futimens(&held.file, &Timestamps { last_access: now, last_modification: now })
                    .map_err(|errno| errno_error("futimens", shared.file(&id), errno))?;
                Ok(None)
            }
            Outcome::Write { id, record, new, removes } => {
                let path = shared.file(&id);
                if new {
                    create(&path, &id, &record)?;
                } else {
                    let Some(held) = &held else {
                        return Err(Error::Internal { what: "a found session holds no file" });
                    };
                    held.file.write_all_at(&record, 0).map_err(|error| io_error("write", &path, &error))?;
                    let length = u64::try_from(record.len())
                        .map_err(|_error| Error::Internal { what: "a record passes u64" })?;
                    held.file.set_len(length).map_err(|error| io_error("ftruncate", &path, &error))?;
                }
                if let Some(old) = removes {
                    let old = shared.file(&old);
                    rustix::fs::unlink(&old).map_err(|errno| errno_error("unlink", &old, errno))?;
                }
                drop(held);
                Ok(if new { Some(id) } else { None })
            }
        }
    }
}

/// Creates the file of a new identifier with the mode 0600 and writes its record (SF-4.4).
fn create(path: &Path, id: &str, record: &[u8]) -> Result<(), Error> {
    let flags = OFlags::WRONLY.union(OFlags::CREATE).union(OFlags::EXCL).union(OFlags::NOFOLLOW).union(OFlags::CLOEXEC);
    let file = match rustix::fs::open(path, flags, Mode::RUSR.union(Mode::WUSR)) {
        Ok(descriptor) => File::from(descriptor),
        Err(Errno::EXIST) => return Err(Error::IdentifierTaken { id: id.to_owned() }),
        Err(errno) => return Err(errno_error("open", path, errno)),
    };
    file.write_all_at(record, 0).map_err(|error| io_error("write", path, &error))
}

// B-10: a session that holds a lock has the cleanup call close.
impl binding_core::Cleanup for FileSession {}

impl Store for FileSession {
    fn get(&mut self, key: &str) -> Result<Option<PhpValue>, Failure> {
        self.record.get(key)
    }

    fn set(&mut self, key: &str, value: &PhpValue) -> Result<(), Failure> {
        self.record.set(key, value)
    }

    fn remove(&mut self, key: &str) -> Result<(), Failure> {
        self.record.remove(key)
    }

    fn renew(&mut self) -> Result<(), Failure> {
        self.record.renew()
    }
}
