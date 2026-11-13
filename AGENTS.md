<!-- doc-id: agents -->
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
- `packages/php-ext/`, `packages/python/`, `packages/ts/`, `packages/ts/wasm/`, `packages/go/`: the binding packages
  of the five targets that every contract names (binding BD-27), the C ABI of Go in `packages/go/internal/ffi`, whose
  tracked files `binding scaffold` writes and the contract check compares (binding CK-19). They are members of the
  workspace; `make bindings` writes their `generated/`, which is not tracked, and builds of them come after it. The
  targets of the core name the core crates with `-p`, so they need no generated source.
- `scripts/binding.mts`: runs binding at the commit of `polyspec-binding-core` that `Cargo.lock` pins.
- `Cargo.toml` of the root is the workspace of the four crates and the binding crates of the five targets with one
  `Cargo.lock`.
- `contracts/sessions.json`, `contracts/file-sessions.json`, `contracts/redis-sessions.json`: the cases (SE-10).
- `docs/spec/`: the specification. Root: `AGENTS.md`, `README.md`, `CHANGELOG.md` and their Korean translations
  (`*.ko.md`), `Makefile`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `.cargo/config.toml`.

## Changes

- A change of a rule changes the specification first, then the cases, then the code. A test names the rule statement
  that it checks (`SE-n.m`, `SF-n.m`, `SR-n.m`).
- A rule states what it requires and its basis: a rule id of this library, a rule of php-serialization (RQ-n), a section
  of a standard or the behavior of a peer (PHP, phpredis). A tracked file does not record where a rule was taken from or
  who asked for it.
- The library does not know what a session holds: no rule of this repository names a key of a session or gives a value
  a meaning (SE-1.2).
- A failure has a kind; the library never turns it into a plausible value. A blocking Rust function takes the wait
  token of binding-core (`Io`, `ProcessWait`) as its first argument.
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

## Bindings

- `make bindings` generates; every make target that builds or tests a binding runs it first. `make test-bindings` runs
  clippy on the whole workspace, `make check-declarations`, then `make test-python`, `make test-node`, `make
  test-wasm`, `make test-php` and `make test-go`, one for each of the five targets of the contract, each of which runs
  the tests that binding generates and the contract cases of the binding on the standard runner of its language:
  unittest, node --test (Node and WASM, the WASM binding through the subpath `./wasm` of the same installed package),
  PHPUnit and go test. The contract cases are `packages/python/tests`, `packages/ts/tests/cases.test.mjs`,
  `packages/ts/tests/cases.wasm.test.mjs`, `packages/php-ext/tests` and `packages/go/tests`, which read the directory
  `contracts` through `SESSION_STORE_CASES`. A target that the generator cannot take stays a failing target with its
  cause; no target is left out or skipped (binding BD-27).
- No contract case runner is written yet: `make bindings` refuses the contract while the contract check reports the
  kinds of SE-10.6, so no binding API exists that a runner could call and be checked against. Each of the five targets
  fails at `make bindings` with that cause.
- In the Go module binding owns the files `zz_generated_*.go` (binding GN-3); they are tracked, because a Go module is
  fetched as source, and `make bindings` writes them again. `packages/ts/wasm/generated` and
  `packages/go/internal/ffi/generated` are not tracked; wasm-bindgen writes the module and its glue into the first.
  The variables `PYTHON` (3.14 or later), `PHP`, `PHP_CONFIG`, `PHPUNIT`, `TSC`, `GO`, `GOFMT` and `WASM_BINDGEN` name
  the runtimes and tools. `make test-go` fails a Go of another release than the go directive of `packages/go/go.mod`
  and runs it with `GOTOOLCHAIN=local`, so that it fetches no toolchain, and `make test-wasm` fails a wasm-bindgen CLI
  of another release than the `wasm-bindgen` crate of `packages/ts/wasm/Cargo.toml`, because the CLI reads only a
  module of its own release (binding GN-18). `CARGOFLAGS` adds arguments to every cargo build. On macOS a PHP extension
  and a Node addon take the symbols of their runtime when it loads them, so `.cargo/config.toml` tells the linker to
  leave them undefined, and the recipes name the library that Cargo builds by the extension of the platform (`dylib`
  on macOS, `so` on Linux).

## Platforms

Linux x64 and arm64 (glibc) and macOS arm64. Windows is not supported until the Rust cores and the bindings of polyspec
are stable and ext-php-rs builds on Windows with a pinned stable toolchain.

## Commits

- A commit message is `<type>(<scope>): <Subject> (#id)`, factual and concise, with a body that states what changed and
  why. Unrelated changes are separate commits.
- No tracked file names a Git commit.
