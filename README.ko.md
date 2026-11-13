<!-- doc-id: readme -->
<!-- source-sha256: f78ea06a4c934cdbd320d0141dc3bb79f068e822a09c153a18d473a0f8757bd5 -->
# session-store

[English](README.md)

브라우저 세션의 값을 Rust 구현 한 벌로 저장한다: 메모리, PHP files handler 형식의 디렉터리 파일, 또는 phpredis의 키와 잠금을 쓰는
Redis. 라이브러리는 세션에 무엇이 담기는지 모른다: key는 텍스트이고 값은 php-serialization의 PHP 값이다. 규칙은
[docs/spec/sessions.ko.md](docs/spec/sessions.ko.md)에 있고, 모든 구현이 통과하는 case는 [contracts/](contracts/)에 있다.

- `polyspec-session-store-core`: 애플리케이션이 자기 언어로 구현할 수도 있는 저장소 인터페이스(`Store`), 메모리 저장소, 세션 식별자,
  세션 레코드(`php_serialize`), 처음 쓸 때 시작하고 저장소가 쓸 것을 내주는 요청 하나의 레코드 세션.
- `polyspec-session-store-file`: 세션마다 파일 `sess_<id>` 하나를 `flock(LOCK_EX)` 아래에 두고, 한 프로세스 안 같은 세션 요청의
  queue, 갱신, 끝난 세션 삭제(`collect`)를 한다.
- `polyspec-session-store-redis`: phpredis 프로토콜을 I/O 없는 상태기계로 두고(`EXISTS`, `SET NX PX`, `GET`, `SETEX` 전의 잠금 비교,
  바뀌지 않은 세션의 `EXPIRE`, 해제 스크립트), redis-rs로 서버 하나, Sentinel, Cluster에 TLS와 ACL 사용자로 연결한다.

## Rust

```rust
use binding_core::{Io, ProcessWait};
use polyspec_session_store::{FileSessions, Store};

let store = FileSessions::in_directory(io, "/var/lib/sessions".as_ref(), None)?;
let mut session = store.open(wait, cookie.as_deref(), None)?;
let value = session.get("cart")?;
let created = session.close(io)?; // 새 식별자. 호출자가 세션 쿠키로 쓴다
```

기다림 토큰 `Io`와 `ProcessWait`는 생성된 바인딩이 만든다. 블로킹 함수를 부르는 Rust 호출자는 그것을 실행하는 바인딩 층에서
토큰을 받는다.

crate `polyspec-session-store`(`packages/rust`)가 세 crate의 항목을 한 라이브러리로 내보낸다. 모든 계약이 적는 다섯 target인 PHP, Python, Node, 브라우저(WASM), Go 바인딩은 그 패키지가
담은 인터페이스 계약 `packages/rust/interface.json`에서 생성할 예정이며, 아직 바인딩은 없다.

## 검사

```
make check   # clippy -D warnings, rustfmt --check, 시험, 계약 검사
```

필요한 것: `rust-toolchain.toml`의 Rust 릴리스, `php`, `openssl`, 그리고 `make redis-image`가 받는 Redis image를 가진 플랫폼의
container 프로그램(macOS는 Apple `container`, Linux는 `docker`). 시험은 PHP로 레코드를 다시 읽고, Redis 저장소를 그 image의 container 안에 있는 실제 서버에 대해 실행한다.
peer가 없는 시험은 실패한다. Windows는 아직 지원하지 않는다.
