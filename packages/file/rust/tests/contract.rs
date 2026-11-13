//! Every case of `contracts/file-sessions.json` (SE-10.1), one trial each, on a new temporary directory. A case with
//! `faults` arms the fault injection points that it names for its one call, as a generated binding test does (SE-10.5).

use std::path::Path;
use std::time::{Duration, SystemTime};

use binding_core::{Failure, Io, Kinds, ProcessWait};
use libtest_mimic::{Arguments, Failed, Trial};
use polyspec_php_serialization::{serialize, unserialize};
use polyspec_session_store_file::{FileSession, FileSessions};
use serde_json::Value;

const CONTRACT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../contracts/file-sessions.json"));
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

fn failed(error: impl std::fmt::Display) -> Failed {
    Failed::from(error.to_string())
}

/// The files of the directory by name, each created identifier written as `#n` (SE-10.2).
fn records(directory: &Path, created: &[String]) -> Result<Vec<(String, String)>, Failed> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(failed)? {
        let entry = entry.map_err(failed)?;
        let mut name = entry.file_name().to_string_lossy().into_owned();
        for (index, id) in created.iter().enumerate() {
            if name == format!("sess_{id}") {
                name = format!("sess_#{index}");
            }
        }
        found.push((name, text_of(&std::fs::read(entry.path()).map_err(failed)?)));
    }
    found.sort();
    Ok(found)
}

fn expected_records(value: &Value) -> Result<Vec<(String, String)>, Failed> {
    value
        .as_array()
        .ok_or("records is no list")?
        .iter()
        .map(|pair| match pair.as_array().map(Vec::as_slice) {
            Some([Value::String(name), Value::String(bytes)]) => Ok((name.clone(), bytes.clone())),
            _ => Err(Failed::from("a record is no pair of texts")),
        })
        .collect()
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

/// The result of one call: the serialize form of a value, or null.
fn call(session: &mut FileSession, call: &Value) -> Result<Result<Value, polyspec_session_store_core::Error>, Failed> {
    use polyspec_session_store_core::Store;
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
    match result {
        Ok(Some(bytes)) => Ok(Ok(Value::String(text_of(&bytes.map_err(failed)?)))),
        Ok(None) => Ok(Ok(Value::Null)),
        Err(Failure::Kind(error)) => Ok(Err(error)),
        Err(Failure::Raised(_) | Failure::Callback(_)) => Err("the store raised an exception of its caller".into()),
    }
}

fn expect_raises(kind: &str, status: Option<u16>, message: &str, raises: &Value) -> Result<(), Failed> {
    expect_eq("kind", &kind, &str_of(raises, "kind")?)?;
    expect_eq("status", &status.map(u64::from), &raises.get("status").and_then(Value::as_u64))?;
    if let Some(Value::String(expected)) = raises.get("message") {
        expect_eq("message", &message, &expected.as_str())?;
    }
    Ok(())
}

fn requests(case: &Value) -> Result<(), Failed> {
    let directory = tempfile::tempdir().map_err(failed)?;
    for file in array_of(case, "files")? {
        let name = match file.get("identifier").and_then(Value::as_str) {
            Some(id) => format!("sess_{id}"),
            None => str_of(file, "name")?.to_owned(),
        };
        let path = directory.path().join(name);
        std::fs::write(&path, latin1(str_of(file, "bytes")?)?).map_err(failed)?;
        let age = file.get("age").and_then(Value::as_u64).ok_or("a file has no age")?;
        let modified = SystemTime::now().checked_sub(Duration::from_secs(age)).ok_or("the age passes the clock")?;
        std::fs::File::options()
            .write(true)
            .open(&path)
            .and_then(|file| file.set_modified(modified))
            .map_err(failed)?;
    }
    let lifetime = case.get("lifetime").and_then(Value::as_u64);
    let store = FileSessions::new(Io::for_generated_call(), directory.path(), lifetime).map_err(failed)?;
    let raises = case.get("raises");
    let failing_step = raises.and_then(|raises| raises.get("step")).and_then(Value::as_u64);
    let mut created: Vec<String> = Vec::new();
    for (index, step) in array_of(case, "steps")?.iter().enumerate() {
        let failing = failing_step == Some(u64::try_from(index).map_err(failed)?);
        let mut step_created = None;
        let result = if step.get("collect").is_some() {
            Value::from(store.collect(Io::for_generated_call()).map_err(failed)?)
        } else {
            let cookie = match step.get("cookie") {
                Some(Value::String(cookie)) => match cookie.strip_prefix('#') {
                    Some(number) => {
                        let number: usize = number.parse().map_err(failed)?;
                        Some(created.get(number).ok_or("the cookie names no created identifier")?.as_bytes().to_vec())
                    }
                    None => Some(latin1(cookie)?),
                },
                _ => None,
            };
            let mut session = store.open(ProcessWait::for_generated_call(), cookie.as_deref(), None).map_err(failed)?;
            // SE-7.1: the identifier that the opening found, where the case states it; a created one is `#n`.
            if let Some(expected) = case.get("found").and_then(Value::as_array).and_then(|found| found.get(index)) {
                let found = session.found().map(|id| match created.iter().position(|made| made == id) {
                    Some(number) => format!("#{number}"),
                    None => id.to_owned(),
                });
                expect_eq("found", &found.as_deref(), &expected.as_str())?;
            }
            let mut results = Vec::new();
            for call_value in array_of(step, "calls")? {
                match call(&mut session, call_value)? {
                    Ok(value) => results.push(value),
                    Err(error) => {
                        let Some(raises) = raises.filter(|_| failing) else {
                            return Err(format!("a call fails: {error}").into());
                        };
                        expect_eq("at", &"call", &str_of(raises, "at")?)?;
                        expect_raises(error.kind(), error.status(), &error.to_string(), raises)?;
                        expect_eq("close after a failure", &session.close(Io::for_generated_call()), &Ok(None))?;
                        let expected = expected_records(array_of(case, "records")?.get(index).ok_or("no records")?)?;
                        return expect_eq("records", &records(directory.path(), &created)?, &expected);
                    }
                }
            }
            match session.close(Io::for_generated_call()) {
                Ok(Some(id)) => {
                    step_created = Some(format!("#{}", created.len()));
                    created.push(id);
                }
                Ok(None) => {}
                Err(error) => {
                    let Some(raises) = raises.filter(|_| failing) else {
                        return Err(format!("the close fails: {error}").into());
                    };
                    expect_eq("at", &"close", &str_of(raises, "at")?)?;
                    expect_raises(error.kind(), error.status(), &error.to_string(), raises)?;
                    let expected = expected_records(array_of(case, "records")?.get(index).ok_or("no records")?)?;
                    return expect_eq("records", &records(directory.path(), &created)?, &expected);
                }
            }
            Value::Array(results)
        };
        if failing {
            return Err("the step does not fail".into());
        }
        expect_eq("result", &Some(&result), &array_of(case, "results")?.get(index))?;
        let expected_created = array_of(case, "created")?.get(index).and_then(Value::as_str).map(str::to_owned);
        expect_eq("created", &step_created, &expected_created)?;
        let expected = expected_records(array_of(case, "records")?.get(index).ok_or("no records of the step")?)?;
        expect_eq("records", &records(directory.path(), &created)?, &expected)?;
        if let Some(Value::Bool(true)) =
            case.get("touched").and_then(Value::as_array).and_then(|touched| touched.get(index))
        {
            for entry in std::fs::read_dir(directory.path()).map_err(failed)? {
                let modified =
                    entry.map_err(failed)?.metadata().and_then(|metadata| metadata.modified()).map_err(failed)?;
                let age = SystemTime::now().duration_since(modified).map_err(failed)?;
                if age > Duration::from_secs(10) {
                    return Err(format!("the file was modified {age:?} ago").into());
                }
            }
        }
    }
    Ok(())
}

fn open(case: &Value) -> Result<(), Failed> {
    let temporary = tempfile::tempdir().map_err(failed)?;
    std::fs::write(temporary.path().join("file"), b"").map_err(failed)?;
    let directory = str_of(case, "directory")?.replace("<temporary>", &temporary.path().to_string_lossy());
    let lifetime = case.get("lifetime").and_then(Value::as_u64);
    match (FileSessions::new(Io::for_generated_call(), Path::new(&directory), lifetime), case.get("raises")) {
        (Err(error), Some(raises)) => {
            expect_eq("category", &"value", &str_of(raises, "category")?)?;
            expect_raises(error.kind(), error.status(), &error.to_string(), raises)
        }
        (Ok(store), None) => {
            expect_eq("lifetime", &Some(store.lifetime()), &case.get("store_lifetime").and_then(Value::as_u64))?;
            let expected = str_of(case, "store_directory")?.replace("<temporary>", &temporary.path().to_string_lossy());
            expect_eq("directory", &store.directory(), &Path::new(&expected))
        }
        (Ok(_), Some(_)) => Err("the store is made".into()),
        (Err(error), None) => Err(format!("the store is not made: {error}").into()),
    }
}

/// A runner of the cases of one group.
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

/// The value of the errno symbol of a case on this machine (binding design G-5).
fn errno_of(raises: &Value) -> Result<Option<i32>, Failed> {
    match raises.get("errno").and_then(Value::as_str) {
        None => Ok(None),
        Some("EIO") => Ok(Some(rustix::io::Errno::IO.raw_os_error())),
        Some(symbol) => Err(format!("the runner knows no errno {symbol}").into()),
    }
}

/// The failure of a case that arms faults: its kind, status, category and errno, and its message where a rule states
/// one (SE-9.1).
fn expect_failure(error: &polyspec_session_store_core::Error, raises: &Value) -> Result<(), Failed> {
    expect_raises(error.kind(), error.status(), &error.to_string(), raises)?;
    expect_eq("category", &category(error.kind())?.as_str(), &str_of(raises, "category")?)?;
    expect_eq("errno", &error.errno(), &errno_of(raises)?)
}

/// The store of the directory of a case of one call (SE-10.5), which the armed point fails before the directory is read.
fn store_of(case: &Value) -> Result<FileSessions, Failed> {
    FileSessions::new(Io::for_generated_call(), Path::new(str_of(case, "directory")?), None).map_err(failed)
}

/// SE-9.6: `open` whose open of the session file fails; only `file_sessions.open` reads the group.
fn open_faults(case: &Value) -> Result<(), Failed> {
    let raises = case.get("raises").ok_or("the case has no raises")?;
    let store = store_of(case)?;
    let cookie = latin1(str_of(case, "cookie")?)?;
    match armed(case, "file_sessions.open", || store.open(ProcessWait::for_generated_call(), Some(&cookie), None))? {
        Ok(_) => Err("the session is opened".into()),
        Err(error) => expect_failure(&error, raises),
    }
}

/// SE-9.6: `collect` whose read of the directory fails; only `file_sessions.collect` reads the group.
fn collect_faults(case: &Value) -> Result<(), Failed> {
    let raises = case.get("raises").ok_or("the case has no raises")?;
    let store = store_of(case)?;
    match armed(case, "file_sessions.collect", || store.collect(Io::for_generated_call()))? {
        Ok(removed) => Err(format!("collect removes {removed} files").into()),
        Err(error) => expect_failure(&error, raises),
    }
}

/// A cancellation of a case (binding GN-13f): `{"cancel": "fired"}`, which fired before the call, or `{"cancel":
/// "open"}`, which does not fire. The trigger is kept with it, so that it fires no later.
fn cancel_of(value: &Value) -> Result<(binding_core::CancelTrigger, binding_core::Cancel), Failed> {
    let (trigger, cancel) = binding_core::cancel_pair();
    match str_of(value, "cancel")? {
        "fired" => trigger.cancel(),
        "open" => {}
        other => return Err(format!("a cancellation is fired or open, not {other}").into()),
    }
    Ok((trigger, cancel))
}

/// The cookie of the arguments of an opening: null for none, or the text whose characters are its bytes.
fn cookie_of(args: &Value) -> Result<Option<Vec<u8>>, Failed> {
    match args.get("cookie") {
        Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(latin1(text)?)),
        _ => Err("an opening has no cookie".into()),
    }
}

/// The store of the arguments of `file_sessions.new`.
fn store_with(args: &Value) -> Result<FileSessions, Failed> {
    let directory = Path::new(str_of(args, "directory")?);
    FileSessions::new(Io::for_generated_call(), directory, args.get("lifetime").and_then(Value::as_u64)).map_err(failed)
}

/// A session of a request that earlier calls made (binding GN-13f): `{"new": "file_sessions.open", "receiver":
/// {"new": "file_sessions.new", "args": {...}}, "args": {"cookie", "cancel"}}`.
fn made_session(value: &Value) -> Result<FileSession, Failed> {
    expect_eq("new", &str_of(value, "new")?, &"file_sessions.open")?;
    let store = value.get("receiver").ok_or("an opening has no receiver")?;
    expect_eq("new", &str_of(store, "new")?, &"file_sessions.new")?;
    let store = store_with(store.get("args").ok_or("a store has no args")?)?;
    let args = value.get("args").ok_or("an opening has no args")?;
    let (_trigger, cancel) = cancel_of(args.get("cancel").ok_or("an opening has no cancel")?)?;
    let cookie = cookie_of(args)?;
    store.open(ProcessWait::for_generated_call(), cookie.as_deref(), Some(&cancel)).map_err(failed)
}

/// SE-5.4, SE-7.2: one call of a record session of a session that `open` made, with the faults of the case; the group
/// of each member is read by that member alone.
fn made_call(case: &Value, name: &str) -> Result<(), Failed> {
    let raises = case.get("raises").ok_or("the case has no raises")?;
    let mut session = made_session(case.get("receiver").ok_or("the case has no receiver")?)?;
    let mut arguments = Vec::new();
    for key in ["key", "value"] {
        if let Some(argument) = case.get(key) {
            arguments.push(argument.clone());
        }
    }
    let pair = Value::Array(vec![Value::String(name.to_owned()), Value::Array(arguments)]);
    match armed(case, &format!("file_session.{name}"), || call(&mut session, &pair))?? {
        Ok(result) => Err(format!("the call gives {result}").into()),
        Err(error) => expect_failure(&error, raises),
    }
}

/// SF-3.8: an opening whose cancellation fired before the call; only `file_sessions.open` reads the group.
fn open_cancel(case: &Value) -> Result<(), Failed> {
    let raises = case.get("raises").ok_or("the case has no raises")?;
    let store = store_with(case)?;
    let (_trigger, cancel) = cancel_of(case.get("cancel").ok_or("the case has no cancel")?)?;
    let cookie = cookie_of(case)?;
    match store.open(ProcessWait::for_generated_call(), cookie.as_deref(), Some(&cancel)) {
        Ok(_) => Err("the session is opened".into()),
        Err(error) => expect_failure(&error, raises),
    }
}

type Group = fn(&Value) -> Result<(), Failed>;

fn main() {
    let arguments = Arguments::from_args();
    let contract: Value = match serde_json::from_str(CONTRACT) {
        Ok(contract) => contract,
        Err(error) => panic!("contracts/file-sessions.json is no JSON: {error}"),
    };
    let groups: [(&str, Group); 9] = [
        ("requests", requests),
        ("open", open),
        ("openFaults", open_faults),
        ("collectFaults", collect_faults),
        ("file_session.get.made", |case| made_call(case, "get")),
        ("file_session.set.made", |case| made_call(case, "set")),
        ("file_session.remove.made", |case| made_call(case, "remove")),
        ("file_session.renew.made", |case| made_call(case, "renew")),
        ("file_sessions.open.cancel", open_cancel),
    ];
    let mut trials = Vec::new();
    for (group, run) in groups {
        let Some(Value::Array(cases)) = contract.get(group) else {
            panic!("contracts/file-sessions.json has no group {group}");
        };
        for case in cases.clone() {
            let Some(id) = case.get("id").and_then(Value::as_str) else {
                panic!("a case of the group {group} has no id");
            };
            let name = format!("{group}::{id}");
            // A value that no u64 holds is refused by the binding (SF-8.2); the bindings run such a case.
            let binding = case.get("binding").is_some();
            trials.push(Trial::test(name, move || run(&case)).with_ignored_flag(binding));
        }
    }
    libtest_mimic::run(&arguments, trials).exit();
}
