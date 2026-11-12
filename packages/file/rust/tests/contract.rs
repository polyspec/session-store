//! Every case of `contracts/file-sessions.json` (SE-10.1), one trial each, on a new temporary directory.

use std::path::Path;
use std::time::{Duration, SystemTime};

use bindgen_core::{Failure, Io, Kinds, ProcessWait};
use libtest_mimic::{Arguments, Failed, Trial};
use polyspec_php_serialization::{serialize, unserialize};
use polyspec_session_store_file::{FileSession, FileSessions};
use serde_json::Value;

const CONTRACT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../contracts/file-sessions.json"));

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
            let value = unserialize(&latin1(text(1)?)?).map_err(failed)?;
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
                    return expect_raises(error.kind(), error.status(), &error.to_string(), raises);
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
            expect_eq("lifetime", &Some(store.lifetime()), &case.get("store_lifetime").and_then(Value::as_u64))
        }
        (Ok(_), Some(_)) => Err("the store is made".into()),
        (Err(error), None) => Err(format!("the store is not made: {error}").into()),
    }
}

/// A runner of the cases of one group.
type Group = fn(&Value) -> Result<(), Failed>;

fn main() {
    let arguments = Arguments::from_args();
    let contract: Value = match serde_json::from_str(CONTRACT) {
        Ok(contract) => contract,
        Err(error) => panic!("contracts/file-sessions.json is no JSON: {error}"),
    };
    let groups: [(&str, Group); 2] = [("requests", requests), ("open", open)];
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
