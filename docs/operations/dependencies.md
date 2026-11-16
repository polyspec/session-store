<!-- doc-id: operations-dependencies -->
# Pinned dependencies

## redis (redis-rs)

`packages/redis/rust/Cargo.toml` takes the crate `redis` from the fork <https://github.com/min-0-max/redis-rs> at the
tag `redis-1.8.2-p1`, which is redis-rs 1.8.2 with two changes (its `FORK.md`):

- `create_rustls_config` does not read the trust store of the machine when the TLS parameters give a root store. redis-rs
  1.8.2 reads it for each connection, inside its connection timeout of 1 second by default, and on macOS the read of the
  keychain took 0.33 to 1.19 seconds, so a connection of TLS failed by its timeout (SR-6.1a).
- `Sentinel::build_with_tls` gives the certificates of the caller to the connections to the Sentinel nodes and to the
  primary and the replicas that they name. redis-rs 1.8.2 has no public way to give them (SR-6.1a, SR-6.2).

The store reads the trust store once in a process and gives its authorities to every client of TLS (SR-6.1a). The test
`a_tls_connection_reads_and_writes` (`packages/redis/rust/tests/server.rs`) shows that a connection with those
authorities reads no trust store: in a process of its own whose trust store is the file of `SSL_CERT_FILE`, it connects, removes that file and connects again.

The pin is removed, and `redis` is taken from crates.io again, when a release of redis-rs holds both changes.
