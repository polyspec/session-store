//! Every case of `contracts/redis-sessions.json` (SE-10.4): the protocol against a model of the commands that it sends,
//! with the command records of the cases, and the same requests through the connection against a real Redis server,
//! which the test starts when the environment names a `redis-server` binary (`REDIS_SERVER`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as Process, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use bindgen_core::{Failure, Kinds};
use libtest_mimic::{Arguments, Failed, Trial};
use polyspec_php_serialization::{serialize, unserialize};
use polyspec_session_store_core::Error;
use polyspec_session_store_core::Store;
use polyspec_session_store_redis::protocol::{Closing, Opening, Step, RELEASE};
use polyspec_session_store_redis::{Command, Options, RedisSessions, Reply, Setting, StoreOptions};
use serde_json::Value;

const CONTRACT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../contracts/redis-sessions.json"));

fn latin1(text: &str) -> Result<Vec<u8>, Failed> {
    text.chars()
        .map(|character| {
            u8::try_from(u32::from(character))
                .map_err(|error| Failed::from(format!("{text:?} is no byte string: {error}")))
        })
        .collect()
}

fn text_of(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}

fn failed(error: impl std::fmt::Display) -> Failed {
    Failed::from(error.to_string())
}

fn expect_eq<T: PartialEq + std::fmt::Debug>(what: &str, actual: &T, expected: &T) -> Result<(), Failed> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{what}: {actual:?}, expected {expected:?}").into())
    }
}

fn str_of<'a>(case: &'a Value, key: &str) -> Result<&'a str, Failed> {
    case.get(key).and_then(Value::as_str).ok_or_else(|| Failed::from(format!("the case has no text {key}")))
}

fn array_of<'a>(case: &'a Value, key: &str) -> Result<&'a Vec<Value>, Failed> {
    case.get(key).and_then(Value::as_array).ok_or_else(|| Failed::from(format!("the case has no list {key}")))
}

fn pairs(value: &[Value]) -> Result<Vec<(String, String)>, Failed> {
    value
        .iter()
        .map(|pair| match pair.as_array().map(Vec::as_slice) {
            Some([Value::String(key), Value::String(bytes)]) => Ok((key.clone(), bytes.clone())),
            _ => Err(Failed::from("a key is no pair of texts")),
        })
        .collect()
}

fn options(case: &Value) -> Result<Options, Failed> {
    options_of(case).map_err(failed)
}

/// The lock expiry of a case. A case without one is one that the binding refuses (SR-1.2), and its 0 is refused here.
#[allow(
    clippy::manual_unwrap_or_default,
    clippy::manual_unwrap_or,
    reason = "SR-1.2: the 0 of a missing expiry is refused, not taken"
)]
fn lock_expire(case: &Value) -> u64 {
    match case.get("options").and_then(|options| options.get("lock_expire")).and_then(Value::as_u64) {
        Some(seconds) => seconds,
        None => 0,
    }
}

fn store_options(case: &Value) -> StoreOptions {
    let options = case.get("options");
    let number = |name: &str| options.and_then(|options| options.get(name)).and_then(Value::as_u64);
    StoreOptions {
        lifetime: number("lifetime"),
        prefix: options.and_then(|options| options.get("prefix")).and_then(Value::as_str).map(str::to_owned),
        lock_expire: lock_expire(case),
        lock_retries: number("lock_retries"),
        lock_wait: number("lock_wait"),
    }
}

fn options_of(case: &Value) -> Result<Options, Error> {
    let options = case.get("options");
    let number = |name: &str| options.and_then(|options| options.get(name)).and_then(Value::as_u64);
    Options::new(
        number("lifetime"),
        options.and_then(|options| options.get("prefix")).and_then(Value::as_str).map(str::to_owned),
        lock_expire(case),
        number("lock_retries"),
        number("lock_wait"),
    )
}

/// The calls of a step on the session of a request, each as the serialize form of a value or null.
fn calls(session: &mut dyn Store, step: &Value) -> Result<Vec<Value>, Failed> {
    let mut results = Vec::new();
    for call in array_of(step, "calls")? {
        let (name, arguments) = match call.as_array().map(Vec::as_slice) {
            Some([Value::String(name), Value::Array(arguments)]) => (name.as_str(), arguments),
            _ => return Err("a call is no pair".into()),
        };
        let text = |index: usize| -> Result<&str, Failed> {
            arguments.get(index).and_then(Value::as_str).ok_or_else(|| Failed::from("an argument is no text"))
        };
        let result = match name {
            "get" => session.get(text(0)?).map(|value| value.map(|value| serialize(&value))),
            "set" => session.set(text(0)?, &unserialize(&latin1(text(1)?)?).map_err(failed)?).map(|()| None),
            "remove" => session.remove(text(0)?).map(|()| None),
            "renew" => session.renew().map(|()| None),
            other => return Err(format!("unknown call {other}").into()),
        };
        results.push(match result {
            Ok(Some(bytes)) => Value::String(text_of(&bytes.map_err(failed)?)),
            Ok(None) => Value::Null,
            Err(Failure::Kind(error)) => return Err(format!("a call fails: {error}").into()),
            Err(Failure::Raised(_) | Failure::Callback(_)) => {
                return Err("the store raised an exception of its caller".into())
            }
        });
    }
    Ok(results)
}

fn cookie(step: &Value) -> Result<Option<Vec<u8>>, Failed> {
    match step.get("cookie") {
        Some(Value::String(cookie)) => Ok(Some(latin1(cookie)?)),
        _ => Ok(None),
    }
}

fn expect_raises(error: &Error, raises: &Value, at: &str) -> Result<(), Failed> {
    expect_eq("at", &at, &str_of(raises, "at")?)?;
    expect_eq("kind", &error.kind(), &str_of(raises, "kind")?)?;
    expect_eq("status", &error.status().map(u64::from), &raises.get("status").and_then(Value::as_u64))
}

/// A model of the commands of the protocol (SE-10.4), which records them.
struct Model {
    keys: BTreeMap<Vec<u8>, Vec<u8>>,
    text_replies: bool,
    log: Vec<Vec<Vec<u8>>>,
}

impl Model {
    fn reply(&mut self, command: &Command) -> Reply {
        self.log.push(command.words.clone());
        let words: Vec<&[u8]> = command.words.iter().map(Vec::as_slice).collect();
        match words.as_slice() {
            [b"EXISTS", key] => Reply::Integer(i64::from(self.keys.contains_key(*key))),
            [b"SET", key, value, b"NX", b"PX", _] => {
                if self.keys.contains_key(*key) {
                    Reply::Nil
                } else {
                    self.keys.insert(key.to_vec(), value.to_vec());
                    Reply::Status("OK".to_owned())
                }
            }
            [b"GET", key] => match self.keys.get(*key) {
                Some(value) if self.text_replies => Reply::Status(text_of(value)),
                Some(value) => Reply::Bulk(value.clone()),
                None => Reply::Nil,
            },
            [b"SETEX", key, _, value] => {
                self.keys.insert(key.to_vec(), value.to_vec());
                Reply::Status("OK".to_owned())
            }
            [b"EXPIRE", key, _] => Reply::Integer(i64::from(self.keys.contains_key(*key))),
            [b"DEL", key] => Reply::Integer(i64::from(self.keys.remove(*key).is_some())),
            [b"EVAL", script, b"1", key, secret] if *script == RELEASE.as_bytes() => {
                if self.keys.get(*key).map(Vec::as_slice) == Some(*secret) {
                    self.keys.remove(*key);
                    Reply::Integer(1)
                } else {
                    Reply::Integer(0)
                }
            }
            _ => Reply::Error(format!("ERR unknown command {}", text_of(&command.words.concat()))),
        }
    }

    fn drive<T>(&mut self, mut next: impl FnMut(Option<Reply>) -> Result<Step<T>, Error>) -> Result<T, Error> {
        let mut reply = None;
        loop {
            match next(reply.take())? {
                Step::Send(command) => reply = Some(self.reply(&command)),
                Step::Wait(_) => {}
                Step::Done(value) => return Ok(value),
            }
        }
    }
}

/// A text of the case with every created identifier written as `#n` and the secret as `<secret>` (SE-10.2).
fn shown(bytes: &[u8], created: &[String], secrets: &[Vec<u8>]) -> String {
    if bytes == RELEASE.as_bytes() {
        return "<release>".to_owned();
    }
    if secrets.iter().any(|secret| secret == bytes) {
        return "<secret>".to_owned();
    }
    let mut text = text_of(bytes);
    for (index, id) in created.iter().enumerate() {
        text = text.replace(id, &format!("#{index}"));
    }
    text
}

fn stored(keys: impl IntoIterator<Item = (Vec<u8>, Vec<u8>)>, created: &[String]) -> Vec<(String, String)> {
    let mut stored: Vec<(String, String)> =
        keys.into_iter().map(|(key, value)| (shown(&key, created, &[]), shown(&value, created, &[]))).collect();
    stored.sort();
    stored
}

fn model(case: &Value) -> Result<(), Failed> {
    let options = options(case)?;
    let mut model = Model {
        keys: pairs(array_of(case, "keys")?)?
            .into_iter()
            .map(|(key, value)| Ok((latin1(&key)?, latin1(&value)?)))
            .collect::<Result<_, Failed>>()?,
        text_replies: case.get("textReplies").is_some(),
        log: Vec::new(),
    };
    let raises = case.get("raises");
    let mut created = Vec::new();
    let mut results = Vec::new();
    let mut created_steps = Vec::new();
    for step in array_of(case, "steps")? {
        let mut session = match Opening::new(&options, cookie(step)?.as_deref()).map_err(failed)? {
            Err(session) => session,
            Ok(mut opening) => match model.drive(|reply| opening.next(reply)) {
                Ok(session) => session,
                Err(error) => {
                    expect_raises(&error, raises.ok_or_else(|| failed(&error))?, "open")?;
                    break;
                }
            },
        };
        results.push(Value::Array(calls(&mut session, step)?));
        if let Some(meanwhile) = step.get("meanwhile").and_then(Value::as_array) {
            for (key, value) in pairs(meanwhile)? {
                model.keys.insert(latin1(&key)?, latin1(&value)?);
            }
        }
        let mut closing = Closing::new(&options, session);
        match model.drive(|reply| closing.next(reply)) {
            Ok(Some(id)) => {
                created_steps.push(Value::String(format!("#{}", created.len())));
                created.push(id);
            }
            Ok(None) => created_steps.push(Value::Null),
            Err(error) => {
                expect_raises(&error, raises.ok_or_else(|| failed(&error))?, "close")?;
                break;
            }
        }
    }
    if raises.is_none() {
        expect_eq("results", &results, array_of(case, "results")?)?;
        expect_eq("created", &created_steps, array_of(case, "created")?)?;
    }
    let secrets: Vec<Vec<u8>> = model
        .log
        .iter()
        .filter_map(|words| match words.as_slice() {
            [set, _, secret, nx, ..] if set == b"SET" && nx == b"NX" => Some(secret.clone()),
            _ => None,
        })
        .collect();
    let log: Vec<Vec<String>> =
        model.log.iter().map(|words| words.iter().map(|word| shown(word, &created, &secrets)).collect()).collect();
    let expected: Vec<Vec<String>> = array_of(case, "commands")?
        .iter()
        .map(|words| words.as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect())
        .collect();
    expect_eq("commands", &log, &expected)?;
    expect_eq("stored", &stored(model.keys, &created), &pairs(array_of(case, "stored")?)?)
}

fn open_case(case: &Value) -> Result<(), Failed> {
    match (options_of(case), case.get("raises")) {
        (Err(error), Some(raises)) => {
            expect_eq("kind", &error.kind(), &str_of(raises, "kind")?)?;
            expect_eq("status", &error.status().map(u64::from), &raises.get("status").and_then(Value::as_u64))
        }
        (Ok(options), None) => {
            let defaults = case.get("defaults").ok_or("the case has no defaults")?;
            expect_eq("lifetime", &Some(options.lifetime()), &defaults.get("lifetime").and_then(Value::as_u64))?;
            expect_eq("prefix", &Some(options.prefix()), &defaults.get("prefix").and_then(Value::as_str))?;
            expect_eq(
                "lock_expire",
                &Some(options.lock_expire()),
                &defaults.get("lock_expire").and_then(Value::as_u64),
            )?;
            expect_eq(
                "lock_retries",
                &Some(options.lock_retries()),
                &defaults.get("lock_retries").and_then(Value::as_u64),
            )?;
            expect_eq("lock_wait", &Some(options.lock_wait()), &defaults.get("lock_wait").and_then(Value::as_u64))
        }
        (Ok(_), Some(_)) => Err("the options are taken".into()),
        (Err(error), None) => Err(failed(error)),
    }
}

/// A Redis server that the test started on a socket of a temporary directory, for every trial.
struct Server {
    child: std::sync::Mutex<Child>,
    socket: PathBuf,
    _directory: tempfile::TempDir,
}

fn server() -> Result<&'static Server, String> {
    static SERVER: OnceLock<Result<Server, String>> = OnceLock::new();
    SERVER.get_or_init(start_server).as_ref().map_err(String::clone)
}

fn start_server() -> Result<Server, String> {
    let binary = std::env::var("REDIS_SERVER").map_err(|error| format!("REDIS_SERVER: {error}"))?;
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let socket = directory.path().join("redis.sock");
    let child = Process::new(binary)
        .args(["--port", "0", "--save", "", "--appendonly", "no", "--unixsocket"])
        .arg(&socket)
        .arg("--dir")
        .arg(directory.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("redis-server does not start: {error}"))?;
    let start = Instant::now();
    while !socket.exists() {
        if start.elapsed() > Duration::from_secs(10) {
            return Err("redis-server makes no socket in 10 seconds".to_owned());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(Server { child: std::sync::Mutex::new(child), socket, _directory: directory })
}

/// Stops the server that the test started, if it started one.
fn stop_server() {
    let Ok(server) = server() else {
        return;
    };
    if let Ok(mut child) = server.child.lock() {
        let stopped: std::io::Result<()> = child.kill().and_then(|()| child.wait().map(|_status| ()));
        if let Err(error) = stopped {
            eprintln!("the Redis server does not stop: {error}");
        }
    }
}

fn url(socket: &Path, database: usize) -> String {
    format!("redis+unix://{}?db={database}", socket.display())
}

async fn server_case(case: &Value, database: usize) -> Result<(), Failed> {
    let server = server().map_err(Failed::from)?;
    let address = url(&server.socket, database);
    let client = redis::Client::open(address.as_str()).map_err(failed)?;
    let mut connection = client.get_multiplexed_async_connection().await.map_err(failed)?;
    redis::cmd("FLUSHDB").query_async::<()>(&mut connection).await.map_err(failed)?;
    for (key, value) in pairs(array_of(case, "keys")?)? {
        redis::cmd("SET")
            .arg(latin1(&key)?)
            .arg(latin1(&value)?)
            .query_async::<()>(&mut connection)
            .await
            .map_err(failed)?;
    }
    let store = RedisSessions::new(Setting::Server { url: address }, store_options(case)).map_err(failed)?;
    let raises = case.get("raises");
    let mut created = Vec::new();
    let mut results = Vec::new();
    let mut created_steps = Vec::new();
    for step in array_of(case, "steps")? {
        let mut opened = match store.open(cookie(step)?.as_deref()).await {
            Ok(opened) => opened,
            Err(error) => {
                expect_raises(&error, raises.ok_or_else(|| failed(&error))?, "open")?;
                break;
            }
        };
        results.push(Value::Array(calls(&mut opened, step)?));
        if let Some(meanwhile) = step.get("meanwhile").and_then(Value::as_array) {
            for (key, value) in pairs(meanwhile)? {
                redis::cmd("SET")
                    .arg(latin1(&key)?)
                    .arg(latin1(&value)?)
                    .query_async::<()>(&mut connection)
                    .await
                    .map_err(failed)?;
            }
        }
        match opened.close().await {
            Ok(Some(id)) => {
                created_steps.push(Value::String(format!("#{}", created.len())));
                created.push(id);
            }
            Ok(None) => created_steps.push(Value::Null),
            Err(error) => {
                expect_raises(&error, raises.ok_or_else(|| failed(&error))?, "close")?;
                break;
            }
        }
    }
    if raises.is_none() {
        expect_eq("results", &results, array_of(case, "results")?)?;
        expect_eq("created", &created_steps, array_of(case, "created")?)?;
    }
    let keys: Vec<Vec<u8>> = redis::cmd("KEYS").arg("*").query_async(&mut connection).await.map_err(failed)?;
    let mut found = Vec::new();
    for key in keys {
        let value: Vec<u8> = redis::cmd("GET").arg(&key).query_async(&mut connection).await.map_err(failed)?;
        found.push((key, value));
    }
    expect_eq("stored", &stored(found, &created), &pairs(array_of(case, "stored")?)?)
}

fn main() {
    let arguments = Arguments::from_args();
    let contract: Value = match serde_json::from_str(CONTRACT) {
        Ok(contract) => contract,
        Err(error) => panic!("contracts/redis-sessions.json is no JSON: {error}"),
    };
    let (Some(Value::Array(requests)), Some(Value::Array(opens))) = (contract.get("requests"), contract.get("open"))
    else {
        panic!("contracts/redis-sessions.json has no groups requests and open");
    };
    let mut trials = Vec::new();
    let with_server = std::env::var_os("REDIS_SERVER").is_some();
    for (index, case) in requests.iter().enumerate() {
        let Some(id) = case.get("id").and_then(Value::as_str) else {
            panic!("a case has no id");
        };
        let model_case = case.clone();
        trials.push(Trial::test(format!("model::{id}"), move || model(&model_case)));
        // A reply that is not bytes comes only from a model; a real server gives bulk strings.
        let server_only = case.get("textReplies").is_none();
        let server_case_value = case.clone();
        trials.push(
            Trial::test(format!("server::{id}"), move || {
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(failed)?;
                runtime.block_on(server_case(&server_case_value, index % 16))
            })
            .with_ignored_flag(!(with_server && server_only)),
        );
    }
    for case in opens.clone() {
        let Some(id) = case.get("id").and_then(Value::as_str) else {
            panic!("a case has no id");
        };
        let name = format!("open::{id}");
        // A value that no u64 holds is refused by the binding (SR-8.2); the bindings run such a case.
        let binding = case.get("binding").is_some();
        trials.push(Trial::test(name, move || open_case(&case)).with_ignored_flag(binding));
    }
    let conclusion = libtest_mimic::run(&arguments, trials);
    if with_server {
        stop_server();
    }
    conclusion.exit();
}
