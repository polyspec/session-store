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
- `scripts/test-run.sh`: runs the tests of `make test` as a run that owns its servers and its temporary directory (SE-10.4).
- `scripts/servers.sh`: the commands of the Redis servers of the tests, which the tests, `scripts/test-run.sh` and
  `make redis-image` run (SE-10.4): bay starts and removes the servers as Compose projects, and the engine of the
  platform, Apple `container` on Darwin and `docker` on Linux, pulls and finds the image. `scripts/servers.test.mts`
  is the test of both scripts, which `make servers-test` runs against stand-ins of bay and of the engines.
- `Cargo.toml` of the root is the workspace of the four crates and the binding crates of the five targets with one
  `Cargo.lock`.
- `contracts/sessions.json`, `contracts/file-sessions.json`, `contracts/redis-sessions.json`: the cases (SE-10).
- `scripts/workflows.test.mts`: the test of the rules of the GitHub workflows, which `make workflows-test` runs.
- `scripts/check-commit-names.mts`: the check of `make commit-names-check`, with its test
  `scripts/commit-names.test.mts`, which `make commit-names-test` runs.
- `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/actions/bootstrap`: CI, the release and the
  bootstrap of their jobs, which runs `make bootstrap`, `make install-tools TOOLS=npm` and `make install-packages`.
- `mise.toml`, `mise.lock`, `.mise/locks`, `config/toolchain.json`: the pins of the tools of the checkout ("Tools").
  `scripts/tool-pins.test.mts` is their test, which `make tool-pins-test` runs.
- `config/owner-checks.json`: the make targets that own each tracked path. `make owner-validate` fails a tracked path
  that no rule owns and a target that the Makefile does not define; a change that adds a path adds its owner.
  `make owner-check PATHS="<paths>"` runs the owners of the named paths.
- `config/release.json`: the classification of every manifest for the release, which the release utilities of kit read.
- `docs/spec/`: the specification. Root: `AGENTS.md`, `README.md`, `CHANGELOG.md` and their Korean translations
  (`*.ko.md`), `Makefile`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `.cargo/config.toml`.
- `LICENSE`: the text of the MIT license, which the manifests name (`license` of `[workspace.package]` of `Cargo.toml`,
  of `package.json`, of `packages/php-ext/composer.json` and of `packages/ts/package.json`).

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

## Tools

- Every tool whose release changes a result is pinned in a tracked file, because a tool of the machine gives a result
  that another machine does not repeat: Node.js and npm in `mise.toml` at the digests of `mise.lock`, npm also in
  `devEngines` of `package.json`, mise in `config/toolchain.json` with the sha256 of its archive of each platform,
  Rust in `rust-toolchain.toml`, and the PHP of CI by its minor in `.php-version`. No `.node-version` is kept beside `mise.toml`, so that Node.js has one pin.
- The tools are installed into `var/tools` of the checkout, never into the machine. A fresh checkout runs
  `make bootstrap` (the npm of `devEngines` installs `package-lock.json`, @polyspec/kit included, on the node of the
  machine), `make install-tools TOOLS=npm` (the bootstrap of kit installs mise by its sha256, then the node and the npm
  of `mise.toml` with `mise install` under `locked = true`) and `make install-packages` (`npm ci` with the npm of
  `var/tools`). `make toolchain-lock` writes `mise.lock` again after a release of `mise.toml` changes.
- A recipe starts node by its path in `var/tools` (`NODE` of the Makefile). Only `make bootstrap` and
  `make install-tools`, which run before that node exists, start the node of the machine.
  Enforced by: `make tool-pins-test`.

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
- A test that needs a peer (PHP, bay and the engine of the platform with the Redis image, openssl) fails when the peer is not given and
  names what gives it; no test is reported as ignored because a peer is missing. The only ignored tests are the cases
  of the group `open` that carry `binding`, which the bindings run (SF-8.2, SR-8.2).
- A Redis server of a test runs in a container from the image that `REDIS_IMAGE` of the Makefile pins by its digest,
  never a server installed on the machine (SE-10.4). bay starts and removes the servers as Compose projects, on Apple
  `container` on Darwin and on Docker on Linux, one image and one digest for both, so that both platforms test the
  same bytes of the server. `scripts/servers.sh` is the one file that holds the commands; no test and no recipe
  starts bay or an engine by itself. bay, the engine of the platform or the machine setup of bay that is missing
  fails `make test` at its start with what is missing and what gives it (`bay install` applies the setup; no run
  applies it). The image is the official image of the highest stable release of Redis; `make redis-image` pulls it
  with the engine of the platform, and no test pulls an image.
- A test owns the servers that it needs as one Compose project, because the arguments of a server are part of what a
  test states and the state of a server must not reach another test. It writes `compose.yaml` into a directory of
  its own below `SESSION_STORE_PROJECTS`, the directory `session-store-test-<pid>` of the run, before it starts the
  project, and the file names the project after that directory, the test process and a count. The return of `up` is
  the event of a start; a test takes the address of a server from `bay status --json`, connects to it, and waits for
  a later report of a server by its line in `bay logs -f`, never for a time. No file states the domain of the names
  of bay, and no port is published. `scripts/test-run.sh` runs `down` for every Compose file that the directory of
  the run still holds and removes the directory when the run ends, fails or is stopped by SIGINT or SIGTERM.

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
  `make ci-shared` (`CI_SHARED` of the Makefile: `make docs-check`, `make owner-validate`, `make workflows-test`,
  `make tool-pins-test`, `make servers-test`, `make commit-names-check`, `make commit-names-test` and `make release-coverage`) and its job `rust` runs `make install-rust` and `make ci-rust`
  (`CI_RUST`: `make lint-rust`, `make fmt-check` and `make binding-check`); each runs its targets past failures with
  ci-targets of kit and uploads the report `var/report/ci-targets`.
- The last job of the workflow `ci` is `ci-passed`. It runs after every other job, also after a failed one, needs each
  of them, and `make ci-passed FAIL_SKIPPED=1` (ci-passed of kit) passes only when each has the result success, so that
  one job states the result of the run. `make workflows-test` fails a workflow `ci` whose last job is another or whose
  `needs` leaves a job out.
- The job `test` of the workflow `ci` sets up the PHP of `.php-version` and runs `make install-rust`,
  `make redis-image` and `make test`: bay runs the Redis servers of the tests on Docker of the runner from the image
  of `REDIS_IMAGE` (SE-10.4). No step installs bay yet, so the job fails at the start of `make test` and names bay,
  and `ci-passed` fails with it, until the job installs bay (row T1.26 of the checklist).
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
- No tracked file names a Git commit, of this repository or of another, except the pin of an action in a workflow. A
  release and a lock pin the name of a commit, and a name written in a document is a wrong value once the history
  changes; a record states the fact instead: the input, the behavior, the expected behavior and the count of the tests.
  Enforced by: `make commit-names-check`, which reads the tracked Markdown files; a commit named in another tracked
  file is found by review. `scripts/check-commit-names.mts` is a copy of the check that fsm and php-serialization hold,
  kept until kit releases the check as a utility.
