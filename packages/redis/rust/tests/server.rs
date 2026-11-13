//! The connection against real Redis servers, each in a container of the pinned image (`tests/servers`, SE-10.4):
//! the queue of one process (SR-3.2), an ACL user (SR-6.1, SR-6.4), TLS (SR-6.1), Sentinel (SR-6.2) and Cluster
//! (SR-6.3). The certificates of TLS are made by `openssl` of the host (`OPENSSL`).

mod servers;

use std::process::{Command, Stdio};
use std::sync::Arc;

use binding_core::{Failure, Kinds};
use libtest_mimic::{Arguments, Failed, Trial};
use polyspec_php_serialization::{Node, PhpValue};
use polyspec_session_store_core::Store;
use polyspec_session_store_redis::{RedisSessions, Setting, StoreOptions};
use servers::{Server, MOUNT, PLAIN, PORT, READY};

fn failed(error: impl std::fmt::Display) -> Failed {
    Failed::from(error.to_string())
}

/// A server of the run with the arguments of every server and `arguments`.
fn plain(arguments: &[&str], directory: Option<&std::path::Path>) -> Result<Server, Failed> {
    let mut all = PLAIN.to_vec();
    all.extend_from_slice(arguments);
    Server::start(&all, directory, READY).map_err(Failed::from)
}

fn integer(value: i64) -> PhpValue {
    let mut php = PhpValue::new();
    match php.set_root(Node::Int(value)) {
        Ok(()) => php,
        Err(error) => panic!("{error}"),
    }
}

/// The options of every test: the lock expiry, which has no default (SR-1.2), and the defaults.
fn options() -> StoreOptions {
    StoreOptions { lifetime: None, prefix: None, lock_expire: 30, lock_retries: None, lock_wait: None }
}

fn runtime() -> Result<tokio::runtime::Runtime, Failed> {
    tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build().map_err(failed)
}

/// Writes a session with `n` = 1 and reads it back through a store of the setting.
async fn round_trip(setting: Setting) -> Result<(), Failed> {
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

fn the_requests_of_one_session_in_one_process_lose_no_write() -> Result<(), Failed> {
    // SE-8.1, SR-3.2: sixteen requests of one session on four threads add one each.
    let server = plain(&[], None)?;
    runtime()?.block_on(async {
        let setting = Setting::Server { url: format!("redis://{}:{PORT}/", server.address()) };
        let store = Arc::new(RedisSessions::new(setting, options()).map_err(failed)?);
        let mut opened = store.open(None).await.map_err(failed)?;
        opened.set("n", &integer(0)).map_err(|error| failed(format!("{error:?}")))?;
        let Some(id) = opened.close().await.map_err(failed)? else {
            return Err(Failed::from("no identifier is created"));
        };
        let mut tasks = Vec::new();
        for _ in 0..16 {
            let store = Arc::clone(&store);
            let id = id.clone();
            tasks.push(tokio::spawn(async move {
                let mut opened = store.open(Some(id.as_bytes())).await.map_err(|error| error.to_string())?;
                let number = match opened.get("n") {
                    Ok(Some(value)) => match value.root() {
                        Node::Int(number) => *number,
                        other => return Err(format!("n is {other:?}")),
                    },
                    Ok(None) => return Err("n is gone".to_owned()),
                    Err(Failure::Kind(error)) => return Err(error.to_string()),
                    Err(Failure::Raised(_) | Failure::Callback(_)) => return Err("raised".to_owned()),
                };
                // Another request runs between the read and the write of this one, which only the lock keeps out.
                tokio::task::yield_now().await;
                if let Err(error) = opened.set("n", &integer(number + 1)) {
                    return Err(format!("{error:?}"));
                }
                opened.close().await.map(|_created| ()).map_err(|error| error.to_string())
            }));
        }
        for task in tasks {
            task.await.map_err(failed)?.map_err(Failed::from)?;
        }
        let mut opened = store.open(Some(id.as_bytes())).await.map_err(failed)?;
        let value = opened.get("n").map_err(|error| failed(format!("{error:?}")))?;
        opened.close().await.map_err(failed)?;
        match value.as_ref().map(PhpValue::root) {
            Some(Node::Int(16)) => Ok(()),
            other => Err(format!("n is {other:?}").into()),
        }
    })
}

fn an_acl_user_connects_and_a_wrong_password_tells_nothing() -> Result<(), Failed> {
    // SR-6.1, SR-6.4
    let server = plain(&["--user", "alice", "on", ">sesame", "~*", "&*", "+@all"], None)?;
    let address = server.address();
    let runtime = runtime()?;
    runtime.block_on(round_trip(Setting::Server { url: format!("redis://alice:sesame@{address}:{PORT}/") }))?;
    let wrong = runtime.block_on(async {
        let store = RedisSessions::new(
            Setting::Server { url: format!("redis://alice:wrongword@{address}:{PORT}/") },
            options(),
        )
        .map_err(failed)?;
        Ok::<_, Failed>(store.open(Some(&[b'a'; 32])).await)
    })?;
    match wrong {
        Err(error) if error.kind() == "connection" => {
            let message = error.to_string();
            if message.contains("wrongword") || message.contains("alice") || message.contains(address) {
                return Err(format!("the message tells the setting: {message}").into());
            }
            Ok(())
        }
        other => Err(format!("a wrong password gives {other:?}").into()),
    }
}

fn a_tls_connection_reads_and_writes() -> Result<(), Failed> {
    // SR-6.1: rediss:// with a certificate of the address of the server that the trust store of the process
    // (SSL_CERT_FILE) holds. The server starts without TLS, because the certificate names its address, and takes the
    // certificate and the port of TLS with CONFIG SET, whose reply comes after the port listens.
    let openssl = match std::env::var("OPENSSL") {
        Ok(openssl) if !openssl.is_empty() => openssl,
        _ => {
            return Err("OPENSSL is not set: `make test` names the openssl of PATH, which makes the certificates".into())
        }
    };
    let directory = tempfile::tempdir().map_err(failed)?;
    let server = plain(&[], Some(directory.path()))?;
    let address = server.address().to_owned();
    let path = |name: &str| directory.path().join(name).to_string_lossy().into_owned();
    let run = |arguments: &[&str]| -> Result<(), Failed> {
        let output = Command::new(&openssl).args(arguments).stdin(Stdio::null()).output().map_err(failed)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!("openssl {arguments:?} fails: {}", String::from_utf8_lossy(&output.stderr)).into())
        }
    };
    run(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-days",
        "1",
        "-subj",
        "/CN=sessions test CA",
        "-keyout",
        &path("ca.key"),
        "-out",
        &path("ca.pem"),
    ])?;
    run(&[
        "req",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-subj",
        "/CN=sessions test server",
        "-keyout",
        &path("server.key"),
        "-out",
        &path("server.csr"),
    ])?;
    std::fs::write(directory.path().join("ext"), format!("subjectAltName=IP:{address}\n")).map_err(failed)?;
    run(&[
        "x509",
        "-req",
        "-in",
        &path("server.csr"),
        "-CA",
        &path("ca.pem"),
        "-CAkey",
        &path("ca.key"),
        "-CAcreateserial",
        "-days",
        "1",
        "-extfile",
        &path("ext"),
        "-out",
        &path("server.pem"),
    ])?;
    let tls_port = "6380";
    for setting in [
        vec![
            "tls-cert-file".to_owned(),
            format!("{MOUNT}/server.pem"),
            "tls-key-file".to_owned(),
            format!("{MOUNT}/server.key"),
            "tls-ca-cert-file".to_owned(),
            format!("{MOUNT}/ca.pem"),
            "tls-auth-clients".to_owned(),
            "no".to_owned(),
        ],
        vec!["tls-port".to_owned(), tls_port.to_owned()],
    ] {
        let mut arguments = vec!["-h", address.as_str(), "config", "set"];
        arguments.extend(setting.iter().map(String::as_str));
        let reply = server.cli(&arguments).map_err(Failed::from)?;
        if reply.trim() != "OK" {
            return Err(format!("CONFIG SET {setting:?} gives {reply:?}").into());
        }
    }
    // The trust store of rustls-native-certs reads SSL_CERT_FILE; no other test of this binary uses TLS.
    std::env::set_var("SSL_CERT_FILE", path("ca.pem"));
    runtime()?.block_on(round_trip(Setting::Server { url: format!("rediss://{address}:{tls_port}/") }))
}

fn sentinel_names_the_primary() -> Result<(), Failed> {
    // SR-6.2: the store asks Sentinel for the primary of the service.
    let primary = plain(&[], None)?;
    let directory = tempfile::tempdir().map_err(failed)?;
    // Sentinel writes its state into its configuration file, which is in the mounted directory.
    std::fs::write(
        directory.path().join("sentinel.conf"),
        format!("protected-mode no\nsentinel monitor service {} {PORT} 1\n", primary.address()),
    )
    .map_err(failed)?;
    let configuration = format!("{MOUNT}/sentinel.conf");
    // Sentinel writes no line of readiness; it reports the primary that it monitors after it listens.
    let sentinel = Server::start(&[&configuration, "--sentinel"], Some(directory.path()), "+monitor master service")
        .map_err(Failed::from)?;
    runtime()?.block_on(round_trip(Setting::Sentinel {
        urls: vec![format!("redis://{}:26379/", sentinel.address())],
        service: "service".to_owned(),
        tls: false,
        user: None,
        password: None,
    }))
}

fn a_cluster_keeps_sessions() -> Result<(), Failed> {
    // SR-6.3: three primaries; a session and its lock may live on different primaries.
    let mut nodes = Vec::new();
    for _ in 0..3 {
        nodes.push(plain(&["--cluster-enabled", "yes", "--cluster-config-file", "nodes.conf"], None)?);
    }
    let addresses: Vec<String> = nodes.iter().map(|node| format!("{}:{PORT}", node.address())).collect();
    let mut arguments = vec!["--cluster", "create"];
    arguments.extend(addresses.iter().map(String::as_str));
    arguments.extend(["--cluster-replicas", "0", "--cluster-yes"]);
    let Some(first) = nodes.first() else {
        return Err("no node is started".into());
    };
    first.cli(&arguments).map_err(Failed::from)?;
    // Each node reports the state of the cluster when it changes; the cluster serves when every node reports ok.
    for node in &mut nodes {
        node.wait_for("Cluster state changed: ok").map_err(Failed::from)?;
    }
    let urls = addresses.iter().map(|address| format!("redis://{address}/")).collect();
    runtime()?.block_on(round_trip(Setting::Cluster { urls }))
}

fn main() {
    let arguments = Arguments::from_args();
    let trials = vec![
        Trial::test(
            "the_requests_of_one_session_in_one_process_lose_no_write",
            the_requests_of_one_session_in_one_process_lose_no_write,
        ),
        Trial::test(
            "an_acl_user_connects_and_a_wrong_password_tells_nothing",
            an_acl_user_connects_and_a_wrong_password_tells_nothing,
        ),
        Trial::test("a_tls_connection_reads_and_writes", a_tls_connection_reads_and_writes),
        Trial::test("sentinel_names_the_primary", sentinel_names_the_primary),
        Trial::test("a_cluster_keeps_sessions", a_cluster_keeps_sessions),
    ];
    libtest_mimic::run(&arguments, trials).exit();
}
