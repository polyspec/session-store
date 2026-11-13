//! The connection against real Redis servers, each a service of a Compose project of its test that bay runs from the
//! pinned image (`tests/servers`, SE-10.4): the queue of one process (SR-3.2), an ACL user (SR-6.1, SR-6.4), TLS
//! (SR-6.1), Sentinel (SR-6.2) and Cluster (SR-6.3). The certificates of TLS are made by `openssl` of the host
//! (`OPENSSL`), until bay issues a certificate to a service without a domain.

mod servers;

use std::process::{Command, Stdio};
use std::sync::Arc;

use binding_core::{Failure, Kinds};
use libtest_mimic::{Arguments, Failed, Trial};
use polyspec_php_serialization::{Node, PhpValue};
use polyspec_session_store_core::Store;
use polyspec_session_store_redis::{RedisSessions, Setting, StoreOptions};
use servers::{Address, Project, Service, MOUNT, PLAIN, PORT};

fn failed(error: impl std::fmt::Display) -> Failed {
    Failed::from(error.to_string())
}

/// A project of one server with the arguments of every server and `arguments`, and the address of that server. The
/// server runs until the project is dropped.
fn plain(arguments: &[&str]) -> Result<(Project, Address), Failed> {
    let mut project = Project::new().map_err(Failed::from)?;
    let mut all = PLAIN.to_vec();
    all.extend_from_slice(arguments);
    let address = project.start(Service::new("server", PORT, &all)).map_err(Failed::from)?;
    Ok((project, address))
}

/// Sends one command of Redis, `words`, to the server at `address` and takes its reply OK.
async fn command(address: &str, words: &[&str]) -> Result<(), Failed> {
    let Some((name, arguments)) = words.split_first() else {
        return Err("no command is given".into());
    };
    let url = format!("redis://{address}/");
    let client = redis::Client::open(url.as_str()).map_err(failed)?;
    let mut connection = client.get_multiplexed_async_connection().await.map_err(failed)?;
    redis::cmd(name).arg(arguments).query_async::<()>(&mut connection).await.map_err(failed)
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
    let (_project, server) = plain(&[])?;
    runtime()?.block_on(async {
        let setting = Setting::Server { url: format!("redis://{}/", server.name) };
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
    let (_project, server) = plain(&["--user", "alice", "on", ">sesame", "~*", "&*", "+@all"])?;
    let address = server.name.as_str();
    let runtime = runtime()?;
    runtime.block_on(round_trip(Setting::Server { url: format!("redis://alice:sesame@{address}/") }))?;
    let wrong = runtime.block_on(async {
        let store =
            RedisSessions::new(Setting::Server { url: format!("redis://alice:wrongword@{address}/") }, options())
                .map_err(failed)?;
        Ok::<_, Failed>(store.open(Some(&[b'a'; 32])).await)
    })?;
    match wrong {
        Err(error) if error.kind() == "connection" => {
            let message = error.to_string();
            if message.contains("wrongword") || message.contains("alice") || message.contains(server.host()) {
                return Err(format!("the message tells the setting: {message}").into());
            }
            Ok(())
        }
        other => Err(format!("a wrong password gives {other:?}").into()),
    }
}

fn a_tls_connection_reads_and_writes() -> Result<(), Failed> {
    // SR-6.1: rediss:// with a certificate of the name of the server that the trust store of the process
    // (SSL_CERT_FILE) holds. The first start gives the name of the server, which the certificate names; the second
    // start replaces the server by one that reads the certificate and accepts TLS alone, on the same port.
    let openssl = match std::env::var("OPENSSL") {
        Ok(openssl) if !openssl.is_empty() => openssl,
        _ => {
            return Err("OPENSSL is not set: `make test` names the openssl of PATH, which makes the certificates".into())
        }
    };
    let mut project = Project::new().map_err(Failed::from)?;
    let directory = project.directory().join("tls");
    std::fs::create_dir(&directory).map_err(failed)?;
    let server = project.start(Service::new("server", PORT, &PLAIN).mount("tls", true)).map_err(Failed::from)?;
    let path = |name: &str| directory.join(name).to_string_lossy().into_owned();
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
    std::fs::write(directory.join("ext"), format!("subjectAltName=DNS:{}\n", server.host())).map_err(failed)?;
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
    let port = PORT.to_string();
    let certificate = format!("{MOUNT}/server.pem");
    let key = format!("{MOUNT}/server.key");
    let authority = format!("{MOUNT}/ca.pem");
    let mut arguments = PLAIN.to_vec();
    arguments.extend_from_slice(&[
        "--port",
        "0",
        "--tls-port",
        &port,
        "--tls-cert-file",
        &certificate,
        "--tls-key-file",
        &key,
        "--tls-ca-cert-file",
        &authority,
        "--tls-auth-clients",
        "no",
    ]);
    let server = project.start(Service::new("server", PORT, &arguments).mount("tls", true)).map_err(Failed::from)?;
    // The trust store of rustls-native-certs reads SSL_CERT_FILE; no other test of this binary uses TLS.
    std::env::set_var("SSL_CERT_FILE", path("ca.pem"));
    runtime()?.block_on(round_trip(Setting::Server { url: format!("rediss://{}/", server.name) }))
}

fn sentinel_names_the_primary() -> Result<(), Failed> {
    // SR-6.2: the store asks Sentinel for the primary of the service. Sentinel starts by a second start of the
    // project, after bay gave the name of the primary, which its configuration names.
    let mut project = Project::new().map_err(Failed::from)?;
    let primary = project.start(Service::new("primary", PORT, &PLAIN)).map_err(Failed::from)?;
    let directory = project.directory().join("sentinel");
    std::fs::create_dir(&directory).map_err(failed)?;
    // Sentinel writes its state into its configuration file, which is in the mounted directory.
    std::fs::write(
        directory.join("sentinel.conf"),
        format!(
            "protected-mode no\nsentinel resolve-hostnames yes\nsentinel monitor service {} {PORT} 1\n",
            primary.host()
        ),
    )
    .map_err(failed)?;
    let configuration = format!("{MOUNT}/sentinel.conf");
    // Sentinel reads the primary of its configuration before it accepts connections.
    let sentinel = project
        .start(
            Service::new("sentinel", 26379, &[&configuration, "--sentinel"]).mount("sentinel", false).after("primary"),
        )
        .map_err(Failed::from)?;
    runtime()?.block_on(round_trip(Setting::Sentinel {
        urls: vec![format!("redis://{}/", sentinel.name)],
        service: "service".to_owned(),
        tls: false,
        user: None,
        password: None,
    }))
}

fn a_cluster_keeps_sessions() -> Result<(), Failed> {
    // SR-6.3: three primaries; a session and its lock may live on different primaries. The test forms the cluster
    // with the commands of Redis, in the order of `redis-cli --cluster create`: each node takes a third of the slots
    // and an epoch of its own, and every other node meets the first by its address, because CLUSTER MEET takes no
    // name. A node learns its own address from the MEET or the PING that it receives.
    let mut project = Project::new().map_err(Failed::from)?;
    let mut nodes = Vec::new();
    let mut reports = Vec::new();
    for name in ["node-0", "node-1", "node-2"] {
        let mut arguments = PLAIN.to_vec();
        arguments.extend_from_slice(&["--cluster-enabled", "yes", "--cluster-config-file", "nodes.conf"]);
        nodes.push(project.start(Service::new(name, PORT, &arguments)).map_err(Failed::from)?);
        reports.push(project.follow(name).map_err(Failed::from)?);
    }
    let Some(first) = nodes.first() else {
        return Err("no node is started".into());
    };
    let port = PORT.to_string();
    let runtime = runtime()?;
    runtime.block_on(async {
        let slots = [("0", "5460", "1"), ("5461", "10922", "2"), ("10923", "16383", "3")];
        for (node, (low, high, epoch)) in nodes.iter().zip(slots) {
            command(&node.name, &["CLUSTER", "ADDSLOTSRANGE", low, high]).await?;
            command(&node.name, &["CLUSTER", "SET-CONFIG-EPOCH", epoch]).await?;
        }
        for node in nodes.iter().skip(1) {
            command(&node.name, &["CLUSTER", "MEET", &first.ipv4, &port]).await?;
        }
        Ok::<(), Failed>(())
    })?;
    // Each node reports the state of the cluster when it changes; the cluster serves when every node reports ok.
    for report in &mut reports {
        report.wait_for("Cluster state changed: ok").map_err(Failed::from)?;
    }
    let urls = nodes.iter().map(|node| format!("redis://{}/", node.name)).collect();
    runtime.block_on(round_trip(Setting::Cluster { urls }))
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
