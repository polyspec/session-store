//! The connection against real Redis servers, each a service of a Compose project of its test that bay runs from the
//! pinned image (`tests/servers`, SE-10.4): the queue of one process (SR-3.2), an ACL user (SR-6.1, SR-6.4), TLS
//! (SR-6.1), Sentinel (SR-6.2) and Cluster (SR-6.3). The certificate of the server of TLS is the one that bay issues
//! for its name.

mod servers;

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

/// A started project of one server with the arguments of every server and `arguments`, and the address of that
/// server. The server runs until the project is dropped.
fn plain(arguments: &[&str]) -> Result<(Project, Address), Failed> {
    let mut all = PLAIN.to_vec();
    all.extend_from_slice(arguments);
    let mut project = Project::new(vec![Service::new("server", PORT, &all)]).map_err(Failed::from)?;
    project.start().map_err(Failed::from)?;
    let address = project.address("server").map_err(Failed::from)?.clone();
    Ok((project, address))
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
            let host = match address.rsplit_once(':') {
                Some((host, _port)) => host,
                None => address,
            };
            if message.contains("wrongword") || message.contains("alice") || message.contains(host) {
                return Err(format!("the message tells the setting: {message}").into());
            }
            Ok(())
        }
        other => Err(format!("a wrong password gives {other:?}").into()),
    }
}

fn a_tls_connection_reads_and_writes() -> Result<(), Failed> {
    // SR-6.1: rediss:// to a server that accepts TLS alone, with the certificate that bay issues for its name. The
    // plan of the project gives the paths of the certificate in the container and the file of the authority, which
    // the trust store of the process (SSL_CERT_FILE) holds.
    let mut project = Project::new(vec![Service::new("server", PORT, &PLAIN)]).map_err(Failed::from)?;
    let certificate = project.planned("server").map_err(Failed::from)?.certificate.clone();
    let port = PORT.to_string();
    project
        .extend(
            "server",
            &[
                "--port",
                "0",
                "--tls-port",
                &port,
                "--tls-cert-file",
                &certificate.server_certificate,
                "--tls-key-file",
                &certificate.server_key,
                "--tls-ca-cert-file",
                &certificate.server_authority,
                "--tls-auth-clients",
                "no",
            ],
        )
        .map_err(Failed::from)?;
    project.start().map_err(Failed::from)?;
    let server = project.address("server").map_err(Failed::from)?;
    // The trust store of rustls-native-certs reads SSL_CERT_FILE; no other test of this binary uses TLS.
    std::env::set_var("SSL_CERT_FILE", &certificate.authority);
    // A failure of the client tells nothing of its cause (SR-6.4), so the test adds what the server wrote.
    runtime()?.block_on(round_trip(Setting::Server { url: format!("rediss://{}/", server.name) })).map_err(|error| {
        let message = match error.message() {
            Some(message) => message.to_owned(),
            None => String::new(),
        };
        Failed::from(format!("{message}\nthe server at {} wrote:\n{}", server.name, project.output("server")))
    })
}

fn sentinel_names_the_primary() -> Result<(), Failed> {
    // SR-6.2: the store asks Sentinel for the primary of the service. The configuration of Sentinel names the
    // primary by the name that the plan of the project gives, so one start runs both.
    let configuration = format!("{MOUNT}/sentinel.conf");
    let mut project = Project::new(vec![
        Service::new("primary", PORT, &PLAIN),
        Service::new("sentinel", 26379, &[&configuration, "--sentinel"]).mount("sentinel", false).after("primary"),
    ])
    .map_err(Failed::from)?;
    let primary = project.planned("primary").map_err(Failed::from)?.host.clone();
    // Sentinel writes its state into its configuration file, which is in the mounted directory.
    std::fs::write(
        project.directory().join("sentinel").join("sentinel.conf"),
        format!("protected-mode no\nsentinel resolve-hostnames yes\nsentinel monitor service {primary} {PORT} 1\n"),
    )
    .map_err(failed)?;
    // Sentinel reads the primary of its configuration before it accepts connections.
    project.start().map_err(Failed::from)?;
    let sentinel = project.address("sentinel").map_err(Failed::from)?;
    runtime()?.block_on(round_trip(Setting::Sentinel {
        urls: vec![format!("redis://{}/", sentinel.name)],
        service: "service".to_owned(),
        tls: false,
        user: None,
        password: None,
    }))
}

fn a_cluster_keeps_sessions() -> Result<(), Failed> {
    // SR-6.3: three primaries; a session and its lock may live on different primaries. `redis-cli --cluster create`
    // of the image creates the cluster in the container of the first node, by the IPv4 addresses of the nodes: a
    // node tells a client the addresses that the command gave, and a name gives the command more than one address.
    let names = ["node-0", "node-1", "node-2"];
    let mut arguments = PLAIN.to_vec();
    arguments.extend_from_slice(&["--cluster-enabled", "yes", "--cluster-config-file", "nodes.conf"]);
    let mut project =
        Project::new(names.iter().map(|name| Service::new(name, PORT, &arguments)).collect()).map_err(Failed::from)?;
    project.start().map_err(Failed::from)?;
    let mut reports = Vec::new();
    let mut nodes = Vec::new();
    let mut addresses = Vec::new();
    for name in names {
        reports.push(project.follow(name).map_err(Failed::from)?);
        let node = project.address(name).map_err(Failed::from)?;
        nodes.push(format!("redis://{}/", node.name));
        addresses.push(format!("{}:{PORT}", node.ipv4));
    }
    let mut create = vec!["redis-cli", "--cluster", "create"];
    create.extend(addresses.iter().map(String::as_str));
    create.extend(["--cluster-replicas", "0", "--cluster-yes"]);
    project.exec("node-0", &create).map_err(Failed::from)?;
    // Each node reports the state of the cluster when it changes; the cluster serves when every node reports ok.
    for report in &mut reports {
        report.wait_for("Cluster state changed: ok").map_err(Failed::from)?;
    }
    runtime()?.block_on(round_trip(Setting::Cluster { urls: nodes }))
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
