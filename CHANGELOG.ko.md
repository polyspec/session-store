<!-- doc-id: changelog -->
<!-- source-sha256: 28b6d6ffb7f7a189c0e5511dd88f2255ce4c65e73c4595c7aa95b427a78bdeb6 -->

## Unreleased

- T1.19, 곧 files 저장소의 정적 생성(`FileSessions::in_directory`, `file_sessions.in_directory`, 생성자가 없는
  class)과 variant `setting`의 core enum이 끝났다. 이 Mac에서 이 branch의 깨끗한 tree로 `make test`는 157개가 통과하며
  status 0으로 끝났고, `make -k lint-rust fmt-check binding-check`도 status 0으로 끝났다.

- `/bin/sh`가 dash인 workflow `ci`의 runner에서 `make servers-test`의 run 시험 세 개가 표준 오류에 `scripts/test-run.sh:
  41: set: can't access tty; job control turned off`를 적고 실패했고, SIGINT나 SIGTERM이 멈추는 run의 시험은 끝나지
  않았다. dash는 terminal 없이 job control을 켜지 않으므로, run의 명령은 run의 process group에 남았고, shell의 비동기
  명령이 그렇듯 SIGINT를 무시했으며, 명령의 group에 보내는 kill은 실패했다. 그래서 run은 어떤 신호도 닿지 않은 명령을
  기다렸다. POSIX shell은 script에게 process group을 만드는 방법을 주지 않고 macOS에는 `setsid` 프로그램이 없으므로,
  run은 이제 checkout의 node가 실행하는 `scripts/test-run.mts`다. 이 script는 명령을 자기 session의 process group
  leader로 시작하고(`detached`를 준 `spawn`), SIGINT와 SIGTERM을 그 group에 전하며, 명령의 종료를 기다린다(SE-10.4).
  `scripts/test-run.sh`는 지웠고, `make test`는 `$(NODE) scripts/test-run.mts`를 시작한다. run의 시험은 자기 명령을
  `/bin/sh`로, 그리고 있으면 `/bin/dash`로 실행하며, 새 시험은 명령이 자기 process group을 이끌 것을 요구한다. 이
  Mac의 `/bin/dash`로 그 commit에서 시험이 실패했다. 신호 시험을 뺀 15개 중 12개가 통과하고 3개가 그 줄로 실패했고,
  손으로 SIGTERM을 보낸 run은 명령이 FIFO에서 읽은 뒤에야 끝났으며 `ps`는 명령이 run의 group에 있음을 보였다. 변경
  뒤에 `make servers-test`는 17개가 통과한다. 미실행: 변경 뒤의 Linux 시험과 workflow.

- `config/toolchain.json`은 CI가 설치하는 릴리스로 bay v0.0.7을, 그 릴리스의 `SHA256SUMS`에서 받은 archive별 sha256과
  함께 고정한다. 이 Mac의 bay가 v0.0.7이고, CI는 여기의 run이 시작하는 릴리스를 실행하기 때문이다. runner는 매 run의
  `make setup-bay`로 그 릴리스의 설정을 적용하며, 그 agent unit은 v0.0.6의 것과 다르다. 이 Mac에서 `make install-bay`는
  darwin-arm64의 archive를 받고 그 sha256을 받아들여 bay v0.0.7을 설치한다. `make tool-pins-test`는 9개,
  `make install-bay-test`는 4개가 통과한다. 미실행: Linux에서의 설치, 이 릴리스로 실행하는 workflow job `test`.

- workflow `ci`의 runner에서 `make redis-image`가 `bay: compose.yaml: project name "tmp.x4bmuo3xsw" must be
  letters, digits, - or _`로 실패했고, 이 Mac의 bay도 같은 파일을 같은 방법으로 거부한다. `scripts/servers.sh
  pull`은 받기 위한 Compose 파일을 template 없이 `mktemp -d`가 주는 디렉터리 `tmp.<글자>`에 썼고, bay는 프로젝트의
  이름을 그 디렉터리에서 받으며 그 이름의 점을 거부한다. 이제 pull은 `TMPDIR` 아래 디렉터리
  `session-store-pull-<글자>`에 쓴다(SE-10.4). pull의 시험은 그 이름을 요구한다. 그 commit에서 `make servers-test`는
  15개 통과, 1개 실패였고(이름 `tmp.4hb9OWSrEy`), 변경 뒤에는 16개가 통과하며, 이 Mac의 `bay plan --json`은 그런
  디렉터리의 Compose 파일을 받아들인다. 미실행: image를 내려받는 pull 자체, 변경 뒤의 workflow job `test`.

- 어떤 파일도 bay의 릴리스를 적지 않았고, workflow `ci`는 bay를 설치하지 않아 job `test`가 `make test`를 시작할 때
  실패했으며, `scripts/servers.sh`는 PATH의 bay만 시작했다. bay는 make처럼 장비가 주는 도구다. run은 PATH의 `bay`를
  시작하고, version을 비교하지 않고, `bay version`의 출력을 첫 줄로 쓰며, harness는 `schemaVersion`이 1이 아닌 bay의
  문서를 실패시킨다. harness의 결과를 바꾸는 것은 harness가 읽는 출력의 계약이고 릴리스의 번호가 아니기 때문이다.
  `config/toolchain.json`은 CI가 설치하는 릴리스인 bay v0.0.6을 플랫폼별 archive의 sha256과 함께 고정한다. runner에는
  bay가 없고, 받은 것은 고정한 digest로만 쓰기 때문이다. `make install-bay`(`scripts/install-bay.mts`)는 그 플랫폼의
  archive를 받고, 다른 sha256을 거부하며, `bay`와 `bayd`를 `var/tools/bay`에 둔다. run의 어떤 경로도 그 고정 값을
  읽지 않는다. Makefile의 `BAY`가 bay를 시작하는 방법을 정하는 값 하나이고, `scripts/servers.sh`만 그 값을 읽는다.
  기본값은 `bay`이고, `BAY_SUDO=1`이면 `sudo -n` 뒤의 `var/tools/bay`의 bay다. Linux에서는 bay의 설정과 명령이 root로
  실행되고 bay는 자기 상태를 실행한 사용자의 home 아래에 두기 때문이다. job `test`는 `make install-bay`를 실행한 뒤
  `make setup-bay`(`bay install`. 장비의 설정을 바꾸는 target은 이것 하나다), `make redis-image`, `make test`를
  `BAY_SUDO=1`로 실행한다. `make servers-test`는 16개, `make install-bay-test`는 4개, `make tool-pins-test`는 9개,
  `make workflows-test`는 6개가 통과한다. 각 script를 바꾸기 전에는 그 commit의 시험이 실패했다. 이 Mac에서
  `make install-bay`는 v0.0.6을 설치하고 그 version 명령은 v0.0.6을 주며, `scripts/test-run.sh true`는 bay의 version
  줄을 쓰고 status 0으로 끝난다. 이 Mac에서, CI의 릴리스를 v0.0.2 위로 올리기 전인 이 변경의 깨끗한
  tree에서 `make test`가 세 번 연속 status 0으로 끝났다. 서버의 모든 시험이 병렬 실행 그대로 통과했고, run 뒤에 bay의
  container와 프로젝트가 남지 않았다. `make -k lint-rust fmt-check`도 status 0으로 끝났다. 로그는 추적하지 않는 실행
  기록에 있다. 그 tree 뒤에 바뀐 것은 CI가 설치하는 릴리스의 값과 문서뿐이고, 어떤 run도 그 값을 읽지 않는다.
  이 Mac의 bay v0.0.6에서 `make test`는 다섯 번 중 한 번 status 2로 끝났다. TLS의 시험이 저장소의 말,
  곧 the connection to Redis failed: I/O error만 적고 실패했고, 같은 방법으로 손으로 시작한 TLS 서버는 certificate를
  검증한 client 32개 모두에 응답했다. 원인은 시험 binary에 있었다. `tests/server.rs`의 TLS 시도가 같은 process의 다른
  시도 네 개가 실행되는 동안 `set_var`로 `SSL_CERT_FILE`에 bay의 authority를 적었고, 그 시도들은 프로그램을 시작하고
  이름을 풀 때 환경을 읽으므로 그 옆의 쓰기는 정의되지 않는다. SE-10.4는 시도가 병렬로 실행되는 process의 환경을
  어떤 시험도 바꾸지 않는다고 적는다. 이제 `tests/tls.rs`는 그 시도 하나만 가진 별도의 시험 binary다. 그 `main`은
  프로젝트의 plan을 읽고 둘째 thread가 시작하기 전에 `SSL_CERT_FILE`에 authority 파일을 적으며, 시도는 client가
  실패하면 서버가 쓴 것을 실패에 덧붙인다(`scripts/servers.sh output`). `tests/server.rs`는 나머지 시도 네 개를 갖고
  환경을 바꾸지 않는다. 이를 요구하는 `scripts/servers.test.mts`의 시험은 그 commit에서 실패했고(`make
  servers-test`: 15개 통과, 1개 실패) 변경 뒤에 통과한다(16개 통과). 변경의 깨끗한 tree에서 `make test`는 열 번
  연속 status 0으로 끝났고, 각 run에서 TLS의 시험을 포함한 157개가 통과하고 실패는 없었다. 미실행: `make setup-bay`, Linux에서의 시험, workflow. CI의 첫 run까지 미확인:
  runner에서 `bay install`이 끝나는 것, root가 실행한 bay의 authority 파일을 시험 process가 읽는 것, run의 신호가
  `sudo`가 시작한 bay에 닿는 것.

- 시험의 Redis 서버 harness는 두 engine이 서버를 실행하는 명령을 갖고 있었고(`scripts/container.sh`: Darwin의 Apple
  `container`와 Linux의 `docker`), 서버의 주소를 그 명령이 쓴 줄에서 받았고, TLS의 certificate를 `openssl`로 만들었고,
  `redis-cli`를 별도의 container에서 실행했다. SE-10.4는 두 플랫폼에서 bay가 서버를 Compose 프로젝트로 시작하고
  지우며 모든 이름을 준다고 적는다. 시험은 프로젝트 하나를 소유한다. 시험은 시작하기 전에 `compose.yaml`을
  `SESSION_STORE_PROJECTS`, 곧 run의 디렉터리 `session-store-test-<pid>` 아래 자기 디렉터리에 쓴다. 그 파일은 digest로
  적은 image, `command`로 적은 `redis-server`의 인자, `expose`의 port를 적고, port를 공개하지 않는다. 그 디렉터리의
  이름이 프로젝트의 이름이다. 서버의 인자와 상태는 한 시험의 것이므로 시험끼리 서버를 함께 쓰지 않는다. harness는 각
  service의 host 이름과 certificate의 경로를 `bay plan --json`에서 읽고, 한 번의 `up -no-pull --json`으로 한 시험의
  모든 서버를 시작하고, 그 출력에서 주소를 읽는다. 그래서 어떤 파일도 이름의 domain이나 certificate의 경로를 적지
  않는다. harness는 `schemaVersion`이 1이 아닌 문서를 실패시킨다. TLS 서버는 bay가 자기 이름에 발급한 certificate를
  읽고, 시험은 plan이 적는 authority 파일을 신뢰한다. `make test`는 `OPENSSL`을 넘기지 않는다. Sentinel의 설정은
  plan의 이름으로 primary를 적는다. cluster의 시험은 `bay exec`로 첫 node의 container에서
  `redis-cli --cluster create`를 실행하고, 각 node의 state ok를 `bay logs -f`의 출력에서 기다린다.
  `scripts/servers.sh`가 `scripts/container.sh`를 대신하고 bay의 명령만 갖는다(`check`, `pull`, `plan`, `up`, `images`,
  `exec`, `logs`, `output`, `down`). `make redis-image`는 `bay pull`로 받는다. `scripts/test-run.sh`는 bay나 프로젝트에 필요한
  장비 설정의 일부가 없으면(`bay check -scope run`) 빌드 전에 실패하고, run이 끝날 때, 실패할 때, SIGINT나 SIGTERM으로
  멈출 때 자기 디렉터리에 남은 모든 Compose 파일에 `down`을 실행하며, 프로젝트를 지우지 못한 run을 실패시킨다.
  `make servers-test`가 `make container-test`를 대신한다. bay의 대역으로 장비의 Node.js에서 `node --test`를 실행하면
  engine의 명령을 가진 script로는 0개가 통과하고 13개가 실패했고, 이 변경의 script로는 13개가 통과하고 0개가
  실패했다. 이 Mac에서 `scripts/servers.sh check`와 `scripts/test-run.sh true`는 status 0으로 끝난다. 이 Mac의 bay에서
  고정된 image의 임시 프로젝트로 관찰한 것: 두 프로젝트가 동시에 시작한다. 한 번의 `up -no-pull --json`이 bay의
  certificate를 쓰는 TLS 서버, primary와 그 Sentinel, node 셋을 시작하고, plan의 authority 파일을 신뢰하는 client가
  TLS 서버의 이름에서 `+PONG`을 읽는다. `bay exec`는 표준 입력이 `/dev/null`이어도 `redis-cli --cluster create`를
  status 0으로 실행하고, 그 직후 한 node는 state ok의 줄을 아직 쓰지 않았다. 이름으로 만든 cluster의 node는 IPv6
  주소를 알린다. SIGTERM이 멈춘 `up`은 status 143으로 끝나고 container도 프로젝트도 남기지 않는다. port가 없고 이미
  끝난 service가 있는 `up`은 status 0으로 끝난다. 끝난 service의 `logs -f`는 끝난다. 설치되지 않은 image의
  `up -no-pull`은 실패하고 `bay pull`을 적으며 프로젝트를 남기지 않는다. `make servers-test`, `make docs-check`, `make lint-rust`, `make test`는 위
  항목의 변경과 함께 실행했고, 그 결과는 위 항목이 적는다.

- 계약은 생성자 `file_sessions.new`를 wait `blocking_io`로 적었고, variant `setting`을 코어 enum 없이 적었다.
  `binding generate`는 둘을 모든 target에서 거부했다. 생성자를 await할 수 없는 언어에는 기다리는 생성자의 꼴이
  없고(binding GN-6m), variant는 `core_type`에 자기 Rust enum을 적기 때문이다(binding T-9, GN-6v). SF-1.5가 파일
  저장소를 만드는 일을 `FileSessions::new`를 대신하는 Rust static 함수 `FileSessions::in_directory`와, 저장소를
  돌려주는 계약의 static member `file_sessions.in_directory`로 적는다. 클래스 `file_sessions`에는 생성자가
  없다(`constructible: false`). `contracts/file-sessions.json`에서 저장소를 만드는 instance 값 여섯 개가 그 member를
  적고, variant `setting`은 `polyspec_session_store::Setting`을 적는다. `packages/file/rust/tests/contract.rs`의 trial
  두 개가 계약을 읽는다. 기다리는 member는 생성자도 property도 아니고, `file_sessions.in_directory`는 저장소를
  돌려주는 static member다. 이 규칙을 가진 binding의 `binding check`는 계약의 발견을 보고하지 않고,
  `binding generate`는 두 항목을 더는 거부하지 않는다. 생성기가 아직 쓰지 않는 것은 계속 거부한다. 모든 target의
  `file_sessions.open`(wait `process_wait`), 그리고 Go와 WASM에서 세션 클래스의 wait, cleanup, receiver `self`
  메서드다. `make fmt-check`와 `make docs-check`는 통과한다. 실행하지 않음: `make test`, `make lint-rust`. 그래서 바뀐
  코어와 그 시험은 아직 compile되지 않았다.

- Redis 저장소의 시험은 macOS에서만 실행됐다. SE-10.4와 harness가 Apple `container`를 적었고, 그래서 workflow
  `ci`는 시험을 실행하지 않았다. SE-10.4는 container 프로그램을 플랫폼으로 정한다. Darwin은 Apple `container`,
  Linux는 `docker`이고, image는 `REDIS_IMAGE`가 digest로 고정한 하나다. 그래서 두 플랫폼이 같은 bytes의 서버를
  시험하고, 어떤 시험도 장비의 서버를 실행하지 않는다. `scripts/container.sh`가 두 프로그램의 명령(`program`,
  `pull`, `present`, `run`, `delete`, `list`)을 가진 유일한 파일이다. 이 script는 프로그램을 `uname -s`만으로
  정하고, PATH에 없는 프로그램은 그 이름과 설치 방법을 적고 즉시 실패한다. harness
  `packages/redis/rust/tests/servers`는 그 script를 시작하고, `scripts/test-run.sh`는 그 script로 자기 run의
  container를 나열하고 지우며 프로그램을 찾지 못하면 시험 전에 실패하고, `make redis-image`는 그 script로 image를
  받는다. Makefile의 변수 `CONTAINER`는 지웠다. container의 주소는 두 프로그램 모두에서 그 명령이 쓰는 첫
  줄(`hostname -i`)이고, 시작은 전과 같이 서버가 쓰는 줄을 기다리며 시간을 기다리지 않는다. workflow의 job
  `test`는 `.php-version`의 PHP(8.4)를 설치하고 `make install-rust`, `make redis-image`, `make test`를 실행하며,
  `ci-passed`가 이 job을 needs에 둔다. `make container-test`는 `uname`, `docker`, `container`의 대역으로 script를
  실행한다. script가 없을 때는 0개 통과, 4개 실패였고, 있으면 4개 통과, 0개 실패다. `make ci-shared`는 target 8개를
  통과하고 `make fmt-check`가 통과한다. 이 Mac에서 script는 `container`를 적고, 고정한 image를 찾고,
  `scripts/test-run.sh true`는 상태 0으로 끝난다. 미실행: `make test`와 `make lint-rust`. 그래서 바뀐 harness는 아직
  compile하지 않았다. runner `ubuntu-26.04-arm`에 `docker`가 있다는 것을 보인 CI run도 아직 없다.

- `.node-version` 하나가 CI의 Node.js를 적었고, 개발 장비는 자기 Node.js를 썼고, 어떤 파일도 도구의 digest를
  고정하지 않았다. 도구를 다른 polyspec 저장소의 형식으로 고정한다. `mise.toml`은 Node.js 26.12.0과 npm 12.2.0을
  적고, `mise.lock`은 Node.js를 platform 6개의 archive checksum으로, npm을 lock project
  `.mise/locks/npm-npm/12.2.0`의 digest로 고정하고, `config/toolchain.json`은 mise 2026.10.8을 platform 4개의
  archive sha256과 함께 적는다. `make install-tools TOOLS=npm`이 이 도구를 `var/tools`에 설치하고,
  `make install-packages`가 그 npm으로 `npm ci`를 실행하고, `make toolchain-lock`이 `mise.lock`을 쓴다. Makefile의
  `NODE`는 `var/tools/bin/node`이고, Node package의 recipe는 그 node와 그 npm을 시작한다. bootstrap action은
  `mise.toml`을 읽고 `make bootstrap`, `make install-tools TOOLS=npm`, `make install-packages`를 실행한다.
  `.node-version`은 지웠다. `make tool-pins-test`는 추적 파일에서 고정을 검사한다. 고정이 없을 때는 3개 통과, 5개
  실패였고, 있으면 8개 통과, 0개 실패다. `make ci-shared`는 `var/tools`의 node 26.12.0에서 target 7개를 통과한다.
  `config/release.json`은 mise의 lock project의 manifest를 릴리스하지 않는 것으로 분류한다.

- 추적되는 문서에서 Git commit의 이름을 읽는 검사가 없었다. 그 이름은 이력이 바뀌면 틀린 값이 된다.
  `make commit-names-check`는 `scripts/check-commit-names.mts`를 실행한다. 이 검사는 추적되는 Markdown 파일에서
  숫자와 글자를 모두 가진 소문자 16진수 7~40자리 단어마다 `<file>:<line>: <name>`으로 실패한다. 64자리 digest, 숫자,
  경로, 식별자는 commit 이름이 아니다. `make commit-names-test`는 3개 통과, 0개 실패다. 검사가 있기 전에는 실행된
  시험 없이 끝났다. 이 tree에서 검사는 이름을 찾지 않는다. 둘 다 target 6개를 실행하는 `make ci-shared`와
  `make check`의 target이다. 이 검사는 fsm과 php-serialization이 가진 검사의 복사본이고, kit이 이 검사를 utility로
  릴리스하면 지운다.

- workflow `ci`의 어떤 job도 run의 결과를 말하지 않았다. 실패한 job이 있는 run에서 읽을 job 하나가 없었다. job
  `ci-passed`가 workflow의 마지막 job이다. 이 job은 다른 모든 job 뒤에, 실패한 job 뒤에도 실행되고
  (`if: ${{ always() }}`), `shared`와 `rust`를 `needs`에 두고, 자기 needs의 JSON인 `RESULTS`와 함께
  `make ci-passed FAIL_SKIPPED=1`을 실행한다. kit의 ci-passed가 그 값을 판정한다. 두 결과가 `success`이면 target은
  상태 0으로 끝난다. 하나가 `failure`이거나 `skipped`이면 실패하고 그 job을 적는다. `RESULTS`가 없으면 판정한 것이
  없으므로 실패한다. workflow 시험은 마지막 job이 `ci-passed`가 아닌 workflow `ci`, 조건이 다른 `ci-passed`,
  `needs`가 다른 모든 job이 아닌 것, `RESULTS`나 `FAIL_SKIPPED=1`을 넘기지 않는 것을 실패시킨다. 이 job이 있기
  전에 `make workflows-test`는 5개 통과, 1개 실패였고(`the last job is rust, not ci-passed`), job이 있으면 6개 통과,
  0개 실패다.

- manifest는 라이선스 `MIT`를 적었고 저장소에는 그 본문이 없었다. `LICENSE`는 MIT 라이선스의 본문을 polyspec
  저장소의 저작권 줄 `Copyright (c) 2026 Polyspec`과 함께 둔다. `config/owner-checks.json`이 이 경로를 소유한다.
  파일이 있고 그 규칙이 없으면 `make owner-validate`는 `LICENSE: the path matches no owner`로 실패하고, 규칙이
  있으면 추적되는 경로 83개를 통과한다.

- 추적되는 경로를 소유하는 검사를 적은 파일이 없었다. `config/owner-checks.json`은 추적되는 경로 82개 각각에 make
  target을 준다. 문서는 `make docs-check`, 라이브러리의 crate와 case는 `make lint-rust`, `make fmt-check`,
  `make test`, 바인딩 package는 `make binding-check`, workflow는 `make workflows-test`, manifest는
  `make release-coverage`다. `make owner-validate`는 kit의 owner-check를 실행하고 통과한다. `.node-version`의 규칙을
  빼고 Makefile이 정의하지 않는 target을 적으면 두 오류로 실패한다. 이 target은 target 4개를 통과하는
  `make ci-shared`와 `make check`의 target이다. `make owner-check`는 `PATHS`가 지정하는 경로의 소유자를 실행한다.

- 저장소에 GitHub workflow가 없어서 `make workflows-test`가 실패했다. `.github/workflows/ci.yml`이 추적되지 않았다
  (4 passed, 1 failed). workflow `ci`는 `main`의 push와 `workflow_dispatch`에서 실행한다. job `shared`는
  `make ci-shared`(`make docs-check`, `make workflows-test`, `make release-coverage`)를, job `rust`는
  `make install-rust`와 `make ci-rust`(`make lint-rust`, `make fmt-check`, `make binding-check`)를 실행한다. workflow
  `release`는 성공한 `ci` run의 commit에 tag를 달고 Makefile의 릴리스 target으로 그 GitHub Release를 공개한다.
  `.github/actions/bootstrap`은 `.node-version`의 Node.js 26.12.0을 설치하고 `make bootstrap`을 실행한다. 모든 job은
  `ubuntu-26.04-arm`에서 실행하고, 모든 action은 commit으로 고정한다. `make test`는 CI의 target이 아니다. Redis
  저장소의 시험에는 Apple container가 필요한데 Linux runner에는 그것이 없다. 시험은 dispatch의 inputs의 key `run`을
  run step이 아니라 mapping으로 읽는다. `make workflows-test`는 5 passed, 0 failed를 내고, `make ci-shared`는 target
  3개를 통과한다. GitHub에서 실행된 workflow는 아직 없다.

- 저장소의 manifest를 릴리스에 대해 분류하는 파일이 없었고, `Makefile`에는 릴리스 target이 없었다.
  `config/release.json`은 변경 기록을 적고 manifest 15개를 각각 이유와 함께 릴리스하지 않는 것으로 분류한다. 바인딩
  package는 platform asset을 만드는 kit 0.0.111과 Go module의 tag 형식, 곧 release commit과 `packages/go/vX.Y.Z`를
  기다린다. `make release-coverage`는 kit의 릴리스 유틸리티의 coverage를 실행하고 통과한다. 파일에서
  `packages/go/go.mod`와 `packages/ts/package.json`을 빼면 실패하고 둘을 적는다. `make release-tag`,
  `make release-checkout`, `make release-tools`, `make release-assets`, `make release-publish`, `make release-undraft`는
  kit의 릴리스 유틸리티를 실행한다. `RUN` 없는 `make release-tag`는 실패하고 `RUN`을 적는다. `make check`는
  `make workflows-test`와 `make release-coverage`도 실행한다.

- checklist `docs/plans/rust-core/checklist.md`에는 릴리스의 구조에 대한 행이 없었다. 행 T1.10부터 T1.14는 GitHub
  workflow와 그 규칙의 시험, manifest의 릴리스 분류, 추적되는 경로의 소유자, 라이선스 본문, 0.0.1 릴리스를 적고, 계획은
  그것을 여섯째 단계로 적는다. T1.13은 라이선스 본문을, T1.14는 origin의 branch `main`을 기다린다. `make docs-check`는
  통과한다: 문서 쌍 6개, checklist 1개, 변경 기록 1개.

- `docs/plans/rust-core/checklist.md`의 행과 이 변경 기록의 두 항목이 Git commit을 적고 있었다. 추적되는 파일은 Git commit을
  적지 않는다(AGENTS.md, "Commits"). 이제 행은 자기 task만 적고, 두 항목은 commit을 적지 않는다.

- `contracts/redis-sessions.json`의 group `requests`의 case 키 `textReplies`와 `errorReplies`는 snake_case가 아니었다.
  그래서 binding의 end-to-end 검사가 모든 target에서 case 파일을 거부했다(G-5). 이제 그것은 Redis 저장소의 시험이 읽는
  `text_replies`와 `error_replies`이고, 코어 시험의 trial `convention::case_keys`는 인터페이스 계약의 `cases`가 가리키는
  group의 case 키가 모두 snake_case이기를 요구한다.

- `Cargo.lock`은 binding의 더 새로운 commit을 고정하고, `binding scaffold`가 바인딩 패키지를 다시 쓴다. 바인딩 crate의 allow는
  자기 규칙 id를 이유로 주고, Go crate의 manifest에는 새 줄이 있다. 그 binding의 검사는 fault 주입점을 가지면서 자기
  시험이 feature `fault-test`로 그것을 받지 않는 코어를 실패시킨다(CK-65, GN-5). 그래서 코어, 파일 저장소, Redis
  저장소처럼 `packages/rust/Cargo.toml`이 라이브러리 자신을 `fault-test`와 함께 dev-dependency로 적는다.
  `make binding-check`는 통과하고, `make test`는 통과 154, 실패 0, 무시 13을 낸다.

- 저장소에는 계획도 checklist도 문서 검사도 없었다. `docs/plans/rust-core`가 계획과 checklist를 한국어 쌍과 함께
  담는다. 바인딩과 그 end-to-end 검사와 Go 바인딩은 binding을 기다린다. binding이 모든
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
