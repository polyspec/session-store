//! The lock, the queue and the waits of the store of files (SF-3, SF-4, SF-5), with threads of one process and with a
//! descriptor that the test locks in the place of another process.

use std::fs::File;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, SystemTime};

use bindgen_core::{cancel_pair, Io, Kinds, ProcessWait};
use polyspec_php_serialize::{serialize, unserialize, Node, PhpValue};
use polyspec_sessions_core::Error;
use polyspec_sessions_core::Store;
use polyspec_sessions_file::FileSessions;
use rustix::fs::FlockOperation;

const ID: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

fn store(directory: &Path) -> FileSessions {
    match FileSessions::new(Io::for_generated_call(), directory, None) {
        Ok(store) => store,
        Err(error) => panic!("the store is not made: {error}"),
    }
}

fn session_file(directory: &Path, bytes: &[u8], age: u64) -> File {
    let path = directory.join(format!("sess_{ID}"));
    assert!(std::fs::write(&path, bytes).is_ok());
    let file = match File::options().read(true).write(true).open(&path) {
        Ok(file) => file,
        Err(error) => panic!("the file does not open: {error}"),
    };
    if let Some(modified) = SystemTime::now().checked_sub(Duration::from_secs(age)) {
        assert!(file.set_modified(modified).is_ok());
    }
    file
}

fn integer(value: i64) -> PhpValue {
    let mut php = PhpValue::new();
    assert_eq!(php.set_root(Node::Int(value)), Ok(()));
    php
}

fn read_integer(store: &FileSessions) -> Option<i64> {
    let Ok(mut session) = store.open(ProcessWait::for_generated_call(), Some(ID.as_bytes()), None) else {
        panic!("the session does not open");
    };
    let value = match session.get("n") {
        Ok(Some(value)) => match value.root() {
            Node::Int(number) => Some(*number),
            _ => None,
        },
        _ => None,
    };
    assert!(session.close(Io::for_generated_call()).is_ok());
    value
}

#[test]
fn the_requests_of_one_session_lose_no_write() {
    // SE-8.1, SF-3.2, SF-3.4: eight threads add one each, one after another.
    let Ok(directory) = tempfile::tempdir() else { panic!("no temporary directory") };
    drop(session_file(directory.path(), b"a:1:{s:1:\"n\";i:0;}", 0));
    let store = store(directory.path());
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                let Ok(mut session) = store.open(ProcessWait::for_generated_call(), Some(ID.as_bytes()), None) else {
                    panic!("the session does not open");
                };
                let number = match session.get("n") {
                    Ok(Some(value)) => match value.root() {
                        Node::Int(number) => *number,
                        other => panic!("n is {other:?}"),
                    },
                    other => panic!("n gives {other:?}"),
                };
                std::thread::sleep(Duration::from_millis(5));
                assert!(session.set("n", &integer(number + 1)).is_ok());
                assert_eq!(session.close(Io::for_generated_call()), Ok(None));
            });
        }
    });
    assert_eq!(read_integer(&store), Some(8));
}

#[test]
fn a_cancelled_waiter_lets_the_next_request_go_on() {
    // SF-3.8: the lock of another process holds the first request in flock and the second in the queue; the first is
    // cancelled, and when the other process releases the lock the first gives it up at once and the second goes on.
    let Ok(directory) = tempfile::tempdir() else { panic!("no temporary directory") };
    let other = session_file(directory.path(), b"a:1:{s:1:\"n\";i:7;}", 0);
    assert!(rustix::fs::flock(&other, FlockOperation::LockExclusive).is_ok());
    let store = store(directory.path());
    let (trigger, cancel) = cancel_pair();
    std::thread::scope(|scope| {
        let first = scope.spawn(|| store.open(ProcessWait::for_generated_call(), Some(ID.as_bytes()), Some(&cancel)));
        std::thread::sleep(Duration::from_millis(50));
        let second = scope.spawn(|| read_integer(&store));
        std::thread::sleep(Duration::from_millis(50));
        trigger.cancel();
        assert!(rustix::fs::flock(&other, FlockOperation::Unlock).is_ok());
        match first.join() {
            Ok(Err(error)) => assert_eq!(error.kind(), "cancelled"),
            Ok(Ok(_)) => panic!("the cancelled request opened its session"),
            Err(_) => panic!("the first request panicked"),
        }
        assert!(matches!(second.join(), Ok(Some(7))));
    });
}

#[test]
fn a_cancelled_request_in_the_queue_returns_at_once() {
    // SF-3.8: the holder keeps the session while the second request waits in the queue; its cancellation wakes it
    // at once, and the third request gets the turn when the holder closes.
    let Ok(directory) = tempfile::tempdir() else { panic!("no temporary directory") };
    drop(session_file(directory.path(), b"a:1:{s:1:\"n\";i:3;}", 0));
    let store = store(directory.path());
    let Ok(holder) = store.open(ProcessWait::for_generated_call(), Some(ID.as_bytes()), None) else {
        panic!("the session does not open");
    };
    let (trigger, cancel) = cancel_pair();
    std::thread::scope(|scope| {
        let waiter = scope.spawn(|| {
            let started = std::time::Instant::now();
            let result = store.open(ProcessWait::for_generated_call(), Some(ID.as_bytes()), Some(&cancel));
            (result.map(|_session| ()), started.elapsed())
        });
        std::thread::sleep(Duration::from_millis(50));
        trigger.cancel();
        match waiter.join() {
            Ok((Err(error), waited)) => {
                assert_eq!(error.kind(), "cancelled");
                assert!(waited < Duration::from_secs(5), "the cancelled request waited {waited:?}");
            }
            Ok((Ok(()), _)) => panic!("the cancelled request opened the session"),
            Err(_) => panic!("the waiter panicked"),
        }
        let third = scope.spawn(|| read_integer(&store));
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(holder.close(Io::for_generated_call()), Ok(None));
        assert!(matches!(third.join(), Ok(Some(3))));
    });
}

#[test]
fn a_request_cancelled_before_it_waits_fails_at_once() {
    // SF-3.8: a cancellation that came before the request is seen at once.
    let Ok(directory) = tempfile::tempdir() else { panic!("no temporary directory") };
    drop(session_file(directory.path(), b"a:0:{}", 0));
    let (trigger, cancel) = cancel_pair();
    trigger.cancel();
    match store(directory.path()).open(ProcessWait::for_generated_call(), Some(ID.as_bytes()), Some(&cancel)) {
        Err(error) => assert_eq!(error.kind(), "cancelled"),
        Ok(_) => panic!("the cancelled request opened the session"),
    }
}

#[test]
fn a_request_that_waited_for_a_removed_file_starts_a_new_session() {
    // SF-3.5, SF-4.4: the holder renews the session and removes the file while another request waits for its lock.
    let Ok(directory) = tempfile::tempdir() else { panic!("no temporary directory") };
    drop(session_file(directory.path(), b"a:1:{s:1:\"n\";i:1;}", 0));
    let store = store(directory.path());
    let Ok(mut holder) = store.open(ProcessWait::for_generated_call(), Some(ID.as_bytes()), None) else {
        panic!("the session does not open");
    };
    let other = FileSessions::clone(&store);
    std::thread::scope(|scope| {
        // A second store of the same directory has its own queue, as another process has, so the waiter waits in flock.
        let separate = match FileSessions::new(Io::for_generated_call(), other.directory(), None) {
            Ok(separate) => separate,
            Err(error) => panic!("{error}"),
        };
        let waiter =
            scope.spawn(move || match separate.open(ProcessWait::for_generated_call(), Some(ID.as_bytes()), None) {
                Ok(session) => session.found().map(str::to_owned),
                Err(error) => panic!("{error}"),
            });
        std::thread::sleep(Duration::from_millis(50));
        assert!(holder.renew().is_ok());
        assert!(matches!(holder.close(Io::for_generated_call()), Ok(Some(_))));
        assert!(matches!(waiter.join(), Ok(None)));
    });
}

#[test]
fn collect_skips_a_session_whose_lock_is_held() {
    // SF-5.2
    let Ok(directory) = tempfile::tempdir() else { panic!("no temporary directory") };
    let other = session_file(directory.path(), b"", 5000);
    assert!(rustix::fs::flock(&other, FlockOperation::LockExclusive).is_ok());
    let store = store(directory.path());
    assert_eq!(store.collect(Io::for_generated_call()), Ok(0));
    assert!(rustix::fs::flock(&other, FlockOperation::Unlock).is_ok());
    assert_eq!(store.collect(Io::for_generated_call()), Ok(1));
}

#[test]
fn a_symbolic_link_in_the_place_of_a_session_file_fails() {
    // SF-2.4: O_NOFOLLOW refuses the link.
    let Ok(directory) = tempfile::tempdir() else { panic!("no temporary directory") };
    let target = directory.path().join("target");
    assert!(std::fs::write(&target, b"a:0:{}").is_ok());
    assert!(std::os::unix::fs::symlink(&target, directory.path().join(format!("sess_{ID}"))).is_ok());
    match store(directory.path()).open(ProcessWait::for_generated_call(), Some(ID.as_bytes()), None) {
        Err(error @ Error::Io { .. }) => {
            assert_eq!(error.kind(), "io");
            assert_eq!(error.errno(), Some(rustix::io::Errno::LOOP.raw_os_error()));
        }
        other => panic!("the link gives {other:?}"),
    }
}

#[test]
fn a_created_session_file_has_the_mode_0600() {
    // SF-2.1, SF-4.4: the mode, under the umask 022 of the test process, and the record.
    let Ok(directory) = tempfile::tempdir() else { panic!("no temporary directory") };
    let store = store(directory.path());
    let Ok(mut session) = store.open(ProcessWait::for_generated_call(), None, None) else {
        panic!("the session does not open");
    };
    assert!(session.set("n", &integer(1)).is_ok());
    let Ok(Some(id)) = session.close(Io::for_generated_call()) else { panic!("no identifier is created") };
    let path = directory.path().join(format!("sess_{id}"));
    let Ok(metadata) = std::fs::metadata(&path) else { panic!("no file") };
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    let Ok(bytes) = std::fs::read(&path) else { panic!("no bytes") };
    assert_eq!(unserialize(&bytes).and_then(|value| serialize(&value)), Ok(b"a:1:{s:1:\"n\";i:1;}".to_vec()));
}
