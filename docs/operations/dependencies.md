<!-- doc-id: operations-dependencies -->
# Dependencies with a reason

## redis (redis-rs): the feature `tls-rustls-webpki-roots`

`packages/redis/rust/Cargo.toml` takes `redis` 1.8.1 of crates.io with its feature `tls-rustls-webpki-roots`, whose
roots of webpki (the crate `webpki-roots`) are built in and serve no connection.

redis-rs 1.8.1 builds the configuration of rustls for each connection of TLS (`create_rustls_config`,
`src/connection.rs:1232`, called by `src/aio/tokio.rs:145`). Without the feature it reads the trust store of the machine
there (`load_native_certs`, `src/connection.rs:1240-1253`), also when the caller gives its roots, which replace that
read only after it (`src/connection.rs:1257`). The read runs inside the connection timeout of 1 second, and the read of
the keychain of macOS can exceed it, so a connection of TLS failed by its timeout. The feature is the one public means
that leaves out that read: it compiles the roots of webpki in its place. redis-rs gives no public way to pass a
configuration of rustls.

The store reads the trust store once in a process and gives its authorities to every client of TLS that it builds
(SR-6.1a), through public setters alone: `Client::build_with_tls`, `SentinelClientBuilder::set_client_to_sentinel_certificates`
and `set_client_to_redis_certificates`, and `ClusterClientBuilder::certs`. No connection therefore uses the roots of
webpki. The test `a_tls_connection_reads_and_writes` (`packages/redis/rust/tests/server.rs`) shows that a connection reads
no trust store: in a process of its own whose trust store is the file of `SSL_CERT_FILE`, it connects, removes that
file and connects again.

The feature is removed when a release of redis-rs does not read the trust store for a connection whose roots are given.
