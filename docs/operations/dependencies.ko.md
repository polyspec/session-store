<!-- doc-id: operations-dependencies -->
<!-- source-sha256: e24b1361a41c909e26ef8415bd4032d2a9f76782ad5f62bf34dce2ce959e92e9 -->
# 고정한 의존성

## redis (redis-rs)

`packages/redis/rust/Cargo.toml`은 crate `redis`를 fork <https://github.com/min-0-max/redis-rs>의 tag
`redis-1.8.2-p1`에서 가져온다. 이것은 redis-rs 1.8.2에 두 변경을 더한 것이다(그 `FORK.md`).

- `create_rustls_config`는 TLS 매개변수가 root store를 주면 기계의 신뢰 저장소를 읽지 않는다. redis-rs 1.8.2는 연결마다
  그것을 연결 제한 시간 안에서 읽는다. 그 제한 시간은 기본값이 1초이고, macOS에서 keychain을 읽는 데 0.33초에서
  1.19초가 걸려서 TLS 연결이 제한 시간으로 실패했다(SR-6.1a).
- `Sentinel::build_with_tls`는 호출자의 인증서를 Sentinel 노드로 가는 연결과, 그 노드가 알려 주는 primary와 replica로
  가는 연결에 준다. redis-rs 1.8.2에는 그것을 주는 공개 경로가 없다(SR-6.1a, SR-6.2).

저장소는 신뢰 저장소를 프로세스에서 한 번 읽고 그 인증 기관들을 모든 TLS client에 준다(SR-6.1a). 시험
`a_tls_connection_reads_and_writes`(`packages/redis/rust/tests/server.rs`)는 그 인증 기관을 받은 연결이 신뢰 저장소를
읽지 않는다는 것을 보인다. `SSL_CERT_FILE`이 없는 파일을 가리키는 동안 다시 연결한다.

redis-rs의 release가 두 변경을 모두 담으면 이 pin을 지우고 `redis`를 다시 crates.io에서 가져온다.
