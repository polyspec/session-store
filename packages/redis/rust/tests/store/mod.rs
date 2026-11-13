//! What the tests of the connection share (`tests/server.rs`, `tests/tls.rs`): a store of a connection setting that
//! writes a session and reads it back.

use libtest_mimic::Failed;
use polyspec_php_serialization::{Node, PhpValue};
use polyspec_session_store_core::Store;
use polyspec_session_store_redis::{RedisSessions, Setting, StoreOptions};

pub fn failed(error: impl std::fmt::Display) -> Failed {
    Failed::from(error.to_string())
}

pub fn integer(value: i64) -> PhpValue {
    let mut php = PhpValue::new();
    match php.set_root(Node::Int(value)) {
        Ok(()) => php,
        Err(error) => panic!("{error}"),
    }
}

/// The options of every test: the lock expiry, which has no default (SR-1.2), and the defaults.
pub fn options() -> StoreOptions {
    StoreOptions { lifetime: None, prefix: None, lock_expire: 30, lock_retries: None, lock_wait: None }
}

pub fn runtime() -> Result<tokio::runtime::Runtime, Failed> {
    tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build().map_err(failed)
}

/// Writes a session with `n` = 1 and reads it back through a store of the setting.
pub async fn round_trip(setting: Setting) -> Result<(), Failed> {
    let store = RedisSessions::new(setting, options()).map_err(failed)?;
    let mut opened = store.open(None).await.map_err(failed)?;
    opened.set("n", &integer(1)).map_err(|error| failed(format!("{error:?}")))?;
    let Some(id) = opened.close().await.map_err(failed)? else {
        return Err("no identifier is created".into());
    };
    let mut opened = store.open(Some(id.as_bytes())).await.map_err(failed)?;
    let value = opened.get("n").map_err(|error| failed(format!("{error:?}")))?;
    opened.close().await.map_err(failed)?;
    match value.as_ref().map(PhpValue::root) {
        Some(Node::Int(1)) => Ok(()),
        other => Err(format!("n is {other:?}").into()),
    }
}
