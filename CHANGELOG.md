<!-- doc-id: changelog -->

## Unreleased

## 0.0.1

- kit 0.0.113 and binding 0.0.7 replace kit 0.0.109 and binding 0.0.6 (T1.36): the devDependencies name the assets `polyspec-kit-0.0.113.tgz` and `polyspec-binding-0.0.7.tgz` of their GitHub Releases with the integrity of `package-lock.json`, and every dependency line on `polyspec-binding-core` names the tag `v0.0.7`. Before, `Cargo.lock` held a second `polyspec-binding-core` from the branch `v1/rust-core` of binding, which php-serialization required; it now holds only the tag `v0.0.7`. binding 0.0.6 refused the key `services` of the contract, so `make workflows-test` failed; with 0.0.7 it passes. `.gitignore` ignores the Go files `zz_generated_*.go` that `make bindings` writes into `packages/go`.
- Every job of the workflows has one `run` step, the recipe of the job in the `Makefile` (T1.35): `make ci-<job>` for a job of `ci.yml`, `make ci-release-<job>` for a job of `release.yml` and `make ci-assets-<job>` for a job of `assets.yml`, with make variables as its only arguments, a matrix value as `NAME=${{ matrix.<key> }}` and the input `release` as `RELEASE=${{ inputs.release }}`; `ci-passed` passes `RESULTS='${{ toJSON(needs) }}'` on its command line instead of a step `env`. The job `tag` of `release.yml` passes `RUN` and `AUTO` on the command line, and its recipe gives the later jobs the `TAG` and `VERSION` that `release-tag` appended to `GITHUB_ENV`; the job `release` passes `TAG` and `VERSION` from the outputs of `tag`, downloads the platform assets before its run step into `var/release/downloaded`, which its recipe adds to `var/release/assets` after `release-publish`, runs its later lines with the tools of `release-tools` on PATH instead of `release-tools-path`, and sets the Git identity of the commit of the Go module from `GITHUB_ACTOR` in the recipe instead of a step `env`. The recipe runs the installs and prerequisites of the job first and stops at their failure, then its checks or builds, each to its end, so that a local run is the call that CI makes. The new recipe `ci-test` installs the toolchain and pulls the Redis image before `make test`, and `ci-bindings` does the same before the binding tests; no recipe applies the machine setup of bay or runs `sudo`, because the setup is applied once per machine with `bay install` and bay keeps its state for the user who runs it: on a runner the new action `.github/actions/bay` runs `make install-bay`, the new `make bay-path`, which puts `var/tools/bay` on the PATH of the later steps, and `make setup-bay` before the run step, `BAY` is `bay` of PATH, and `BAY_SUDO` is removed, so a run on a machine without the setup fails at `bay check` and names `bay install`; the Redis servers stay owned by `scripts/test-run.mts` inside each test target, which removes them also when the target fails. Before, the jobs held up to seven `run` steps each, with step conditions `${{ !cancelled() }}` and `${{ inputs.release }}` on `run` steps, and a local run had to repeat the order and the conditions of the YAML. In the job `php` of `assets.yml` the PHP setup now comes before the bootstrap, because it must precede the one `run` step. `make workflows-test` requires one `run` step per job, its recipe name and arguments, no condition on a `run` step, no `env` in a step and no `env` in a job but `GH_TOKEN`, requires the action bay before the run step of the job `bindings` and no machine setup of bay in `ci-test` and `ci-bindings`, and reads the order of the job `bindings` and the asset builds in the recipes.
- PHP extension assets use independent platform-and-minor jobs (T1.33-1). Before, a platform job installed PHP 8.4 and then 8.5, allowing loaded PHP library state to cross minor boundaries. The PHP job sets up its minor before one build with EXPECTED_PHP_MINOR and publishes a unique artifact. Non-PHP native assets build once per platform. Owner assertions reject expanded matrices, detached setup, wrong runners and duplicate native ownership. The tracked-workflow assertion failed before the split and passes afterward; the complete `make workflows-test` reports 12 passed and one failure because the installed generator refuses `services` (CK-1). No full owner pass is claimed. Native asset and full-job evidence remain pending released provider recipes.

- The binding plan separates API generation, missing runner development, native provider installation inputs and
  full-job verification (T1.6-1). Before, T1.7 and T1.9 waited for T1.30 to finish while its service verification
  needed those runners. Their retry conditions now require the resolved tagged graph and generated APIs instead.
  T1.34-1 to T1.34-4 record the pending provider wheel and npm archive inputs, Go sums and actual service fault-case
  execution. No runtime task is completed by this planning change.

- `reply` of `redis_sessions.open` and of `redis_session.close` had no case that a generated binding test runs,
  because no state of a Redis server gives a reply of another type than the command gives, and the contract check
  reported both (CK-22), which stopped `binding generate` for every target. A failure that no input reaches is what a
  fault injection point imitates (SE-9.6): the points `session_store.open_reply` and `session_store.close_reply` of
  `polyspec-session-store-core`, armed as `broken`, take the reply of `GET` in the opening and of `EXPIRE` in the close
  of a session that changed nothing as a reply of another type, and the interface contract names them in `faults`
  with the fill `reply`. The groups `redis_sessions.open.served` and `redis_session.close.served` give each a case of
  one call against the service `redis` that arms its point and checks that the lock is released, and the Rust tests
  run both against the model (`model_served::`, 2 tests). With the source of binding 0.0.7 the contract check now
  reports no kind without a case, and `binding generate` writes all five targets (T1.30-4).

- 15 kinds of the store of Redis had no case that a generated binding test runs, because only a state of a Redis
  server reaches them and binding had no form of a case against a service of the test run; the contract check reported
  each (CK-22). The interface contract now declares the service `redis` in `services` (binding GN-13j, CK-95), and
  `contracts/redis-sessions.json` gives 13 of the kinds a case of one call against it in the groups
  `redis_sessions.open.served`, `redis_session.<member>.served` and `redis_session.close.served`, whose rows of `cases`
  leave out WASM, which has no socket (SE-10.7). A case sets up the keys of its scope, the `prefix` of its store, and
  changes and checks them with `scripts/redis-state.mts`, a program that speaks RESP over `node:net` and needs no
  package, started by `scripts/redis-state.sh` on the node of the checkout; `make redis-state-test` tests it against a
  stand-in server (6 tests). `scripts/test-run.mts --service redis` starts the server of the pinned image for the
  generated tests of `make test-python`, `make test-node`, `make test-php` and `make test-go` and gives them
  `SESSION_STORE_REDIS_URL` and `SESSION_STORE_REDIS_STATE` (`make servers-test`, 19 tests), and the job `bindings` of
  `ci` installs bay and the Redis image as the job `test` does. `reply` of `redis_sessions.open` and of
  `redis_session.close` keeps only its trial against the model, because no state of a Redis server gives a reply of
  another type, so the contract check still reports those two (CK-22). The form needs binding 0.0.7: the pinned 0.0.6
  refuses `services` (CK-1), so `make binding-check` and `make workflows-test` fail until 0.0.7 is pinned (T1.30-9).

- binding was pinned at its release 0.0.3, whose Go generator refused the parameter `cancel` of `file_sessions.open`
  although its `only` names python and node. binding is now the release 0.0.6 (B2.253 reads the `only` of a
  parameter): `package.json` names its asset and `package-lock.json` its integrity, and every manifest and
  `Cargo.lock` name `polyspec-binding-core` by the tag `v0.0.6`. `make binding-check` reports the 15 findings CK-22 of
  T1.30-4 and the finding CK-20 of T1.32, and `binding generate` still stops at the CK-22 findings for every target
  (T1.30-5).

- The interface contract declared the groups `requests` and `store`, whose cases hold several calls, and the generator
  of binding takes the keys of a case as the arguments of one call: it refused every target (G-4) for the cases of
  `file_sessions.open` and `redis_sessions.open`, and wrote a failing test for each case of `store`. The contract now
  declares cases of one call alone. `record` and `record_not_array` of `get`, `set`, `remove` and `renew` of
  `file_session` have cases that open the session of a stored file, which the case writes (`files`, binding GN-13i);
  each member of `memory_store` has cases on a store that `then` made, observed by `after`; and every path of a case of
  one call is a relative path in the directory of the test (binding GN-6y), so the cases of a directory that is not
  absolute are the group `openNotAbsolute`, which only the Rust tests run. 15 kinds of the store of Redis still need a
  Redis server, which no case form of binding gives (binding B2.269): the contract check reports each (CK-22), which
  stops `binding generate` for every target, and SE-10.6 names the Rust trials that reach each kind. The Rust tests
  give `record` and `record_not_array` of `set`, `remove` and `renew` of the store of Redis a case of the group
  `requests`, and `reply` of its close a case whose model answers `EXPIRE` with a status (`status_replies`)
  (T1.30-7, T1.30-8).

- The workflow `ci` had no job that tests the generated bindings, so a release would carry binding packages that no
  run tested. Its job `bindings` now runs `make ci-bindings`, the lint of the workspace, the generated declarations and
  the tests of python, node, wasm, php and go with their contract cases, and `ci-passed` needs it. The tools of these
  tests are pinned in the checkout: TypeScript 7.0.2 in `package.json`, maturin and Composer in `mise.toml`, PHPUnit in
  `composer.json` and `composer.lock` (`make install-php-tools`), and Go of `mise.toml`; `make test-python` no longer
  installs maturin from an index. The job fails at `make bindings` for every target until the generator takes the
  contract (G-4: the cases of `file_sessions.open` and `redis_sessions.open` in the group `requests` give no
  `cookie`), and it starts no Redis server, because the cases of a Redis server have no form that a generated test runs
  yet (T1.34).

- The repository built no asset of a binding for a release: `config/release.json` named no package, and the workflow
  `release` only tagged and published notes. binding 0.0.3 writes the release recipes of the contract into the ignored
  file `packages/release.mk` (binding release-recipes, GN-19h), which the Makefile now includes, and
  `.github/workflows/assets.yml` builds the static library of the Go module, the abi3 wheel, the PHP extension for PHP
  8.4 and 8.5 and the npm platform package on the runner of each platform, and the npm package itself (GN-19a, GN-19i).
  The workflow `ci` calls it in the job `assets`, which `ci-passed` needs; the workflow `release` calls it for the tag,
  releases the Composer package of the PHP extension, carries the Go module with its tag `packages/go/vX.Y.Z` (GN-19f)
  and uploads the assets of each platform. binding is the release 0.0.3 with the napi CLI 3.10.12, LLVM 21 is pinned in
  `config/toolchain.json`, `rust-toolchain.toml` adds `llvm-tools` and the targets of WASM and musl, and `mise.toml`
  adds uv, Python, Go and the wasm-bindgen CLI, which `WASM_BINDGEN` of the Makefile now names. `make release-checkout`
  is removed, because each job of the release checks out the tag (T1.33).

- binding was cloned at the commit of `Cargo.lock` into `var/binding` and named by the branch `v1/rust-core` in every
  manifest, a development form. It is now the npm package of its release 0.0.2, pinned by the URL of its asset and its
  integrity and installed with the other packages, every manifest names `polyspec-binding-core` by the tag `v0.0.2`,
  and `scripts/binding.mts` and `make install-binding` are removed; the binding targets run Cargo offline (T1.32).
  `make binding-check` fails CK-20 on the second copy of `polyspec-binding-core`, 0.0.1 of the branch `v1/rust-core`,
  until php-serialization depends on binding by the tag of a release.

- The row of `cases` that holds the case of `file_sessions.open` whose cancellation fired had no `only`, although its
  parameter `cancel` exists only in Python and Node (SF-3.8), so binding 0.0.2 reported CK-93 for php and go. The row
  now has `only` with the targets `python` and `node` (T1.32).

- `file_session.found` and `redis_session.found` had cases only in the group `requests`, which no generated binding
  test runs, and `record_write` of `file_session.close` was reached only there, although a session made without a
  cookie gives both in one call; the cases of one call wrote a PHP value as its serialize text (`"i:1;"`), which is
  no value of the alias `php_value`. The groups `file_session.found.made` and `redis_session.found.made` and a case of
  `file_session.close.made` that sets 4096 nested lists now give them, every PHP value of a case of one call is a
  `php_value` (`{"kind": "int", "value": 1}`), and the Rust tests read that form (SE-10.2, SE-10.6, T1.30-6). Without
  the group `requests` the contract check now reports 23 kinds, which wait for a form of a case of binding with a
  stored record or a Redis server (T1.30-4). SF-3.8 states why the parameter `cancel` is for Python and Node alone.

- `make bindings` refused every target because the parameter `cancel` of `file_sessions.open` had the type `cancel`
  with a computed default, which binding does not generate (U-2), the record `store_options` named no `core_type`,
  and the cases of `redis_sessions.new` gave no `setting` and wrote a setting as `{"server": {...}}` (G-5). The
  cancellation is now an `optional` of `cancel` with the default `{"computed": "SF-3.8"}`, which
  `FileSessions::open` takes by value (`Option<Cancel>`, SF-3.8); `store_options` names `StoreOptions` (SR-1.2);
  every case of the group `open` of the store of Redis gives `setting` and `options`, the case without `lock_expire`
  is the group `openMissing`, which the interface contract does not declare, and every setting of the cases is a value
  of the variant with its tag `kind` (SE-10.2). Three trials of the Rust tests read the interface contract and the
  cases for these forms (T1.30-1 to T1.30-3). SE-10.6 now states that `file_session.close.made` gives `io` and
  `identifier_taken` of `file_session.close` through the point `session_store.create`; it stated that the contract
  had no such case. `make bindings` still refuses every target: the cases of the group `requests` hold several
  requests, which the generator takes from no group, and without that group the check reports 26 findings, 24 kinds and the two members `found` without a case of one call
  (T1.30-4); Go takes the parameter `cancel` only as its first parameter (T1.30-5).

- `make binding-check` failed with CK-20, because `Cargo.lock` pinned `polyspec-binding-core` at an older commit than
  the binding that the check ran, and with CK-19 for ten files: the manifests of the five binding packages differed
  from what `binding scaffold` of the head of binding writes, and `packages/python/build.rs` was missing. `Cargo.lock`
  now pins the head of binding whose CI passed, and the manifests and `packages/python/build.rs` are the output of its
  `binding scaffold` (T1.29): the binding crates take the feature `driver` of `polyspec-binding-core` and name the
  license, the Node crate takes `tokio_rt` of napi, the WASM crate takes `wasm-bindgen-futures`, `packages/ts/package.json`
  is a module with the targets of napi and the dependency `@polyspec/php-serialization`, the PHP package names its
  prebuilt binary, and the Go module requires the Go module of php-serialization. `make binding-check` reports no
  finding. `make bindings` still refuses every target (G-4); T1.7 and T1.9 name the refusals and T1.30 the changes of
  the contract that end them.

- The test of TLS failed in the job `test` of the workflow `ci` with `the connection to Redis failed: I/O error`. The
  test named in `SSL_CERT_FILE` the authority file that `bay plan --json` gives (`certificate.ca`). With `BAY_SUDO=1`
  bay runs as root and keeps that file below the home of root, which the test process, which does not run as root,
  cannot read; rustls-native-certs reports the read as an error and redis-rs returns it as an I/O error before it
  opens a connection. The two lines `unexpected eof while reading` in the output of the server were the connections
  with which bay checks that the port accepts connections, not the client. The test now trusts the authority of bay
  through the trust store of the machine, to which `bay install` adds it and whose entry `bay check -scope run`
  checks at the start of a run (SE-10.4): no test reads the authority file or names it in `SSL_CERT_FILE`, the
  harness no longer reads `certificate.ca` of the plan, and the trial is again one of `tests/server.rs`, because no
  test changes the environment of its process; `tests/tls.rs` is removed. A test of `make servers-test` requires
  this: it fails a test of the store of Redis that calls `set_var` or `remove_var`, names `SSL_CERT_FILE` or
  `SSL_CERT_DIR` or reads the field `"ca"`, which the previous `tests/tls.rs` and `tests/servers/mod.rs` did, and
  `make servers-test` gives 17 passed, 0 failed with the change. The test of TLS itself runs in CI.

- CI installs bay v0.0.9 (`config/toolchain.json`). With v0.0.7 a Linux host could not resolve the name
  `<project>-<service>.bay.test` of a service of a run, so the 13 `server::*` tests of the job `test` failed with
  `failed to lookup address information`; v0.0.8 names each running service container in the hosts block of the host,
  and v0.0.9 rewrites that block on each start, stop and removal of a container, which Docker reports as an event.

- Only `make install-binding` downloads binding (T1.28). It runs `scripts/binding.mts install`, which clones binding at
  the commit of `Cargo.lock` and installs the packages of its tool with the npm of `var/tools`, and the job `rust` of the
  workflow `ci` runs it after `make install-rust`. Every other command of the script runs `cargo metadata --offline`
  and fails with `run make install-binding` while binding is not installed. Before, `make binding-check` cloned binding
  and ran `npm ci` with the npm of the `PATH` when `var/binding` held no install, and `cargo metadata` could fetch. A
  new test of `make tool-pins-test` reads the dry run of `make install-binding`, the script and the job `rust`: before
  the change it gave 9 passed, 1 failed, and with it 10 passed, 0 failed.

- T1.19, the static making of the store of files (`FileSessions::in_directory`, `file_sessions.in_directory`, a class
  without a constructor) and the core enum of the variant `setting`, is done. On the clean tree of this branch on
  this Mac, `make test` ended with status 0 with 157 tests passed, and `make -k lint-rust fmt-check binding-check`
  ended with status 0.

- On the runner of the workflow `ci`, whose `/bin/sh` is dash, three tests of the run in `make servers-test` failed
  with `scripts/test-run.sh: 41: set: can't access tty; job control turned off` on standard error, and the test of a
  run that SIGINT or SIGTERM stops never ended. dash turns no job control on without a terminal, so the command of
  the run stayed in the process group of the run, ignored SIGINT as an asynchronous command of a shell does, and the
  kill of the group of the command failed; the run then waited for a command that no signal had reached. A POSIX
  shell gives a script no way to make a process group, and macOS has no `setsid` program, so the run is now
  `scripts/test-run.mts`, which the node of the checkout runs: it starts the command as the leader of a process group
  in a session of its own (`spawn` with `detached`), passes SIGINT and SIGTERM to that group and waits for the exit of
  the command (SE-10.4). `scripts/test-run.sh` is removed, and `make test` starts `$(NODE) scripts/test-run.mts`. The
  tests of the run run their commands with `/bin/sh` and with `/bin/dash` where it exists, and a new test requires
  that the command leads its own process group. With `/bin/dash` of this Mac the tests failed on their commit: 15
  tests without the signal test, 12 passed and 3 failed with that line, and by hand a run that received SIGTERM ended
  only after its command read from its FIFO, with `ps` showing the command in the group of the run. After the change
  `make servers-test` gives 17 passed. Not run: the tests on Linux and the workflow after the change.

- `config/toolchain.json` pins bay v0.0.7 as the release that CI installs, with the sha256 of each archive from the
  `SHA256SUMS` of that release, because the bay of this Mac is v0.0.7 and CI runs the release that a run here starts.
  A runner applies the setup of that release, whose agent unit differs from the one of v0.0.6, with `make setup-bay`
  in each run. On this Mac `make install-bay` downloads the archive of darwin-arm64, accepts its sha256 and installs
  bay v0.0.7; `make tool-pins-test` gives 9 passed and `make install-bay-test` 4 passed. Not run: the install on
  Linux and the job `test` of the workflow with this release.

- `make redis-image` failed on the runner of the workflow `ci` with `bay: compose.yaml: project name
  "tmp.x4bmuo3xsw" must be letters, digits, - or _`, and bay of this Mac refuses the same file in the same way:
  `scripts/servers.sh pull` wrote the Compose file of the pull into the directory that `mktemp -d` gives without a
  template, `tmp.<letters>`, and bay names a project after its directory and refuses a dot in that name. The pull
  now writes into a directory `session-store-pull-<letters>` below `TMPDIR` (SE-10.4). The test of the pull requires
  that name: on its commit `make servers-test` gave 15 passed and 1 failed (the name `tmp.4hb9OWSrEy`), after the
  change 16 passed, and `bay plan --json` of this Mac accepts a Compose file in such a directory. Not run: the pull
  itself, which downloads the image, and the job `test` of the workflow after the change.

- No file named a release of bay, the workflow `ci` installed no bay, so its job `test` failed at the start of
  `make test`, and `scripts/servers.sh` started the bay of PATH alone. bay is a tool that the machine gives, as make
  is: a run starts `bay` of PATH, compares no version, writes the output of `bay version` as its first line, and the
  harness fails a document of bay whose `schemaVersion` is not 1, because the contract of the output that the harness
  reads changes its result and the number of a release does not. `config/toolchain.json` pins the release that CI
  installs, bay v0.0.6, with the sha256 of its archive of each platform, because a runner holds no bay and a download
  is used only by its pinned digest; `make install-bay` (`scripts/install-bay.mts`) downloads the archive of the
  platform, refuses another sha256 and puts `bay` and `bayd` into `var/tools/bay`, and no path of a run reads the
  pin. `BAY` of the Makefile is the one value that says how bay is started, which `scripts/servers.sh` alone reads:
  `bay` by default, and with `BAY_SUDO=1` the bay of `var/tools/bay` behind `sudo -n`, because on Linux the setup and
  the commands of bay run as root and bay keeps its state below the home of the user who runs it. The job `test` runs
  `make install-bay`, then `make setup-bay` (`bay install`, the one target that changes the setup of a machine),
  `make redis-image` and `make test` with `BAY_SUDO=1`. `make servers-test` gives 16 passed, `make install-bay-test`
  4 passed, `make tool-pins-test` 9 passed and `make workflows-test` 6 passed; before the change of each script the
  tests of its commit failed. On this Mac `make install-bay` installs v0.0.6, whose version command gives v0.0.6,
  and `scripts/test-run.sh true` writes the version line of bay and ends with status 0. On this Mac, on the clean tree of
  this change before the release of CI was raised above v0.0.2, `make test` ended with status 0 three times in a row:
  every test of the servers passed with the tests in parallel, and no container and no project of bay remained after
  a run; `make -k lint-rust fmt-check` ended with status 0. The logs are in the record of the runs, which is not
  tracked. Only the values of the release that CI installs and documents changed after that tree, and no run reads
  those values. With bay v0.0.6 on this Mac `make test` ended with status 2 once in five
  runs: the test of TLS failed with the words of the store alone, the connection to Redis failed: I/O error, while a
  server of TLS that was started by hand in the same way answered every one of 32 clients that verified its
  certificate. The cause was in the test binary: the trial of TLS in `tests/server.rs` named the authority of bay in
  `SSL_CERT_FILE` with `set_var` while four other trials of the same process ran, and those trials read the
  environment when they start programs and resolve names, so a write beside them is not defined. SE-10.4 states
  that no test changes the environment of a process whose trials run in parallel. `tests/tls.rs` is now a test
  binary of its own with that one trial: its `main` reads the plan of the project and names the authority file in
  `SSL_CERT_FILE` before a second thread starts, and the trial adds what its server wrote to a failure of its client
  (`scripts/servers.sh output`); `tests/server.rs` holds the four other trials and changes no environment. The test
  of `scripts/servers.test.mts` that requires this failed on its commit (`make servers-test`: 15 passed, 1 failed) and
  passes after the change (16 passed). On the clean tree of the change `make test` ended with status 0 ten times in
  a row, each run with 157 tests passed and none failed, the test of TLS among them. Not run: `make setup-bay`, the tests on Linux, and the workflow. Not confirmed until the first run of
  CI: that `bay install` completes on
  the runner, that the test process reads the authority file of a bay that root runs, and that a signal of the run
  reaches a bay that `sudo` started.

- The harness of the Redis servers of the tests held the commands with which two engines run a server
  (`scripts/container.sh`: Apple `container` on Darwin and `docker` on Linux), took the address of a server from a
  line that its command wrote, made the certificate of TLS with `openssl` and ran `redis-cli` in containers of its
  own. SE-10.4 states that bay starts and removes the servers as Compose projects on both platforms and gives every
  name. A test owns one project: it writes `compose.yaml` into a directory of its own below `SESSION_STORE_PROJECTS`,
  the directory `session-store-test-<pid>` of the run, before the start, with the image by its digest, `redis-server`
  with its arguments as `command`, the port in `expose` and no published port; the name of that directory is the
  name of the project, and no server is shared between tests, because the arguments and the state of a server belong
  to one test. The harness reads the host name and the certificate paths of each service from `bay plan --json`,
  starts every server of a test by one `up -no-pull --json` and reads the addresses from its output, so no file
  states the domain of the names or a path of a certificate; it fails a document whose `schemaVersion` is not 1. The
  server of TLS reads the certificate that bay issues for its name, and the test trusts the authority file of the
  plan; `make test` passes no `OPENSSL`. The configuration of Sentinel names its primary by the name of the plan. The
  test of the cluster runs `redis-cli --cluster create` with `bay exec` in the container of the first node and waits
  for the state ok of each node in the output of `bay logs -f`. `scripts/servers.sh` replaces `scripts/container.sh`
  and holds commands of bay alone (`check`, `pull`, `plan`, `up`, `images`, `exec`, `logs`, `output`, `down`);
  `make redis-image` pulls with `bay pull`. `scripts/test-run.sh` fails before the build when bay or a part of the
  machine setup that a project needs is missing (`bay check -scope run`), runs `down` for every Compose file that its
  directory still holds when the run ends, fails or is stopped by SIGINT or SIGTERM, and fails a run whose project
  is not removed. `make servers-test` replaces `make container-test`: with a stand-in of bay, `node --test` on the
  Node.js of the machine gave 0 passed and 13 failed with the script of the commands of the engines, and 13 passed
  and 0 failed with the script of this change; on this Mac `scripts/servers.sh check` and
  `scripts/test-run.sh true` end with status 0. Observed with bay on this Mac, with throwaway projects of the pinned
  image: two projects start at once; one `up -no-pull --json` starts a server of TLS with the certificate of bay, a
  primary with its Sentinel and three nodes, and a client that trusts the authority file of the plan reads `+PONG`
  from the server of TLS at its name; `bay exec` runs `redis-cli --cluster create` with status 0, also with
  `/dev/null` as its standard input, and right after it a node had not yet written the line of the state ok; the
  nodes of a cluster that is created by names tell IPv6 addresses; an `up` that SIGTERM stops ends with status 143
  and leaves no container and no project; `up` with a service without a port that has ended ends with status 0;
  `logs -f` of an ended service ends; `up -no-pull` with an image that is not installed fails, names `bay pull` and
  leaves no project. `make servers-test`, `make docs-check`, `make lint-rust` and `make test` ran with the change of the entry
  above, which states their results.

- The contract named the constructor `file_sessions.new` with the wait `blocking_io`, and the variant `setting`
  without its core enum. `binding generate` refused both for every target: a language whose constructor cannot be
  awaited has no form of a constructor that waits (binding GN-6m), and a variant names its Rust enum in `core_type`
  (binding T-9, GN-6v). SF-1.5 states the making of the store of files as the static function
  `FileSessions::in_directory` of Rust, which replaces `FileSessions::new`, and the static member
  `file_sessions.in_directory` of the contract, which returns the store; the class `file_sessions` has no constructor
  (`constructible: false`). The six instance values of `contracts/file-sessions.json` that make a store name that
  member, and the variant `setting` names `polyspec_session_store::Setting`. Two trials of
  `packages/file/rust/tests/contract.rs` read the contract: no member that waits is a constructor or a property, and
  `file_sessions.in_directory` is a static member that returns the store. `binding check` of a binding that holds
  these rules reports no finding of the contract, and `binding generate` no longer refuses the two items; it still
  refuses `file_sessions.open` (the wait `process_wait`) for every target and the waits, the cleanups and the methods
  with the receiver `self` of the session classes for Go and WASM, which the generator does not write yet.
  `make fmt-check` and `make docs-check` pass. Not run: `make test` and `make lint-rust`, so the changed core and its
  tests are not compiled yet.

- The tests of the store of Redis ran on macOS alone: SE-10.4 and the harness named Apple `container`, so the workflow
  `ci` ran no test. SE-10.4 names the container program by the platform, Apple `container` on Darwin and `docker` on
  Linux, with the one image that `REDIS_IMAGE` pins by its digest, so that both platforms test the same bytes of the
  server and no test runs a server of the machine. `scripts/container.sh` is the one file with the commands of both
  programs (`program`, `pull`, `present`, `run`, `delete`, `list`); it takes the program from `uname -s` alone and
  fails at once with the name of a program that PATH does not hold and what installs it. The harness
  `packages/redis/rust/tests/servers` starts that script, `scripts/test-run.sh` lists and deletes the containers of
  its run through it and fails before the tests when the program is not found, and `make redis-image` pulls through
  it; the variable `CONTAINER` of the Makefile is removed. The address of a container is the first line that its
  command writes (`hostname -i`) on both programs, and a start waits for the lines of the server as before, never for
  a time. The job `test` of the workflow sets up the PHP of `.php-version` (8.4) and runs `make install-rust`,
  `make redis-image` and `make test`, and `ci-passed` needs it. `make container-test` runs the script against
  stand-ins of `uname`, `docker` and `container`: without the script it gave 0 passed and 4 failed, and with it 4
  passed and 0 failed. `make ci-shared` passes its 8 targets and `make fmt-check` passes. On this Mac the script
  names `container`, finds the pinned image and `scripts/test-run.sh true` ends with status 0. Not run: `make test`
  and `make lint-rust`, so the changed harness is not compiled yet, and no run of CI has shown that the runner
  `ubuntu-26.04-arm` holds `docker`.

- A lone `.node-version` named the Node.js of CI, a development machine ran its own Node.js, and no file pinned the
  digest of a tool. The tools are pinned in the form of the other polyspec repositories: `mise.toml` names Node.js
  26.12.0 and npm 12.2.0, `mise.lock` pins Node.js by the checksum of its archive of 6 platforms and npm by the digest
  of its lock project `.mise/locks/npm-npm/12.2.0`, and `config/toolchain.json` names mise 2026.10.8 with the sha256 of
  its archive of 4 platforms. `make install-tools TOOLS=npm` installs them into `var/tools`, `make install-packages`
  runs `npm ci` with that npm and `make toolchain-lock` writes `mise.lock`. `NODE` of the Makefile is
  `var/tools/bin/node`, and the recipes of the Node package start that node and that npm. The bootstrap action reads
  `mise.toml` and runs `make bootstrap`, `make install-tools TOOLS=npm` and `make install-packages`; `.node-version`
  is removed. `make tool-pins-test` checks the pins on the tracked files: without them it gave 3 passed and 5 failed,
  and with them 8 passed and 0 failed. `make ci-shared` passes its 7 targets on the node of `var/tools`, 26.12.0.
  `config/release.json` classifies the manifest of the lock project of mise as not released.

- No check read the tracked documents for the name of a Git commit, which is a wrong value once the history changes.
  `make commit-names-check` runs `scripts/check-commit-names.mts`, which fails with `<file>:<line>: <name>` for every
  word of 7 to 40 lowercase hexadecimal digits with a digit and a letter in a tracked Markdown file; a digest of 64
  digits, a number, a path and an identifier are no commit names. `make commit-names-test` gives 3 passed and 0 failed;
  before the checker existed it ended with no test that ran. On this tree the check finds no name. Both are targets of
  `make ci-shared`, which runs 6 targets, and of `make check`. The checker is a copy of the check that fsm and
  php-serialization hold and is removed once kit releases the check as a utility.

- No job of the workflow `ci` stated the result of a run: a run with a failed job had no one job to read. The job
  `ci-passed` is the last job of the workflow. It runs after every other job, also after a failed one
  (`if: ${{ always() }}`), needs `shared` and `rust`, and runs `make ci-passed FAIL_SKIPPED=1` with `RESULTS`, the JSON
  of its needs, which ci-passed of kit judges. With both results `success` the target ends with status 0; with one
  `failure` or one `skipped` it fails and names the job; without `RESULTS` it fails, because nothing was judged. The
  workflow test fails a workflow `ci` whose last job is not `ci-passed`, whose `ci-passed` has another condition, whose
  `needs` is not every other job, or that does not pass `RESULTS` or `FAIL_SKIPPED=1`: before the job existed
  `make workflows-test` gave 5 passed and 1 failed (`the last job is rust, not ci-passed`), and with it 6 passed and 0
  failed.

- The manifests named the license `MIT` and the repository held no text of it. `LICENSE` holds the text of the MIT
  license with the copyright line of the polyspec repositories, `Copyright (c) 2026 Polyspec`.
  `config/owner-checks.json` owns the path: with the file and without its rule `make owner-validate` fails with
  `LICENSE: the path matches no owner`, and with the rule it passes the 83 tracked paths.

- No file named the checks that own the tracked paths. `config/owner-checks.json` gives each of the 82 tracked paths its
  make targets: the documents `make docs-check`, the crates of the library and the cases `make lint-rust`,
  `make fmt-check` and `make test`, the binding packages `make binding-check`, the workflows `make workflows-test`
  and the manifests `make release-coverage`. `make owner-validate` runs owner-check of kit and passes; without the rule
  of `.node-version` and with a target that the Makefile does not define it fails with both errors. It is a target of
  `make ci-shared`, which passes its 4 targets, and of `make check`. `make owner-check` runs the owners of the paths
  that `PATHS` names.

- The repository had no GitHub workflow, so `make workflows-test` failed: `.github/workflows/ci.yml` was not tracked
  (4 passed, 1 failed). The workflow `ci` runs on a push of `main` and on `workflow_dispatch`: its job `shared` runs
  `make ci-shared` (`make docs-check`, `make workflows-test`, `make release-coverage`) and its job `rust` runs
  `make install-rust` and `make ci-rust` (`make lint-rust`, `make fmt-check`, `make binding-check`). The workflow
  `release` tags the commit of a successful run of `ci` and publishes its GitHub Release with the release targets of the
  Makefile. `.github/actions/bootstrap` sets up the Node.js of `.node-version`, 26.12.0, and runs `make bootstrap`.
  Every job runs on `ubuntu-26.04-arm`, and every action is pinned by its commit. `make test` is no target of CI,
  because its tests of the store of Redis need Apple container, which a Linux runner does not have. The test takes the
  key `run` of the inputs of a dispatch as a mapping, not as a run step. `make workflows-test` gives 5 passed and 0
  failed, and `make ci-shared` passes its 3 targets. No workflow has run on GitHub yet.

- No file classified the manifests of the repository for a release, and the `Makefile` had no release target.
  `config/release.json` names the change logs and classifies the 15 manifests as not released, each with its reason: the
  binding packages wait for kit 0.0.111, which makes the platform assets, and for the tag form of the Go module, the
  release commit and `packages/go/vX.Y.Z`. `make release-coverage` runs the coverage of the release utility of kit and
  passes; with `packages/go/go.mod` and `packages/ts/package.json` taken out of the file it fails and names both.
  `make release-tag`, `make release-checkout`, `make release-tools`, `make release-assets`, `make release-publish`
  and `make release-undraft` run the release utilities of kit; `make release-tag` without `RUN` fails and names it.
  `make check` runs `make workflows-test` and `make release-coverage` too.

- The checklist `docs/plans/rust-core/checklist.md` had no row for the structure of a release. The rows T1.10 to T1.14
  name the GitHub workflows and the test of their rules, the classification of the manifests for the release, the owners
  of the tracked paths, the text of the license and the release 0.0.1, and the plan names them as its sixth step. T1.13
  waits for the text of the license, and T1.14 for the branch `main` of origin. `make docs-check` passes: 6 document
  pairs, 1 checklist and 1 change log.

- The rows of `docs/plans/rust-core/checklist.md` and two entries of this change log named Git commits, which no
  tracked file does (AGENTS.md, "Commits"). The rows state their tasks alone, and the entries name no commit.

- The keys `textReplies` and `errorReplies` of the cases of the group `requests` of `contracts/redis-sessions.json`
  were not in snake_case, so the end-to-end check of binding refused the case files for every target (G-5). They are
  `text_replies` and `error_replies`, which the tests of the store of Redis read, and the trial
  `convention::case_keys` of the tests of the core requires every key of a case of a group that the `cases` of the
  interface contract name to be in snake_case.

- `Cargo.lock` pins a newer commit of binding, and `binding scaffold` writes the binding packages again: the allow of a binding
  crate gives its rule id as its reason, and the Go crate holds a new line of its manifest. The check of that binding
  fails a core that has fault injection points and whose tests do not take it with the feature `fault-test` (CK-65,
  GN-5), so `packages/rust/Cargo.toml` names the library itself as a dev-dependency with `fault-test`, as the core,
  the store of files and the store of Redis do. `make binding-check` passes, and `make test` gives 154 passed, 0
  failed and 13 ignored.

- The repository had no plan, no checklist and no check of its documents. `docs/plans/rust-core` holds the plan and
  the checklist with their Korean twins: the bindings, their end-to-end check
  and the Go binding wait for binding, which refuses to generate the contract for every target. `make docs-check` runs
  check-documents of @polyspec/kit, which `package.json` pins by the URL of its release and `package-lock.json` by its
  integrity and `make bootstrap` installs, on the documents of `config/documents.json` and the checklist of
  `config/checklist.json`, and `make check` runs it. `make documents-stamp` writes the `source-sha256` of a Korean twin.

- The interface contract states `core.result: true` for `memory_store.get`, `memory_store.set` and
  `memory_store.remove` (binding CK-21). They raise no kind, so without it the contract read their core paths as
  returning no `Result`, but the paths are the methods of the trait `Store`, which return a `Result`; a generated
  crate would not compile against them.

- The exclusive creation of the file of a new session, in `close` of `file_session`, has the fault injection point
  `session_store.create` (SE-9.6, SF-4.4, SF-9.2), so `io` and `identifier_taken` of `file_session.close` have their
  cases. Before, no point and no input reached them, and `make binding-check` gave these two CK-22 findings. The point
  takes `io`, fills `io` and `identifier_taken`, and imitates the failed system call with the `errno` of the fault:
  `EEXIST` is an identifier that another process took. The group `file_session.close.made` of
  `contracts/file-sessions.json` closes a session that `open` made and that set a value, with the fault `EIO`, which
  gives `io`, and with the fault `EEXIST`, which gives `identifier_taken`. `make binding-check` gives no CK-22
  finding.

- The kinds that only a session made by `open` reaches have their cases (SE-10.6, binding GN-13f, CK-22), and
  `Cargo.lock` pins the binding whose case forms and check take them. A case gives the session under the key
  `receiver` as an instance value: `open` without a cookie of a store made by its constructor, the store of files of
  the directory `/` with a cancellation that does not fire and the store of Redis of a socket under `/dev/null`. The
  new groups `file_session.get.made`, `file_session.set.made`, `file_session.remove.made` and
  `file_session.renew.made` of `contracts/file-sessions.json`, and the same four of `redis_session` of
  `contracts/redis-sessions.json`, arm `session_store.random` and give `random`; `redis_session.close.made` gives
  `connection` of the close of a session that set a value; `file_sessions.open.cancel` gives `cancelled` of
  `file_sessions.open` by a cancellation that fired before the call. The tests of the store of files and of the store
  of Redis run the ten cases. `make binding-check` gives 2 CK-22 findings where it gave 12: `io` and
  `identifier_taken` of `file_session.close`, which no fault injection point and no input reach. `make test` gives 152
  passed, 0 failed and 13 ignored.

- The library takes binding of the current `v1/rust-core` and php-serialization of the current `v1/rust-core`, whose
  fault injection point is the macro `binding_core::fault::point!`, which judges a point by the feature `fault-test`
  of the crate that writes it (binding GN-5). The module `fault` of the core calls `point!`, and the tests of the core,
  of the store of files and of the store of Redis turn on `fault-test` of `polyspec-session-store-core` through a
  dev-dependency, so that their points fire; before, the dev-dependency turned on only that of binding. The point
  `session_store.internal` now holds two invariant branches: the renewal of `MemoryStore` (SE-4.3), as before, and the
  start of a record session, `session.rs` line 70, which the point `session_store.session_waits` held before, so that
  point is removed. The start of a record session is on the path of every caller of `file_session` and
  `redis_session`, so a contract that passes on the kinds of this library arms the point `internal` for a member of
  its own (binding X-1); before, its one point was in `MemoryStore`, which no such caller reaches. The case of
  `recordFaults` arms `session_store.internal` and runs the same call without the arming first. `binding scaffold`
  writes the binding packages again: each takes the lints of the workspace (`[lints] workspace = true`), and
  `packages/php-ext/build.rs` is new. The workspace denies unsafe code instead of forbidding it, because a binding
  crate allows it with its reason (binding B-4); each crate of the library forbids it in its `lib.rs`. `make bindings`
  generates each target apart (`make bindings-<target>`, binding GN-1b), and each make target of a binding needs only
  the generation of its target. `make test` gives 142 passed, 0 failed and 13 ignored.

- Every invariant branch of the kind `internal` that `get`, `set` and `remove` of a record session reach has a fault
  injection point (SE-9.6). Before, the only point of the kind `internal` was `session_store.internal` in `renew` of
  `MemoryStore`, so a caller of `get`, `set` and `remove` of `file_session` or `redis_session` had no point that gives
  that kind. Each new point takes `broken`, fills `internal`, is declared in `faults` of the interface contract and is
  checked by `fault::invariant` just before its branch, as the point of `renew` is. The points and their branches in
  `packages/core/rust/src`: `session_store.session_waits`, `session.rs` line 74 (`get`, `set`, `remove`, `renew`);
  `session_store.record_root`, `record.rs` line 47 (the first use of a found record); `session_store.record_index`,
  `record.rs` line 63; `session_store.value_reference`, `copy.rs` line 116; `session_store.value_root`, `copy.rs` line
  123; `session_store.copy_new_reference`, `copy.rs` line 69; `session_store.copy_array`, line 82;
  `session_store.copy_element`, line 86; `session_store.copy_object`, line 91; `session_store.copy_property`, line 95;
  `session_store.copy_reference`, line 100; `session_store.copy_set_reference`, line 103 (`get`, and `close` through
  the copy of the record). The new group `recordFaults` of `contracts/sessions.json` holds one case of one call for
  each point, which the tests of the core run on a record session: the call without the arming succeeds, and the
  armed call fails with the kind `internal` and the message of its branch. `get`, `set` and `remove` of
  `MemoryStore` (`memory.rs` lines 73 to 85) return no error, and `random_hex` (`identifier.rs` lines 30 to 47) fails
  only with the kind `random`, so these paths give no `internal` and have no point. The branches of `encode` that only
  `close` reaches (`record.rs` lines 97 and 104) have no point.

- The tests of a real Redis server run in containers of Apple `container` and fail when their peer is missing
  (SE-10.4). Before, `make test` looked up `redis-server`, `redis-cli` and `openssl` on PATH and reported the 21 tests
  that need them as ignored when they were missing: the 13 cases of the group `requests` against a server
  (`server::...` of `tests/contract.rs`; 3 more were ignored trials of the cases with `textReplies` or `errorReplies`,
  which run against the model only and now have no trial against a server) and the queue of one process, the ACL
  user, TLS, Sentinel and Cluster of `tests/server.rs`; a test that needed PHP passed without PHP. Now every server is
  `redis-server` of Redis 8.10.3, the highest stable release, from the official image
  `docker.io/library/redis:8.10.3-trixie@sha256:de1068af6f552240dfab67bcc9bfef99bfc60f776bb4580ef077ea4c8c97e848`,
  which `REDIS_IMAGE` of the Makefile pins and `make redis-image` pulls. The module `tests/servers` starts each server
  in a container named after the run, takes the address that the container writes before the server starts, waits for
  the line of the server (ready, the primary that Sentinel monitors, the state ok of each Cluster node) and runs
  redis-cli in a container of its own; TLS takes a certificate of the address of the container and its port through
  CONFIG SET. No test waits for a time or polls: the waits on a socket file, on a connection and on `cluster info`, and
  the sleep of 2 ms between the read and the write of the queue test, are removed. `scripts/test-run.sh` runs the
  tests in a process group of their own and deletes the containers and the temporary directory of the run when it
  ends, fails or is stopped by SIGINT or SIGTERM. A test whose container, image, openssl or PHP is not given fails and
  names `make test` or `make redis-image`. `make test` gives 130 passed, 0 failed and 13 ignored. The 13 ignored tests
  are the cases of the group `open` that carry `binding`: a `lifetime` or a `lock_expire` that is negative, fractional,
  a text or a boolean, and a missing `lock_expire`, which a binding refuses before the store runs (SF-8.2, `docs/spec/sessions.md` line 215; SR-8.2, line
  305). They are `contracts/file-sessions.json` lines 1030, 1047, 1064 and 1081 and `contracts/redis-sessions.json`
  lines 1119, 1138, 1157, 1176, 1194, 1212, 1230, 1248 and 1263. They run in the contract case runners of the
  bindings, and no runner runs yet, because `make bindings` refuses the contract while the contract check reports the
  kinds of SE-10.6.

- `make test-bindings` builds and tests every target of the contract (binding BD-27): it runs clippy on the whole
  workspace, `make check-declarations`, `make test-python`, `make test-node`, `make test-wasm`, `make test-php` and
  `make test-go`, each on the standard runner of its language (unittest, node --test for Node and for WASM, PHPUnit and
  go test) with the tests that binding generates and the contract cases of `packages/<language>/tests`, which read the
  directory `contracts` through `SESSION_STORE_CASES`. The module and the glue of WASM are written by the wasm-bindgen
  CLI, whose release must be that of the `wasm-bindgen` crate, before the package is packed, so `make test-node` and
  `make test-wasm` share the step `node-package`. `make test-go` builds the static library with the features
  `panic-test` and `codec-test`, runs gofmt and go vet, then go test, with the Go of the go directive of
  `packages/go/go.mod` and `GOTOOLCHAIN=local`. The variables `PYTHON`, `PHP_CONFIG`, `PHPUNIT`, `TSC`, `GO`, `GOFMT`
  and `WASM_BINDGEN` name the runtimes and tools; `.cargo/config.toml` leaves the symbols of the PHP and Node runtimes
  undefined on macOS. The Go files `zz_generated_*.go` are tracked once binding writes them (binding GN-3), and
  `packages/ts/wasm/generated` and `packages/go/internal/ffi/generated` are ignored. Before, no make target built or
  tested a binding. Every one of the five targets fails at `make bindings`, because binding generate refuses the
  contract while the contract check reports the twelve kinds of SE-10.6 (binding CK-22); no contract case runner is
  written, because no binding API is generated that a runner could call and be checked against.

- The kinds that one call reaches have their cases (binding CK-22, SE-9.6, SE-10.5). The module `fault` of
  `polyspec-session-store-core`, which `polyspec-session-store` gives again, names four fault injection points:
  `session_store.random` in `random_hex` (`random`, SE-5.4), `session_store.internal` in the renewal of a memory store
  (`internal`, the one case of X-1), `session_store.open` at the open of a session file and `session_store.read_dir` at
  the read of the directory of the store of files (`io`, SF-9.2). The core crate has the feature `fault-test`, which
  `polyspec-session-store` turns on through its own feature `fault-test`, and the binding crates of PHP, Python and
  Node gain the feature `fault-test` that binding scaffold writes; no release build has it. The contract names the
  points in `faults`, and new groups that only one member reads hold the cases of one call: `created` gains the failing
  random source of `new_identifier`, `internal` (`memory_store.renew`), `openFaults` (`file_sessions.open`) and
  `collectFaults` (`file_sessions.collect`) on the directory `/`, and `openFaults` (`redis_sessions.open`, `random`)
  of the store of Redis. `connection` of `redis_sessions.open` is reached by an input, a setting that names a socket
  under `/dev/null`, so the new group `opening` holds that input case and no point imitates it. The Rust tests arm the
  faults of a case as a generated test does and fail a case whose fault did not fire. `make binding-check` gives 12
  findings of CK-22 where it gave 18; SE-10.6 states the case form that each needs: `random` of the four store calls of
  both sessions, `io` and `identifier_taken` of `file_session.close` and `connection` of `redis_session.close` need a
  receiver that `open` of another unit makes, and `cancelled` of `file_sessions.open` needs a value of the type
  `cancel`.

- `Cargo.lock` pins `polyspec-php-serialization` at the head of its branch `v1/rust-core`, whose crate has the fault
  injection point `php_serialization.internal` and the feature `fault-test`. Nothing else in the lock changes.

- The contract names the five targets of binding (BD-27): `targets` of `packages/rust/interface.json` is php, python,
  node, go and wasm, `packages.go` names the Go module `github.com/polyspec/session-store/packages/go`, the entry `.` of
  the npm package takes the conditions `{node: napi, browser: wasm}`, each error class names its WASM class, the Node
  one, and `errors.fixed` names `TypeError` for WASM as for Node. Before, the contract named php, python and node, so
  no Go or WASM binding was generated. The store of files has no WASM binding (SF-1.4, binding Q-4): the units
  `file_sessions` and `file_session` are `only` php, python, node and go, which the contract check requires of a member
  that waits on blocking I/O or on a process or takes a path (CK-15). The tracked binding packages gain what binding
  scaffold writes for the two targets (`packages/go/go.mod`, `packages/go/internal/ffi`, `packages/ts/wasm`, the browser
  exports of `packages/ts/package.json`), and both crates are members of the workspace. The contract check keeps its 18
  CK-22 findings from before this change (raised kinds without a case, and no case that arms
  `session_store.internal` as broken, X-1). No make target builds or tests the Go and WASM bindings yet. `make
  fmt-check` fails on `packages/go/internal/ffi/src/lib.rs`: binding scaffold writes its `allow` attribute in one line of
  125 characters, beyond `max_width = 120` of `rustfmt.toml`, and the file is not edited by hand.

- `scripts/binding.mts` clones the checkout that Cargo made of the pinned binding commit into `var/binding/<commit>`
  and no longer copies the files of its `packages/ts`. `binding check` reads its own commit from its Git checkout and
  compares it with the commit of `Cargo.lock` (CK-20); a copy of the files has no commit, which the check reports once
  the contract has no other finding.
- `Cargo.lock` pins the binding whose generator holds the value aliases, the fault injection and the error units in one
  line of development, and the binding packages are those that its scaffold writes: each binding crate takes
  `polyspec-binding-core` with the features `construct` and `lock`, and the lock gains `tokio` as a dependency of
  `polyspec-binding-core`. `make binding-check` still fails, now with 18 findings of CK-22 where it had 31, and
  `make bindings` writes nothing because of them: 17 name a member and a kind that no case raises by an input or by a
  fault injection point (`random`, `io`, `cancelled`, `connection`, `identifier_taken`), and one requires a case that
  arms the point `session_store.internal` as `broken`. The contract declares no fault injection point and the crates
  have none.
- The repository depends on `polyspec-binding-core` of the repository binding (`binding_core`, the Git dependency
  `https://github.com/polyspec/binding.git`, which `Cargo.lock` pins), the new name of bindgen, and runs its tool with
  `scripts/binding.mts`; the binding packages are those that `binding scaffold` writes. The crate
  `polyspec-session-store` depends on `polyspec-php-serialization`, whose `php_value` its contract names, and
  `Cargo.lock` takes the commit of `polyspec-php-serialization` on the same `binding_core`. The error classes stay
  `SessionStore<Category>Error` in every language: the repository name has no acronym word (design N-2).
- The contract lists `Internal` in the raises of `memory_store.renew`: the count of renewals that passes u64 is the kind
  `internal` (`MemoryStore::renew` in `packages/core/rust/src/memory.rs`), and an internal kind stays in raises (X-1).
- The binding packages are those that `binding scaffold` of the binding that `Cargo.lock` pins writes: their crates
  take `polyspec-binding-core` with its feature `construct`, and `packages/ts/package.json` names the Node packages of
  the three platforms as optional dependencies in place of the `.node` files.
- The contract names, for each member of the callback unit `store` (`get`, `set`, `remove`, `renew`), the method of the
  trait `polyspec_session_store::Store` that it is and its receiver `&mut self` in `core` (binding CK-17, design U-3):
  the generated type implements the trait, and the receiver of a trait method is not read from the Rust source.
- The kinds `random` and `identifier_taken` are of the category `io`, not `internal` (SE-9.5): the random source of the
  operating system and a file that another process created are the environment of the process, not a defect, so their
  errors are `SessionStoreIoError`. The category `internal` holds only the kind `internal`.
- `make test` names a peer (`PHP`, `REDIS_SERVER`, `REDIS_CLI`, `OPENSSL`) to the tests only when its binary is found.
  It gave a peer that is not found as an empty variable, which the tests of Redis read as a `redis-server` to start, so
  they failed on a machine without Redis; they are now reported as ignored there.
- Every public member of a generated unit names its cases (binding CK-22, design K-1), and every kind that an input
  reaches has a case. New cases: group `created` of `contracts/sessions.json` (`new_identifier`, SE-5.3); a value too
  deep for a record fails the close with `record_write` in both stores (SE-6.8); for the store of Redis a record that
  holds no array and one that unserialize refuses (`record_not_array`, `record`), an error reply to `GET` and to `SETEX`
  (`redis`, SR-5.2) and four connection settings that are refused (`invalid_setting`, SR-6.1). Cases state the
  identifier that an opening found (`found`, SE-7.1) and the directory of a made store (`store_directory`). SE-10.2 and
  SE-10.4 state the new keys of a case (`{"nested": n}`, `found`, `store_directory`, `setting`, `errorReplies`), and
  SE-10.4 names `REDIS_SERVER`, the variable that the tests read, in place of `REDIS_URL`.
- Every tracked document is a pair of an English file `X.md` and a Korean file `X.ko.md`. `AGENTS.ko.md` and
  `docs/spec/sessions.ko.md` are new, and `README.ko.md` links to the Korean specification. The first line of both files
  of a pair is the marker `doc-id` with the same name, and the second line of the Korean file is the marker
  `source-sha256`, the sha256 of its English file, so a Korean file whose English file changed after it is found by
  comparing the two values. No check of this repository reads the markers.
