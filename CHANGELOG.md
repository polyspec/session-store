
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
