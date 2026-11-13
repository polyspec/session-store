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
- `scripts/test-run.sh`: runs the tests of `make test` as a run that owns its containers and its temporary directory (SE-10.4).
- `Cargo.toml` of the root is the workspace of the four crates and the binding crates of the five targets with one
  `Cargo.lock`.
- `contracts/sessions.json`, `contracts/file-sessions.json`, `contracts/redis-sessions.json`: the cases (SE-10).
- `scripts/workflows.test.mts`: the test of the rules of the GitHub workflows, which `make workflows-test` runs.
- `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/actions/bootstrap`: CI, the release and the
  bootstrap of their jobs. `.node-version` names the Node.js that the bootstrap sets up; a development machine runs its
  own Node.js within the `engines` of kit.
- `config/release.json`: the classification of every manifest for the release, which the release utilities of kit read.
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
  the tests, the contract check and `make docs-check`.
- `make docs-check` reads each document that `config/documents.json` selects, with its links, anchors and fences and
  the `doc-id` and `source-sha256` of each Korean twin, and the checklist `docs/plans/rust-core/checklist.md`, whose
  rows `config/checklist.json` declares; `make bootstrap` installs the @polyspec/kit that it runs. The plan of the
  repository is `docs/plans/rust-core/plan.md`; a task is a row of that checklist, marked `[o]` only after its
  verification command passed on the committed tree. Run it before a commit. After a Korean twin is translated again,
  `make documents-stamp PATHS=<twin>` writes its `source-sha256`; no marker is written by hand.
- The lints of the workspace deny every warning, unsafe code, `unwrap`, `expect`, `as` conversions and the casts that
  lose a value, `let _` of a value that must be used or of an untyped value, and an allow attribute without a reason.
  `clippy.toml` disallows `Option::{unwrap_or, unwrap_or_default, unwrap_or_else, map_or, map_or_else}` and
  `Result::{ok, unwrap_or, unwrap_or_default, unwrap_or_else, map_or, map_or_else}`. A use that a rule needs carries
  `#[allow(..., reason = "<rule id>")]`.
- A test that needs a peer (PHP, Apple container with the Redis image, openssl) fails when the peer is not given and
  names what gives it; no test is reported as ignored because a peer is missing. The only ignored tests are the cases
  of the group `open` that carry `binding`, which the bindings run (SF-8.2, SR-8.2).
- A Redis server of a test runs in a container of Apple `container` (`CONTAINER`) from the image that `REDIS_IMAGE` of
  the Makefile pins by its digest, never a server installed on the machine (SE-10.4). The image is the official image
  of the highest stable release of Redis; `make redis-image` pulls it, and no test pulls an image. The containers of a
  run are named `session-store-test-<pid>-...`, and `scripts/test-run.sh` deletes them and the temporary directory of
  the run when the run ends, fails or is stopped by SIGINT or SIGTERM. A test connects to the address of its container
  and waits for the lines of the server, never for a time.

## Bindings

- `make bindings` generates, each target of the contract apart (`make bindings-<target>`, binding GN-1b), so that a
  target that the generator refuses fails its own make targets alone; every make target that builds or tests a binding
  runs the generation of its target first. `make test-bindings` runs
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

## CI and release

- Every job of a GitHub workflow runs on `ubuntu-26.04-arm`. Every action is a local action of `.github/actions` or is
  pinned by its full commit, and a comment of the file names its release; an action pin is the one Git commit that a
  tracked file names. No job and no step sets `timeout-minutes`. Every `run` step is one make target that the
  `Makefile` defines, so that the settings of the Makefile apply to every command of CI. `make workflows-test` fails a
  workflow or an action that breaks one of these rules.
- The workflow `ci` starts on a push of `main` and on `workflow_dispatch` alone. Its job `shared` runs
  `make ci-shared` (`CI_SHARED` of the Makefile) and its job `rust` runs `make install-rust` and `make ci-rust`
  (`CI_RUST`: `make lint-rust`, `make fmt-check` and `make binding-check`); each runs its targets past failures with
  ci-targets of kit and uploads the report `var/report/ci-targets`.
- `make test` is no target of CI. Its tests of the store of Redis start Redis servers in Apple container (SE-10.4),
  which a Linux runner does not have, and a test whose peer is missing fails. So no run of CI is evidence of the tests:
  `make test` runs on a development machine, and its counts are recorded in the change log.
- The workflow `release` starts when a run of `ci` succeeds on `main`, or by a dispatch with the id of such a run. It
  runs `make release-tag`, `make release-checkout`, `make release-tools`, `make release-assets`,
  `make release-publish` and `make release-undraft` in that order.
- `config/release.json` classifies every manifest of the repository, and `make release-coverage` fails a manifest that
  it does not classify. It names no package: the binding packages are released once kit 0.0.111 makes the platform
  assets and the Go module has its tag form, the release commit and `packages/go/vX.Y.Z`. Until then a release is the
  tag `vX.Y.Z` and the GitHub Release with the notes of the section `## X.Y.Z` of the change log.
- A version changes only in the commit `chore(release): Release X.Y.Z`, which is made at the release. It sets the
  version in `[workspace.package]` of `Cargo.toml` and in the binding manifests that `binding scaffold` writes again
  from it, writes `Cargo.lock` again, and renames `## Unreleased` of both change logs to `## X.Y.Z` with a new empty
  `## Unreleased` above it. No tag is created by hand: `make release-tag RUN=<id>` tags the commit of the successful
  run `<id>` of the workflow `ci` on `main` with the version of the first section `## X.Y.Z` of its change log, and
  `make release-assets`, `make release-publish` and `make release-undraft` build the archives of the packages of
  `config/release.json` and publish the GitHub Release of the tag.

## Commits

- A commit message is `<type>(<scope>): <Subject> (#id)`, factual and concise, with a body that states what changed and
  why. Unrelated changes are separate commits.
- No tracked file names a Git commit, except the pin of an action in a workflow.
