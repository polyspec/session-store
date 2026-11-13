//! The Redis servers of the tests (SE-10.4): each runs as `redis-server` of the pinned image (`REDIS_IMAGE`) in a
//! container of Apple `container` (`CONTAINER`), which the run owns. A container is named after the run
//! (`SESSION_STORE_CONTAINERS`), so that `make test` removes every container of its run when the run ends or is
//! stopped by a signal. A test connects to the address of the container, where the server listens on the port of its
//! own; a start waits for the line in which the server reports that it accepts connections, and a test waits for a
//! later report of the server by its line, never by a time. A peer that is not given is a failure that names the make
//! target which gives it.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver};

/// The port of a server in its container.
pub const PORT: u16 = 6379;

/// The arguments of a server that keeps nothing on disk and takes connections from the host, which reaches the
/// container by its address.
pub const PLAIN: [&str; 6] = ["--save", "", "--appendonly", "no", "--protected-mode", "no"];

/// Where a test directory is mounted in a container.
#[allow(dead_code, reason = "SE-10.4: tests/server.rs mounts a directory, tests/contract.rs does not")]
pub const MOUNT: &str = "/mnt/session-store";

/// The line in which redis-server reports that it accepts connections.
pub const READY: &str = "Ready to accept connections";

fn peer(name: &str) -> Result<String, String> {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => Ok(value),
        _ => Err(format!(
            "{name} is not set: the tests of a real Redis server run through `make test`, which names Apple container \
             (CONTAINER), the pinned image (REDIS_IMAGE) and the containers of the run (SESSION_STORE_CONTAINERS); \
             `make redis-image` pulls the image"
        )),
    }
}

/// A name of a container of this run, which no other container of the run has.
fn name() -> Result<String, String> {
    static STARTED: AtomicUsize = AtomicUsize::new(0);
    let run = peer("SESSION_STORE_CONTAINERS")?;
    Ok(format!("{run}-{}-{}", std::process::id(), STARTED.fetch_add(1, Ordering::Relaxed)))
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

/// A Redis server in a container of the run. Dropping it deletes the container and stops the reader of its output.
pub struct Server {
    container: String,
    image: String,
    name: String,
    child: Child,
    lines: Receiver<String>,
    address: String,
}

impl Server {
    /// Starts `redis-server` with `arguments`, mounting `directory` at [`MOUNT`] when it is given, and returns once
    /// the server writes a line that contains `ready`: [`READY`] of a server, which Sentinel does not write.
    pub fn start(arguments: &[&str], directory: Option<&Path>, ready: &str) -> Result<Self, String> {
        let container = peer("CONTAINER")?;
        let image = peer("REDIS_IMAGE")?;
        let name = name()?;
        if run(Command::new(&container).args(["image", "inspect", &image]).stdout(Stdio::null())).is_err() {
            return Err(format!("the image {image} is not installed: `make redis-image` pulls it"));
        }
        let mut command = Command::new(&container);
        command.args(["run", "--rm", "--progress", "none", "--name", &name]);
        if let Some(directory) = directory {
            command.arg("--volume").arg(format!("{}:{MOUNT}", directory.display()));
        }
        // The container writes its own address before the server starts, because the state that `container inspect`
        // gives is not yet that of a running container when the server already writes. The entry point of the image
        // would run the server as the user redis, who cannot read the files that a test mounts; the server runs as
        // root of its own container.
        command
            .args(["--entrypoint", "sh", &image, "-c", "hostname -i && exec redis-server \"$@\"", "redis-server"])
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|error| format!("{container} does not run: {error}"))?;
        let (sender, lines) = channel();
        if let Some(stdout) = child.stdout.take() {
            let sender = sender.clone();
            std::thread::spawn(move || read(stdout, &sender));
        }
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || read(stderr, &sender));
        }
        let mut server = Self { container, image, name, child, lines, address: String::new() };
        server.address = server.own_address()?;
        server.wait_for(ready)?;
        Ok(server)
    }

    /// The first line of the container: the addresses of its host name, of which the test takes the IPv4 address.
    fn own_address(&mut self) -> Result<String, String> {
        let Ok(line) = self.lines.recv() else {
            return Err(format!("the container {} ends before it writes its address", self.name));
        };
        line.split_whitespace()
            .find(|word| word.parse::<std::net::Ipv4Addr>().is_ok())
            .map(str::to_owned)
            .ok_or_else(|| format!("the container {} writes no IPv4 address: {line:?}", self.name))
    }

    /// The address of the container, on which the server listens on [`PORT`].
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Waits for a line of the server that contains `text`, and fails when the server ends first.
    pub fn wait_for(&mut self, text: &str) -> Result<(), String> {
        let mut seen = Vec::new();
        while let Ok(line) = self.lines.recv() {
            if line.contains(text) {
                return Ok(());
            }
            seen.push(line);
        }
        Err(format!("the server {} ends before it reports {text:?}:\n{}", self.name, seen.join("\n")))
    }

    /// Runs `redis-cli` with `arguments` in a container of its own of the image, which ends with it, and returns its
    /// standard output.
    #[allow(dead_code, reason = "SE-10.4: tests/server.rs runs redis-cli, tests/contract.rs does not")]
    pub fn cli(&self, arguments: &[&str]) -> Result<String, String> {
        let output = run(Command::new(&self.container)
            .args(["run", "--rm", "--progress", "none", "--name", &name()?])
            .args(["--entrypoint", "redis-cli", &self.image])
            .args(arguments))?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Sends every line of the output of a container to the server that reads it.
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

impl Drop for Server {
    fn drop(&mut self) {
        if let Err(error) = run(Command::new(&self.container).args(["delete", "--force", &self.name])) {
            eprintln!("the container {} is not deleted: {error}", self.name);
        }
        if let Err(error) = self.child.wait() {
            eprintln!("the client of the container {} does not end: {error}", self.name);
        }
    }
}
