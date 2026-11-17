<!-- doc-id: operations-dependencies -->
<!-- source-sha256: 8370449ea7a5cc7c6b7fe33339e787bf41d1be34cf4dcf23f95beddc03cb0a05 -->
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

저장소는 신뢰 저장소를 프로세스에서 한 번 읽고, 자기가 만드는 TLS의 모든 client에 그 인증 기관들을 준다(SR-6.1b). 공개
setter만 쓴다. `Client::build_with_tls`, `SentinelClientBuilder::set_client_to_sentinel_certificates`와
`set_client_to_redis_certificates`, `ClusterClientBuilder::certs`다. 그래서 어떤 연결도 webpki의 루트를 쓰지 않는다.

redis-rs의 release가 루트를 받은 연결에서 신뢰 저장소를 읽지 않게 되면 이 feature를 뺀다.

## rcgen: 시험 전용 의존성

`packages/redis/rust/Cargo.toml`은 `[dev-dependencies]`에 `rcgen` 0.14.10을 둔다. 기본 feature는 끄고 `crypto`, `pem`,
그리고 rustls가 이미 가져오는 암호인 `ring`을 켠다. 시험 `a_file_of_authorities_is_the_only_trust_of_its_setting`
(`packages/redis/rust/tests/server.rs`, SR-6.7)이 그것으로 기계의 신뢰 저장소에 없는 인증 기관과, 그 기관이 서버에
발급하는 인증서를 만든다. 설정이 자기 파일의 인증 기관만 믿는다는 것을 보이기 위해서다. 그 시험에는 기계가 믿지
않는 인증 기관이 필요한데, bay는 자기 기관을 기계의 신뢰 저장소에 넣는다. 시험은 파일을 스스로 만들고, host의
`openssl`을 돌리지 않으며, bay의 파일을 읽지 않는다. release되는 어떤 crate도 `rcgen`에 의존하지 않는다.
dev-dependency는 그 crate의 시험을 위해서만 빌드된다. `Cargo.lock`은 `rcgen`의 선택적 의존성도 이름하지만, 어떤
빌드도 그것을 컴파일하지 않는다.
