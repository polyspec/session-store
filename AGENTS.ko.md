<!-- doc-id: agents -->
<!-- source-sha256: f9b93bb074b35c595ac034655e372e7481c79c229f74650c2d26932ff0f3714b -->
# AGENTS.md

session-store의 모든 변경이 따르는 규칙이다. 라이브러리의 규칙은 [docs/spec/sessions.ko.md](docs/spec/sessions.ko.md)에 있다.

## 배치

- `packages/rust/`: crate `polyspec-session-store`, 곧 라이브러리다. 이 crate는 세 crate의 항목을 다시 내보내고
  (저장소는 feature `file`과 `redis` 아래에 둔다), 그 package는 바인딩의 하나뿐인 인터페이스 계약
  `interface.json`을 담는다(`Cargo.toml`의 `include`). 그래서 이 crate에 의존하는 레포는 계약을 Cargo로
  읽는다. 손으로 쓰는 바인딩은 없다.
- `packages/core/rust/`: crate `polyspec-session-store-core`, 곧 저장소 인터페이스, 식별자, 레코드, 요청 하나의
  레코드 세션이다(SE-1부터 SE-10). 이 crate는 I/O를 하지 않는다.
- `packages/file/rust/`: crate `polyspec-session-store-file`, 곧 파일 저장소다(SF-1부터 SF-9).
- `packages/redis/rust/`: crate `polyspec-session-store-redis`, 곧 Redis 저장소다: I/O 없는 프로토콜과 그
  연결이다(SR-1부터 SR-9).
- `packages/php-ext/`, `packages/python/`, `packages/ts/`: 바인딩 package다. 그 추적 파일은 `binding scaffold`가
  쓰고 계약 검사가 비교한다(binding CK-19). 이 package들은 workspace의 member다. `make bindings`가 추적하지 않는
  `generated/`를 쓰고, 바인딩 package의 빌드는 그 뒤에 한다. 코어의 target은 코어 crate를 `-p`로 지정하므로
  생성된 소스가 필요 없다.
- `scripts/binding.mts`: `Cargo.lock`이 고정하는 `polyspec-binding-core`의 commit에서 binding을 실행한다.
- 루트의 `Cargo.toml`은 crate 넷과 바인딩 crate 셋의 workspace이며 `Cargo.lock`은 하나다.
- `contracts/sessions.json`, `contracts/file-sessions.json`, `contracts/redis-sessions.json`: case다(SE-10).
- `docs/spec/`: 명세다. 루트: `AGENTS.md`, `README.md`, `CHANGELOG.md`와 그 한국어 번역
  (`*.ko.md`), `Makefile`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`.

## 변경

- 규칙을 바꾸는 변경은 명세를 먼저 바꾸고, 그다음 case를, 그다음 코드를 바꾼다. 시험은 자신이 검사하는 규칙
  문장을 적는다(`SE-n.m`, `SF-n.m`, `SR-n.m`).
- 규칙은 요구하는 것과 그 근거를 적는다: 이 라이브러리의 규칙 id, php-serialization의 규칙(RQ-n), 표준의
  조항 또는 peer(PHP, phpredis)의 동작이다. 추적 파일은 규칙을 어디에서 가져왔는지와 누가 요청했는지를
  기록하지 않는다.
- 라이브러리는 세션에 무엇이 담기는지 모른다: 이 레포의 어떤 규칙도 세션의 key를 지정하지 않고 값에 뜻을
  주지 않는다(SE-1.2).
- 실패에는 kind가 있다. 라이브러리는 실패를 그럴듯한 값으로 바꾸지 않는다. 블로킹 Rust 함수는 binding-core의
  기다림 토큰(`Io`, `ProcessWait`)을 첫 인자로 받는다.
- 명세, case, README, 이 파일, 변경 기록(영어와 한국어)은 코드와 같은 변경에서
  바뀐다.

## Rust

- toolchain은 `rust-toolchain.toml`이 고정하는 릴리스다. `make check`는 `-D warnings`를 준 clippy, rustfmt,
  시험, 계약 검사를 실행한다.
- workspace의 lint는 모든 경고, unsafe 코드, `unwrap`, `expect`, `as` 변환과 값을 잃는 cast, 써야 하는 값이나
  타입을 적지 않은 값의 `let _`, 이유 없는 allow attribute를 거부한다.
  `clippy.toml`은 `Option::{unwrap_or, unwrap_or_default, unwrap_or_else, map_or, map_or_else}`와
  `Result::{ok, unwrap_or, unwrap_or_default, unwrap_or_else, map_or, map_or_else}`를 금지한다. 규칙이 필요로 하는 사용에는
  `#[allow(..., reason = "<rule id>")]`를 붙인다.
- peer(PHP, redis-server, redis-cli, openssl)가 필요한 시험은 Makefile이 그 peer를 찾으면 실행되고, 찾지 못하면
  무시된 시험으로 보고된다.

## 플랫폼

Linux x64와 arm64(glibc), macOS arm64다. polyspec의 Rust 코어와 바인딩이 안정되고 ext-php-rs가 고정된 stable
toolchain으로 Windows에서 빌드될 때까지 Windows는 지원하지 않는다.

## Commit

- commit 메시지는 `<type>(<scope>): <Subject> (#id)`이고, 사실만 간결하게 적으며, 본문은 무엇이 왜 바뀌었는지를
  적는다. 서로 관련 없는 변경은 별도의 commit으로 한다.
- 어떤 추적 파일도 Git commit을 지정하지 않는다.
