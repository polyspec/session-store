# Changelog

[한국어](CHANGELOG.ko.md)

## Unreleased

- Added the specification `docs/spec/sessions.md`: the store of the values of browser sessions, which knows no meaning
  of its keys and values (SE-1 to SE-10), the store of files (SF-1 to SF-9) and the store of Redis (SR-1 to SR-9).
- Added the cases `contracts/sessions.json`, `contracts/file-sessions.json` and `contracts/redis-sessions.json`, with the
  command records of the protocol of Redis.
- Added the crate `polyspec-sessions-core`: the trait `Store`, the memory store, identifiers, the record of a session in
  the form `php_serialize` and the record session of a request, with the kinds `random`, `record`,
  `record_not_array`, `record_write` and `internal`.
- Added the crate `polyspec-sessions-file`: `FileSessions` with `open` (a queue of the process, `flock(LOCK_EX)`, the
  file checked after the lock, ended sessions), `FileSession::close` and `collect`, with the kinds
  `invalid_directory`, `invalid_lifetime`, `cancelled`, `identifier_taken` and the kinds of the file system.
- Added the crate `polyspec-sessions-redis`: the protocol of phpredis as a state machine and `RedisSessions` with
  redis-rs to one server, Sentinel or Cluster with TLS and an ACL user, with the kinds `invalid_lifetime`,
  `invalid_lock_expire`, `invalid_setting`, `lock_timeout`, `lock_lost`, `reply`, `redis` and `connection`.
- Added the crate `polyspec-sessions`, the library, which gives the items of the three crates again (the stores under
  the features `file` and `redis`) and holds the one interface contract `packages/rust/interface.json`, with the
  classes `Sessions<Category>Error`. Every kind is a variant of the one enum `Error`. No binding is generated yet.
- A session identifier is 22 to 256 characters, and the lock expiry of the store of Redis has no default.
- Windows is not supported.
