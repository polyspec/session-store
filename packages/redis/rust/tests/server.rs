//! The connection against real Redis servers, each a service of a Compose project of its test that bay runs from the
//! pinned image (`tests/servers`, SE-10.4): the queue of one process (SR-3.2), an ACL user (SR-6.1, SR-6.4), Sentinel
//! (SR-6.2), Cluster (SR-6.3) and TLS (SR-6.1). The trials run in parallel in this process, so none changes its
//! environment.

mod servers;
mod store;

use std::sync::Arc;

use binding_core::{Failure, Kinds};
use libtest_mimic::{Arguments, Failed, Trial};
use polyspec_php_serialization::{Node, PhpValue};
use polyspec_session_store_core::Store;
use polyspec_session_store_redis::{RedisSessions, Setting};
use servers::{Address, Project, Service, MOUNT, PLAIN, PORT};
use store::{failed, integer, options, round_trip, runtime};

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

fn the_requests_of_one_session_in_one_process_lose_no_write() -> Result<(), Failed> {
    // SE-8.1, SR-3.2: sixteen requests of one session on four threads add one each.
    let (_project, server) = plain(&[])?;
    runtime()?.block_on(async {
        let setting = Setting::Server { url: format!("redis://{}/", server.name) };
        let store = Arc::new(RedisSessions::new(&setting, &options()).map_err(failed)?);
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
            RedisSessions::new(&Setting::Server { url: format!("redis://alice:wrongword@{address}/") }, &options())
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

fn a_tls_connection_reads_and_writes() -> Result<(), Failed> {
    // SR-6.1: rediss:// to a server that accepts TLS alone, with the certificate that bay issues for its name, at the
    // paths in the container that the plan of the project gives. The store verifies the server against the trust
    // store of its process, which holds the authority of bay because `bay install` adds it to the trust store of the
    // machine (SE-10.4).
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
    // A failure of the client tells nothing of its cause (SR-6.4), so the test adds what the server wrote.
    let url = format!("rediss://{}/", server.name);
    let wrote = |when: &str, error: Failed| {
        let message = match error.message() {
            Some(message) => message.to_owned(),
            None => String::new(),
        };
        Failed::from(format!("{when}{message}\nthe server at {} wrote:\n{}", server.name, project.output("server")))
    };
    // SR-6.1a: a process whose trust store is one file, the authority of bay (SSL_CERT_FILE), connects twice, and the
    // file is removed between the two connections; the second connection succeeds only with the authorities that the
    // process read once. The variable is set for that child process alone, before it starts, because a test process
    // whose threads run sets no variable of its environment.
    let authorities = project.directory().join("authorities.pem");
    let authority = project.exec("server", &["cat", &certificate.server_authority]).map_err(Failed::from)?;
    std::fs::write(&authorities, authority).map_err(failed)?;
    let child = std::process::Command::new(std::env::current_exe().map_err(failed)?)
        .args(["--exact", ONCE, "--ignored", "--test-threads=1"])
        .env("SSL_CERT_FILE", &authorities)
        .env(ONCE_URL, &url)
        .env(ONCE_FILE, &authorities)
        .output()
        .map_err(failed)?;
    if !child.status.success() {
        let output = format!("{}{}", String::from_utf8_lossy(&child.stdout), String::from_utf8_lossy(&child.stderr));
        return Err(wrote("the process whose trust store is the authority of bay fails: ", output.into()));
    }
    // SR-6.1: this process trusts the authority of bay through the trust store of the machine.
    runtime()?.block_on(round_trip(Setting::Server { url })).map_err(|error| wrote("", error))
}

/// The trial that the test of TLS runs in a process of its own (SR-6.1a).
const ONCE: &str = "the_authorities_read_once_serve_the_next_connection";
/// The URL of the server of TLS and the file of its authority, which the test of TLS gives that process.
const ONCE_URL: &str = "SESSION_STORE_TLS_URL";
const ONCE_FILE: &str = "SESSION_STORE_TLS_AUTHORITIES";

/// SR-6.1a: in a process whose trust store is the file of SSL_CERT_FILE, a store connects, the file is removed, and a
/// second store connects with the authorities that the process read once, because a read of the trust store now fails.
fn the_authorities_read_once_serve_the_next_connection() -> Result<(), Failed> {
    let (Ok(url), Ok(file)) = (std::env::var(ONCE_URL), std::env::var(ONCE_FILE)) else {
        return Err(format!("{ONCE} runs in the process that a_tls_connection_reads_and_writes starts").into());
    };
    runtime()?.block_on(round_trip(Setting::Server { url: url.clone() }))?;
    std::fs::remove_file(&file).map_err(failed)?;
    runtime()?.block_on(round_trip(Setting::Server { url })).map_err(|error| {
        let message = match error.message() {
            Some(message) => message.to_owned(),
            None => String::new(),
        };
        Failed::from(format!("after the file of the trust store is removed: {message}"))
    })
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
        Trial::test("sentinel_names_the_primary", sentinel_names_the_primary),
        Trial::test("a_cluster_keeps_sessions", a_cluster_keeps_sessions),
        Trial::test("a_tls_connection_reads_and_writes", a_tls_connection_reads_and_writes),
        Trial::test(ONCE, the_authorities_read_once_serve_the_next_connection).with_ignored_flag(true),
    ];
    libtest_mimic::run(&arguments, trials).exit();
}
