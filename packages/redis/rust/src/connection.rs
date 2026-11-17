//! The connection of the store of Redis (SR-6): redis-rs on the tokio runtime of the process, which runs the protocol.

use std::sync::Arc;

use redis::aio::MultiplexedConnection;
use redis::cluster_async::ClusterConnection;
use redis::sentinel::{SentinelClient, SentinelClientBuilder, SentinelServerType};
use redis::{IntoConnectionInfo, TlsMode};
use tokio::sync::Mutex;

use polyspec_php_serialization::PhpValue;
use polyspec_session_store_core::Error;
use polyspec_session_store_core::{Failure, Store};

use crate::protocol::{Closing, Command, Opening, Options, Reply, Session, Step, StoreOptions};
use crate::trust;
use crate::turns::{Turn, Turns};

/// The connection setting that the application gives (SR-6.1). An address is a URL of redis-rs: `redis://` or
/// `rediss://` (TLS), with the ACL user and password as its user information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Setting {
    /// One server.
    Server { url: String },
    /// The primary of a service that Sentinel names (SR-6.2); `tls`, `user` and `password` are those of the primary.
    Sentinel { urls: Vec<String>, service: String, tls: bool, user: Option<String>, password: Option<String> },
    /// A Redis Cluster from its first nodes (SR-6.3).
    Cluster { urls: Vec<String> },
}

/// The connections of one process (SR-6.5).
enum Link {
    Server { client: redis::Client, connection: Option<MultiplexedConnection> },
    Sentinel { sentinel: Box<SentinelClient>, primary: Option<(String, MultiplexedConnection)> },
    Cluster { client: redis::cluster::ClusterClient, connection: Option<ClusterConnection> },
}

/// The links and the process that made them (SR-6.5).
struct Links {
    process: u32,
    link: Link,
}

/// The state of a store of Redis that its sessions share.
struct Inner {
    options: Options,
    setting: Setting,
    links: Mutex<Option<Links>>,
    turns: Turns,
}

/// The store of Redis of a process (SR-1).
#[derive(Clone)]
pub struct RedisSessions {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for RedisSessions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The setting holds passwords, which no message tells (SR-6.4).
        formatter.debug_struct("RedisSessions").field("options", &self.inner.options).finish_non_exhaustive()
    }
}

/// The session of one request of the store of Redis: its store (SE-2), its lock and the turn of its identifier, which
/// [`RedisSession::close`] ends (SR-3.2, SR-4).
pub struct RedisSession {
    inner: Arc<Inner>,
    session: Session,
    turn: Option<Turn>,
}

impl std::fmt::Debug for RedisSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("RedisSession").field("session", &self.session).finish_non_exhaustive()
    }
}

fn connection_error(error: &redis::RedisError) -> Error {
    // SR-6.4: the category of redis-rs names no address, user or password.
    Error::Connection { detail: error.category().to_owned() }
}

fn reply_of(value: redis::Value) -> Reply {
    match value {
        redis::Value::Nil => Reply::Nil,
        redis::Value::Int(number) => Reply::Integer(number),
        redis::Value::BulkString(bytes) => Reply::Bulk(bytes),
        redis::Value::SimpleString(text) => Reply::Status(text),
        redis::Value::Okay => Reply::Status("OK".to_owned()),
        redis::Value::ServerError(error) => Reply::Error(error.to_string()),
        redis::Value::Array(_) => Reply::Other("an array"),
        redis::Value::Map(_) => Reply::Other("a map"),
        redis::Value::Set(_) => Reply::Other("a set"),
        redis::Value::Double(_) => Reply::Other("a double"),
        redis::Value::Boolean(_) => Reply::Other("a boolean"),
        _ => Reply::Other("another type"),
    }
}

/// Whether an address of redis-rs connects with TLS (SR-6.1).
fn tls(url: &str) -> bool {
    url.starts_with("rediss://")
}

fn invalid(detail: &str) -> Error {
    Error::InvalidSetting { detail: detail.to_owned() }
}

/// The client of Sentinel (SR-6.1b, SR-6.2): the Sentinel nodes share one scheme, one user and one password, and every
/// connection of TLS, to a node or to the primary that it names, trusts the authorities read once.
fn sentinel(
    urls: &[String],
    service: &str,
    tls: bool,
    user: &Option<String>,
    password: &Option<String>,
) -> Result<SentinelClient, Error> {
    let nodes = urls
        .iter()
        .map(|url| url.as_str().into_connection_info().map_err(|error| invalid(error.category())))
        .collect::<Result<Vec<_>, Error>>()?;
    let Some(first) = nodes.first() else {
        return Err(invalid("the list of Sentinel addresses is empty"));
    };
    let nodes_tls = self::tls(&urls[0]);
    if urls.iter().any(|url| self::tls(url) != nodes_tls) {
        return Err(invalid("the Sentinel addresses are not all redis:// or all rediss://"));
    }
    let (node_user, node_password) = (first.redis_settings().username(), first.redis_settings().password());
    if nodes
        .iter()
        .any(|node| node.redis_settings().username() != node_user || node.redis_settings().password() != node_password)
    {
        return Err(invalid("the Sentinel addresses do not share one user and password"));
    }
    let (node_user, node_password) = (node_user.map(str::to_owned), node_password.map(str::to_owned));
    let addresses = nodes.into_iter().map(|node| node.addr().clone());
    let mut builder = SentinelClientBuilder::new(addresses, service, SentinelServerType::Master)
        .map_err(|error| invalid(error.category()))?;
    if let Some(name) = node_user {
        builder = builder.set_client_to_sentinel_username(name);
    }
    if let Some(secret) = node_password {
        builder = builder.set_client_to_sentinel_password(secret);
    }
    if let Some(name) = user {
        builder = builder.set_client_to_redis_username(name);
    }
    if let Some(secret) = password {
        builder = builder.set_client_to_redis_password(secret);
    }
    if nodes_tls {
        builder = builder.set_client_to_sentinel_certificates(trust::certificates(&trust::authorities()?));
    }
    if tls {
        builder = builder
            .set_client_to_redis_tls_mode(TlsMode::Secure)
            .set_client_to_redis_certificates(trust::certificates(&trust::authorities()?));
    }
    builder.build().map_err(|error| invalid(error.category()))
}

impl Link {
    fn new(setting: &Setting) -> Result<Self, Error> {
        match setting {
            Setting::Server { url } => {
                let client = if tls(url) {
                    redis::Client::build_with_tls(url.as_str(), trust::certificates(&trust::authorities()?))
                } else {
                    redis::Client::open(url.as_str())
                };
                Ok(Self::Server { client: client.map_err(|error| invalid(error.category()))?, connection: None })
            }
            Setting::Sentinel { urls, service, tls, user, password } => {
                if urls.is_empty() {
                    return Err(invalid("the list of Sentinel addresses is empty"));
                }
                Ok(Self::Sentinel { sentinel: Box::new(sentinel(urls, service, *tls, user, password)?), primary: None })
            }
            Setting::Cluster { urls } => {
                if urls.is_empty() {
                    return Err(invalid("the list of Cluster addresses is empty"));
                }
                let mut builder = redis::cluster::ClusterClientBuilder::new(urls.iter().map(String::as_str));
                if urls.iter().any(|url| tls(url)) {
                    builder = builder.certs(trust::certificates(&trust::authorities()?));
                }
                let client = builder.build().map_err(|error| invalid(error.category()))?;
                Ok(Self::Cluster { client, connection: None })
            }
        }
    }

    /// Prepares the request: with Sentinel it asks for the primary before the first command (SR-6.2).
    async fn prepare(&mut self) -> Result<(), Error> {
        let Self::Sentinel { sentinel, primary } = self else {
            return Ok(());
        };
        // SR-6.2: async_get_client asks Sentinel for the primary at each call.
        let client = sentinel.async_get_client().await.map_err(|error| connection_error(&error))?;
        let address = client.get_connection_info().addr().to_string();
        if let Some((current, _)) = primary {
            if *current == address {
                return Ok(());
            }
        }
        let connection = client.get_multiplexed_async_connection().await.map_err(|error| connection_error(&error))?;
        *primary = Some((address, connection));
        Ok(())
    }

    /// The connection of the next commands: one multiplexed connection, which every request of the process shares.
    async fn connection(&mut self) -> Result<Connection, Error> {
        match self {
            Self::Server { client, connection } => {
                let connection = match connection {
                    Some(connection) => connection,
                    None => connection.insert(
                        client.get_multiplexed_async_connection().await.map_err(|error| connection_error(&error))?,
                    ),
                };
                Ok(Connection::Server(connection.clone()))
            }
            Self::Sentinel { primary, .. } => match primary {
                Some((_, connection)) => Ok(Connection::Server(connection.clone())),
                None => Err(Error::Internal { what: "a command is sent before Sentinel named the primary" }),
            },
            Self::Cluster { client, connection } => {
                let connection = match connection {
                    Some(connection) => connection,
                    None => connection
                        .insert(client.get_async_connection().await.map_err(|error| connection_error(&error))?),
                };
                Ok(Connection::Cluster(connection.clone()))
            }
        }
    }
}

/// A connection that sends the commands of one request.
enum Connection {
    Server(MultiplexedConnection),
    Cluster(ClusterConnection),
}

impl Connection {
    async fn send(&mut self, command: &Command) -> Result<Reply, Error> {
        let mut cmd = redis::Cmd::new();
        for word in &command.words {
            cmd.arg(word.as_slice());
        }
        let result = match self {
            Self::Server(connection) => cmd.query_async::<redis::Value>(connection).await,
            Self::Cluster(connection) => cmd.query_async::<redis::Value>(connection).await,
        };
        match result {
            Ok(value) => Ok(reply_of(value)),
            Err(error) => match error.kind() {
                redis::ErrorKind::Server(_) | redis::ErrorKind::Extension => Ok(Reply::Error(error.to_string())),
                _ => Err(connection_error(&error)),
            },
        }
    }
}

impl Inner {
    /// Runs the steps of the protocol on the connection of this process; a link that another process made is left
    /// unused and unclosed (SR-6.5).
    async fn run<T>(
        &self,
        mut next: impl FnMut(Option<Reply>) -> Result<Step<T>, Error>,
        prepare: bool,
    ) -> Result<T, Error> {
        let mut connection = {
            let mut links = self.links.lock().await;
            let process = std::process::id();
            if links.as_ref().is_some_and(|links| links.process != process) {
                if let Some(inherited) = links.take() {
                    // SR-6.5: the sockets of the parent are not used or closed by its child.
                    std::mem::forget(inherited);
                }
            }
            let link = match &mut *links {
                Some(links) => &mut links.link,
                None => &mut links.insert(Links { process, link: Link::new(&self.setting)? }).link,
            };
            if prepare {
                link.prepare().await?;
            }
            link.connection().await?
        };
        let mut reply = None;
        loop {
            match next(reply.take())? {
                Step::Send(command) => reply = Some(connection.send(&command).await?),
                Step::Wait(duration) => tokio::time::sleep(duration).await,
                Step::Done(value) => return Ok(value),
            }
        }
    }
}

impl RedisSessions {
    /// The store of a connection setting with its options (SR-1.2, SR-6.1). It connects at its first request.
    pub fn new(setting: &Setting, options: &StoreOptions) -> Result<Self, Error> {
        let options = Options::of(options)?;
        Link::new(setting)?;
        Ok(Self {
            inner: Arc::new(Inner {
                options,
                setting: setting.clone(),
                links: Mutex::new(None),
                turns: Turns::default(),
            }),
        })
    }

    /// The options (SR-1.2).
    #[must_use]
    pub fn options(&self) -> &Options {
        &self.inner.options
    }

    /// Opens the session of a request from the bytes of its session cookie (SR-3). The turn of the identifier lasts until
    /// [`RedisSession::close`].
    pub async fn open(&self, cookie: Option<&[u8]>) -> Result<RedisSession, Error> {
        let inner = Arc::clone(&self.inner);
        let mut opening = match Opening::new(&inner.options, cookie)? {
            Ok(opening) => opening,
            Err(session) => return Ok(RedisSession { inner, session, turn: None }),
        };
        let turn = inner.turns.enter(opening.id()).await?;
        let session = inner.run(|reply| opening.next(reply), true).await?;
        // A request without a found session holds no lock; it keeps its turn only while it holds one.
        let turn = session.found().map(|_| turn);
        Ok(RedisSession { inner, session, turn })
    }
}

impl RedisSession {
    /// The identifier of the existing session that the request found, or `None`.
    #[must_use]
    pub fn found(&self) -> Option<&str> {
        self.session.found()
    }

    /// Writes what the session gives, releases its lock and ends its turn (SR-4), and gives the new identifier, or
    /// `None`.
    pub async fn close(self) -> Result<Option<String>, Error> {
        let Self { inner, session, turn } = self;
        let mut closing = Closing::new(&inner.options, session);
        let result = inner.run(|reply| closing.next(reply), turn.is_none()).await;
        drop(turn);
        result
    }
}

// B-10: a session that holds a lock has the cleanup call close.
impl binding_core::Cleanup for RedisSession {}

impl Store for RedisSession {
    fn get(&mut self, key: &str) -> Result<Option<PhpValue>, Failure> {
        self.session.get(key)
    }

    fn set(&mut self, key: &str, value: &PhpValue) -> Result<(), Failure> {
        self.session.set(key, value)
    }

    fn remove(&mut self, key: &str) -> Result<(), Failure> {
        self.session.remove(key)
    }

    fn renew(&mut self) -> Result<(), Failure> {
        self.session.renew()
    }
}

#[cfg(test)]
mod trust_tests {
    use super::{Link, Setting};

    /// SR-6.1b: the links of TLS of a server, of Sentinel and of a Cluster take the authorities that this crate reads
    /// once in the process (`trust::authorities`), however many links there are; it counts the reads of this crate.
    #[test]
    fn the_links_of_tls_read_the_trust_store_once() {
        let settings = [
            Setting::Server { url: "rediss://localhost:6379/".to_owned() },
            Setting::Server { url: "rediss://localhost:6380/".to_owned() },
            Setting::Sentinel {
                urls: vec!["redis://localhost:26379/".to_owned()],
                service: "main".to_owned(),
                tls: true,
                user: None,
                password: None,
            },
            Setting::Cluster { urls: vec!["rediss://localhost:7000/".to_owned()] },
        ];
        for setting in &settings {
            assert!(Link::new(setting).is_ok(), "the link of {setting:?} is not made");
        }
        assert_eq!(crate::trust::reads(), 1, "the reads of the trust store");
    }
}
