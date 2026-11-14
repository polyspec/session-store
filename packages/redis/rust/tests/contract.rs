//! Every case of `contracts/redis-sessions.json` (SE-10.4): the protocol against a model of the commands that it sends,
//! with the command records of the cases, and the same requests through the connection against a real Redis server in a
//! container of the pinned image (`tests/servers`). A case with `faults`
//! arms the fault injection points that it names for its one call, as a generated binding test does (SE-10.5).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use binding_core::{Failure, Kinds};
use libtest_mimic::{Arguments, Failed, Trial};
use polyspec_php_serialization::{serialize, unserialize};
use polyspec_session_store_core::Error;
use polyspec_session_store_core::Store;
use polyspec_session_store_redis::protocol::{Closing, Opening, Step, RELEASE};
use polyspec_session_store_redis::{Command, Options, RedisSessions, Reply, Setting, StoreOptions};
use serde_json::Value;
use servers::{Project, Service, PLAIN, PORT};

mod servers;

const CONTRACT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../contracts/redis-sessions.json"));
/// The interface contract, whose `errors.kinds` give the category of each kind (SE-9.2).
const INTERFACE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rust/interface.json"));

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

/// The serialize form of a value of a case: a text, or `{"nested": n}`, n arrays that each hold the next at the key 0,
/// the innermost holding null (SE-10.2).
fn value_text(argument: Option<&Value>) -> Result<String, Failed> {
    match argument {
        Some(Value::String(text)) => Ok(text.clone()),
        Some(Value::Object(form)) => {
            let depth = form.get("nested").and_then(Value::as_u64).ok_or("a value form has no nested")?;
            let depth = usize::try_from(depth).map_err(failed)?;
            Ok(format!("{}N;{}", "a:1:{i:0;".repeat(depth), "}".repeat(depth)))
        }
        _ => Err("an argument is no value".into()),
    }
}

/// SE-7.1: the identifier that the opening of a step found, where the case states it.
fn expect_found(case: &Value, index: usize, found: Option<&str>) -> Result<(), Failed> {
    match case.get("found").and_then(Value::as_array).and_then(|found| found.get(index)) {
        Some(expected) => expect_eq("found", &found, &expected.as_str()),
        None => Ok(()),
    }
}

/// The calls of a step on the session of a request, each as the serialize form of a value or null, or the error of the
/// call that fails.
fn calls(session: &mut dyn Store, step: &Value) -> Result<Result<Vec<Value>, Error>, Failed> {
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
            "set" => {
                let value = unserialize(&latin1(&value_text(arguments.get(1))?)?).map_err(failed)?;
                session.set(text(0)?, &value).map(|()| None)
            }
            "remove" => session.remove(text(0)?).map(|()| None),
            "renew" => session.renew().map(|()| None),
            other => return Err(format!("unknown call {other}").into()),
        };
        results.push(match result {
            Ok(Some(bytes)) => Value::String(text_of(&bytes.map_err(failed)?)),
            Ok(None) => Value::Null,
            Err(Failure::Kind(error)) => return Ok(Err(error)),
            Err(Failure::Raised(_) | Failure::Callback(_)) => {
                return Err("the store raised an exception of its caller".into())
            }
        });
    }
    Ok(Ok(results))
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
    expect_eq("status", &error.status().map(u64::from), &raises.get("status").and_then(Value::as_u64))?;
    // SE-9.1: a message is compared where a rule states it.
    match raises.get("message") {
        Some(Value::String(expected)) => expect_eq("message", &error.to_string().as_str(), &expected.as_str()),
        _ => Ok(()),
    }
}

/// A model of the commands of the protocol (SE-10.4), which records them.
struct Model {
    keys: BTreeMap<Vec<u8>, Vec<u8>>,
    text_replies: bool,
    /// The error reply that the model gives to each command of a name, in place of what the command does (SR-5.2).
    error_replies: BTreeMap<Vec<u8>, String>,
    log: Vec<Vec<Vec<u8>>>,
}

impl Model {
    fn reply(&mut self, command: &Command) -> Reply {
        self.log.push(command.words.clone());
        let words: Vec<&[u8]> = command.words.iter().map(Vec::as_slice).collect();
        if let Some(message) = words.first().and_then(|name| self.error_replies.get(*name)) {
            return Reply::Error(message.clone());
        }
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
        text_replies: case.get("text_replies").is_some(),
        error_replies: match case.get("error_replies") {
            Some(Value::Object(replies)) => replies
                .iter()
                .map(|(name, message)| Ok((name.as_bytes().to_vec(), message.as_str().ok_or("no text")?.to_owned())))
                .collect::<Result<_, Failed>>()?,
            _ => BTreeMap::new(),
        },
        log: Vec::new(),
    };
    let raises = case.get("raises");
    let mut created = Vec::new();
    let mut results = Vec::new();
    let mut created_steps = Vec::new();
    for (index, step) in array_of(case, "steps")?.iter().enumerate() {
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
        expect_found(case, index, session.found())?;
        let called = calls(&mut session, step)?;
        if let Some(meanwhile) = step.get("meanwhile").and_then(Value::as_array) {
            for (key, value) in pairs(meanwhile)? {
                model.keys.insert(latin1(&key)?, latin1(&value)?);
            }
        }
        let mut closing = Closing::new(&options, session);
        match called {
            Ok(values) => results.push(Value::Array(values)),
            Err(error) => {
                // SE-7.2, SR-4.1: a session whose record fails writes nothing, and its close releases the lock.
                expect_raises(&error, raises.ok_or_else(|| failed(&error))?, "call")?;
                expect_eq("close after a failure", &model.drive(|reply| closing.next(reply)), &Ok(None))?;
                break;
            }
        }
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

/// The connection setting of a case, a value of the variant `setting`: its tag `kind` names `server`, `sentinel` or
/// `cluster`, whose fields are beside it (SR-6.1, binding G-5).
fn setting_of(setting: &Value) -> Result<Setting, Failed> {
    let urls = |form: &Value| -> Result<Vec<String>, Failed> {
        Ok(array_of(form, "urls")?.iter().filter_map(Value::as_str).map(str::to_owned).collect())
    };
    let text = |form: &Value, key: &str| form.get(key).and_then(Value::as_str).map(str::to_owned);
    match str_of(setting, "kind")? {
        "server" => Ok(Setting::Server { url: str_of(setting, "url")?.to_owned() }),
        "sentinel" => Ok(Setting::Sentinel {
            urls: urls(setting)?,
            service: str_of(setting, "service")?.to_owned(),
            tls: setting.get("tls").and_then(Value::as_bool).ok_or("a Sentinel setting has no tls")?,
            user: text(setting, "user"),
            password: text(setting, "password"),
        }),
        "cluster" => Ok(Setting::Cluster { urls: urls(setting)? }),
        other => Err(format!("a setting of the kind {other} is none of server, sentinel and cluster").into()),
    }
}

/// A case with a setting makes the store, which connects to nothing before its first request (SR-6.1).
fn setting_case(case: &Value, setting: &Value) -> Result<(), Failed> {
    match (RedisSessions::new(setting_of(setting)?, store_options(case)), case.get("raises")) {
        (Err(error), Some(raises)) => {
            expect_eq("kind", &error.kind(), &str_of(raises, "kind")?)?;
            expect_eq("status", &error.status().map(u64::from), &raises.get("status").and_then(Value::as_u64))
        }
        (Ok(_), None) => Ok(()),
        (Ok(_), Some(_)) => Err("the store is made".into()),
        (Err(error), None) => Err(failed(error)),
    }
}

/// A case of the groups `open` and `openMissing`: a case with `defaults` reads the options that the store takes, and
/// every other case makes the store of its setting and options (SR-1.2, SR-6.1).
fn open_case(case: &Value) -> Result<(), Failed> {
    if case.get("defaults").is_none() {
        return setting_case(case, case.get("setting").ok_or("the case has no setting")?);
    }
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

/// Calls a member under the faults that a case arms, as a generated binding test does (binding design G-5, SE-10.5): it
/// arms the list for the member, the call takes the arming as the entry point of the member does, and a fault that did
/// not fire fails the case.
fn armed<T>(case: &Value, member: &str, call: impl FnOnce() -> T) -> Result<T, Failed> {
    let faults = case.get("faults").ok_or("the case arms no faults")?;
    let id = binding_core::fault::arm(member, &faults.to_string()).map_err(failed)?;
    let outcome = binding_core::fault::take(member).run(call);
    let left = binding_core::fault::left(id).map_err(failed)?;
    if left.is_empty() {
        Ok(outcome)
    } else {
        Err(format!("the faults of {} did not fire", left.join(", ")).into())
    }
}

/// The kind of a variant of `errors.kinds`: its name in snake case (binding design E-3).
fn variant_kind(variant: &str) -> String {
    let name = match variant.rsplit_once("::") {
        Some((_, name)) => name,
        None => variant,
    };
    let mut kind = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_ascii_uppercase() && index > 0 {
            kind.push('_');
        }
        kind.push(character.to_ascii_lowercase());
    }
    kind
}

/// The category of a kind in `errors.kinds` of the interface contract (SE-9.2).
fn category(kind: &str) -> Result<String, Failed> {
    let interface: Value = serde_json::from_str(INTERFACE).map_err(failed)?;
    let rows =
        interface.pointer("/errors/kinds").and_then(Value::as_array).ok_or("the contract has no errors.kinds")?;
    for row in rows {
        if variant_kind(str_of(row, "variant")?) == kind {
            return Ok(str_of(row, "category")?.to_owned());
        }
    }
    Err(format!("errors.kinds has no kind {kind}").into())
}

/// The record `store_options` of the interface contract names its Rust type `StoreOptions` in `core_type` (SR-1.2):
/// binding names the Rust type of a unit that a parameter takes by its `core_type` and refuses a unit without one.
fn the_options_name_their_core_type() -> Result<(), Failed> {
    let interface: Value = serde_json::from_str(INTERFACE).map_err(failed)?;
    let units = interface.get("units").and_then(Value::as_array).ok_or("the contract has no list units")?;
    let options = units
        .iter()
        .find(|unit| unit.get("id").and_then(Value::as_str) == Some("store_options"))
        .ok_or("the contract has no unit store_options")?;
    expect_eq("kind", &options.get("kind").and_then(Value::as_str), &Some("record"))?;
    expect_eq(
        "core_type",
        &options.get("core_type").and_then(Value::as_str),
        &Some("polyspec_session_store::StoreOptions"),
    )
}

/// Every case of a group that the interface contract declares for `redis_sessions.new` gives the arguments of one call
/// under the names of its parameters, `setting` and `options` with its required field `lock_expire`, because a
/// generated binding test takes the keys of a case as the arguments of the call (binding G-5).
fn the_cases_of_new_give_its_parameters() -> Result<(), Failed> {
    let interface: Value = serde_json::from_str(INTERFACE).map_err(failed)?;
    let contract: Value = serde_json::from_str(CONTRACT).map_err(failed)?;
    let rows = interface.get("cases").and_then(Value::as_array).ok_or("the contract has no list cases")?;
    let mut checked = 0_usize;
    for row in rows.iter().filter(|row| row.get("member").and_then(Value::as_str) == Some("redis_sessions.new")) {
        let group = str_of(row, "group")?;
        let cases = contract.get(group).and_then(Value::as_array).ok_or_else(|| format!("no group {group}"))?;
        for case in cases {
            let id = str_of(case, "id")?;
            if case.get("setting").is_none() {
                return Err(format!("the case {id} of the group {group} gives no setting").into());
            }
            if case.get("options").and_then(|options| options.get("lock_expire")).is_none() {
                return Err(format!("the case {id} of the group {group} gives no options with lock_expire").into());
            }
            checked += 1;
        }
    }
    if checked == 0 {
        return Err("the interface contract declares no case of redis_sessions.new".into());
    }
    Ok(())
}

/// Every `setting` of the cases, at any depth, is a case value of the variant `setting`: an object whose tag `kind`
/// names its case (`server`, `sentinel` or `cluster`) beside the fields of that case, the form in which a generated
/// binding test reads a variant (binding G-5, T-9).
fn the_settings_of_the_cases_name_their_case_by_kind() -> Result<(), Failed> {
    fn settings<'a>(value: &'a Value, found: &mut Vec<&'a Value>) {
        match value {
            Value::Object(entries) => {
                for (key, entry) in entries {
                    if key == "setting" {
                        found.push(entry);
                    }
                    settings(entry, found);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| settings(item, found)),
            _ => {}
        }
    }
    let contract: Value = serde_json::from_str(CONTRACT).map_err(failed)?;
    let mut found = Vec::new();
    settings(&contract, &mut found);
    if found.is_empty() {
        return Err("no case gives a setting".into());
    }
    for setting in found {
        match setting.get("kind").and_then(Value::as_str) {
            Some("server" | "sentinel" | "cluster") => {}
            _ => return Err(format!("the setting {setting} names no case in kind").into()),
        }
    }
    Ok(())
}

/// The cases that a generated binding test runs (binding G-5): every group that the interface contract declares for a
/// member of `redis_session`, other than the group `requests`, writes each PHP value of a call as a value of the alias
/// `php_value` (`{"kind": <kind>, ...}`), and `redis_session.found` names such a group. The group `requests` holds
/// cases of several requests, which only the Rust tests run (SE-10.6).
fn the_cases_of_one_call_reach_their_members() -> Result<(), Failed> {
    fn values<'a>(value: &'a Value, found: &mut Vec<&'a Value>) {
        match value {
            Value::Object(entries) => {
                for (key, entry) in entries {
                    if key == "value" && entries.contains_key("key") {
                        found.push(entry);
                    }
                    values(entry, found);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| values(item, found)),
            _ => {}
        }
    }
    let interface: Value = serde_json::from_str(INTERFACE).map_err(failed)?;
    let contract: Value = serde_json::from_str(CONTRACT).map_err(failed)?;
    let rows = interface.get("cases").and_then(Value::as_array).ok_or("the contract has no list cases")?;
    let mut groups: Vec<(&str, &str)> = Vec::new();
    for row in rows {
        let (member, group) = (str_of(row, "member")?, str_of(row, "group")?);
        if member.starts_with("redis_session.") && group != "requests" {
            groups.push((member, group));
        }
    }
    if !groups.iter().any(|(member, _)| *member == "redis_session.found") {
        return Err("redis_session.found names no group of one call".into());
    }
    for (_, group) in &groups {
        let cases = contract.get(*group).and_then(Value::as_array).ok_or_else(|| format!("no group {group}"))?;
        for case in cases {
            let mut found = Vec::new();
            values(case, &mut found);
            if let Some(value) = found.iter().find(|value| value.get("kind").and_then(Value::as_str).is_none()) {
                let id = str_of(case, "id")?;
                return Err(format!("the case {id} of {group} gives the value {value}, no php_value").into());
            }
        }
    }
    Ok(())
}

/// An opening of one call on a store whose setting names a socket under `/dev/null`, which no server has (SE-10.5):
/// the group `opening`, whose connection fails (SR-6.4), and the group `openFaults`, whose case arms the fault
/// injection points that it names (SE-9.6). Only `redis_sessions.open` reads the two groups. The future runs on a
/// runtime of this thread, which is the thread of the arming.
fn opening(case: &Value) -> Result<(), Failed> {
    let raises = case.get("raises").ok_or("the case has no raises")?;
    let setting = setting_of(case.get("setting").ok_or("the case has no setting")?)?;
    let store = RedisSessions::new(setting, store_options(case)).map_err(failed)?;
    let cookie = latin1(str_of(case, "cookie")?)?;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(failed)?;
    let open = || runtime.block_on(store.open(Some(&cookie)));
    let opened = if case.get("faults").is_some() { armed(case, "redis_sessions.open", open)? } else { open() };
    match opened {
        Ok(_) => Err("the session is opened".into()),
        Err(error) => {
            expect_eq("kind", &error.kind(), &str_of(raises, "kind")?)?;
            expect_eq("status", &error.status().map(u64::from), &raises.get("status").and_then(Value::as_u64))?;
            expect_eq("category", &category(error.kind())?.as_str(), &str_of(raises, "category")?)?;
            expect_eq("errno", &error.errno(), &None)
        }
    }
}

/// A session of a request that earlier calls made on a store whose setting names a socket under `/dev/null` (binding
/// GN-13f): `{"new": "redis_sessions.open", "receiver": {"new": "redis_sessions.new", "args": {"setting", "options"}},
/// "args": {"cookie": null}, "then": [...]}`, whose calls of `then` are calls of a record session that succeed.
fn made_session(
    value: &Value,
    runtime: &tokio::runtime::Runtime,
) -> Result<polyspec_session_store_redis::RedisSession, Failed> {
    expect_eq("new", &str_of(value, "new")?, &"redis_sessions.open")?;
    let store = value.get("receiver").ok_or("an opening has no receiver")?;
    expect_eq("new", &str_of(store, "new")?, &"redis_sessions.new")?;
    let args = store.get("args").ok_or("a store has no args")?;
    let store =
        RedisSessions::new(setting_of(args.get("setting").ok_or("a store has no setting")?)?, store_options(args))
            .map_err(failed)?;
    let cookie = match value.get("args").and_then(|args| args.get("cookie")) {
        Some(Value::Null) => None,
        Some(Value::String(text)) => Some(latin1(text)?),
        _ => return Err("an opening has no cookie".into()),
    };
    let mut session = runtime.block_on(store.open(cookie.as_deref())).map_err(failed)?;
    let mut steps = Vec::new();
    for then in value.get("then").and_then(Value::as_array).into_iter().flatten() {
        let name =
            str_of(then, "call")?.strip_prefix("redis_session.").ok_or("a call of then is no call of a session")?;
        let args = then.get("args");
        let mut arguments = Vec::new();
        for key in ["key", "value"] {
            if let Some(argument) = args.and_then(|args| args.get(key)) {
                arguments.push(argument.clone());
            }
        }
        steps.push(Value::Array(vec![Value::String(name.to_owned()), Value::Array(arguments)]));
    }
    let step = serde_json::json!({ "calls": steps });
    if let Err(error) = calls(&mut session, &step)? {
        return Err(format!("a call of then fails: {error}").into());
    }
    Ok(session)
}

/// SE-5.4, SE-7.2: one call of a record session of a session that `open` made, with the faults of the case; the group
/// of each member is read by that member alone.
fn made_call(case: &Value, name: &str) -> Result<(), Failed> {
    let raises = case.get("raises").ok_or("the case has no raises")?;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(failed)?;
    let mut session = made_session(case.get("receiver").ok_or("the case has no receiver")?, &runtime)?;
    let mut arguments = Vec::new();
    for key in ["key", "value"] {
        if let Some(argument) = case.get(key) {
            arguments.push(argument.clone());
        }
    }
    let step = serde_json::json!({ "calls": [[name, arguments]] });
    match armed(case, &format!("redis_session.{name}"), || calls(&mut session, &step))?? {
        Ok(results) => Err(format!("the call gives {results:?}").into()),
        Err(error) => {
            expect_eq("kind", &error.kind(), &str_of(raises, "kind")?)?;
            expect_eq("status", &error.status().map(u64::from), &raises.get("status").and_then(Value::as_u64))?;
            expect_eq("category", &category(error.kind())?.as_str(), &str_of(raises, "category")?)
        }
    }
}

/// SR-6.4: the close of a session that set a value connects to the server of its setting, which no file can be.
fn made_close(case: &Value) -> Result<(), Failed> {
    let raises = case.get("raises").ok_or("the case has no raises")?;
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(failed)?;
    let session = made_session(case.get("receiver").ok_or("the case has no receiver")?, &runtime)?;
    match runtime.block_on(session.close()) {
        Ok(found) => Err(format!("the session is closed as {found:?}").into()),
        Err(error) => {
            expect_eq("kind", &error.kind(), &str_of(raises, "kind")?)?;
            expect_eq("status", &error.status().map(u64::from), &raises.get("status").and_then(Value::as_u64))?;
            expect_eq("category", &category(error.kind())?.as_str(), &str_of(raises, "category")?)
        }
    }
}

type Group = fn(&Value) -> Result<(), Failed>;

/// The Redis server of every trial, in a project of the run (`tests/servers`); each trial uses a database of its own.
/// `address` is the name and the port that bay gives for the server.
struct Shared {
    address: String,
    server: std::sync::Mutex<Option<Project>>,
}

static SERVER: OnceLock<Result<Shared, String>> = OnceLock::new();

fn server() -> Result<&'static Shared, String> {
    SERVER
        .get_or_init(|| {
            let mut project = Project::new(vec![Service::new("server", PORT, &PLAIN)])?;
            project.start()?;
            let address = project.address("server")?.name.clone();
            Ok(Shared { address, server: std::sync::Mutex::new(Some(project)) })
        })
        .as_ref()
        .map_err(String::clone)
}

/// Removes the project of the server, if a trial started it.
fn stop_server() {
    let Some(Ok(shared)) = SERVER.get() else {
        return;
    };
    match shared.server.lock() {
        Ok(mut server) => drop(server.take()),
        Err(error) => eprintln!("the Redis server is not stopped: {error}"),
    }
}

fn url(address: &str, database: usize) -> String {
    format!("redis://{address}/{database}")
}

async fn server_case(case: &Value, database: usize) -> Result<(), Failed> {
    let address = url(&server().map_err(Failed::from)?.address, database);
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
    for (index, step) in array_of(case, "steps")?.iter().enumerate() {
        let mut opened = match store.open(cookie(step)?.as_deref()).await {
            Ok(opened) => opened,
            Err(error) => {
                expect_raises(&error, raises.ok_or_else(|| failed(&error))?, "open")?;
                break;
            }
        };
        expect_found(case, index, opened.found())?;
        let called = calls(&mut opened, step)?;
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
        match called {
            Ok(values) => results.push(Value::Array(values)),
            Err(error) => {
                expect_raises(&error, raises.ok_or_else(|| failed(&error))?, "call")?;
                expect_eq("close after a failure", &opened.close().await, &Ok(None))?;
                break;
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
    let mut trials = vec![
        Trial::test("interface::the_options_name_their_core_type", the_options_name_their_core_type),
        Trial::test("interface::the_cases_of_new_give_its_parameters", the_cases_of_new_give_its_parameters),
        Trial::test("interface::the_cases_of_one_call_reach_their_members", the_cases_of_one_call_reach_their_members),
        Trial::test(
            "interface::the_settings_of_the_cases_name_their_case_by_kind",
            the_settings_of_the_cases_name_their_case_by_kind,
        ),
    ];
    for (index, case) in requests.iter().enumerate() {
        let Some(id) = case.get("id").and_then(Value::as_str) else {
            panic!("a case has no id");
        };
        let model_case = case.clone();
        trials.push(Trial::test(format!("model::{id}"), move || model(&model_case)));
        // A reply that is not bytes and an error reply to a command that the case names come only from a model, so such
        // a case has no trial against a server (SE-10.4).
        if case.get("text_replies").is_some() || case.get("error_replies").is_some() {
            continue;
        }
        let server_case_value = case.clone();
        trials.push(Trial::test(format!("server::{id}"), move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(failed)?;
            runtime.block_on(server_case(&server_case_value, index % 16))
        }));
    }
    let Some(Value::Array(missing)) = contract.get("openMissing") else {
        panic!("contracts/redis-sessions.json has no group openMissing");
    };
    let opens = opens.iter().map(|case| ("open", case)).chain(missing.iter().map(|case| ("openMissing", case)));
    for (group, case) in opens {
        let case = case.clone();
        let Some(id) = case.get("id").and_then(Value::as_str) else {
            panic!("a case has no id");
        };
        let name = format!("{group}::{id}");
        // A value that no u64 holds is refused by the binding (SR-8.2); the bindings run such a case.
        let binding = case.get("binding").is_some();
        trials.push(Trial::test(name, move || open_case(&case)).with_ignored_flag(binding));
    }
    for group in ["opening", "openFaults"] {
        let Some(Value::Array(cases)) = contract.get(group) else {
            panic!("contracts/redis-sessions.json has no group {group}");
        };
        for case in cases.clone() {
            let Some(id) = case.get("id").and_then(Value::as_str) else {
                panic!("a case has no id");
            };
            trials.push(Trial::test(format!("{group}::{id}"), move || opening(&case)));
        }
    }
    let made: [(&str, Group); 5] = [
        ("redis_session.get.made", |case| made_call(case, "get")),
        ("redis_session.set.made", |case| made_call(case, "set")),
        ("redis_session.remove.made", |case| made_call(case, "remove")),
        ("redis_session.renew.made", |case| made_call(case, "renew")),
        ("redis_session.close.made", made_close),
    ];
    for (group, run) in made {
        let Some(Value::Array(cases)) = contract.get(group) else {
            panic!("contracts/redis-sessions.json has no group {group}");
        };
        for case in cases.clone() {
            let Some(id) = case.get("id").and_then(Value::as_str) else {
                panic!("a case has no id");
            };
            trials.push(Trial::test(format!("{group}::{id}"), move || run(&case)));
        }
    }
    let conclusion = libtest_mimic::run(&arguments, trials);
    stop_server();
    conclusion.exit();
}
