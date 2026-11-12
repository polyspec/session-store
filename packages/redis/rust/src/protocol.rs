//! The protocol of the store of Redis (SR-2 to SR-5): a state machine without I/O. It gives the command to send or the
//! time to wait, and takes the reply; the order of its commands is the interface with phpredis (SR-3.1).

use std::time::Duration;

use polyspec_php_serialization::PhpValue;
use polyspec_session_store_core::{accept, random_hex, Failure, Found, Outcome, RecordSession, Store};

use polyspec_session_store_core::Error;

/// The release script (SR-2.3).
pub const RELEASE: &str =
    r#"if redis.call("get",KEYS[1]) == ARGV[1] then return redis.call("del",KEYS[1]) else return 0 end"#;

/// The options that the caller states, each `None` for its default, and the lock expiry, which has none (SR-1.2): the
/// record of the interface contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreOptions {
    pub lifetime: Option<u64>,
    pub prefix: Option<String>,
    pub lock_expire: u64,
    pub lock_retries: Option<u64>,
    pub lock_wait: Option<u64>,
}

/// The options of the store, checked (SR-1.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    lifetime: u64,
    prefix: String,
    lock_expire: u64,
    lock_expire_milliseconds: u64,
    lock_retries: u64,
    lock_wait: u64,
}

impl Options {
    /// The default lifetime, `session.gc_maxlifetime` of PHP (SR-1.2).
    pub const LIFETIME: u64 = 1440;
    /// The default prefix of phpredis (SR-1.2).
    pub const PREFIX: &'static str = "PHPREDIS_SESSION:";
    /// The default number of further tries, `redis.session.lock_retries` of phpredis (SR-1.2).
    pub const LOCK_RETRIES: u64 = 100;
    /// The default microseconds between two tries, `redis.session.lock_wait_time` of phpredis (SR-1.2).
    pub const LOCK_WAIT: u64 = 20_000;

    /// The options, each the default when it is `None`; the lock expiry has none. A lifetime or a lock expiry of 0 fails
    /// (SR-1.2).
    pub fn new(
        lifetime: Option<u64>,
        prefix: Option<String>,
        lock_expire: u64,
        lock_retries: Option<u64>,
        lock_wait: Option<u64>,
    ) -> Result<Self, Error> {
        let lifetime = match lifetime {
            None => Self::LIFETIME,
            Some(0) => return Err(Error::InvalidLifetime),
            Some(seconds) => seconds,
        };
        // SR-1.2: the lock expiry has no default.
        if lock_expire == 0 {
            return Err(Error::InvalidLockExpire);
        }
        let lock_expire_milliseconds = lock_expire.checked_mul(1000).ok_or(Error::InvalidLockExpire)?;
        Ok(Self {
            lifetime,
            prefix: match prefix {
                None => Self::PREFIX.to_owned(),
                Some(prefix) => prefix,
            },
            lock_expire,
            lock_expire_milliseconds,
            lock_retries: match lock_retries {
                None => Self::LOCK_RETRIES,
                Some(retries) => retries,
            },
            lock_wait: match lock_wait {
                None => Self::LOCK_WAIT,
                Some(wait) => wait,
            },
        })
    }

    /// The options of a record of the interface contract (SR-1.2).
    pub fn of(options: &StoreOptions) -> Result<Self, Error> {
        Self::new(
            options.lifetime,
            options.prefix.clone(),
            options.lock_expire,
            options.lock_retries,
            options.lock_wait,
        )
    }

    #[must_use]
    pub const fn lifetime(&self) -> u64 {
        self.lifetime
    }

    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    #[must_use]
    pub const fn lock_expire(&self) -> u64 {
        self.lock_expire
    }

    #[must_use]
    pub const fn lock_retries(&self) -> u64 {
        self.lock_retries
    }

    #[must_use]
    pub const fn lock_wait(&self) -> u64 {
        self.lock_wait
    }

    /// The key of a session (SR-2.1).
    #[must_use]
    pub fn key(&self, id: &str) -> String {
        format!("{}{id}", self.prefix)
    }
}

/// A command: its name, which errors name, and its words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub name: &'static str,
    pub words: Vec<Vec<u8>>,
}

impl Command {
    fn new(name: &'static str, words: &[&[u8]]) -> Self {
        let mut all = vec![name.as_bytes().to_vec()];
        all.extend(words.iter().map(|word| word.to_vec()));
        Self { name, words: all }
    }
}

/// A reply of Redis, by its type (SR-5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Nil,
    Integer(i64),
    /// A bulk string: bytes, never text (SR-5.1).
    Bulk(Vec<u8>),
    /// A simple string, such as `OK`.
    Status(String),
    /// An error reply (SR-5.2).
    Error(String),
    /// A reply of another type, by its name.
    Other(&'static str),
}

impl Reply {
    const fn name(&self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Integer(_) => "an integer",
            Self::Bulk(_) => "a bulk string",
            Self::Status(_) => "a simple string",
            Self::Error(_) => "an error",
            Self::Other(name) => name,
        }
    }
}

/// The lock that a request holds (SR-2).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Lock {
    key: String,
    secret: String,
}

impl Lock {
    fn release(&self) -> Command {
        Command::new("EVAL", &[RELEASE.as_bytes(), b"1", self.key.as_bytes(), self.secret.as_bytes()])
    }
}

/// The state of the session of one request in the protocol: the record session (SE-7) and the lock that it holds.
#[derive(Debug)]
pub struct Session {
    record: RecordSession,
    lock: Option<Lock>,
}

impl Session {
    /// The identifier of the existing session that the request found, or `None`.
    #[must_use]
    pub fn found(&self) -> Option<&str> {
        self.record.found()
    }

    fn without_session() -> Self {
        Self { record: RecordSession::new(None), lock: None }
    }
}

impl Store for Session {
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

/// What the protocol asks for next.
#[derive(Debug)]
pub enum Step<T> {
    /// Send the command and give its reply.
    Send(Command),
    /// Wait for the duration and then ask again without a reply.
    Wait(Duration),
    /// The protocol ended.
    Done(T),
}

fn unexpected(command: &'static str, reply: &Reply) -> Error {
    match reply {
        Reply::Error(message) => Error::Redis { command, message: message.clone() },
        other => Error::Reply { command, found: other.name() },
    }
}

/// The step of a release whose reply ends the protocol with `then`. An error reply of the release fails a protocol that
/// would succeed; a failure that came before it stays the failure.
#[derive(Debug)]
enum After<T> {
    Ok(T),
    Err(Error),
}

/// The opening of the session of a request (SR-3.2 to SR-3.4).
#[derive(Debug)]
pub struct Opening {
    options: Options,
    id: String,
    key: String,
    lock: Lock,
    tries: u64,
    state: OpeningState,
}

#[derive(Debug)]
enum OpeningState {
    Start,
    Exists,
    Lock,
    Waited,
    Get,
    Release(Box<After<Session>>),
    Ended,
}

impl Opening {
    /// The opening of the session that the bytes of a session cookie name; bytes that are not an identifier give the
    /// session of a request without an existing session at once (SR-3.2).
    pub fn new(options: &Options, cookie: Option<&[u8]>) -> Result<Result<Self, Session>, Error> {
        let Some(id) = cookie.and_then(accept) else {
            return Ok(Err(Session::without_session()));
        };
        let key = options.key(&id);
        let lock = Lock { key: format!("{key}_LOCK"), secret: random_hex(16)? };
        Ok(Ok(Self { options: options.clone(), id, key, lock, tries: 0, state: OpeningState::Start }))
    }

    /// The identifier that the cookie names, whose turn the caller holds (SR-3.2).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    fn set_lock(&self) -> Command {
        let milliseconds = self.options.lock_expire_milliseconds.to_string();
        Command::new(
            "SET",
            &[self.lock.key.as_bytes(), self.lock.secret.as_bytes(), b"NX", b"PX", milliseconds.as_bytes()],
        )
    }

    fn release(&mut self, then: After<Session>) -> Step<Session> {
        self.state = OpeningState::Release(Box::new(then));
        Step::Send(self.lock.release())
    }

    /// The next step, given the reply of the command of the last step (`None` at the start and after a wait).
    pub fn next(&mut self, reply: Option<Reply>) -> Result<Step<Session>, Error> {
        let state = std::mem::replace(&mut self.state, OpeningState::Ended);
        match (state, reply) {
            (OpeningState::Start, None) => {
                self.state = OpeningState::Exists;
                Ok(Step::Send(Command::new("EXISTS", &[self.key.as_bytes()])))
            }
            (OpeningState::Exists, Some(Reply::Integer(0))) => Ok(Step::Done(Session::without_session())),
            (OpeningState::Exists, Some(Reply::Integer(_))) | (OpeningState::Waited, None) => {
                self.state = OpeningState::Lock;
                Ok(Step::Send(self.set_lock()))
            }
            (OpeningState::Exists, Some(reply)) => Err(unexpected("EXISTS", &reply)),
            (OpeningState::Lock, Some(Reply::Status(status))) if status == "OK" => {
                self.state = OpeningState::Get;
                Ok(Step::Send(Command::new("GET", &[self.key.as_bytes()])))
            }
            (OpeningState::Lock, Some(Reply::Nil)) => {
                if self.tries >= self.options.lock_retries {
                    return Err(Error::LockTimeout { key: self.key.clone() });
                }
                self.tries += 1;
                self.state = OpeningState::Waited;
                Ok(Step::Wait(Duration::from_micros(self.options.lock_wait)))
            }
            // The lock may be held after a reply of another type, so the release follows (SR-5.1).
            (OpeningState::Lock, Some(reply)) => Ok(self.release(After::Err(unexpected("SET", &reply)))),
            (OpeningState::Get, Some(Reply::Bulk(record))) => Ok(Step::Done(Session {
                record: RecordSession::new(Some(Found { id: self.id.clone(), record })),
                lock: Some(self.lock.clone()),
            })),
            (OpeningState::Get, Some(Reply::Nil)) => Ok(self.release(After::Ok(Session::without_session()))),
            (OpeningState::Get, Some(reply)) => Ok(self.release(After::Err(unexpected("GET", &reply)))),
            (OpeningState::Release(then), Some(reply)) => finish_release(*then, &reply),
            (_, _) => Err(Error::Internal { what: "the opening is given a reply that it did not ask for" }),
        }
    }
}

fn finish_release<T>(then: After<T>, reply: &Reply) -> Result<Step<T>, Error> {
    match (then, reply) {
        (After::Ok(value), Reply::Integer(_)) => Ok(Step::Done(value)),
        (After::Ok(_), reply) => Err(unexpected("EVAL", reply)),
        (After::Err(error), _) => Err(error),
    }
}

/// The closing of the session of a request (SR-4).
#[derive(Debug)]
pub struct Closing {
    options: Options,
    lock: Option<Lock>,
    state: ClosingState,
}

#[derive(Debug)]
enum ClosingState {
    Start(Result<Outcome, Error>),
    Expire,
    CompareLock { id: String, record: Vec<u8>, new: bool, removes: Option<String> },
    Write { id: String, new: bool, removes: Option<String> },
    Delete { result: Option<String> },
    Release(After<Option<String>>),
    Ended,
}

impl Closing {
    /// The closing of a session; it writes what `finish` gives (SR-4.1).
    #[must_use]
    pub fn new(options: &Options, session: Session) -> Self {
        let Session { record, lock } = session;
        Self { options: options.clone(), lock, state: ClosingState::Start(record.finish()) }
    }

    /// Releases a held lock and then ends with `then`, or ends at once without a lock (SR-4.5).
    fn release(&mut self, then: After<Option<String>>) -> Result<Step<Option<String>>, Error> {
        let Some(lock) = &self.lock else {
            return match then {
                After::Ok(value) => Ok(Step::Done(value)),
                After::Err(error) => Err(error),
            };
        };
        let command = lock.release();
        self.state = ClosingState::Release(then);
        Ok(Step::Send(command))
    }

    fn write(&mut self, id: String, record: &[u8], new: bool, removes: Option<String>) -> Step<Option<String>> {
        let key = self.options.key(&id);
        let lifetime = self.options.lifetime.to_string();
        self.state = ClosingState::Write { id, new, removes };
        Step::Send(Command::new("SETEX", &[key.as_bytes(), lifetime.as_bytes(), record]))
    }

    /// The next step, given the reply of the command of the last step (`None` at the start).
    pub fn next(&mut self, reply: Option<Reply>) -> Result<Step<Option<String>>, Error> {
        let state = std::mem::replace(&mut self.state, ClosingState::Ended);
        match (state, reply) {
            (ClosingState::Start(Err(error)), None) => self.release(After::Err(error)),
            (ClosingState::Start(Ok(Outcome::Unused)), None) => self.release(After::Ok(None)),
            (ClosingState::Start(Ok(Outcome::Unchanged { id })), None) => {
                let key = self.options.key(&id);
                let lifetime = self.options.lifetime.to_string();
                self.state = ClosingState::Expire;
                Ok(Step::Send(Command::new("EXPIRE", &[key.as_bytes(), lifetime.as_bytes()])))
            }
            (ClosingState::Start(Ok(Outcome::Write { id, record, new, removes })), None) => match &self.lock {
                // SR-4.3: a request that holds a lock compares it before it writes anything.
                Some(lock) => {
                    let command = Command::new("GET", &[lock.key.as_bytes()]);
                    self.state = ClosingState::CompareLock { id, record, new, removes };
                    Ok(Step::Send(command))
                }
                None => Ok(self.write(id, &record, new, removes)),
            },
            (ClosingState::Expire, Some(Reply::Integer(_))) => self.release(After::Ok(None)),
            (ClosingState::Expire, Some(reply)) => self.release(After::Err(unexpected("EXPIRE", &reply))),
            (ClosingState::CompareLock { id, record, new, removes }, Some(reply)) => {
                let Some(lock) = &self.lock else {
                    return Err(Error::Internal { what: "a closing without a lock compares a lock" });
                };
                match reply {
                    Reply::Bulk(held) if held == lock.secret.as_bytes() => Ok(self.write(id, &record, new, removes)),
                    Reply::Bulk(_) | Reply::Nil => {
                        let key = lock.key.clone();
                        self.release(After::Err(Error::LockLost { key }))
                    }
                    other => self.release(After::Err(unexpected("GET", &other))),
                }
            }
            (ClosingState::Write { id, new, removes }, Some(Reply::Status(status))) if status == "OK" => {
                let result = if new { Some(id) } else { None };
                match removes {
                    Some(old) => {
                        let key = self.options.key(&old);
                        self.state = ClosingState::Delete { result };
                        Ok(Step::Send(Command::new("DEL", &[key.as_bytes()])))
                    }
                    None => self.release(After::Ok(result)),
                }
            }
            (ClosingState::Write { .. }, Some(reply)) => self.release(After::Err(unexpected("SETEX", &reply))),
            (ClosingState::Delete { result }, Some(Reply::Integer(_))) => self.release(After::Ok(result)),
            (ClosingState::Delete { .. }, Some(reply)) => self.release(After::Err(unexpected("DEL", &reply))),
            (ClosingState::Release(then), Some(reply)) => finish_release(then, &reply),
            (_, _) => Err(Error::Internal { what: "the closing is given a reply that it did not ask for" }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bindgen_core::Kinds;

    use super::{Opening, Options, Reply, Step};

    #[test]
    fn a_taken_lock_is_tried_again_after_the_wait_of_the_options() {
        // SR-3.3: one try and then lock_retries more, each after lock_wait microseconds, and then lock_timeout.
        let Ok(options) = Options::new(None, None, 30, Some(3), Some(1500)) else { panic!("the options are refused") };
        let Ok(Ok(mut opening)) = Opening::new(&options, Some(&[b'a'; 32])) else { panic!("no opening") };
        assert!(matches!(opening.next(None), Ok(Step::Send(command)) if command.name == "EXISTS"));
        let mut reply = Some(Reply::Integer(1));
        let mut waits = Vec::new();
        let mut tries = 0;
        loop {
            match opening.next(reply.take()) {
                Ok(Step::Send(command)) => {
                    assert_eq!(command.name, "SET");
                    tries += 1;
                    reply = Some(Reply::Nil);
                }
                Ok(Step::Wait(duration)) => waits.push(duration),
                Ok(Step::Done(_)) => panic!("a taken lock opens the session"),
                Err(error) => {
                    assert_eq!(error.kind(), "lock_timeout");
                    break;
                }
            }
        }
        assert_eq!(tries, 4);
        assert_eq!(waits, vec![Duration::from_micros(1500); 3]);
    }

    #[test]
    fn a_lock_expiry_whose_milliseconds_pass_u64_is_refused() {
        // SR-1.2
        assert!(
            matches!(Options::new(None, None, u64::MAX / 1000 + 1, None, None), Err(error) if error.kind() == "invalid_lock_expire")
        );
        assert!(Options::new(None, None, u64::MAX / 1000, None, None).is_ok());
    }
}
