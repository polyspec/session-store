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

/// Makes the service `service` of `project` accept TLS alone at `port`, with the certificate that bay issues for its
/// name, at the paths in the container that the plan of the project gives (SR-6.1). `replication` makes its own
/// connections to other servers, of Sentinel to a primary and between the nodes of a Cluster, use TLS too.
fn tls_only(project: &mut Project, service: &str, port: u16, replication: bool) -> Result<(), Failed> {
    let certificate = project.planned(service).map_err(Failed::from)?.certificate.clone();
    let port = port.to_string();
    let mut arguments = vec![
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
    ];
    if replication {
        arguments.extend_from_slice(&["--tls-replication", "yes", "--tls-cluster", "yes"]);
    }
    project.extend(service, &arguments).map_err(Failed::from)
}

fn the_requests_of_one_session_in_one_process_lose_no_write() -> Result<(), Failed> {
    // SE-8.1, SR-3.2: sixteen requests of one session on four threads add one each.
    let (_project, server) = plain(&[])?;
    runtime()?.block_on(async {
        let setting = Setting::Server { url: format!("redis://{}/", server.name), authorities: None };
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
    runtime
        .block_on(round_trip(Setting::Server { url: format!("redis://alice:sesame@{address}/"), authorities: None }))?;
    let wrong = runtime.block_on(async {
        let store = RedisSessions::new(
            &Setting::Server { url: format!("redis://alice:wrongword@{address}/"), authorities: None },
            &options(),
        )
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
        authorities: None,
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
    runtime()?.block_on(round_trip(Setting::Cluster { urls: nodes, user: None, password: None, authorities: None }))
}

fn a_cluster_with_an_acl_user_keeps_sessions() -> Result<(), Failed> {
    // SR-6.8, SR-6.3: the user and password of a Cluster are those of its addresses or beside them. The nodes hold the ACL user
    // alice beside the default user, with which the nodes and `redis-cli --cluster create` speak to each other.
    let names = ["node-0", "node-1", "node-2"];
    let mut arguments = PLAIN.to_vec();
    arguments.extend_from_slice(&["--cluster-enabled", "yes", "--cluster-config-file", "nodes.conf"]);
    arguments.extend_from_slice(&["--user", "alice", "on", ">sesame", "~*", "&*", "+@all"]);
    let mut project =
        Project::new(names.iter().map(|name| Service::new(name, PORT, &arguments)).collect()).map_err(Failed::from)?;
    project.start().map_err(Failed::from)?;
    let mut reports = Vec::new();
    let mut hosts = Vec::new();
    let mut addresses = Vec::new();
    for name in names {
        reports.push(project.follow(name).map_err(Failed::from)?);
        let node = project.address(name).map_err(Failed::from)?;
        hosts.push(node.name.clone());
        addresses.push(format!("{}:{PORT}", node.ipv4));
    }
    let mut create = vec!["redis-cli", "--cluster", "create"];
    create.extend(addresses.iter().map(String::as_str));
    create.extend(["--cluster-replicas", "0", "--cluster-yes"]);
    project.exec("node-0", &create).map_err(Failed::from)?;
    for report in &mut reports {
        report.wait_for("Cluster state changed: ok").map_err(Failed::from)?;
    }
    let urls = |password: &str| hosts.iter().map(|host| format!("redis://alice:{password}@{host}/")).collect();
    let runtime = runtime()?;
    runtime.block_on(round_trip(Setting::Cluster {
        urls: urls("sesame"),
        user: None,
        password: None,
        authorities: None,
    }))?;
    let without = Setting::Cluster { urls: urls("wrongword"), user: None, password: None, authorities: None };
    refused(&runtime, without, "a wrong password of a Cluster")?;
    // SR-6.8: the user and password of a Cluster beside its URLs, which give none, reach each of its nodes.
    let seeds: Vec<String> = hosts.iter().map(|host| format!("redis://{host}/")).collect();
    let beside = |password: &str| Setting::Cluster {
        urls: seeds.clone(),
        user: Some("alice".to_owned()),
        password: Some(password.to_owned()),
        authorities: None,
    };
    runtime.block_on(round_trip(beside("sesame")))?;
    refused(&runtime, beside("wrongword"), "a wrong password beside the URLs of a Cluster")
}

/// Opens a session with `setting`, whose connection must fail with the kind `connection` (SR-6.4).
fn refused(runtime: &tokio::runtime::Runtime, setting: Setting, what: &str) -> Result<(), Failed> {
    let opened = runtime.block_on(async {
        let store = RedisSessions::new(&setting, &options()).map_err(failed)?;
        Ok::<_, Failed>(store.open(Some(&[b'a'; 32])).await)
    })?;
    match opened {
        Err(error) if error.kind() == "connection" => Ok(()),
        other => Err(format!("{what} gives {other:?}").into()),
    }
}

/// An authority that the trust store of the machine does not hold and the certificate that it issues for `host`: the
/// PEM texts of the authority, of the certificate and of its key. The test makes them, so it reads no file of bay.
fn issued(host: &str) -> Result<(String, String, String), Failed> {
    let authority_key = rcgen::KeyPair::generate().map_err(failed)?;
    let mut authority = rcgen::CertificateParams::new(Vec::<String>::new()).map_err(failed)?;
    authority.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    authority.distinguished_name.push(rcgen::DnType::CommonName, "the authority of a test of session-store");
    authority.key_usages = vec![rcgen::KeyUsagePurpose::KeyCertSign, rcgen::KeyUsagePurpose::CrlSign];
    let authority_pem = authority.self_signed(&authority_key).map_err(failed)?.pem();
    let issuer = rcgen::Issuer::new(authority, authority_key);
    let key = rcgen::KeyPair::generate().map_err(failed)?;
    let certificate = rcgen::CertificateParams::new(vec![host.to_owned()])
        .map_err(failed)?
        .signed_by(&key, &issuer)
        .map_err(failed)?;
    Ok((authority_pem, certificate.pem(), key.serialize_pem()))
}

fn a_file_of_authorities_is_the_only_trust_of_its_setting() -> Result<(), Failed> {
    // SR-6.7: the server `own` serves TLS with a certificate of an authority that the test makes and the trust store
    // of the machine does not hold, and the server `bay` with the certificate that bay issues. A setting that names
    // the file of that authority connects to `own`; without the file the connection fails, because the machine does
    // not trust the authority; and with the file a connection to `bay` fails, because the file alone is trusted.
    let mut project =
        Project::new(vec![Service::new("own", PORT, &PLAIN).mount("own", true), Service::new("bay", PORT, &PLAIN)])
            .map_err(Failed::from)?;
    let host = project.planned("own").map_err(Failed::from)?.host.clone();
    let (authority, certificate, key) = issued(&host)?;
    let directory = project.directory().join("own");
    std::fs::create_dir_all(&directory).map_err(failed)?;
    for (name, text) in [("authority.pem", &authority), ("server.pem", &certificate), ("server.key", &key)] {
        std::fs::write(directory.join(name), text).map_err(failed)?;
    }
    let port = PORT.to_string();
    let (served, served_key, trusted) =
        (format!("{MOUNT}/server.pem"), format!("{MOUNT}/server.key"), format!("{MOUNT}/authority.pem"));
    project
        .extend(
            "own",
            &[
                "--port",
                "0",
                "--tls-port",
                &port,
                "--tls-cert-file",
                &served,
                "--tls-key-file",
                &served_key,
                "--tls-ca-cert-file",
                &trusted,
                "--tls-auth-clients",
                "no",
            ],
        )
        .map_err(Failed::from)?;
    tls_only(&mut project, "bay", PORT, false)?;
    project.start().map_err(Failed::from)?;
    let own = format!("rediss://{}/", project.address("own").map_err(Failed::from)?.name);
    let bay = format!("rediss://{}/", project.address("bay").map_err(Failed::from)?.name);
    let file = directory.join("authority.pem").to_str().ok_or("the path of the authority is no text")?.to_owned();
    let runtime = runtime()?;
    runtime.block_on(round_trip(Setting::Server { url: own.clone(), authorities: Some(file.clone()) })).map_err(
        |error| {
            let message = match error.message() {
                Some(message) => message.to_owned(),
                None => String::new(),
            };
            Failed::from(format!("{message}\nthe server own wrote:\n{}", project.output("own")))
        },
    )?;
    refused(
        &runtime,
        Setting::Server { url: own, authorities: None },
        "a server of an authority that the machine does not trust",
    )?;
    refused(
        &runtime,
        Setting::Server { url: bay, authorities: Some(file) },
        "a server of bay with the authorities of a file",
    )
}

fn a_tls_connection_reads_and_writes() -> Result<(), Failed> {
    // SR-6.1: rediss:// to a server that accepts TLS alone, with the certificate that bay issues for its name, at the
    // paths in the container that the plan of the project gives. The store verifies the server against the trust
    // store of its process, which holds the authority of bay because `bay install` adds it to the trust store of the
    // machine (SE-10.4).
    let mut project = Project::new(vec![Service::new("server", PORT, &PLAIN)]).map_err(Failed::from)?;
    tls_only(&mut project, "server", PORT, false)?;
    project.start().map_err(Failed::from)?;
    let server = project.address("server").map_err(Failed::from)?;
    // A failure of the client tells nothing of its cause (SR-6.4), so the test adds what the server wrote.
    let url = format!("rediss://{}/", server.name);
    runtime()?.block_on(round_trip(Setting::Server { url, authorities: None })).map_err(|error| {
        let message = match error.message() {
            Some(message) => message.to_owned(),
            None => String::new(),
        };
        Failed::from(format!("{message}\nthe server at {} wrote:\n{}", server.name, project.output("server")))
    })
}

fn sentinel_of_tls_names_a_primary_of_tls() -> Result<(), Failed> {
    // SR-6.1b, SR-6.2: the Sentinel node and the primary accept TLS alone with the certificates that bay issues, whose
    // authority the roots of webpki do not hold: the store connects to both only with the authorities that it read.
    let configuration = format!("{MOUNT}/sentinel.conf");
    let mut project = Project::new(vec![
        Service::new("primary", PORT, &PLAIN),
        Service::new("sentinel", 26379, &[&configuration, "--sentinel"]).mount("sentinel", false).after("primary"),
    ])
    .map_err(Failed::from)?;
    tls_only(&mut project, "primary", PORT, false)?;
    tls_only(&mut project, "sentinel", 26379, true)?;
    let primary = project.planned("primary").map_err(Failed::from)?.host.clone();
    std::fs::write(
        project.directory().join("sentinel").join("sentinel.conf"),
        format!(
            "protected-mode no\nsentinel resolve-hostnames yes\nsentinel announce-hostnames yes\nsentinel monitor service {primary} {PORT} 1\n"
        ),
    )
    .map_err(failed)?;
    project.start().map_err(Failed::from)?;
    let sentinel = project.address("sentinel").map_err(Failed::from)?;
    let setting = Setting::Sentinel {
        urls: vec![format!("rediss://{}/", sentinel.name)],
        service: "service".to_owned(),
        tls: true,
        user: None,
        password: None,
        authorities: None,
    };
    runtime()?.block_on(round_trip(setting)).map_err(|error| {
        let message = match error.message() {
            Some(message) => message.to_owned(),
            None => String::new(),
        };
        Failed::from(format!(
            "{message}\nsentinel wrote:\n{}\nprimary wrote:\n{}",
            project.output("sentinel"),
            project.output("primary")
        ))
    })
}

fn a_cluster_of_tls_keeps_sessions() -> Result<(), Failed> {
    // SR-6.1b, SR-6.3: the nodes accept TLS alone, also between themselves, and name each other by the names of their
    // certificates, whose authority the roots of webpki do not hold.
    let names = ["node-0", "node-1", "node-2"];
    let mut services = Vec::new();
    for name in names {
        let mut arguments = PLAIN.to_vec();
        arguments.extend_from_slice(&["--cluster-enabled", "yes", "--cluster-config-file", "nodes.conf"]);
        services.push(Service::new(name, PORT, &arguments));
    }
    let mut project = Project::new(services).map_err(Failed::from)?;
    for name in names {
        tls_only(&mut project, name, PORT, true)?;
        let host = project.planned(name).map_err(Failed::from)?.host.clone();
        project
            .extend(name, &["--cluster-announce-hostname", &host, "--cluster-preferred-endpoint-type", "hostname"])
            .map_err(Failed::from)?;
    }
    let authority = project.planned("node-0").map_err(Failed::from)?.certificate.server_authority.clone();
    project.start().map_err(Failed::from)?;
    let mut reports = Vec::new();
    let mut nodes = Vec::new();
    let mut addresses = Vec::new();
    for name in names {
        reports.push(project.follow(name).map_err(Failed::from)?);
        let node = project.address(name).map_err(Failed::from)?;
        nodes.push(format!("rediss://{}/", node.name));
        addresses.push(format!("{}:{PORT}", node.ipv4));
    }
    let mut create = vec!["redis-cli", "--tls", "--cacert", &authority, "--insecure", "--cluster", "create"];
    create.extend(addresses.iter().map(String::as_str));
    create.extend(["--cluster-replicas", "0", "--cluster-yes"]);
    project.exec("node-0", &create).map_err(Failed::from)?;
    for report in &mut reports {
        report.wait_for("Cluster state changed: ok").map_err(Failed::from)?;
    }
    runtime()?.block_on(round_trip(Setting::Cluster { urls: nodes, user: None, password: None, authorities: None }))
}

fn the_stores_of_one_setting_share_one_connection() -> Result<(), Failed> {
    // SR-6.9: a store made for each request takes the connection that an earlier store of an equal setting made, and
    // a store of a setting that differs in its password shares none, so the wrong password fails and tells nothing.
    let (project, server) = plain(&["--user", "alice", "on", ">sesame", "~*", "&*", "+@all"])?;
    let address = server.name.as_str();
    let right = Setting::Server { url: format!("redis://alice:sesame@{address}/"), authorities: None };
    let runtime = runtime()?;
    for _request in 0..3 {
        runtime.block_on(round_trip(right.clone()))?;
    }
    let clients = project.exec("server", &["redis-cli", "CLIENT", "LIST"]).map_err(Failed::from)?;
    let alice = clients.lines().filter(|line| line.contains(" user=alice ")).count();
    if alice != 1 {
        return Err(format!("three stores of one setting hold {alice} connections, not 1: {clients}").into());
    }
    let wrong = Setting::Server { url: format!("redis://alice:wrongword@{address}/"), authorities: None };
    refused(&runtime, wrong.clone(), "a setting that differs in its password")?;
    let store = RedisSessions::new(&wrong, &options()).map_err(failed)?;
    let opened = runtime.block_on(store.open(Some(&[b'a'; 32])));
    let told = format!("{store:?} {opened:?}");
    if told.contains("wrongword") || told.contains("sesame") {
        return Err(format!("the store or its error tells a password: {told}").into());
    }
    Ok(())
}

fn a_failed_connection_is_made_again_by_the_next_request() -> Result<(), Failed> {
    // SR-6.9, SR-6.4: the shared connection of a setting ends with the runtime that drives it; the request that finds it
    // ended fails with connection, and the next request of an equal setting connects again.
    let (_project, server) = plain(&[])?;
    let setting = Setting::Server { url: format!("redis://{}/", server.name), authorities: None };
    let first = runtime()?;
    first.block_on(round_trip(setting.clone()))?;
    drop(first);
    let second = runtime()?;
    let store = RedisSessions::new(&setting, &options()).map_err(failed)?;
    match second.block_on(store.open(Some(&[b'a'; 32]))) {
        Err(error) if error.kind() == "connection" => {}
        Ok(opened) => {
            second.block_on(opened.close()).map_err(failed)?;
        }
        Err(error) => return Err(format!("the request on an ended connection gives {error:?}").into()),
    }
    second.block_on(round_trip(setting))
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
        Trial::test("the_stores_of_one_setting_share_one_connection", the_stores_of_one_setting_share_one_connection),
        Trial::test(
            "a_failed_connection_is_made_again_by_the_next_request",
            a_failed_connection_is_made_again_by_the_next_request,
        ),
        Trial::test("sentinel_names_the_primary", sentinel_names_the_primary),
        Trial::test("a_cluster_keeps_sessions", a_cluster_keeps_sessions),
        Trial::test("a_cluster_with_an_acl_user_keeps_sessions", a_cluster_with_an_acl_user_keeps_sessions),
        Trial::test(
            "a_file_of_authorities_is_the_only_trust_of_its_setting",
            a_file_of_authorities_is_the_only_trust_of_its_setting,
        ),
        Trial::test("a_tls_connection_reads_and_writes", a_tls_connection_reads_and_writes),
        Trial::test("sentinel_of_tls_names_a_primary_of_tls", sentinel_of_tls_names_a_primary_of_tls),
        Trial::test("a_cluster_of_tls_keeps_sessions", a_cluster_of_tls_keeps_sessions),
    ];
    libtest_mimic::run(&arguments, trials).exit();
}
