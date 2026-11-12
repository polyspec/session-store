# AGENTS.md

Rules for every change of session-store. The rules of the library are in [docs/spec/sessions.md](docs/spec/sessions.md).

## Layout

- `packages/rust/`: the crate `polyspec-session-store`, the library, which gives the items of the three crates again
  (the stores under the features `file` and `redis`) and whose package holds the one interface contract of the bindings,
  `interface.json` (`include` of its `Cargo.toml`), so that a repository that depends on the crate reads it through
  Cargo. No binding is written by hand.
- `packages/core/rust/`: the crate `polyspec-session-store-core`, the store interface, identifiers, the record and the
  record session of a request (SE-1 to SE-10). It does no I/O.
- `packages/file/rust/`: the crate `polyspec-session-store-file`, the store of files (SF-1 to SF-9).
- `packages/redis/rust/`: the crate `polyspec-session-store-redis`, the store of Redis: its protocol without I/O and its
  connection (SR-1 to SR-9).
- `packages/php-ext/`, `packages/python/`, `packages/ts/`: the binding packages, whose tracked files `bindgen scaffold`
  writes and the contract check compares (bindgen CK-19). They are members of the workspace; `make bindings` writes
  their `generated/`, which is not tracked, and builds of them come after it. The targets of the core name the core
  crates with `-p`, so they need no generated source.
- `scripts/bindgen.mts`: runs bindgen at the commit of `polyspec-bindgen-core` that `Cargo.lock` pins.
- `Cargo.toml` of the root is the workspace of the four crates and the three binding crates with one `Cargo.lock`.
- `contracts/sessions.json`, `contracts/file-sessions.json`, `contracts/redis-sessions.json`: the cases (SE-10).
- `docs/spec/`: the specification. Root: `AGENTS.md`, `README.md`, `CHANGELOG.md` and their Korean translations
  (`*.ko.md`), `Makefile`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`.

## Changes

- A change of a rule changes the specification first, then the cases, then the code. A test names the rule statement
  that it checks (`SE-n.m`, `SF-n.m`, `SR-n.m`).
- A rule states what it requires and its basis: a rule id of this library, a rule of php-serialization (RQ-n), a section
  of a standard or the behavior of a peer (PHP, phpredis). A tracked file does not record where a rule was taken from or
  who asked for it.
- The library does not know what a session holds: no rule of this repository names a key of a session or gives a value
  a meaning (SE-1.2).
- A failure has a kind; the library never turns it into a plausible value. A blocking Rust function takes the wait
  token of bindgen-core (`Io`, `ProcessWait`) as its first argument.
- The specification, the cases, the README, this file and the change log (English and Korean) change in the same
  change as the code.

## Rust

- The toolchain is the release that `rust-toolchain.toml` pins. `make check` runs clippy with `-D warnings`, rustfmt,
  the tests and the contract check.
- The lints of the workspace deny every warning, unsafe code, `unwrap`, `expect`, `as` conversions and the casts that
  lose a value, `let _` of a value that must be used or of an untyped value, and an allow attribute without a reason.
  `clippy.toml` disallows `Option::{unwrap_or, unwrap_or_default, unwrap_or_else, map_or, map_or_else}` and
  `Result::{ok, unwrap_or, unwrap_or_default, unwrap_or_else, map_or, map_or_else}`. A use that a rule needs carries
  `#[allow(..., reason = "<rule id>")]`.
- The tests that need a peer (PHP, redis-server, redis-cli, openssl) run when the Makefile finds it and are reported
  as ignored otherwise.

## Platforms

Linux x64 and arm64 (glibc) and macOS arm64. Windows is not supported until the Rust cores and the bindings of polyspec
are stable and ext-php-rs builds on Windows with a pinned stable toolchain.

## Commits

- A commit message is `<type>(<scope>): <Subject> (#id)`, factual and concise, with a body that states what changed and
  why. Unrelated changes are separate commits.
- No tracked file names a Git commit.
