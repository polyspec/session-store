//! Every case of `contracts/sessions.json` (SE-10.1), one trial each. When the environment names a PHP binary (`PHP`),
//! every record that a case writes is also read and written again by PHP (SE-10.3).

use std::process::Command;

use bindgen_core::{Failure, Kinds};
use libtest_mimic::{Arguments, Failed, Trial};
use polyspec_php_serialize::{serialize, unserialize, PhpValue};
use polyspec_sessions_core::{accept, Found, MemoryStore, Outcome, RecordSession, Store};
use serde_json::Value;

const CONTRACT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../contracts/sessions.json"));

/// The bytes of a byte string of a case: each character U+0000 to U+00FF is its byte (SE-10.2).
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

fn value_of(text: &str) -> Result<PhpValue, Failed> {
    unserialize(&latin1(text)?).map_err(|error| Failed::from(format!("{text:?} is no PHP value: {error}")))
}

fn written(value: &PhpValue) -> Result<String, Failed> {
    serialize(value).map(|bytes| text_of(&bytes)).map_err(|error| Failed::from(error.to_string()))
}

fn str_of<'a>(case: &'a Value, key: &str) -> Result<&'a str, Failed> {
    case.get(key).and_then(Value::as_str).ok_or_else(|| Failed::from(format!("the case has no text {key}")))
}

fn calls(case: &Value) -> Result<Vec<(String, Vec<String>)>, Failed> {
    let Some(Value::Array(calls)) = case.get("calls") else {
        return Err("the case has no calls".into());
    };
    calls
        .iter()
        .map(|call| match call.as_array().map(Vec::as_slice) {
            Some([Value::String(name), Value::Array(arguments)]) => Ok((
                name.clone(),
                arguments
                    .iter()
                    .map(|argument| {
                        argument.as_str().map(str::to_owned).ok_or_else(|| Failed::from("an argument is no text"))
                    })
                    .collect::<Result<_, _>>()?,
            )),
            _ => Err(Failed::from("a call is no pair of a name and arguments")),
        })
        .collect()
}

/// Runs one call on a store and gives its result as the case writes it: the serialize form of a value, or null.
fn run(
    store: &mut dyn Store,
    name: &str,
    arguments: &[String],
) -> Result<Result<Value, polyspec_sessions_core::Error>, Failed> {
    let result = match (name, arguments) {
        ("get", [key]) => store.get(key).map(|value| value.map(|value| written(&value))),
        ("set", [key, value]) => store.set(key, &value_of(value)?).map(|()| None),
        ("remove", [key]) => store.remove(key).map(|()| None),
        ("renew", []) => store.renew().map(|()| None),
        _ => return Err(format!("unknown call {name}").into()),
    };
    match result {
        Ok(Some(text)) => Ok(Ok(Value::String(text?))),
        Ok(None) => Ok(Ok(Value::Null)),
        Err(Failure::Kind(error)) => Ok(Err(error)),
        Err(Failure::Raised(_)) => Err("a store of this crate raised a language exception".into()),
    }
}

fn expect_eq<T: PartialEq + std::fmt::Debug>(what: &str, actual: &T, expected: &T) -> Result<(), Failed> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{what}: {actual:?}, expected {expected:?}").into())
    }
}

/// SE-10.3: PHP reads the record and writes the same bytes.
fn php_agrees(record: &[u8]) -> Result<(), Failed> {
    let Ok(php) = std::env::var("PHP") else {
        return Ok(());
    };
    let output = Command::new(php)
        .args(["-r", "$r = stream_get_contents(STDIN); echo serialize(unserialize($r)) === $r ? 'same' : 'other';"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() {
                std::io::Write::write_all(&mut stdin, record)?;
            }
            child.wait_with_output()
        })
        .map_err(|error| Failed::from(format!("PHP does not run: {error}")))?;
    expect_eq("PHP writes the record back", &output.stdout, &b"same".to_vec())
}

fn identifier(case: &Value) -> Result<(), Failed> {
    let cookie = latin1(str_of(case, "cookie")?)?;
    let expected = case.get("identifier").and_then(Value::as_str).map(str::to_owned);
    expect_eq("identifier", &accept(&cookie), &expected)
}

fn results(case: &Value) -> Result<Vec<Value>, Failed> {
    match case.get("results") {
        Some(Value::Array(results)) => Ok(results.clone()),
        _ => Err("the case has no results".into()),
    }
}

fn store(case: &Value) -> Result<(), Failed> {
    let calls = calls(case)?;
    let expected = results(case)?;
    let mut memory = MemoryStore::new();
    let mut session = RecordSession::new(None);
    let stores: [&mut dyn Store; 2] = [&mut memory, &mut session];
    for store in stores {
        let mut actual = Vec::new();
        for (name, arguments) in &calls {
            actual.push(run(store, name, arguments)?.map_err(|error| Failed::from(error.to_string()))?);
        }
        expect_eq("results", &actual, &expected)?;
    }
    expect_eq("renewals", &Some(memory.renewals()), &case.get("renewals").and_then(Value::as_u64))?;
    let record = latin1(str_of(case, "record")?)?;
    match session.finish() {
        Ok(Outcome::Write { record: actual, new: true, removes: None, .. }) => {
            expect_eq("record", &text_of(&actual), &text_of(&record))?;
            php_agrees(&actual)
        }
        other => Err(format!("the record session gives {other:?}").into()),
    }
}

fn record(case: &Value) -> Result<(), Failed> {
    let found = match case.get("found") {
        Some(Value::String(record)) => {
            Some(Found { id: str_of(case, "identifier")?.to_owned(), record: latin1(record)? })
        }
        _ => None,
    };
    let mut session = RecordSession::new(found.clone());
    let calls = calls(case)?;
    if let Some(raises) = case.get("raises") {
        let mut failure = None;
        for (index, (name, arguments)) in calls.iter().enumerate() {
            match run(&mut session, name, arguments)? {
                Ok(_) if index + 1 < calls.len() => {}
                Ok(value) => return Err(format!("the last call gives {value}, not a failure").into()),
                Err(error) => failure = Some(error),
            }
        }
        let Some(error) = failure else {
            return Err("no call fails".into());
        };
        return expect_raises(&error, raises);
    }
    let mut actual = Vec::new();
    for (name, arguments) in &calls {
        actual.push(run(&mut session, name, arguments)?.map_err(|error| Failed::from(error.to_string()))?);
    }
    expect_eq("results", &actual, &results(case)?)?;
    let outcome = session.finish().map_err(|error| Failed::from(error.to_string()))?;
    let expected = case.get("outcome").ok_or("the case has no outcome")?;
    match (outcome, str_of(expected, "kind")?) {
        (Outcome::Unused, "unused") => Ok(()),
        (Outcome::Unchanged { id }, "unchanged") => {
            expect_eq("identifier", &Some(id.as_str()), &found.as_ref().map(|found| found.id.as_str()))
        }
        (Outcome::Write { id, record, new, removes }, "write") => {
            expect_eq("record", &text_of(&record), &text_of(&latin1(str_of(expected, "record")?)?))?;
            expect_eq("new", &Some(new), &expected.get("new").and_then(Value::as_bool))?;
            let found_id = found.as_ref().map(|found| found.id.clone());
            let expected_removes = match expected.get("removes").and_then(Value::as_bool) {
                Some(true) => found_id.clone(),
                Some(false) => None,
                None => return Err("the outcome has no removes".into()),
            };
            expect_eq("removes", &removes, &expected_removes)?;
            if new {
                expect_eq("a new identifier is accepted", &accept(id.as_bytes()), &Some(id.clone()))?;
                if found_id == Some(id) {
                    return Err("a new identifier equals the found one".into());
                }
            }
            php_agrees(&record)
        }
        (outcome, kind) => Err(format!("the outcome {outcome:?} is no {kind}").into()),
    }
}

fn expect_raises(error: &polyspec_sessions_core::Error, raises: &Value) -> Result<(), Failed> {
    expect_eq("kind", &error.kind(), &str_of(raises, "kind")?)?;
    expect_eq("status", &error.status().map(u64::from), &raises.get("status").and_then(Value::as_u64))?;
    expect_eq("category", &"store", &str_of(raises, "category")?)?;
    if let Some(Value::String(message)) = raises.get("message") {
        expect_eq("message", &error.to_string(), message)?;
    }
    Ok(())
}

/// A runner of the cases of one group.
type Group = fn(&Value) -> Result<(), Failed>;

fn main() {
    let arguments = Arguments::from_args();
    let contract: Value = match serde_json::from_str(CONTRACT) {
        Ok(contract) => contract,
        Err(error) => panic!("contracts/sessions.json is no JSON: {error}"),
    };
    let mut trials = Vec::new();
    let groups: [(&str, Group); 3] = [("identifier", identifier), ("store", store), ("record", record)];
    for (group, run) in groups {
        let Some(Value::Array(cases)) = contract.get(group) else {
            panic!("contracts/sessions.json has no group {group}");
        };
        for case in cases.clone() {
            let Some(id) = case.get("id").and_then(Value::as_str) else {
                panic!("a case of the group {group} has no id");
            };
            let name = format!("{group}::{id}");
            trials.push(Trial::test(name, move || run(&case)));
        }
    }
    libtest_mimic::run(&arguments, trials).exit();
}
