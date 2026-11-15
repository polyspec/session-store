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
- binding is the npm package `@polyspec/binding` of a release, which `package.json` pins by the URL of its release
  asset and `package-lock.json` by its integrity, so `make install-packages` installs it with the other packages; a
  dependency on another polyspec repository is a tag or a release asset, never a branch or a commit. Every manifest
  names `polyspec-binding-core` by the tag of the same release, because the generator and the crate are one release
  (binding CK-20). `BINDING` of the Makefile starts its `dist/cli.js`, and the binding targets run Cargo offline
  (`CARGO_NET_OFFLINE`), because binding reads the crates with `cargo metadata` and a check that downloads depends on
  the network.
- `scripts/test-run.mts`: runs the tests of `make test` as a run that owns its servers and its temporary directory
  (SE-10.4); with `--service redis` it also starts the Redis service of the generated binding tests (SE-10.7).
- `scripts/redis-state.mts` with `scripts/redis-state.sh`, which starts it on the node of the checkout: the program of
  the state of the Redis service of the generated binding tests (SE-10.7). `scripts/redis-state.test.mts` is its test,
  which `make redis-state-test` runs against a stand-in of a server.
- `scripts/servers.sh`: the commands of bay for the Redis servers of the tests, which the tests,
  `scripts/test-run.mts` and `make redis-image` run (SE-10.4): bay starts and removes the servers as Compose projects,
  gives their names, certificates and addresses, and pulls and finds the image. `scripts/servers.test.mts` is the
  test of both scripts, which `make servers-test` runs against a stand-in of bay.
- `scripts/install-bay.mts`: installs the release of bay that `config/toolchain.json` pins for CI into `var/tools/bay`
  by the sha256 of its archive (`make install-bay`). `scripts/install-bay.test.mts` is its test, which
  `make install-bay-test` runs with an archive that it packs.
- `Cargo.toml` of the root is the workspace of the four crates and the binding crates of the five targets with one
  `Cargo.lock`.
- `contracts/sessions.json`, `contracts/file-sessions.json`, `contracts/redis-sessions.json`: the cases (SE-10).
- `scripts/workflows.test.mts`: the test of the rules of the GitHub workflows, which `make workflows-test` runs.
- `scripts/check-commit-names.mts`: the check of `make commit-names-check`, with its test
  `scripts/commit-names.test.mts`, which `make commit-names-test` runs.
- `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/actions/bootstrap`: CI, the release and the
  bootstrap of their jobs, which runs `make bootstrap`, `make install-tools TOOLS=npm` and `make install-packages`.
  `.github/actions/bay`: the bay of a runner, which runs `make install-bay`, `make bay-path` and `make setup-bay`.
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
- bay, which starts the Redis servers of the tests, is a tool that the machine gives, as make is, and is not pinned
  for a run: what changes a result of the harness is the contract of the output of bay that it reads
  (`schemaVersion`, which the harness checks), not the number of a release, and the bay of a development machine is
  often a new build. The recipes start `bay` of PATH (`BAY` of the Makefile), and a run writes the output of
  `bay version` as its first line. A release of bay is pinned for CI alone, in `config/toolchain.json` (`bay`: its
  version, the address of its releases and the sha256 of its archive of each platform), because a runner holds no
  bay and a download is used only by its pinned digest: `make install-bay` installs it into `var/tools/bay`, and
  `make bay-path` puts that directory on the PATH of the later steps of a job of GitHub. The machine setup of bay is
  applied once per machine with `bay install` (`make setup-bay`), never by a recipe of a job and never with `sudo`: on
  a runner the action `.github/actions/bay` runs `make install-bay`, `make bay-path` and `make setup-bay` before the run
  step, and a run on a machine without the setup fails at `bay check` and names `bay install`. To raise the pin, set `version` of `bay` in
  `config/toolchain.json` to the tag of the release and each value of `sha256` to the line of that archive in the
  file `SHA256SUMS` of the release (`darwin-arm64`, `darwin-x64`, `linux-x64` and `linux-arm64` are the archives
  `darwin_arm64`, `darwin_amd64`, `linux_amd64` and `linux_arm64`); no other file names the release. Then run
  `make tool-pins-test` and `make install-bay`. The pin is raised when the tests of CI need a later release, and it
  is removed when a runner gives bay or the tests start no server.
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
- A test that needs a peer (PHP, bay with the Redis image) fails when the peer is not given and
  names what gives it; no test is reported as ignored because a peer is missing. The only ignored tests are the cases
  of the group `open` that carry `binding`, which the bindings run (SF-8.2, SR-8.2).
- A Redis server of a test runs in a container from the image that `REDIS_IMAGE` of the Makefile pins by its digest,
  never a server installed on the machine (SE-10.4). bay starts and removes the servers as Compose projects, on Apple
  `container` on Darwin and on Docker on Linux, one image and one digest for both, so that both platforms test the
  same bytes of the server. `scripts/servers.sh` is the one file that holds the commands of bay, and no file holds a
  command of an engine; no test and no recipe starts bay by itself, and the script starts it by the words of `BAY`.
  bay or a part of its machine setup that is missing fails `make test` at its start, before the build, with what is
  missing and what gives it (`bay check -scope run`; `bay install` applies the setup, and no run applies it); no run
  compares the version of bay. The image is the official image
  of the highest stable release of Redis; `make redis-image` pulls it with `bay pull`, and no test pulls an image: a
  project starts with `up -no-pull`.
- A test owns the servers that it needs as one Compose project, because the arguments of a server are part of what a
  test states and the state of a server must not reach another test. It writes `compose.yaml` into a directory of
  its own below `SESSION_STORE_PROJECTS`, the directory `session-store-test-<pid>` of the run, before it starts the
  project; the name of that directory, with the test process and a count, is the name of the project. bay gives
  every name: the harness reads the host name and the certificate paths of each service from `bay plan --json`
  before the start and the addresses from the output of `bay up --json`, so one `up` starts every server of a test,
  no file states the domain of the names or a path of a certificate, and no port is published. The server of TLS
  reads the certificate that bay issues, and no test makes one. The return of `up` is the event of a start; a
  command of Redis that a test needs in a container, such as `redis-cli --cluster create`, runs with `bay exec`, and
  a test waits for a later report of a server by its line in `bay logs -f`, never for a time. `scripts/test-run.mts`
  runs `down` for every Compose file that the directory of the run still holds and removes the directory when the
  run ends, fails or is stopped by SIGINT or SIGTERM.

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
- No contract case runner is written yet: `make bindings` generates no target, so no binding API exists that a runner
  could call and be checked against. The interface contract declares only cases of one call, because the generator
  refuses a case of several requests (binding G-5): the groups `requests` and `store`, whose cases hold several calls,
  are read by the Rust tests alone (SE-10.6). The 13 kinds of the store of Redis that only a state of a Redis server
  reaches have cases of one call against the service `redis` of the test run (SE-10.7, binding GN-13j): the interface
  contract declares it in `services`, `scripts/test-run.mts --service redis` starts its server for the generated tests
  of PHP, Python, Node and Go and gives them `SESSION_STORE_REDIS_URL` and `SESSION_STORE_REDIS_STATE`, the path of
  `scripts/redis-state.sh`, the program that sets up, changes and checks the keys of the scope of a case, and
  `make redis-state-test` tests that program. That form needs binding 0.0.7, which is not pinned yet: binding 0.0.6
  refuses the key `services` of the contract (CK-1). No state of a Redis server gives a reply of another type, so
  `reply` of `redis_sessions.open` and of `redis_session.close` is imitated by the fault injection points
  `open_reply` and `close_reply` (SE-9.6), whose cases run against the same service; with binding 0.0.7 the contract
  check reports no kind without a case, and `binding generate` writes the five targets (T1.30-4). The pinned
  binding 0.0.6 reads the `only` of the parameter `cancel` of `file_sessions.open`, so Go needs no first parameter
  `ctx` for it (T1.30-5).
- In the Go module binding owns the files `zz_generated_*.go` (binding GN-3); they are tracked, because a Go module is
  fetched as source, and `make bindings` writes them again. `packages/ts/wasm/generated` and
  `packages/go/internal/ffi/generated` are not tracked; wasm-bindgen writes the module and its glue into the first.
  The variables `PYTHON` (3.14 or later), `PHP`, `PHP_CONFIG`, `PHPUNIT`, `TSC`, `GO`, `GOFMT` and `WASM_BINDGEN` name
  the runtimes and tools; unless they are given, `GO`, `GOFMT`, `WASM_BINDGEN`, `MATURIN` and the Python of `PATH`
  are those of `mise.toml`, `TSC` is the TypeScript of `package.json`, and `PHPUNIT` is the PHPUnit of
  `composer.lock`, which `make install-php-tools` installs with the Composer of `mise.toml`. `make test-go` fails a Go of another release than the go directive of `packages/go/go.mod`
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

- Every job of a GitHub workflow runs on `ubuntu-26.04-arm`, except the jobs `platform` and `php` of
  `.github/workflows/assets.yml`, which runs on the runner of each platform of the release (`ubuntu-26.04`,
  `ubuntu-26.04-arm` and `macos-15`), because the asset of a platform is linked and loaded only on that platform and no
  tool of the release compiles for another platform (binding GN-19a). Every action is a local action of `.github/actions` or is
  pinned by its full commit, and a comment of the file names its release; an action pin is the one Git commit that a
  tracked file names. No job and no step sets `timeout-minutes`. Every `run` step is one make target that the
  `Makefile` or the release recipes that it includes (`packages/release.mk`) define, so that the settings of the
  Makefile apply to every command of CI. `make workflows-test` fails a
  workflow or an action that breaks one of these rules.
- Every job of the workflows, `ci.yml`, `release.yml` and `assets.yml`, which both call, has one `run` step,
  `make ci-<job>` for a job of `ci.yml`, `make ci-release-<job>` for a job of `release.yml` and `make ci-assets-<job>`
  for a job of `assets.yml`, which GitHub names `assets / <job>`, whose only arguments are make variables: a value, a
  matrix value as `NAME=${{ matrix.<key> }}`, an input of `assets.yml` as `NAME=${{ inputs.<key> }}`, an output of a job
  that the job needs as `NAME=${{ needs.<job>.outputs.<key> }}`, and a quoted expression of the event as
  `NAME="${{ ... }}"`, such as the run id of the job `tag`. The
  judgement job `ci-passed`, which exists only in CI, runs `make ci-passed FAIL_SKIPPED=1 RESULTS='${{ toJSON(needs) }}'`.
  Every other step is a `uses` step that provisions the runner or carries a result (checkout, setup-php, the bootstrap
  action, the action bay, upload-artifact, download-artifact), only a `uses` step has an `if` condition, no step sets
  `env`, and no job sets `env` other than `GH_TOKEN`, the token of the run that the release targets read with gh. The recipe `ci-<job>` of
  the `Makefile` holds the order and the conditions of the job: its installs and prerequisites first, which stop at a
  failure, then its checks or builds, each to its end, several of them through ci-targets, and its status is theirs;
  the Redis servers of a test target are started and removed by `scripts/test-run.mts` inside that target, also when
  it fails. So a local run is the call that CI makes, `make ci-<job> NAME=value`, and needs no emulation of the runner
  of GitHub, because the content of a job is in the recipe and not in the YAML. `make workflows-test` fails a job of
  these workflows that breaks the rule.
- The workflow `ci` starts on a push of `main` and on `workflow_dispatch` alone. Its job `shared` runs
  `make ci-shared` (`CI_SHARED` of the Makefile: `make docs-check`, `make owner-validate`, `make workflows-test`,
  `make tool-pins-test`, `make servers-test`, `make redis-state-test`, `make install-bay-test`, `make commit-names-check`, `make commit-names-test` and `make release-coverage`) and its job `rust` runs `make ci-rust`, which runs `make install-rust`
  and then `CI_RUST` (`make lint-rust`, `make fmt-check` and `make binding-check`); each runs its targets past failures with
  ci-targets of kit and uploads the report `var/report/ci-targets`.
- The job `bindings` of the workflow `ci` sets up the PHP of `.php-version`, uses the action bay and runs
  `make ci-bindings`, whose recipe runs `make install-rust`, `make install-tools`, `make install-php-tools` and
  `make redis-image`, in that order, and then `CI_BINDINGS` (`make lint-bindings`,
  `make check-declarations`, `make test-python`, `make test-node`, `make test-wasm`, `make test-php` and
  `make test-go`), so that no binding package of a release is untested; `ci-passed` needs it, and
  `make workflows-test` fails a workflow `ci` without it, a job that runs before the action bay, a recipe without
  `make redis-image` before the tests, a recipe of `ci-bindings` or `ci-test` that applies the machine setup of bay, or a
  `CI_BINDINGS` that leaves a target out. bay and the image are given as in the job `test`, because `make test-python`,
  `make test-node`, `make test-php` and `make test-go` run their generated tests in a run of `scripts/test-run.mts`
  that starts the Redis service of the cases (SE-10.7).
- The last job of the workflow `ci` is `ci-passed`. It runs after every other job, also after a failed one, needs each
  of them, and `make ci-passed FAIL_SKIPPED=1` (ci-passed of kit) passes only when each has the result success, so that
  one job states the result of the run. `make workflows-test` fails a workflow `ci` whose last job is another or whose
  `needs` leaves a job out.
- The job `test` of the workflow `ci` sets up the PHP of `.php-version`, uses the action bay and runs `make ci-test`,
  whose recipe runs `make install-rust`, `make redis-image` and `make test`: the bay that `config/toolchain.json` pins
  for CI runs the Redis servers of the tests on Docker of the runner from the image of `REDIS_IMAGE` (SE-10.4). bay,
  its machine setup and the tests run as the user of the runner, the one user that holds the state of bay, so the test
  of TLS reads the authority of that user. Not confirmed until the first run of CI: that `bay install` applies the
  machine setup on the runner without `sudo`.
- binding release-recipes writes the release recipes of the contract into `packages/release.mk` (binding GN-19h),
  which is generated and not tracked; `make release-recipes` writes it, and the Makefile includes it after it defines
  `BINDING_RUN`, `NODE`, `NPM`, `UV` and an empty `BINDING_DEP`. The recipes build the static library of the Go
  module, the abi3 wheel, the PHP extension and the npm package with its platform packages, carry the Go module and
  upload the assets. A recipe of the repository for one of their targets is no copy that the repository keeps: binding
  check reports it, and a Makefile without the include, with CK-94. The repository pins what the recipes start: the
  napi CLI `@napi-rs/cli` in `package.json`, uv, Python, Go and the wasm-bindgen CLI in `mise.toml`, the major release
  of LLVM in `llvm.version` of `config/toolchain.json`, and `llvm-tools` with the musl targets in
  `rust-toolchain.toml`.
- The workflow `assets` builds non-PHP native assets once per platform in job `platform` and the PHP extension
  in a fresh job `php` for every platform and PHP minor (8.4 and 8.5). Both matrices are explicit include-only lists,
  without extra axes or exclusions. Each PHP job owns bootstrap, locked Rust inputs, release recipes and release
  check, installs LLVM, sets up one PHP minor and then runs
  `make release-php-ext PHP=php EXPECTED_PHP_MINOR=${{ matrix.php }}` once. The dynamic minor input belongs to that
  setup step. Artifact `assets-php-${{ matrix.platform }}-${{ matrix.php }}` identifies its isolated result. No other
  job repeats PHP setup or native asset builds. A loaded PHP library may remain after PATH changes, so minors
  never share a job. The job `npm` packs the wrapper and WASM once. CI requires every asset job. Native asset proof
  remains pending until matching released recipes check the expected minor. `make workflows-test` owns this rule.
- The workflow `release` starts when a run of `ci` succeeds on `main`, or by a dispatch with the id of such a run. Its
  job `tag` runs `make release-tag` and `make release-tag-output`; its job `assets` calls the workflow `assets` for the
  tag with `make release-binding-check` first; its job `release` checks out the tag and runs `make release-tools`,
  `make release-tools-path`, `make release-assets` and `make release-publish`, downloads the assets of each platform,
  and runs `make install-rust`, `make release-recipes`, `make release-go-module`, which carries the Go module in a
  commit with the tag `packages/go/vX.Y.Z` (binding GN-19f), `make release-platform-assets` and
  `make release-undraft`, in that order.
- `config/release.json` classifies every manifest of the repository, and `make release-coverage` fails a manifest that
  it does not classify. Its one package is the Composer package of the PHP extension (`packages/php-ext`), whose
  archive kit builds; the binding manifests are released by their version; `packages/go/go.mod` is not released by kit,
  because `make release-go-module` makes the commit and the tag of the Go module, so `goModules` names none.
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
