//! The connection over TLS against a real Redis server (SR-6.1), a service of a Compose project that bay runs from the
//! pinned image with the certificate that bay issues for its name (`tests/servers`, SE-10.4). The store verifies the
//! server against the trust store of its process, which rustls reads from the file that `SSL_CERT_FILE` names. The
//! environment of a process belongs to all its threads, so this test is a binary of its own with one trial: `main`
//! names the authority file of bay before a second thread starts.

mod servers;
mod store;

use std::process::ExitCode;

use libtest_mimic::{Arguments, Failed, Trial};
use polyspec_session_store_redis::Setting;
use servers::{Certificate, Project, Service, PLAIN, PORT};
use store::{round_trip, runtime};

/// The project of the server, not started, and the certificate that bay plans for the server.
fn planned() -> Result<(Project, Certificate), String> {
    let project = Project::new(vec![Service::new("server", PORT, &PLAIN)])?;
    let certificate = project.planned("server")?.certificate.clone();
    Ok((project, certificate))
}

fn a_tls_connection_reads_and_writes(planned: Result<(Project, Certificate), String>) -> Result<(), Failed> {
    // SR-6.1: rediss:// to a server that accepts TLS alone, with the certificate that bay issues for its name, at
    // the paths in the container that the plan of the project gives.
    let (mut project, certificate) = planned.map_err(Failed::from)?;
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
    runtime()?.block_on(round_trip(Setting::Server { url: format!("rediss://{}/", server.name) })).map_err(|error| {
        let message = match error.message() {
            Some(message) => message.to_owned(),
            None => String::new(),
        };
        Failed::from(format!("{message}\nthe server at {} wrote:\n{}", server.name, project.output("server")))
    })
}

fn main() -> ExitCode {
    let arguments = Arguments::from_args();
    // The plan gives the authority file before the server starts. A listing runs no trial and plans nothing.
    let planned = if arguments.list { Err("the trial is listed and does not run".to_owned()) } else { planned() };
    // No second thread of this process runs yet: the trial, the runtime of the store and the readers of rustls start
    // after the environment names the file.
    if let Ok((_project, certificate)) = &planned {
        std::env::set_var("SSL_CERT_FILE", &certificate.authority);
    }
    let trials =
        vec![Trial::test("a_tls_connection_reads_and_writes", move || a_tls_connection_reads_and_writes(planned))];
    libtest_mimic::run(&arguments, trials).exit_code()
}
