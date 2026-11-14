<!-- doc-id: agents -->
<!-- source-sha256: 4588c372bc85ddf9227f22b393a05e58edea9a51a944fdfb9ab4f4c3044107a8 -->
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
- `packages/php-ext/`, `packages/python/`, `packages/ts/`, `packages/ts/wasm/`, `packages/go/`: 모든 계약이 적는 다섯
  target(binding BD-27)의 바인딩 package이며, Go의 C ABI는 `packages/go/internal/ffi`에 있다. 그 추적 파일은 `binding scaffold`가
  쓰고 계약 검사가 비교한다(binding CK-19). 이 package들은 workspace의 member다. `make bindings`가 추적하지 않는
  `generated/`를 쓰고, 바인딩 package의 빌드는 그 뒤에 한다. 코어의 target은 코어 crate를 `-p`로 지정하므로
  생성된 소스가 필요 없다.
- `scripts/binding.mts`: `Cargo.lock`이 고정하는 `polyspec-binding-core`의 commit에서 binding을 실행한다. binding을
  내려받는 것은 `make install-binding`뿐이다. 그 target은 binding을 그 commit으로 `var/binding/<commit>`에 clone하고 그
  도구의 패키지를 `var/tools`의 npm으로 설치한다. 다른 모든 명령은 crate를 `cargo metadata --offline`으로 읽고 binding이
  설치되지 않았으면 실패한다. 내려받는 검사는 네트워크에 의존하기 때문이다(T1.28).
- `scripts/test-run.mts`: `make test`의 시험을 자기 서버와 임시 디렉터리를 소유하는 run으로 실행한다(SE-10.4).
- `scripts/servers.sh`: 시험의 Redis 서버를 위한 bay의 명령이다. 시험, `scripts/test-run.mts`, `make redis-image`가 이
  script를 실행한다(SE-10.4). bay가 서버를 Compose 프로젝트로 시작하고 지우며, 서버의 이름과 certificate와 주소를 주고,
  image를 받고 찾는다. `scripts/servers.test.mts`가 두 script의 시험이며, `make servers-test`가 bay의 대역으로
  실행한다.
- `scripts/install-bay.mts`: `config/toolchain.json`이 CI를 위해 고정한 bay의 릴리스를 archive의 sha256으로 확인하여
  `var/tools/bay`에 설치한다(`make install-bay`). `scripts/install-bay.test.mts`가 그 시험이며,
  `make install-bay-test`가 시험이 직접 묶은 archive로 실행한다.
- 루트의 `Cargo.toml`은 crate 넷과 다섯 target의 바인딩 crate의 workspace이며 `Cargo.lock`은 하나다.
- `contracts/sessions.json`, `contracts/file-sessions.json`, `contracts/redis-sessions.json`: case다(SE-10).
- `scripts/workflows.test.mts`: GitHub workflow의 규칙의 시험이며, `make workflows-test`가 실행한다.
- `scripts/check-commit-names.mts`: `make commit-names-check`의 검사다. 그 시험은 `scripts/commit-names.test.mts`이며,
  `make commit-names-test`가 실행한다.
- `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/actions/bootstrap`: CI, 릴리스, 그 job의
  bootstrap이다. bootstrap은 `make bootstrap`, `make install-tools TOOLS=npm`, `make install-packages`를 실행한다.
- `mise.toml`, `mise.lock`, `.mise/locks`, `config/toolchain.json`: checkout의 도구 고정이다("도구").
  `scripts/tool-pins.test.mts`가 그 시험이며, `make tool-pins-test`가 실행한다.
- `config/owner-checks.json`: 추적되는 각 경로를 소유하는 make target이다. `make owner-validate`는 어떤 규칙도 소유하지
  않는 추적 경로와 Makefile이 정의하지 않는 target을 실패시킨다. 경로를 더하는 변경은 그 소유자를 더한다.
  `make owner-check PATHS="<paths>"`는 지정한 경로의 소유자를 실행한다.
- `config/release.json`: 모든 manifest의 릴리스 분류이며, kit의 릴리스 유틸리티가 읽는다.
- `docs/spec/`: 명세다. 루트: `AGENTS.md`, `README.md`, `CHANGELOG.md`와 그 한국어 번역
  (`*.ko.md`), `Makefile`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `.cargo/config.toml`.
- `LICENSE`: manifest가 적는 MIT 라이선스의 본문이다(`Cargo.toml`의 `[workspace.package]`, `package.json`,
  `packages/php-ext/composer.json`, `packages/ts/package.json`의 `license`).

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

## 도구

- 릴리스가 결과를 바꾸는 모든 도구는 추적 파일에 고정한다. 장비의 도구가 낸 결과는 다른 장비에서 되풀이되지 않기
  때문이다. Node.js와 npm은 `mise.toml`에 적고 `mise.lock`의 digest로 고정하고, npm은 `package.json`의
  `devEngines`에도 적고, mise는 `config/toolchain.json`에 platform별 archive의 sha256과 함께 적고, Rust는
  `rust-toolchain.toml`에 적고, CI의 PHP는 minor로 `.php-version`에 적는다. Node.js의 고정이 하나이도록 `mise.toml` 옆에 `.node-version`을 두지 않는다.
- 도구는 checkout의 `var/tools`에 설치하고, 장비에는 설치하지 않는다. 새 checkout은 `make bootstrap`(`devEngines`의
  npm이 장비의 node에서 `package-lock.json`을 설치한다. @polyspec/kit도 여기에 든다), `make install-tools TOOLS=npm`
  (kit의 bootstrap이 mise를 sha256으로 설치하고, 그 mise가 `locked = true`에서 `mise install`로 `mise.toml`의 node와
  npm을 설치한다), `make install-packages`(`var/tools`의 npm으로 `npm ci`)를 실행한다. `mise.toml`의 릴리스가 바뀌면
  `make toolchain-lock`이 `mise.lock`을 다시 쓴다.
- 시험의 Redis 서버를 시작하는 bay는 make처럼 장비가 주는 도구이고, run을 위해서는 고정하지 않는다. harness의
  결과를 바꾸는 것은 릴리스의 번호가 아니라 harness가 읽는 bay 출력의 계약(`schemaVersion`. harness가 검사한다)이고,
  개발 장비의 bay는 자주 새 빌드가 되기 때문이다. recipe는 PATH의 `bay`를 시작하고(Makefile의 `BAY`), run은
  `bay version`의 출력을 첫 줄로 쓴다. bay의 릴리스는 CI를 위해서만 `config/toolchain.json`에 고정한다(`bay`: version,
  릴리스의 주소, 플랫폼별 archive의 sha256). runner에는 bay가 없고, 받은 것은 고정한 digest로만 쓰기 때문이다.
  `make install-bay`가 그것을 `var/tools/bay`에 설치하고, `BAY_SUDO=1`이면 `BAY`는 `sudo -n` 뒤의 그 bay가 된다.
  고정 값을 올리려면 `config/toolchain.json`의 `bay`에서 `version`을 그 릴리스의 tag로, `sha256`의 각 값을 그 릴리스의
  파일 `SHA256SUMS`에 있는 해당 archive의 줄로 바꾼다(`darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`가
  archive `darwin_arm64`, `darwin_amd64`, `linux_amd64`, `linux_arm64`다). 다른 어떤 파일도 릴리스를 적지 않는다. 그런
  뒤 `make tool-pins-test`와 `make install-bay`를 실행한다. CI의 시험이 더 뒤의 릴리스를 필요로 할 때 고정 값을
  올리고, runner가 bay를 주거나 시험이 서버를 시작하지 않게 되면 고정 값을 지운다.
- recipe는 `var/tools`의 경로로 node를 시작한다(Makefile의 `NODE`). 그 node가 있기 전에 실행되는 `make bootstrap`과
  `make install-tools`만 장비의 node를 시작한다.
  Enforced by: `make tool-pins-test`.

## Rust

- toolchain은 `rust-toolchain.toml`이 고정하는 릴리스다. `make check`는 `-D warnings`를 준 clippy, rustfmt,
  시험, 계약 검사, `make docs-check`를 실행한다.
- `make docs-check`는 `config/documents.json`이 고르는 각 문서를 그 링크, anchor, fence, 각 한국어 쌍의 `doc-id`와
  `source-sha256`과 함께 읽고, `config/checklist.json`이 행을 선언하는 checklist `docs/plans/rust-core/checklist.md`를
  읽는다. 그것이 실행하는 @polyspec/kit은 `make bootstrap`이 설치한다. 저장소의 계획은 `docs/plans/rust-core/plan.md`다.
  작업은 그 checklist의 행이며, 확인 명령이 commit된 tree에서 통과한 뒤에만 `[o]`로 표시한다. commit 전에 그것을
  실행한다. 한국어 쌍을 다시 번역한 뒤에는 `make documents-stamp PATHS=<쌍>`이 그 `source-sha256`을 쓴다. marker는 손으로
  쓰지 않는다.
- workspace의 lint는 모든 경고, unsafe 코드, `unwrap`, `expect`, `as` 변환과 값을 잃는 cast, 써야 하는 값이나
  타입을 적지 않은 값의 `let _`, 이유 없는 allow attribute를 거부한다.
  `clippy.toml`은 `Option::{unwrap_or, unwrap_or_default, unwrap_or_else, map_or, map_or_else}`와
  `Result::{ok, unwrap_or, unwrap_or_default, unwrap_or_else, map_or, map_or_else}`를 금지한다. 규칙이 필요로 하는 사용에는
  `#[allow(..., reason = "<rule id>")]`를 붙인다.
- peer(PHP, Redis image를 가진 bay)가 필요한 시험은 그 peer가 주어지지 않으면 실패하고 그
  peer를 주는 것을 적는다. peer가 없다는 이유로 무시된 시험으로 보고되는 시험은 없다. 무시되는 시험은 `binding`을 가진
  group `open`의 case뿐이고, 이 case는 바인딩이 실행한다(SF-8.2, SR-8.2).
- 시험의 Redis 서버는 Makefile의 `REDIS_IMAGE`가 digest로 고정한 image의 container 안에서 돈다. 기계에 설치된
  서버는 쓰지 않는다(SE-10.4). bay가 서버를 Compose 프로젝트로 시작하고 지운다. Darwin에서는 Apple `container`로,
  Linux에서는 Docker로 실행하고, image와 digest는 둘에 하나다. 그래서 두 플랫폼이 같은 bytes의 서버를 시험한다.
  `scripts/servers.sh`가 bay의 명령을 가진 유일한 파일이고, 어떤 파일도 engine의 명령을 갖지 않는다. 어떤 시험도,
  어떤 recipe도 bay를 직접 시작하지 않고, 그 script는 `BAY`의 낱말로 bay를 시작한다. bay나 그 장비 설정의 일부가
  없으면 `make test`는 빌드 전, 시작할 때 없는 것과 그것을 주는 것을 적고 실패한다(`bay check -scope run`.
  `bay install`이 설정을 적용하고, 어떤 run도 설정을 적용하지 않는다). 어떤 run도 bay의 version을 비교하지 않는다. image는 Redis의 가장 높은 안정판의 공식 image다. `make redis-image`가 `bay pull`로 그 image를
  받고, 어떤 시험도 image를 받지 않는다. 프로젝트는 `up -no-pull`로 시작한다.
- 시험은 자기가 필요한 서버를 Compose 프로젝트 하나로 소유한다. 서버의 인자는 시험이 적는 내용의 일부이고, 서버의
  상태는 다른 시험에 닿으면 안 되기 때문이다. 시험은 프로젝트를 시작하기 전에 `compose.yaml`을
  `SESSION_STORE_PROJECTS`, 곧 run의 디렉터리 `session-store-test-<pid>` 아래 자기 디렉터리에 쓴다. 그 디렉터리의
  이름이 시험 process와 순번을 포함하며, 프로젝트의 이름이다. 모든 이름은 bay가 준다. harness는 시작하기 전에 각
  service의 host 이름과 certificate의 경로를 `bay plan --json`에서 읽고, 주소를 `bay up --json`의 출력에서 읽는다.
  그래서 한 번의 `up`이 한 시험의 모든 서버를 시작하고, 어떤 파일도 이름의 domain이나 certificate의 경로를 적지
  않으며, 어떤 port도 공개하지 않는다. TLS 서버는 bay가 발급한 certificate를 읽고, 어떤 시험도 certificate를 만들지
  않는다. `up`의 반환이 시작의 event다. `redis-cli --cluster create`처럼 시험이 container 안에서 필요로 하는 Redis의
  명령은 `bay exec`로 실행하고, 서버의 나중 보고는 `bay logs -f`의 줄로 기다리며, 시간을 기다리지 않는다.
  `scripts/test-run.mts`는 run이 끝날 때, 실패할 때, SIGINT나 SIGTERM으로 멈출 때 run의 디렉터리에 남은 모든 Compose
  파일에 `down`을 실행하고 그 디렉터리를 지운다.

## Bindings

- `make bindings`가 계약의 target마다 따로 생성한다(`make bindings-<target>`, binding GN-1b). 그래서 생성기가 거부하는
  target은 자기 make target만 실패시킨다. 바인딩을 빌드하거나 시험하는 모든 make target은 자기 target의 생성을 먼저
  실행한다. `make test-bindings`는
  workspace 전체에 clippy를 실행하고, `make check-declarations`를 실행하고, 그다음 계약의 다섯 target마다 하나씩
  `make test-python`, `make test-node`, `make test-wasm`, `make test-php`, `make test-go`를 실행한다. 각각은 binding이
  생성한 시험과 그 바인딩의 contract case를 그 언어의 표준 runner로 실행한다. runner는 unittest, node --test(Node와
  WASM, WASM 바인딩은 같은 설치 패키지의 subpath `./wasm`으로), PHPUnit, go test다. contract case는
  `packages/python/tests`, `packages/ts/tests/cases.test.mjs`, `packages/ts/tests/cases.wasm.test.mjs`,
  `packages/php-ext/tests`, `packages/go/tests`이며, `SESSION_STORE_CASES`로 디렉터리 `contracts`를 읽는다. 생성기가
  받지 못하는 target은 원인과 함께 실패하는 target으로 남는다. 어떤 target도 빠지거나 건너뛰지 않는다(binding BD-27).
- contract case runner는 아직 쓰지 않았다. 계약 검사는 finding을 보고하지 않지만 `make bindings`는 모든 target에서
  계약을 거부하므로(binding G-4), runner가 호출하고 그것으로 검증될 바인딩 API가 없다. 생성기는 타입 `cancel`인
  parameter의 computed default(`file_sessions.open`), `core_type`이 없는 record(`store_options`), 여러 요청으로 된
  group `requests`의 case(SE-10.6)를 받지 않고, Go에서는 타입 `cancel`인 parameter를 첫 parameter로만 받는다
  (T1.30). 다섯 target은 모두 그 원인으로 `make bindings`에서 실패한다.
- Go 모듈에서 binding은 파일 `zz_generated_*.go`를 소유한다(binding GN-3). Go 모듈은 소스로 받아지므로 이 파일들은
  추적되고, `make bindings`가 다시 쓴다. `packages/ts/wasm/generated`와 `packages/go/internal/ffi/generated`는 추적하지
  않는다. wasm-bindgen은 모듈과 그 glue를 앞의 디렉터리에 쓴다.
  변수 `PYTHON`(3.14 이상), `PHP`, `PHP_CONFIG`, `PHPUNIT`, `TSC`, `GO`, `GOFMT`, `WASM_BINDGEN`이 런타임과 도구를
  가리킨다. `make test-go`는 `packages/go/go.mod`의 go 지시문과 다른 판의 Go를 실패시키고, toolchain을 받지 않도록
  `GOTOOLCHAIN=local`로 실행한다. `make test-wasm`은 `packages/ts/wasm/Cargo.toml`의 `wasm-bindgen` crate와 다른 판의
  wasm-bindgen CLI를 실패시킨다. 그 CLI는 자기 판의 모듈만 읽기 때문이다(binding GN-18). `CARGOFLAGS`는 모든 cargo
  build에 인자를 더한다. macOS에서 PHP 확장과 Node addon은 자기 런타임이 그것을 로드할 때 런타임의 기호를 받는다.
  그래서 `.cargo/config.toml`이 링커에게 그 기호를 정의하지 않은 채 두게 하고, recipe는 Cargo가 만든 라이브러리를
  플랫폼의 확장자(macOS `dylib`, Linux `so`)로 가리킨다.

## 플랫폼

Linux x64와 arm64(glibc), macOS arm64다. polyspec의 Rust 코어와 바인딩이 안정되고 ext-php-rs가 고정된 stable
toolchain으로 Windows에서 빌드될 때까지 Windows는 지원하지 않는다.

## CI와 릴리스

- GitHub workflow의 모든 job은 `ubuntu-26.04-arm`에서 실행한다. 모든 action은 `.github/actions`의 local action이거나
  전체 commit으로 고정하고, 그 파일의 주석이 action의 릴리스를 적는다. action 고정은 추적 파일이 적는 유일한 Git
  commit이다. 어떤 job과 step도 `timeout-minutes`를 두지 않는다. 모든 `run` step은 `Makefile`이 정의하는 make target
  하나다. 그래야 Makefile의 설정이 CI의 모든 명령에 적용된다. `make workflows-test`는 이 규칙 중 하나를 어기는
  workflow와 action을 실패시킨다.
- workflow `ci`는 `main`의 push와 `workflow_dispatch`에서만 시작한다. job `shared`는 `make ci-shared`(Makefile의
  `CI_SHARED`: `make docs-check`, `make owner-validate`, `make workflows-test`, `make tool-pins-test`, `make servers-test`, `make install-bay-test`,
  `make commit-names-check`,
  `make commit-names-test`, `make release-coverage`)를 실행하고, job `rust`는 `make install-rust`와 `make ci-rust`(`CI_RUST`: `make lint-rust`,
  `make fmt-check`, `make binding-check`)를 실행한다. 각 job은 kit의 ci-targets로 target을 실패 뒤에도 끝까지 실행하고
  보고서 `var/report/ci-targets`를 올린다.
- workflow `ci`의 마지막 job은 `ci-passed`다. 이 job은 다른 모든 job 뒤에, 실패한 job 뒤에도 실행되고, 그 job을 모두
  `needs`에 둔다. `make ci-passed FAIL_SKIPPED=1`(kit의 ci-passed)은 각 job의 결과가 success일 때만 통과한다. 그래서
  job 하나가 run의 결과를 말한다. `make workflows-test`는 마지막 job이 다른 job이거나 `needs`가 job 하나를 빠뜨린
  workflow `ci`를 실패시킨다.
- workflow `ci`의 job `test`는 `.php-version`의 PHP를 설치하고 `make install-rust`, `make install-bay`,
  `make setup-bay`, `make redis-image`, `make test`를 실행한다. 뒤의 셋은 `BAY_SUDO=1`로 실행한다.
  `config/toolchain.json`이 CI를 위해 고정한 bay가 시험의 Redis 서버를 runner의 Docker에서 `REDIS_IMAGE`의 image로
  실행하고(SE-10.4), bay의 모든 명령은 그 상태를 가진 한 사용자인 root로 실행된다. 시험 process는 root로 실행되지
  않아 root의 home 아래에 있는 authority 파일을 읽지 못하므로, TLS의 시험은 `bay install`이 authority를 더한 시스템의
  certificate store로 그 authority를 신뢰한다. CI의 첫 run까지 미확인: run의 신호가 `sudo`가 시작한 bay에 닿는 것.
- workflow `release`는 `ci`의 run이 `main`에서 성공하면 시작하고, 그런 run의 id를 준 dispatch로도 시작한다.
  `make release-tag`, `make release-checkout`, `make release-tools`, `make release-assets`, `make release-publish`,
  `make release-undraft`를 이 순서로 실행한다.
- `config/release.json`은 저장소의 모든 manifest를 분류하고, `make release-coverage`는 분류되지 않은 manifest를
  실패시킨다. 이 파일은 package를 적지 않는다. 바인딩 package는 kit 0.0.111이 platform asset을 만들고 Go module이
  tag 형식, 곧 release commit과 `packages/go/vX.Y.Z`를 갖춘 뒤에 릴리스한다. 그때까지 릴리스는 tag `vX.Y.Z`와, 변경
  기록의 `## X.Y.Z` 절을 notes로 가진 GitHub Release다.
- 버전은 릴리스 때 만드는 commit `chore(release): Release X.Y.Z`에서만 바뀐다. 이 commit은 `Cargo.toml`의
  `[workspace.package]`와, `binding scaffold`가 그것에서 다시 쓰는 바인딩 manifest에 버전을 적고, `Cargo.lock`을
  다시 쓰고, 두 변경 기록의 `## Unreleased`를 `## X.Y.Z`로 바꾸고 그 위에 빈 `## Unreleased`를 새로 둔다. tag는
  손으로 만들지 않는다. `make release-tag RUN=<id>`가 `main`에서 성공한 workflow `ci`의 run `<id>`의 commit에, 그
  commit의 변경 기록의 첫 `## X.Y.Z` 절의 버전으로 tag를 단다. `make release-assets`, `make release-publish`,
  `make release-undraft`는 `config/release.json`의 package의 archive를 만들고 그 tag의 GitHub Release를 공개한다.

## Commit

- commit 메시지는 `<type>(<scope>): <Subject> (#id)`이고, 사실만 간결하게 적으며, 본문은 무엇이 왜 바뀌었는지를
  적는다. 서로 관련 없는 변경은 별도의 commit으로 한다.
- 어떤 추적 파일도 이 저장소나 다른 저장소의 Git commit을 적지 않는다. workflow의 action 고정만 예외다. commit의
  이름은 릴리스와 lock이 고정하고, 문서에 적은 이름은 이력이 바뀌면 틀린 값이 된다. 기록은 그 대신 사실을 적는다.
  곧 입력, 동작, 기대한 동작, 시험의 수다.
  Enforced by: `make commit-names-check`. 이 검사는 추적되는 Markdown 파일을 읽는다. 다른 추적 파일에 적힌 commit은
  검토로 찾는다. `scripts/check-commit-names.mts`는 fsm과 php-serialization이 가진 검사의 복사본이고, kit이 이
  검사를 utility로 릴리스할 때까지 둔다.
