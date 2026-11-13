<!-- doc-id: changelog -->
<!-- source-sha256: ce7315e9d25a4f71b0372c57788480d685396bb886552e46d09acec1708247a6 -->

## Unreleased

- `Cargo.lock`은 binding 3b4bc6b를 고정하고, `binding scaffold`가 바인딩 패키지를 다시 쓴다. 바인딩 crate의 allow는
  자기 규칙 id를 이유로 주고, Go crate의 manifest에는 새 줄이 있다. 그 binding의 검사는 fault 주입점을 가지면서 자기
  시험이 feature `fault-test`로 그것을 받지 않는 코어를 실패시킨다(CK-65, GN-5). 그래서 코어, 파일 저장소, Redis
  저장소처럼 `packages/rust/Cargo.toml`이 라이브러리 자신을 `fault-test`와 함께 dev-dependency로 적는다.
  `make binding-check`는 통과하고, `make test`는 통과 154, 실패 0, 무시 13을 낸다.

- 저장소에는 계획도 checklist도 문서 검사도 없었다. `docs/plans/rust-core`가 계획과 checklist를 한국어 쌍과 함께
  담는다. 끝난 단계는 자기 commit을 적고, 바인딩과 그 end-to-end 검사와 Go 바인딩은 binding을 기다린다. binding이 모든
  target에서 계약의 생성을 거부하기 때문이다. `make docs-check`는 @polyspec/kit의 check-documents를
  `config/documents.json`의 문서와 `config/checklist.json`의 checklist에 실행하고, `make check`가 그것을 실행한다.
  @polyspec/kit은 `package.json`이 release의 URL로, `package-lock.json`이 integrity로 고정하며 `make bootstrap`이
  설치한다. `make documents-stamp`는 한국어 쌍의 `source-sha256`을 쓴다.

- 인터페이스 계약은 `memory_store.get`, `memory_store.set`, `memory_store.remove`에 `core.result: true`를 적는다
  (binding CK-21). 그것들은 kind를 내지 않으므로, 그것 없이는 계약이 그 코어 경로를 `Result`를 돌려주지 않는 것으로
  읽었다. 그러나 그 경로는 `Result`를 돌려주는 trait `Store`의 메서드이므로, 생성된 crate는 그것에 대해 컴파일되지
  않았을 것이다.

- `file_session`의 `close`에서 새 세션 파일의 배타적 생성이 fault 주입점 `session_store.create`를 가진다(SE-9.6,
  SF-4.4, SF-9.2). 그래서 `file_session.close`의 `io`와 `identifier_taken`이 case를 가진다. 전에는 어떤 주입점도
  입력도 그 둘에 닿지 않았고, `make binding-check`는 이 두 CK-22 지적을 냈다. 주입점은 `io`를 받고 `io`와
  `identifier_taken`을 채우며, 실패한 시스템 호출을 fault의 `errno`로 흉내 낸다. `EEXIST`는 다른 프로세스가 가져간
  식별자다. `contracts/file-sessions.json`의 group `file_session.close.made`는 `open`이 만들고 값을 set한 세션을
  닫으며, fault `EIO`로 `io`를, fault `EEXIST`로 `identifier_taken`을 준다. `make binding-check`는 CK-22 지적을 내지
  않는다.

- `open`이 만든 세션만 닿는 kind가 case를 가진다(SE-10.6, binding GN-13f, CK-22). `Cargo.lock`은 그 case 꼴을 받고
  검사하는 binding을 고정한다. case는 세션을 키 `receiver` 아래에 instance 값으로 준다. 그것은 생성자로 만든 저장소의
  쿠키 없는 `open`이며, 저장소는 디렉터리 `/`의 파일 저장소(발동하지 않는 취소와 함께)와 `/dev/null` 아래 socket의
  Redis 저장소다. `contracts/file-sessions.json`의 새 group `file_session.get.made`, `file_session.set.made`,
  `file_session.remove.made`, `file_session.renew.made`와 `contracts/redis-sessions.json`의 같은 `redis_session` 넷은
  `session_store.random`을 장전하고 `random`을 준다. `redis_session.close.made`는 값을 set한 세션의 close의
  `connection`을 주고, `file_sessions.open.cancel`은 호출 전에 발동한 취소로 `file_sessions.open`의 `cancelled`를
  준다. 파일 저장소와 Redis 저장소의 시험이 이 case 열 개를 실행한다. `make binding-check`는 12건이던 CK-22 finding을
  2건으로 낸다: 결함 주입 지점도 입력도 닿지 않는 `file_session.close`의 `io`와 `identifier_taken`이다. `make test`는
  152개 통과, 0개 실패, 13개 ignored를 낸다.

- 라이브러리가 지금 `v1/rust-core`의 binding과 php-serialization을 받는다. 그 결함 주입 지점은 macro
  `binding_core::fault::point!`이며, 지점을 쓰는 crate의 feature `fault-test`로 지점을 판정한다(binding GN-5). 코어의
  모듈 `fault`가 `point!`를 부르고, 코어, 파일 저장소, Redis 저장소의 시험은 dev-dependency로
  `polyspec-session-store-core`의 `fault-test`를 켜서 지점이 발동하게 한다. 전에는 dev-dependency가 binding의 것만
  켰다. 지점 `session_store.internal`은 이제 불변식 분기 둘을 가진다: 전과 같은 `MemoryStore`의 갱신(SE-4.3)과,
  전에 지점 `session_store.session_waits`가 가졌던 record session의 시작(`session.rs` 70행)이다. 그래서 그 지점은
  없앤다. record session의 시작은 `file_session`과 `redis_session`의 모든 호출자의 경로에 있으므로, 이 라이브러리의
  kind를 넘겨주는 계약은 자기 member에서 지점 `internal`을 장전한다(binding X-1). 전에는 그 지점이 그런 호출자가
  닿지 않는 `MemoryStore`에만 있었다. `recordFaults`의 case는 `session_store.internal`을 장전하고, 장전하지 않은 같은
  호출을 먼저 실행한다. `binding scaffold`가 바인딩 패키지를 다시 쓴다: 각각 workspace의 lint를 받고(`[lints]
  workspace = true`), `packages/php-ext/build.rs`가 새로 생겼다. 바인딩 crate가 이유와 함께 unsafe 코드를 허용하므로
  (binding B-4) workspace는 unsafe 코드를 forbid하지 않고 deny한다. 라이브러리의 crate는 각자 `lib.rs`에서 forbid한다.
  `make bindings`는 target마다 따로 생성하고(`make bindings-<target>`, binding GN-1b), 바인딩의 make target은 자기
  target의 생성만 필요로 한다. `make test`는 142개 통과, 0개 실패, 13개 ignored를 낸다.

- record session의 `get`, `set`, `remove`가 닿는 kind `internal`의 모든 불변식 분기가 실패 주입점을 가진다(SE-9.6).
  전에는 kind `internal`의 주입점이 `MemoryStore`의 `renew`에 있는 `session_store.internal` 하나뿐이어서,
  `file_session`이나 `redis_session`의 `get`, `set`, `remove`를 부르는 쪽에는 그 kind를 내는 주입점이 없었다. 새
  주입점은 각각 `broken`을 받고 `internal`을 채우며, 인터페이스 계약의 `faults`에 선언되고, `renew`의 주입점처럼 분기
  바로 앞에서 `fault::invariant`로 확인된다. `packages/core/rust/src`의 주입점과 그 분기:
  `session_store.session_waits`, `session.rs` 74행(`get`, `set`, `remove`, `renew`); `session_store.record_root`,
  `record.rs` 47행(찾은 레코드의 첫 사용); `session_store.record_index`, `record.rs` 63행;
  `session_store.value_reference`, `copy.rs` 116행; `session_store.value_root`, `copy.rs` 123행;
  `session_store.copy_new_reference`, `copy.rs` 69행; `session_store.copy_array`, 82행; `session_store.copy_element`,
  86행; `session_store.copy_object`, 91행; `session_store.copy_property`, 95행; `session_store.copy_reference`,
  100행; `session_store.copy_set_reference`, 103행(`get`, 그리고 레코드의 복사를 거치는 `close`).
  `contracts/sessions.json`의 새 group `recordFaults`는 주입점마다 한 호출 case를 하나씩 담고, 코어의 시험이 그
  case를 record session에서 실행한다. 장전하지 않은 호출은 성공하고, 장전한 호출은 kind `internal`과 그 분기의
  메시지로 실패한다. `MemoryStore`의 `get`, `set`, `remove`(`memory.rs` 73~85행)는 오류를 돌려주지 않고,
  `random_hex`(`identifier.rs` 30~47행)는 kind `random`으로만 실패하므로, 이 경로는 `internal`을 내지 않으며
  주입점이 없다. `close`만 닿는 `encode`의 분기(`record.rs` 97행과 104행)에는 주입점이 없다.

- 실제 Redis 서버의 시험이 Apple `container`의 container 안에서 돌고, peer가 없으면 실패한다(SE-10.4). 전에는 `make
  test`가 PATH에서 `redis-server`, `redis-cli`, `openssl`을 찾고, 없으면 그것이 필요한 시험 21개를 무시된 시험으로
  보고했다. group `requests`의 case 13개를 서버에 대해 실행하는 시험(`tests/contract.rs`의 `server::...`. 그 밖에
  `textReplies`나 `errorReplies`를 가진 case의 무시된 trial 3개가 있었는데, 이 case는 모델에 대해서만 실행되므로 이제
  서버에 대한 trial이 없다)과 `tests/server.rs`의 한 프로세스의 대기열, ACL 사용자, TLS, Sentinel, Cluster다. PHP가
  필요한 시험은 PHP 없이 통과했다. 이제 모든 서버는 가장 높은 안정판인 Redis 8.10.3의 `redis-server`이고, Makefile의
  `REDIS_IMAGE`가 고정하고 `make redis-image`가 받는 공식 image
  `docker.io/library/redis:8.10.3-trixie@sha256:de1068af6f552240dfab67bcc9bfef99bfc60f776bb4580ef077ea4c8c97e848`에서
  돈다. 모듈 `tests/servers`는 각 서버를 run의 이름을 가진 container 안에서 시작하고, 서버가 시작하기 전에 container가
  쓰는 주소를 받고, 서버의 줄(준비, Sentinel이 감시하는 primary, 각 Cluster node의 state ok)을 기다리며, redis-cli를
  자기 container 안에서 실행한다. TLS는 container 주소의 인증서와 그 port를 CONFIG SET으로 받는다. 어떤 시험도
  시간을 기다리거나 polling하지 않는다. socket 파일, 연결, `cluster info`에 대한 기다림과 대기열 시험의 읽기와 쓰기
  사이의 2 ms sleep을 없앴다. `scripts/test-run.sh`는 시험을 자기 process group에서 실행하고, run이 끝날 때, 실패할 때,
  SIGINT나 SIGTERM으로 멈출 때 그 run의 container와 임시 디렉터리를 지운다. container, image, openssl, PHP가 주어지지
  않은 시험은 실패하고 `make test`나 `make redis-image`를 적는다. `make test`의 결과는 통과 130, 실패 0, 무시 13이다.
  무시되는 13개는 `binding`을 가진 group `open`의 case다. 음수, 소수, 텍스트, 불리언인 `lifetime`이나
  `lock_expire`와 빠진 `lock_expire`이며, 바인딩이 저장소를 실행하기 전에 거부한다(SF-8.2, `docs/spec/sessions.md` 215행; SR-8.2,
  305행). 이 case는 `contracts/file-sessions.json` 1030, 1047, 1064, 1081행과 `contracts/redis-sessions.json` 1119,
  1138, 1157, 1176, 1194, 1212, 1230, 1248, 1263행이다. 이 case는 바인딩의 contract case runner에서 돌고, 아직 어떤
  runner도 돌지 않는다. contract 검사가 SE-10.6의 kind를 보고하는 동안 `make bindings`가 계약을 거부하기 때문이다.

- `make test-bindings`가 계약의 모든 target을 빌드하고 시험한다(binding BD-27). workspace 전체에 clippy를 실행하고,
  `make check-declarations`, `make test-python`, `make test-node`, `make test-wasm`, `make test-php`, `make test-go`를
  실행한다. 각각은 그 언어의 표준 runner(unittest, Node와 WASM의 node --test, PHPUnit, go test)로 binding이 생성한
  시험과 `packages/<language>/tests`의 contract case를 실행하며, contract case는 `SESSION_STORE_CASES`로 디렉터리
  `contracts`를 읽는다. WASM의 모듈과 glue는 패키지를 묶기 전에 wasm-bindgen CLI가 쓰며, 그 CLI의 판은
  `wasm-bindgen` crate의 판이어야 한다. 그래서 `make test-node`와 `make test-wasm`은 단계 `node-package`를 함께 쓴다.
  `make test-go`는 feature `panic-test`와 `codec-test`로 정적 라이브러리를 빌드하고, gofmt와 go vet을 실행한 다음
  go test를 실행한다. Go는 `packages/go/go.mod`의 go 지시문의 판이고 `GOTOOLCHAIN=local`로 실행한다. 변수 `PYTHON`,
  `PHP_CONFIG`, `PHPUNIT`, `TSC`, `GO`, `GOFMT`, `WASM_BINDGEN`이 런타임과 도구를 가리키고, `.cargo/config.toml`은
  macOS에서 PHP와 Node 런타임의 기호를 정의하지 않은 채 둔다. Go 파일 `zz_generated_*.go`는 binding이 쓰면
  추적되고(binding GN-3), `packages/ts/wasm/generated`와 `packages/go/internal/ffi/generated`는 무시된다. 전에는
  바인딩을 빌드하거나 시험하는 make target이 없었다. 다섯 target은 모두 `make bindings`에서 실패한다. 계약 검사가
  SE-10.6의 kind 열두 개를 보고하는 동안 binding generate가 계약을 거부하기 때문이다(binding CK-22). runner가
  호출하고 그것으로 검증될 바인딩 API가 생성되지 않으므로 contract case runner는 쓰지 않았다.

- 한 호출이 닿는 kind가 자기 case를 갖는다(binding CK-22, SE-9.6, SE-10.5). `polyspec-session-store`가 다시 내보내는
  `polyspec-session-store-core`의 모듈 `fault`가 실패 주입점 넷을 이름한다: `random_hex`의
  `session_store.random`(`random`, SE-5.4), 메모리 저장소의 갱신의 `session_store.internal`(`internal`, X-1의 한
  case), 파일 저장소에서 세션 파일의 open의 `session_store.open`과 디렉터리의 읽기의
  `session_store.read_dir`(`io`, SF-9.2). 코어 crate에 feature `fault-test`가 있고 `polyspec-session-store`가 자기
  feature `fault-test`로 그것을 켜며, PHP, Python, Node의 바인딩 crate에는 binding scaffold가 쓰는 feature
  `fault-test`가 더해진다. 어떤 release 빌드에도 그것은 없다. 계약은 주입점을 `faults`에 적고, 한 member만 읽는 새
  group이 한 호출 case를 담는다: `created`에 `new_identifier`의 실패하는 난수원이 더해지고,
  `internal`(`memory_store.renew`), 디렉터리 `/`의 `openFaults`(`file_sessions.open`)와
  `collectFaults`(`file_sessions.collect`), Redis 저장소의 `openFaults`(`redis_sessions.open`, `random`)가 생긴다.
  `redis_sessions.open`의 `connection`에는 입력, 곧 `/dev/null` 아래 socket을 적은 설정이 닿으므로, 새 group
  `opening`이 그 입력 case를 담고 어떤 주입점도 그것을 흉내 내지 않는다. Rust 시험은 생성된 시험처럼 case의
  fault를 장전하고, fault가 터지지 않은 case를 실패시킨다. `make binding-check`는 18건이던 CK-22 지적을 12건으로
  낸다. SE-10.6이 각각에 필요한 case 형식을 적는다: 두 세션의 저장소 호출 넷의 `random`, `file_session.close`의
  `io`와 `identifier_taken`, `redis_session.close`의 `connection`에는 다른 unit의 `open`이 만드는 receiver가
  필요하고, `file_sessions.open`의 `cancelled`에는 타입 `cancel`의 값이 필요하다.

- `Cargo.lock`은 `polyspec-php-serialization`을 그 브랜치 `v1/rust-core`의 끝으로 고정한다. 그 crate에는 실패 주입점
  `php_serialization.internal`과 feature `fault-test`가 있다. lock의 다른 것은 바뀌지 않는다.

- 계약이 binding의 다섯 target을 적는다(BD-27): `packages/rust/interface.json`의 `targets`는 php, python, node,
  go, wasm이고, `packages.go`는 Go 모듈 `github.com/polyspec/session-store/packages/go`를 적으며, npm 패키지의 entry
  `.`는 조건 `{node: napi, browser: wasm}`을 받고, 각 오류 클래스는 WASM 클래스로 Node의 것을 적으며,
  `errors.fixed`는 Node처럼 WASM에도 `TypeError`를 적는다. 전에는 계약이 php, python, node만 적어 Go와 WASM 바인딩이
  생성되지 않았다. 파일 저장소에는 WASM 바인딩이 없다(SF-1.4, binding Q-4): unit `file_sessions`와 `file_session`은
  `only` php, python, node, go이며, 계약 검사는 blocking I/O나 프로세스를 기다리거나 경로를 받는 member에 이것을
  요구한다(CK-15). 추적되는 바인딩 패키지에 binding scaffold가 두 target을 위해 쓰는 것(`packages/go/go.mod`,
  `packages/go/internal/ffi`, `packages/ts/wasm`, `packages/ts/package.json`의 브라우저 exports)이 더해지고, 두
  crate가 workspace의 member가 된다. 계약 검사는 이 변경 전부터 있던 CK-22 finding 18개를 그대로 낸다(case 없는 raise
  kind, 그리고 `session_store.internal`을 broken으로 켜는 case가 없음, X-1). Go와 WASM 바인딩을 빌드하거나 시험하는
  make target은 아직 없다. `make fmt-check`는 `packages/go/internal/ffi/src/lib.rs`에서 실패한다: binding scaffold가
  그 `allow` 속성을 125자의 한 줄로 쓰는데 `rustfmt.toml`의 `max_width = 120`을 넘고, 이 파일은 손으로 고치지 않는다.

- `scripts/binding.mts`는 Cargo가 고정 commit으로 만든 binding checkout을 `var/binding/<commit>`에 clone하고, 그 `packages/ts`의 파일을 더는 복사하지 않는다. `binding check`는 자기 Git checkout에서 자기 commit을 읽어 `Cargo.lock`의 commit과 비교한다(CK-20). 파일의 복사본에는 commit이 없고, 검사는 계약에 다른 지적이 없어지면 그것을 지적한다.
- `Cargo.lock`은 값 alias, 실패 주입, 오류 unit의 생성기를 한 개발 줄기에 가진 binding을 고정하고, 바인딩 package는 그 scaffold가 쓰는 것이다. 각 바인딩 crate는 `polyspec-binding-core`를 feature `construct`와 `lock`과 함께 받고, lock에는 `polyspec-binding-core`의 의존으로 `tokio`가 더해진다. `make binding-check`는 여전히 실패하며, 31건이던 CK-22 지적이 18건이 되었고, 그 때문에 `make bindings`는 아무것도 쓰지 않는다. 17건은 어떤 case도 입력이나 실패 주입점으로 내지 않는 member와 kind(`random`, `io`, `cancelled`, `connection`, `identifier_taken`)를 가리키고, 1건은 주입점 `session_store.internal`을 `broken`으로 장전하는 case를 요구한다. 계약은 실패 주입점을 선언하지 않고 crate에도 없다.
- 레포는 binding 레포(bindgen의 새 이름)의 `polyspec-binding-core`(`binding_core`, `Cargo.lock`이 고정하는 Git 의존 `https://github.com/polyspec/binding.git`)에 의존하고 그 도구를 `scripts/binding.mts`로 실행한다. 바인딩 package는 `binding scaffold`가 쓰는 것이다. crate `polyspec-session-store`는 계약이 `php_value`를 가리키는 `polyspec-php-serialization`에 의존하고, `Cargo.lock`은 같은 `binding_core`를 쓰는 `polyspec-php-serialization` 커밋을 받는다. 레포 이름에 약어 낱말이 없으므로 오류 클래스는 모든 언어에서 `SessionStore<Category>Error`다(설계 N-2).
- 계약은 `memory_store.renew`의 raises에 `Internal`을 둔다. u64를 넘는 갱신 횟수는 kind `internal`이고(`packages/core/rust/src/memory.rs`의 `MemoryStore::renew`), internal kind는 raises에 남는다(X-1).
- 바인딩 package는 `Cargo.lock`이 고정한 binding의 `binding scaffold`가 쓰는 것이다. 그 crate는 `polyspec-binding-core`를 feature `construct`와 함께 받고, `packages/ts/package.json`은 `.node` 파일 대신 세 플랫폼의 Node package를 optional dependency로 둔다.
- 계약은 callback unit `store`의 member(`get`, `set`, `remove`, `renew`)마다 그 member가 trait `polyspec_session_store::Store`의 어느 메서드인지와 receiver `&mut self`를 `core`에 적는다(binding CK-17, 설계 U-3). 생성 타입이 그 trait를 구현하고, trait 메서드의 receiver는 Rust 소스에서 읽지 않기 때문이다.
- kind `random`과 `identifier_taken`은 category `internal`이 아니라 `io`다(SE-9.5). 운영체제의 난수원과 다른 프로세스가 만든 파일은 구현의 결함이 아니라 프로세스의 환경이므로, 그 오류는 `SessionStoreIoError`다. category `internal`에는 kind `internal`만 있다.
- `make test`는 실행 파일을 찾은 peer(`PHP`, `REDIS_SERVER`, `REDIS_CLI`, `OPENSSL`)만 시험에 알린다. 전에는 찾지 못한 peer를 빈 변수로 넘겼고, Redis 시험이 그것을 시작할 `redis-server`로 읽어 Redis가 없는 기계에서 실패했다. 이제 그런 기계에서는 무시된 시험으로 보고된다.
- 생성 unit의 모든 public member가 자기 case를 가리키고(binding CK-22, 설계 K-1), 입력으로 닿는 모든 kind에 case가 있다. 새 case: `contracts/sessions.json`의 group `created`(`new_identifier`, SE-5.3), 레코드로 쓰기에 너무 깊은 값이 두 저장소에서 close를 `record_write`로 실패시키는 case(SE-6.8), Redis 저장소에서 배열이 아닌 레코드와 unserialize가 거부하는 레코드(`record_not_array`, `record`), `GET`과 `SETEX`의 오류 응답(`redis`, SR-5.2), 거부되는 연결 설정 넷(`invalid_setting`, SR-6.1). case는 열 때 찾은 식별자(`found`, SE-7.1)와 만든 저장소의 디렉터리(`store_directory`)를 적는다. SE-10.2와 SE-10.4는 case의 새 키(`{"nested": n}`, `found`, `store_directory`, `setting`, `errorReplies`)를 적고, SE-10.4는 `REDIS_URL` 대신 시험이 읽는 변수 `REDIS_SERVER`를 적는다.
- 추적하는 모든 문서는 영어 파일 `X.md`와 한국어 파일 `X.ko.md`의 쌍이다. `AGENTS.ko.md`와 `docs/spec/sessions.ko.md`가 새로 생겼고, `README.ko.md`는 한국어 명세로 링크한다. 쌍의 두 파일의 첫 줄은 같은 이름의 marker `doc-id`이고, 한국어 파일의 둘째 줄은 영어 파일의 sha256인 marker `source-sha256`이다. 그래서 영어 파일이 나중에 바뀐 한국어 파일은 두 값을 비교해 찾는다. 이 레포의 어떤 검사도 marker를 읽지 않는다.
