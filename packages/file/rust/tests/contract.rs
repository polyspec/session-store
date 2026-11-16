//! Every case of `contracts/file-sessions.json` (SE-10.1), one trial each, on a new temporary directory. A case with
//! `faults` arms the fault injection points that it names for its one call, as a generated binding test does (SE-10.5).
//! A path of a case of one call names a place in that directory, and the case writes its `files` there before it makes
//! the store, as a generated binding test does (SE-10.6). Two trials read the interface contract for the making of the
//! store (SF-1.5).

use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
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

/// The serialize form of a value of the alias `php_value` in the form of a generated binding test (binding G-5): `null`,
/// `bool`, `int` and `list`, whose `value` is the list of its items or, with `repeat`, the one item of each of n lists.
fn php_text(value: &Value) -> Result<String, Failed> {
    match str_of(value, "kind")? {
        "null" => Ok("N;".to_owned()),
        "bool" => {
            let flag = value.get("value").and_then(Value::as_bool).ok_or("a bool value holds no bool")?;
            Ok(format!("b:{};", u8::from(flag)))
        }
        "int" => {
            let number = value.get("value").and_then(Value::as_i64).ok_or("an int value holds no integer")?;
            Ok(format!("i:{number};"))
        }
        "list" => {
            let inner = value.get("value").ok_or("a list value has no value")?;
            if let Some(repeat) = value.get("repeat") {
                let depth = usize::try_from(repeat.as_u64().ok_or("a repeat is no count")?).map_err(failed)?;
                return Ok(format!("{}{}{}", "a:1:{i:0;".repeat(depth), php_text(inner)?, "}".repeat(depth)));
            }
            let items = inner.as_array().ok_or("a list value holds no list")?;
            let mut text = format!("a:{}:{{", items.len());
            for (index, item) in items.iter().enumerate() {
                text.push_str(&format!("i:{index};"));
                text.push_str(&php_text(item)?);
            }
            text.push('}');
            Ok(text)
        }
        other => Err(format!("a value of the kind {other} is not read by this runner").into()),
    }
}

/// The serialize form of a value of a case: a text, or `{"nested": n}`, n arrays that each hold the next at the key 0,
/// the innermost holding null, in the group `requests`, and a value of the alias `php_value` in the cases of one call
/// (SE-10.2).
fn value_text(argument: Option<&Value>) -> Result<String, Failed> {
    match argument {
        Some(Value::String(text)) => Ok(text.clone()),
        Some(value @ Value::Object(form)) if form.contains_key("kind") => php_text(value),
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
    for file in array_of(case, "@files")? {
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
    let store = FileSessions::in_directory(Io::for_generated_call(), directory.path(), lifetime).map_err(failed)?;
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

/// The bytes of a file of a case: a text for its UTF-8, or `{"hex": ...}` (binding GN-13i).
fn file_bytes(content: &Value) -> Result<Vec<u8>, Failed> {
    match content {
        Value::String(text) => Ok(text.as_bytes().to_vec()),
        Value::Object(form) => {
            let hex = form.get("hex").and_then(Value::as_str).ok_or("a file holds no text and no hex")?;
            (0..hex.len())
                .step_by(2)
                .map(|index| {
                    let pair = hex.get(index..index + 2).ok_or("the hex of a file has an odd length")?;
                    u8::from_str_radix(pair, 16).map_err(failed)
                })
                .collect()
        }
        _ => Err("a file holds no text and no hex".into()),
    }
}

/// The place of a path of a case of one call in the directory of the case: `.` is the directory itself, and any other
/// path is relative to it (binding GN-6y).
fn place(root: &Path, path: &str) -> PathBuf {
    if path == "." {
        root.to_path_buf()
    } else {
        root.join(path)
    }
}

/// The directory of a case of one call with the `files` of the case written into it, in the order of the case, each
/// with the mode 0600 after the directories of its path (SE-10.6, binding GN-13i).
fn case_directory(case: &Value) -> Result<tempfile::TempDir, Failed> {
    let directory = tempfile::tempdir().map_err(failed)?;
    if let Some(files) = case.get("@files") {
        let files = files.as_object().ok_or("files is no object")?;
        for (path, content) in files {
            let target = place(directory.path(), path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(failed)?;
            }
            let mut file =
                std::fs::File::options().write(true).create_new(true).mode(0o600).open(&target).map_err(failed)?;
            std::io::Write::write_all(&mut file, &file_bytes(content)?).map_err(failed)?;
        }
    }
    Ok(directory)
}

/// SF-1.2, SF-1.3: the making of the store of a case of the group `open`, whose directory is a place in the directory of
/// the case, as a generated binding test gives it.
fn open(case: &Value) -> Result<(), Failed> {
    let temporary = case_directory(case)?;
    let directory = place(temporary.path(), str_of(case, "directory")?);
    made(case, &directory, temporary.path())
}

/// SF-1.3: the lifetime of a store of a case of the group `file_sessions.lifetime`, the result of that member.
fn lifetime(case: &Value) -> Result<(), Failed> {
    let temporary = case_directory(case)?;
    let directory = place(temporary.path(), str_of(case, "directory")?);
    let store =
        FileSessions::in_directory(Io::for_generated_call(), &directory, case.get("lifetime").and_then(Value::as_u64))
            .map_err(|error| Failed::from(format!("the store is not made: {error}")))?;
    expect_eq("lifetime", &Some(store.lifetime()), &case.get("store_lifetime").and_then(Value::as_u64))
}

/// SF-1.2: the directory of a store of a case of the group `file_sessions.directory`, the result of that member.
fn directory(case: &Value) -> Result<(), Failed> {
    let temporary = case_directory(case)?;
    let directory = place(temporary.path(), str_of(case, "directory")?);
    let store =
        FileSessions::in_directory(Io::for_generated_call(), &directory, case.get("lifetime").and_then(Value::as_u64))
            .map_err(|error| Failed::from(format!("the store is not made: {error}")))?;
    let expected = place(temporary.path(), str_of(case, "store_directory")?);
    expect_eq("directory", &store.directory(), &expected.as_path())
}

/// SF-1.2: the making of the store of a case of the group `openNotAbsolute`, whose directory is given as it is: a path
/// that is not absolute, which no case value of the type `path` of a generated binding test gives (binding GN-6y).
fn open_not_absolute(case: &Value) -> Result<(), Failed> {
    let temporary = case_directory(case)?;
    made(case, Path::new(str_of(case, "directory")?), temporary.path())
}

fn made(case: &Value, directory: &Path, root: &Path) -> Result<(), Failed> {
    let lifetime = case.get("lifetime").and_then(Value::as_u64);
    match (FileSessions::in_directory(Io::for_generated_call(), directory, lifetime), case.get("raises")) {
        (Err(error), Some(raises)) => {
            expect_eq("category", &"value", &str_of(raises, "category")?)?;
            expect_raises(error.kind(), error.status(), &error.to_string(), raises)
        }
        (Ok(store), None) => {
            expect_eq("lifetime", &Some(store.lifetime()), &case.get("store_lifetime").and_then(Value::as_u64))?;
            let expected = place(root, str_of(case, "store_directory")?);
            expect_eq("directory", &store.directory(), &expected.as_path())
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
        Some("EEXIST") => Ok(Some(rustix::io::Errno::EXIST.raw_os_error())),
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

/// The store of the directory of a case of one call (SE-10.5), a place in the directory of the case, which the armed
/// point fails before the directory is read. The directory of the case is kept with the store.
fn store_of(case: &Value) -> Result<(tempfile::TempDir, FileSessions), Failed> {
    let root = case_directory(case)?;
    let directory = place(root.path(), str_of(case, "directory")?);
    let store = FileSessions::in_directory(Io::for_generated_call(), &directory, None).map_err(failed)?;
    Ok((root, store))
}

/// SE-9.6: `open` whose open of the session file fails; only `file_sessions.open` reads the group.
fn open_faults(case: &Value) -> Result<(), Failed> {
    let raises = case.get("raises").ok_or("the case has no raises")?;
    let (_root, store) = store_of(case)?;
    let cookie = latin1(str_of(case, "cookie")?)?;
    match armed(case, "file_sessions.open", || store.open(ProcessWait::for_generated_call(), Some(&cookie), None))? {
        Ok(_) => Err("the session is opened".into()),
        Err(error) => expect_failure(&error, raises),
    }
}

/// SE-9.6: `collect` whose read of the directory fails; only `file_sessions.collect` reads the group.
fn collect_faults(case: &Value) -> Result<(), Failed> {
    let raises = case.get("raises").ok_or("the case has no raises")?;
    let (_root, store) = store_of(case)?;
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

/// The store of the arguments of `file_sessions.in_directory`, whose directory is a place in the directory of the case.
fn store_with(args: &Value, root: &Path) -> Result<FileSessions, Failed> {
    let directory = place(root, str_of(args, "directory")?);
    FileSessions::in_directory(Io::for_generated_call(), &directory, args.get("lifetime").and_then(Value::as_u64))
        .map_err(failed)
}

/// A session of a request that earlier calls made (binding GN-13f): `{"new": "file_sessions.open", "receiver":
/// {"new": "file_sessions.in_directory", "args": {...}}, "args": {"cookie", "cancel"}}`.
fn made_session(value: &Value, root: &Path) -> Result<FileSession, Failed> {
    expect_eq("new", &str_of(value, "new")?, &"file_sessions.open")?;
    let store = value.get("receiver").ok_or("an opening has no receiver")?;
    expect_eq("new", &str_of(store, "new")?, &"file_sessions.in_directory")?;
    let store = store_with(store.get("args").ok_or("a store has no args")?, root)?;
    let args = value.get("args").ok_or("an opening has no args")?;
    let (_trigger, cancel) = cancel_of(args.get("cancel").ok_or("an opening has no cancel")?)?;
    let cookie = cookie_of(args)?;
    let mut session = store.open(ProcessWait::for_generated_call(), cookie.as_deref(), Some(cancel)).map_err(failed)?;
    for then in value.get("then").and_then(Value::as_array).into_iter().flatten() {
        let name =
            str_of(then, "call")?.strip_prefix("file_session.").ok_or("a call of then is no call of a session")?;
        let mut arguments = Vec::new();
        for key in ["key", "value"] {
            if let Some(argument) = then.get("args").and_then(|args| args.get(key)) {
                arguments.push(argument.clone());
            }
        }
        let pair = Value::Array(vec![Value::String(name.to_owned()), Value::Array(arguments)]);
        if let Err(error) = call(&mut session, &pair)? {
            return Err(format!("a call of then fails: {error}").into());
        }
    }
    Ok(session)
}

/// SF-4.4, SE-6.8: the close of a session that `open` made and `then` gave a value, with the faults of the case where it
/// names them; only `file_session.close` reads the group.
fn made_close(case: &Value) -> Result<(), Failed> {
    let raises = case.get("raises").ok_or("the case has no raises")?;
    let root = case_directory(case)?;
    let session = made_session(case.get("receiver").ok_or("the case has no receiver")?, root.path())?;
    let close = || session.close(Io::for_generated_call());
    let closed = if case.get("faults").is_some() { armed(case, "file_session.close", close)? } else { close() };
    match closed {
        Ok(id) => Err(format!("the session is closed as {id:?}").into()),
        Err(error) => expect_failure(&error, raises),
    }
}

/// SE-7.1: `found` of a session that `open` made, which the case states in `result`; only `file_session.found` reads
/// the group.
fn made_found(case: &Value) -> Result<(), Failed> {
    let expected = case.get("result").ok_or("the case has no result")?.as_str();
    let root = case_directory(case)?;
    let session = made_session(case.get("receiver").ok_or("the case has no receiver")?, root.path())?;
    expect_eq("found", &session.found(), &expected)
}

/// SE-5.4, SE-6.3, SE-6.4, SE-7.2: one call of a record session of a session that `open` made, with the faults of the
/// case where it names them, and with the stored file of the case, which the call decodes, where it names one; the
/// group of each member is read by that member alone.
fn made_call(case: &Value, name: &str) -> Result<(), Failed> {
    let raises = case.get("raises").ok_or("the case has no raises")?;
    let root = case_directory(case)?;
    let mut session = made_session(case.get("receiver").ok_or("the case has no receiver")?, root.path())?;
    let mut arguments = Vec::new();
    for key in ["key", "value"] {
        if let Some(argument) = case.get(key) {
            arguments.push(argument.clone());
        }
    }
    let pair = Value::Array(vec![Value::String(name.to_owned()), Value::Array(arguments)]);
    let mut run = || call(&mut session, &pair);
    let called =
        if case.get("faults").is_some() { armed(case, &format!("file_session.{name}"), run)?? } else { run()? };
    match called {
        Ok(result) => Err(format!("the call gives {result}").into()),
        Err(error) => expect_failure(&error, raises),
    }
}

/// SF-3.8: an opening whose cancellation fired before the call; only `file_sessions.open` reads the group.
fn open_cancel(case: &Value) -> Result<(), Failed> {
    let raises = case.get("raises").ok_or("the case has no raises")?;
    let root = case_directory(case)?;
    let store = store_with(case, root.path())?;
    let (_trigger, cancel) = cancel_of(case.get("cancel").ok_or("the case has no cancel")?)?;
    let cookie = cookie_of(case)?;
    match store.open(ProcessWait::for_generated_call(), cookie.as_deref(), Some(cancel)) {
        Ok(_) => Err("the session is opened".into()),
        Err(error) => expect_failure(&error, raises),
    }
}

type Group = fn(&Value) -> Result<(), Failed>;

/// The members of the units of the interface contract.
fn members(interface: &Value) -> Result<Vec<&Value>, Failed> {
    let units = interface.get("units").and_then(Value::as_array).ok_or("the contract has no list units")?;
    Ok(units.iter().filter_map(|unit| unit.get("members").and_then(Value::as_array)).flatten().collect())
}

/// SF-1.5: no member of the contract that waits is a constructor or a property, because a language whose constructor
/// or property cannot be awaited has no form of one that waits (binding GN-6m).
fn no_constructor_and_no_property_waits() -> Result<(), Failed> {
    let interface: Value = serde_json::from_str(INTERFACE).map_err(failed)?;
    let members = members(&interface)?;
    if members.is_empty() {
        return Err("the contract has no member".into());
    }
    let mut found = Vec::new();
    for member in members {
        let wait = member.get("wait").and_then(Value::as_str);
        let kind = member.get("kind").and_then(Value::as_str);
        if !matches!(wait, None | Some("none")) && matches!(kind, Some("constructor" | "property")) {
            found.push(format!("{}: a {kind:?} with wait {wait:?}", str_of(member, "id")?));
        }
    }
    expect_eq("the members that wait in a form that no language awaits", &found, &Vec::new())
}

/// SF-1.5: the store of files is made by the static member `file_sessions.in_directory`, which returns the store and
/// names `FileSessions::in_directory`, and the class has no constructor.
fn the_store_of_files_is_made_by_a_static_member() -> Result<(), Failed> {
    let interface: Value = serde_json::from_str(INTERFACE).map_err(failed)?;
    let units = interface.get("units").and_then(Value::as_array).ok_or("the contract has no list units")?;
    let unit = units
        .iter()
        .find(|unit| unit.get("id").and_then(Value::as_str) == Some("file_sessions"))
        .ok_or("the contract has no unit file_sessions")?;
    expect_eq("constructible of file_sessions", &unit.get("constructible"), &Some(&Value::Bool(false)))?;
    let members = unit.get("members").and_then(Value::as_array).ok_or("file_sessions has no members")?;
    let constructors: Vec<&str> = members
        .iter()
        .filter(|member| member.get("kind").and_then(Value::as_str) == Some("constructor"))
        .filter_map(|member| member.get("id").and_then(Value::as_str))
        .collect();
    expect_eq("the constructors of file_sessions", &constructors, &Vec::new())?;
    let making = members
        .iter()
        .find(|member| member.get("id").and_then(Value::as_str) == Some("file_sessions.in_directory"))
        .ok_or("the contract has no member file_sessions.in_directory")?;
    expect_eq("kind", &str_of(making, "kind")?, &"static")?;
    expect_eq("returns", &making.get("returns"), &Some(&serde_json::json!({"unit": "file_sessions"})))?;
    expect_eq("wait", &str_of(making, "wait")?, &"blocking_io")?;
    let core = making.get("core").ok_or("file_sessions.in_directory has no core")?;
    expect_eq("core.path", &str_of(core, "path")?, &"polyspec_session_store::FileSessions::in_directory")
}

/// The cases that a generated binding test runs (binding G-5): the interface contract declares no group `requests`,
/// whose cases hold several requests that only the Rust tests run (SE-10.6); every group that it declares for a member
/// of `file_session` writes each PHP value of a call as a value of the alias `php_value` (`{"kind": <kind>, ...}`),
/// `file_session.found` names such a group, a case of one call of `file_session.close` raises `record_write` (SE-6.8),
/// and a case of one call of each of `get`, `set`, `remove` and `renew` raises `record` and `record_not_array` from the
/// stored file of the case (SE-6.3, SE-6.4).
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
        if group == "requests" {
            return Err(format!("{member} names the group requests, whose cases hold several requests").into());
        }
        if member.starts_with("file_session.") {
            groups.push((member, group));
        }
    }
    if !groups.iter().any(|(member, _)| *member == "file_session.found") {
        return Err("file_session.found names no group of one call".into());
    }
    let mut closes = false;
    let mut stored: Vec<(&str, &str)> = Vec::new();
    for (member, group) in &groups {
        let cases = contract.get(*group).and_then(Value::as_array).ok_or_else(|| format!("no group {group}"))?;
        for case in cases {
            let mut found = Vec::new();
            values(case, &mut found);
            if let Some(value) = found.iter().find(|value| value.get("kind").and_then(Value::as_str).is_none()) {
                let id = str_of(case, "id")?;
                return Err(format!("the case {id} of {group} gives the value {value}, no php_value").into());
            }
            let kind = case.get("raises").and_then(|raises| raises.get("kind")).and_then(Value::as_str);
            if *member == "file_session.close" && kind == Some("record_write") {
                closes = true;
            }
            if let Some(kind @ ("record" | "record_not_array")) = kind {
                if case.get("@files").is_some() {
                    stored.push((member, kind));
                }
            }
        }
    }
    if !closes {
        return Err("no case of one call of file_session.close raises record_write".into());
    }
    for name in ["get", "set", "remove", "renew"] {
        for kind in ["record", "record_not_array"] {
            let member = format!("file_session.{name}");
            if !stored.contains(&(member.as_str(), kind)) {
                return Err(format!("no case of one call of {member} raises {kind} from a stored file").into());
            }
        }
    }
    Ok(())
}

/// SF-3.8: the cancellation of `file_sessions.open` is an `optional` of `cancel` whose default the core decides, so a
/// call may leave it out, and the core takes it by value (binding GN-6i, GN-6aq).
fn the_cancellation_of_open_is_an_optional() -> Result<(), Failed> {
    let interface: Value = serde_json::from_str(INTERFACE).map_err(failed)?;
    let units = interface.get("units").and_then(Value::as_array).ok_or("the contract has no list units")?;
    let opening = units
        .iter()
        .filter_map(|unit| unit.get("members").and_then(Value::as_array))
        .flatten()
        .find(|member| member.get("id").and_then(Value::as_str) == Some("file_sessions.open"))
        .ok_or("the contract has no member file_sessions.open")?;
    let parameters =
        opening.get("parameters").and_then(Value::as_array).ok_or("file_sessions.open has no parameters")?;
    let cancel = parameters
        .iter()
        .find(|parameter| parameter.get("name") == Some(&serde_json::json!(["cancel"])))
        .ok_or("file_sessions.open has no parameter cancel")?;
    expect_eq("type", &cancel.get("type"), &Some(&serde_json::json!({"optional": "cancel"})))?;
    expect_eq("default", &cancel.get("default"), &Some(&serde_json::json!({"computed": "SF-3.8"})))
}

fn main() {
    let arguments = Arguments::from_args();
    let contract: Value = match serde_json::from_str(CONTRACT) {
        Ok(contract) => contract,
        Err(error) => panic!("contracts/file-sessions.json is no JSON: {error}"),
    };
    let groups: [(&str, Group); 18] = [
        ("requests", requests),
        ("open", open),
        ("file_sessions.lifetime", lifetime),
        ("file_sessions.directory", directory),
        ("openNotAbsolute", open_not_absolute),
        ("openFaults", open_faults),
        ("collectFaults", collect_faults),
        ("file_session.get.made", |case| made_call(case, "get")),
        ("file_session.set.made", |case| made_call(case, "set")),
        ("file_session.remove.made", |case| made_call(case, "remove")),
        ("file_session.renew.made", |case| made_call(case, "renew")),
        ("file_sessions.open.cancel", open_cancel),
        ("file_session.close.made", made_close),
        ("file_session.found.made", made_found),
        ("file_session.get.stored", |case| made_call(case, "get")),
        ("file_session.set.stored", |case| made_call(case, "set")),
        ("file_session.remove.stored", |case| made_call(case, "remove")),
        ("file_session.renew.stored", |case| made_call(case, "renew")),
    ];
    let mut trials = vec![
        Trial::test("interface::no_constructor_and_no_property_waits", no_constructor_and_no_property_waits),
        Trial::test(
            "interface::the_store_of_files_is_made_by_a_static_member",
            the_store_of_files_is_made_by_a_static_member,
        ),
        Trial::test("interface::the_cancellation_of_open_is_an_optional", the_cancellation_of_open_is_an_optional),
        Trial::test("interface::the_cases_of_one_call_reach_their_members", the_cases_of_one_call_reach_their_members),
    ];
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
