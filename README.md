# sessions

[한국어](README.ko.md)

Stores the values of browser sessions by one Rust implementation: in memory, in the files of a directory in the form of
the files handler of PHP, or in Redis with the keys and the lock of phpredis. The library does not know what a session
holds: a key is a text and a value is a PHP value of php-serialize. The rules are in
[docs/spec/sessions.md](docs/spec/sessions.md); the cases that every implementation passes are in
[contracts/](contracts/).

- `polyspec-sessions-core`: the store interface (`Store`), which an application may implement in its own language, the
  memory store, session identifiers, the record of a session (`php_serialize`) and the record session of one request,
  which starts at its first use and gives what its store writes.
- `polyspec-sessions-file`: one file `sess_<id>` per session under `flock(LOCK_EX)`, a queue of the requests of one
  session in a process, renewal and the removal of ended sessions (`collect`).
- `polyspec-sessions-redis`: the protocol of phpredis as a state machine without I/O (`EXISTS`, `SET NX PX`, `GET`,
  the lock compared before `SETEX`, `EXPIRE` for an unchanged session, the release script) and its connection with
  redis-rs to one server, Sentinel or Cluster, with TLS and an ACL user.

## Rust

```rust
use bindgen_core::{Io, ProcessWait};
use polyspec_sessions::{FileSessions, Store};

let store = FileSessions::new(io, "/var/lib/sessions".as_ref(), None)?;
let mut session = store.open(wait, cookie.as_deref(), None)?;
let value = session.get("cart")?;
let created = session.close(io)?; // the new identifier, which the caller sets as its session cookie
```

The wait tokens `Io` and `ProcessWait` are made by the generated bindings; a Rust caller of a blocking function gets them
from the binding layer that runs it.

The crate `polyspec-sessions` (`packages/rust`) gives the items of the three crates as one library; PHP, Python and Node
bindings will be generated from its interface contract `packages/rust/interface.json`, which its package holds; there is
no binding yet.

## Checks

```
make check   # clippy -D warnings, rustfmt --check, the tests and the contract check
```

Requirements: the Rust release of `rust-toolchain.toml`. The tests read records back with PHP and run the stores of Redis
against real servers when `php`, `redis-server`, `redis-cli` and `openssl` are installed. Windows is not supported yet.
