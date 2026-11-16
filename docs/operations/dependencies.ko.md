<!-- doc-id: operations-dependencies -->
<!-- source-sha256: 0fc538ed1643e43aaff09912d93e95075ff9a0e08559eeabe04bd6dea58430e7 -->
# 이유가 있는 의존성

## redis (redis-rs): feature `tls-rustls-webpki-roots`

`packages/redis/rust/Cargo.toml`은 crates.io의 `redis` 1.8.1을 feature `tls-rustls-webpki-roots`와 함께 쓴다. 이
feature가 넣는 webpki의 루트(crate `webpki-roots`)는 빌드에 들어가지만 어떤 연결에도 쓰이지 않는다.

redis-rs 1.8.1은 TLS 연결마다 rustls 설정을 만든다(`create_rustls_config`, `src/connection.rs:1232`, 호출은
`src/aio/tokio.rs:145`). feature가 없으면 그 안에서 기계의 신뢰 저장소를 읽는다(`load_native_certs`,
`src/connection.rs:1240-1253`). 호출자가 루트를 줘도 읽고, 준 루트는 그 읽기가 끝난 뒤에야 결과를 대신한다
(`src/connection.rs:1257`). 그 읽기는 1초의 연결 제한 시간 안에서 일어나고, macOS의 keychain을 읽는 시간이 그것을 넘길 수
있어서 TLS 연결이 제한 시간으로 실패했다. 이 feature는 그 읽기를 빼는 유일한 공개 수단이다. 그 자리에 webpki의 루트를
컴파일해 넣는다. redis-rs에는 rustls 설정을 넘기는 공개 방법이 없다.

저장소는 신뢰 저장소를 프로세스에서 한 번 읽고, 자기가 만드는 TLS의 모든 client에 그 인증 기관들을 준다(SR-6.1a). 공개
setter만 쓴다. `Client::build_with_tls`, `SentinelClientBuilder::set_client_to_sentinel_certificates`와
`set_client_to_redis_certificates`, `ClusterClientBuilder::certs`다. 그래서 어떤 연결도 webpki의 루트를 쓰지 않는다. 시험
`a_tls_connection_reads_and_writes`(`packages/redis/rust/tests/server.rs`)는 연결이 신뢰 저장소를 읽지 않는다는 것을
보인다. 신뢰 저장소가 `SSL_CERT_FILE`의 파일인 별도 process에서 연결하고, 그 파일을 지우고, 다시 연결한다.

redis-rs의 release가 루트를 받은 연결에서 신뢰 저장소를 읽지 않게 되면 이 feature를 뺀다.
