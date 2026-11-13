<!-- doc-id: changelog -->

## Unreleased

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
