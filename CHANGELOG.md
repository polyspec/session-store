<!-- doc-id: changelog -->

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
