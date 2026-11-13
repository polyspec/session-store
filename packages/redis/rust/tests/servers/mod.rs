//! The Redis servers of the tests (SE-10.4): each runs as `redis-server` of the pinned image (`REDIS_IMAGE`) in a
//! service of a Compose project, which bay starts and removes on Apple `container` on Darwin and on Docker on Linux.
//! `scripts/servers.sh` holds the commands; this file starts that script alone. A test owns a [`Project`]: its
//! Compose file is in a directory of its own below the directory of the run (`SESSION_STORE_PROJECTS`) and is written
//! before the servers start, so that `make test` removes every project of its run when the run ends or is stopped by a
//! signal. The return of a start is its event: bay returns after the server accepts connections. A test connects to
//! the address that bay gives for the service and waits for a later report of a server by its line, never by a time.
//! A peer that is not given is a failure that names the make target which gives it.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver};

use serde_json::Value;

/// The port of a server in its container.
pub const PORT: u16 = 6379;

/// The arguments of a server that keeps nothing on disk and takes connections from the host.
pub const PLAIN: [&str; 6] = ["--save", "", "--appendonly", "no", "--protected-mode", "no"];

/// Where a directory of a project is mounted in a container.
#[allow(dead_code, reason = "SE-10.4: tests/server.rs mounts a directory, tests/contract.rs does not")]
pub const MOUNT: &str = "/mnt/session-store";

/// The script that holds the commands of the servers (SE-10.4).
const SERVERS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../scripts/servers.sh");

fn peer(name: &str) -> Result<String, String> {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => Ok(value),
        _ => Err(format!(
            "{name} is not set: the tests of a real Redis server run through `make test`, which names the pinned image \
             (REDIS_IMAGE) and the directory of the projects of the run (SESSION_STORE_PROJECTS); `make redis-image` \
             pulls the image"
        )),
    }
}

fn run(command: &mut Command) -> Result<Output, String> {
    let output = command.stdin(Stdio::null()).output().map_err(|error| format!("{command:?} does not run: {error}"))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "{command:?} fails with {}: {}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// A scalar of the Compose file in double quotes. A control character has no place in an argument of a server, and
/// Compose reads `$` as the start of a variable.
fn quote(text: &str) -> Result<String, String> {
    if text.chars().any(|character| character.is_control() || character == '$') {
        return Err(format!("{text:?} is not written into a Compose file: it holds a control character or `$`"));
    }
    Ok(format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\"")))
}

/// A server of a project: `redis-server` with its arguments, the port on which it accepts connections, the directory
/// of the project that it reads at [`MOUNT`] and the services that start before it.
pub struct Service {
    name: String,
    arguments: Vec<String>,
    port: u16,
    mount: Option<(String, bool)>,
    after: Vec<String>,
}

impl Service {
    /// The service `name`, which runs `redis-server` with `arguments` and accepts connections on `port`.
    pub fn new(name: &str, port: u16, arguments: &[&str]) -> Self {
        Self {
            name: name.to_owned(),
            arguments: arguments.iter().map(|argument| (*argument).to_owned()).collect(),
            port,
            mount: None,
            after: Vec::new(),
        }
    }

    /// Mounts `directory`, a directory of the directory of the project, at [`MOUNT`].
    #[allow(dead_code, reason = "SE-10.4: tests/server.rs mounts a directory, tests/contract.rs does not")]
    pub fn mount(mut self, directory: &str, read_only: bool) -> Self {
        self.mount = Some((directory.to_owned(), read_only));
        self
    }

    /// Starts the service after `service` has started.
    #[allow(dead_code, reason = "SE-10.4: tests/server.rs orders two servers, tests/contract.rs has one")]
    pub fn after(mut self, service: &str) -> Self {
        self.after.push(service.to_owned());
        self
    }
}

/// The address that bay gives for a started service.
pub struct Address {
    /// The name and the port, `<name>:<port>`.
    pub name: String,
    /// The IPv4 address of the container, for a command of Redis that takes no name.
    #[allow(dead_code, reason = "SE-10.4: tests/server.rs introduces the nodes of a cluster by it")]
    pub ipv4: String,
}

impl Address {
    /// The name without the port.
    #[allow(dead_code, reason = "SE-10.4: tests/server.rs names a server in a certificate and a configuration")]
    pub fn host(&self) -> &str {
        match self.name.rsplit_once(':') {
            Some((host, _port)) => host,
            None => &self.name,
        }
    }
}

/// A Compose project of one test. Dropping it removes its servers, ends the readers of their output and removes its
/// directory.
pub struct Project {
    name: String,
    directory: PathBuf,
    image: String,
    services: Vec<Service>,
    followers: Vec<Child>,
}

impl Project {
    /// A project without a server, named after the run, this process and a count, with a directory of its own.
    pub fn new() -> Result<Self, String> {
        static MADE: AtomicUsize = AtomicUsize::new(0);
        let image = peer("REDIS_IMAGE")?;
        let projects = PathBuf::from(peer("SESSION_STORE_PROJECTS")?);
        let Some(prefix) = projects.file_name().and_then(std::ffi::OsStr::to_str) else {
            return Err(format!("SESSION_STORE_PROJECTS names no directory: {}", projects.display()));
        };
        let name = format!("{prefix}-{}-{}", std::process::id(), MADE.fetch_add(1, Ordering::Relaxed));
        // An engine that is not found fails here, with its name and what installs it; no start pulls an image.
        if let Err(error) = run(Command::new(SERVERS).args(["present", &image]).stdout(Stdio::null())) {
            return Err(format!("the image {image} is not installed: `make redis-image` pulls it: {error}"));
        }
        let directory = projects.join(&name);
        std::fs::create_dir(&directory)
            .map_err(|error| format!("the directory {} is not made: {error}", directory.display()))?;
        Ok(Self { name, directory, image, services: Vec::new(), followers: Vec::new() })
    }

    /// The directory of the project, whose directories a service mounts.
    #[allow(dead_code, reason = "SE-10.4: tests/server.rs mounts a directory, tests/contract.rs does not")]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    fn file(&self) -> PathBuf {
        self.directory.join("compose.yaml")
    }

    /// The Compose file of the services: the name of the project, and for each service the image by its digest, the
    /// server with its arguments, its port, its volume and the services before it. No port is published.
    fn compose(&self) -> Result<String, String> {
        let mut lines = vec![format!("name: {}", quote(&self.name)?), "services:".to_owned()];
        for service in &self.services {
            let arguments = service
                .arguments
                .iter()
                .map(|argument| quote(argument.as_str()))
                .collect::<Result<Vec<String>, String>>()?
                .join(", ");
            lines.push(format!("  {}:", quote(&service.name)?));
            lines.push(format!("    image: {}", quote(&self.image)?));
            // The entry point of the image would run the server as the user redis, who cannot read the files that a
            // test mounts; the server runs as root of its own container.
            lines.push("    entrypoint: [\"redis-server\"]".to_owned());
            lines.push(format!("    command: [{arguments}]"));
            lines.push(format!("    expose: [\"{}\"]", service.port));
            if let Some((directory, read_only)) = &service.mount {
                let mode = if *read_only { ":ro" } else { "" };
                lines.push(format!("    volumes: [{}]", quote(&format!("./{directory}:{MOUNT}{mode}"))?));
            }
            if !service.after.is_empty() {
                lines.push("    depends_on:".to_owned());
                for other in &service.after {
                    lines.push(format!("      {}:", quote(other)?));
                    lines.push("        condition: service_started".to_owned());
                }
            }
        }
        lines.push(String::new());
        Ok(lines.join("\n"))
    }

    /// Starts `service`, or replaces the service of its name, and returns its address once it accepts connections.
    /// The Compose file is written before the start; bay keeps the services of the file that did not change.
    pub fn start(&mut self, service: Service) -> Result<Address, String> {
        let name = service.name.clone();
        self.services.retain(|other| other.name != name);
        self.services.push(service);
        let file = self.file();
        std::fs::write(&file, self.compose()?)
            .map_err(|error| format!("{} is not written: {error}", file.display()))?;
        run(Command::new(SERVERS).arg("up").arg(&file))?;
        self.address(&name)
    }

    /// The address of a started service, which bay gives in the state of its projects.
    fn address(&self, service: &str) -> Result<Address, String> {
        let output = run(Command::new(SERVERS).arg("status"))?;
        let status: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("`bay status --json` gives no JSON: {error}"))?;
        let found = status
            .get("groups")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|group| group.get("name").and_then(Value::as_str) == Some(self.name.as_str()))
            .filter_map(|group| group.get("services").and_then(Value::as_array))
            .flatten()
            .find(|entry| entry.get("name").and_then(Value::as_str) == Some(service));
        let Some(found) = found else {
            return Err(format!("`bay status --json` names no service {service} of the project {}", self.name));
        };
        let text = |field: &str| -> Result<String, String> {
            match found.get(field).and_then(Value::as_str) {
                Some(value) if !value.is_empty() => Ok(value.to_owned()),
                _ => Err(format!(
                    "`bay status --json` gives no {field} for the service {service} of the project {}: {found}",
                    self.name
                )),
            }
        };
        Ok(Address { name: text("address")?, ipv4: text("ipv4")? })
    }

    /// The output of a started service from its first line, which a reader follows until the project is dropped.
    #[allow(dead_code, reason = "SE-10.4: tests/server.rs waits for a line of a node, tests/contract.rs for none")]
    pub fn follow(&mut self, service: &str) -> Result<Lines, String> {
        let mut command = Command::new(SERVERS);
        command
            .arg("logs")
            .arg(self.file())
            .arg(service)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|error| format!("{command:?} does not run: {error}"))?;
        let (sender, lines) = channel();
        if let Some(stdout) = child.stdout.take() {
            let sender = sender.clone();
            std::thread::spawn(move || read(stdout, &sender));
        }
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || read(stderr, &sender));
        }
        self.followers.push(child);
        Ok(Lines { service: format!("{service} of the project {}", self.name), lines })
    }
}

/// The lines of the output of a service.
#[allow(dead_code, reason = "SE-10.4: tests/server.rs waits for a line of a node, tests/contract.rs for none")]
pub struct Lines {
    service: String,
    lines: Receiver<String>,
}

impl Lines {
    /// Waits for a line that contains `text`, and fails when the output ends first.
    #[allow(dead_code, reason = "SE-10.4: tests/server.rs waits for a line of a node, tests/contract.rs for none")]
    pub fn wait_for(&mut self, text: &str) -> Result<(), String> {
        let mut seen = Vec::new();
        while let Ok(line) = self.lines.recv() {
            if line.contains(text) {
                return Ok(());
            }
            seen.push(line);
        }
        Err(format!("the output of the service {} ends before it reports {text:?}:\n{}", self.service, seen.join("\n")))
    }
}

/// Sends every line of the output of a service to the test that reads it.
#[allow(dead_code, reason = "SE-10.4: tests/server.rs waits for a line of a node, tests/contract.rs for none")]
fn read(output: impl std::io::Read, sender: &std::sync::mpsc::Sender<String>) {
    for line in BufReader::new(output).lines() {
        let Ok(line) = line else {
            break;
        };
        if sender.send(line).is_err() {
            break;
        }
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        for child in &mut self.followers {
            if let Err(error) = child.kill() {
                eprintln!("a reader of the output of the project {} is not ended: {error}", self.name);
            }
            if let Err(error) = child.wait() {
                eprintln!("a reader of the output of the project {} does not end: {error}", self.name);
            }
        }
        // A project without a Compose file started nothing. A project that is not removed keeps its file, so that the
        // run removes it or names it.
        let file = self.file();
        if file.exists() {
            if let Err(error) = run(Command::new(SERVERS).arg("down").arg(&file)) {
                eprintln!("the servers of the project {} are not removed: {error}", self.name);
                return;
            }
        }
        if let Err(error) = std::fs::remove_dir_all(&self.directory) {
            eprintln!("the directory {} is not removed: {error}", self.directory.display());
        }
    }
}
