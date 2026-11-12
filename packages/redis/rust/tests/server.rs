//! The connection against real Redis servers that the test starts when the environment names a `redis-server` binary
//! (`REDIS_SERVER`), and `openssl` for TLS (`OPENSSL`) and for Cluster `redis-cli` (`REDIS_CLI`): the queue of one process (SR-3.2), an ACL user (SR-6.1, SR-6.4),
//! TLS (SR-6.1), Sentinel (SR-6.2) and Cluster (SR-6.3).

use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bindgen_core::{Failure, Kinds};
use libtest_mimic::{Arguments, Failed, Trial};
use polyspec_php_serialize::{Node, PhpValue};
use polyspec_sessions_core::Store;
use polyspec_sessions_redis::{RedisSessions, Setting, StoreOptions};

fn failed(error: impl std::fmt::Display) -> Failed {
    Failed::from(error.to_string())
}

/// A process that the test started and stops when it ends.
struct Started(Child);

impl Drop for Started {
    fn drop(&mut self) {
        let stopped: std::io::Result<()> = self.0.kill().and_then(|()| self.0.wait().map(|_status| ()));
        if let Err(error) = stopped {
            eprintln!("a test server does not stop: {error}");
        }
    }
}

fn free_port() -> Result<u16, Failed> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(failed)?;
    Ok(listener.local_addr().map_err(failed)?.port())
}

fn start(arguments: &[&str], port: u16, directory: &Path) -> Result<Started, Failed> {
    let binary = std::env::var("REDIS_SERVER").map_err(failed)?;
    let child = Command::new(binary)
        .args(arguments)
        .args(["--save", "", "--appendonly", "no", "--dir"])
        .arg(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(failed)?;
    let started = Started(child);
    let begin = Instant::now();
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        if begin.elapsed() > Duration::from_secs(10) {
            return Err(format!("no server listens on {port}").into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(started)
}

fn integer(value: i64) -> PhpValue {
    let mut php = PhpValue::new();
    match php.set_root(Node::Int(value)) {
        Ok(()) => php,
        Err(error) => panic!("{error}"),
    }
}

fn runtime() -> Result<tokio::runtime::Runtime, Failed> {
    tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build().map_err(failed)
}

/// Writes a session with `n` = 1 and reads it back through a store of the setting.
async fn round_trip(setting: Setting) -> Result<(), Failed> {
    let store = RedisSessions::new(setting, StoreOptions::default()).map_err(failed)?;
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
    let directory = tempfile::tempdir().map_err(failed)?;
    let port = free_port()?;
    let _server = start(&["--port", &port.to_string()], port, directory.path())?;
    runtime()?.block_on(async {
        let setting = Setting::Server { url: format!("redis://127.0.0.1:{port}/") };
        let store = Arc::new(RedisSessions::new(setting, StoreOptions::default()).map_err(failed)?);
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
                    Err(Failure::Raised(_)) => return Err("raised".to_owned()),
                };
                tokio::time::sleep(Duration::from_millis(2)).await;
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
    let directory = tempfile::tempdir().map_err(failed)?;
    let port = free_port()?;
    let _server = start(
        &["--port", &port.to_string(), "--user", "alice", "on", ">sesame", "~*", "&*", "+@all"],
        port,
        directory.path(),
    )?;
    let runtime = runtime()?;
    runtime.block_on(round_trip(Setting::Server { url: format!("redis://alice:sesame@127.0.0.1:{port}/") }))?;
    let wrong = runtime.block_on(async {
        let store = RedisSessions::new(
            Setting::Server { url: format!("redis://alice:wrongword@127.0.0.1:{port}/") },
            StoreOptions::default(),
        )
        .map_err(failed)?;
        Ok::<_, Failed>(store.open(Some(&[b'a'; 32])).await)
    })?;
    match wrong {
        Err(error) if error.kind() == "connection" => {
            let message = error.to_string();
            if message.contains("wrongword") || message.contains("alice") || message.contains(&port.to_string()) {
                return Err(format!("the message tells the setting: {message}").into());
            }
            Ok(())
        }
        other => Err(format!("a wrong password gives {other:?}").into()),
    }
}

fn a_tls_connection_reads_and_writes() -> Result<(), Failed> {
    // SR-6.1: rediss:// with a certificate that the trust store of the process (SSL_CERT_FILE) holds.
    let openssl = std::env::var("OPENSSL").map_err(failed)?;
    let directory = tempfile::tempdir().map_err(failed)?;
    let path = |name: &str| directory.path().join(name).to_string_lossy().into_owned();
    let run = |arguments: &[&str]| -> Result<(), Failed> {
        let status = Command::new(&openssl)
            .args(arguments)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(failed)?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("openssl {arguments:?} fails").into())
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
        "/CN=localhost",
        "-keyout",
        &path("server.key"),
        "-out",
        &path("server.csr"),
    ])?;
    std::fs::write(directory.path().join("ext"), "subjectAltName=DNS:localhost\n").map_err(failed)?;
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
    let port = free_port()?;
    let _server = start(
        &[
            "--port",
            "0",
            "--tls-port",
            &port.to_string(),
            "--tls-cert-file",
            &path("server.pem"),
            "--tls-key-file",
            &path("server.key"),
            "--tls-ca-cert-file",
            &path("ca.pem"),
            "--tls-auth-clients",
            "no",
        ],
        port,
        directory.path(),
    )?;
    // The trust store of rustls-native-certs reads SSL_CERT_FILE; no other test of this binary uses TLS.
    std::env::set_var("SSL_CERT_FILE", path("ca.pem"));
    runtime()?.block_on(round_trip(Setting::Server { url: format!("rediss://localhost:{port}/") }))
}

fn sentinel_names_the_primary() -> Result<(), Failed> {
    // SR-6.2: the store asks Sentinel for the primary of the service.
    let directory = tempfile::tempdir().map_err(failed)?;
    let primary_port = free_port()?;
    let _primary = start(&["--port", &primary_port.to_string()], primary_port, directory.path())?;
    let sentinel_port = free_port()?;
    let config = directory.path().join("sentinel.conf");
    std::fs::write(&config, format!("port {sentinel_port}\nsentinel monitor service 127.0.0.1 {primary_port} 1\n"))
        .map_err(failed)?;
    let _sentinel = start(&[&config.to_string_lossy(), "--sentinel"], sentinel_port, directory.path())?;
    runtime()?.block_on(round_trip(Setting::Sentinel {
        urls: vec![format!("redis://127.0.0.1:{sentinel_port}/")],
        service: "service".to_owned(),
        tls: false,
        user: None,
        password: None,
    }))
}

fn a_cluster_keeps_sessions() -> Result<(), Failed> {
    // SR-6.3: three primaries; a session and its lock may live on different primaries.
    let directory = tempfile::tempdir().map_err(failed)?;
    let mut servers = Vec::new();
    let mut ports = Vec::new();
    for index in 0..3 {
        let port = free_port()?;
        let node = directory.path().join(format!("node{index}"));
        std::fs::create_dir(&node).map_err(failed)?;
        servers.push(start(
            &["--port", &port.to_string(), "--cluster-enabled", "yes", "--cluster-config-file", "nodes.conf"],
            port,
            &node,
        )?);
        ports.push(port);
    }
    let cli = std::env::var("REDIS_CLI").map_err(failed)?;
    let nodes: Vec<String> = ports.iter().map(|port| format!("127.0.0.1:{port}")).collect();
    let status = Command::new(&cli)
        .args(["--cluster", "create"])
        .args(&nodes)
        .args(["--cluster-replicas", "0", "--cluster-yes"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(failed)?;
    if !status.success() {
        return Err("redis-cli does not create the cluster".into());
    }
    let begin = Instant::now();
    loop {
        let output =
            Command::new(&cli).args(["-p", &ports[0].to_string(), "cluster", "info"]).output().map_err(failed)?;
        if String::from_utf8_lossy(&output.stdout).contains("cluster_state:ok") {
            break;
        }
        if begin.elapsed() > Duration::from_secs(20) {
            return Err("the cluster is not ready".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let urls = ports.iter().map(|port| format!("redis://127.0.0.1:{port}/")).collect();
    runtime()?.block_on(round_trip(Setting::Cluster { urls }))
}

fn main() {
    let arguments = Arguments::from_args();
    let server = std::env::var_os("REDIS_SERVER").is_some();
    let tls = server && std::env::var_os("OPENSSL").is_some();
    let trials = vec![
        Trial::test(
            "the_requests_of_one_session_in_one_process_lose_no_write",
            the_requests_of_one_session_in_one_process_lose_no_write,
        )
        .with_ignored_flag(!server),
        Trial::test(
            "an_acl_user_connects_and_a_wrong_password_tells_nothing",
            an_acl_user_connects_and_a_wrong_password_tells_nothing,
        )
        .with_ignored_flag(!server),
        Trial::test("a_tls_connection_reads_and_writes", a_tls_connection_reads_and_writes).with_ignored_flag(!tls),
        Trial::test("sentinel_names_the_primary", sentinel_names_the_primary).with_ignored_flag(!server),
        Trial::test("a_cluster_keeps_sessions", a_cluster_keeps_sessions)
            .with_ignored_flag(!(server && std::env::var_os("REDIS_CLI").is_some())),
    ];
    libtest_mimic::run(&arguments, trials).exit();
}
