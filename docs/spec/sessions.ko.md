<!-- doc-id: sessions -->
<!-- source-sha256: cc0b1f1c9f758fff1990d3ff6934b4a7e1b6347ad717120aa96060b2236309b8 -->
# session-store: 규칙

이 문서는 session-store의 명세다. 규칙은 목록 항목 `**SE-n**`(저장소와 그 레코드, crate
`polyspec-session-store-core`), `**SF-n**`(파일 저장소, crate `polyspec-session-store-file`) 또는 `**SR-n**`(Redis
저장소, crate `polyspec-session-store-redis`)이다. 규칙의 문장은 anchor `SE-n.m`을 갖고, 시험은 자신이 검사하는
문장을 적는다. PHP 값과 그 serialize 형식은 php-serialization의 것이다
(`polyspec-php-serialization`, 규칙 RQ-n). phpredis는 PHP 확장 phpredis 6.3.0이고, "PHP"는 PHP 8.4다.

## 범위

- **SE-1** <a id="SE-1.1"></a> session-store는 브라우저 세션의 값을 저장한다: 세션 식별자를 만들고 받아들이며,
  요청 하나의 세션을 열고, 잠그고, 읽고, 쓰고, 닫는 일을 이 순서로 하고, 세션을 새 식별자 아래로 갱신하고,
  수명이 지난 세션을 끝내고, 끝난 세션을 삭제한다. <a id="SE-1.2"></a> session-store는 세션에 무엇이 담기는지
  모른다: key는 텍스트이고 값은 PHP 값이며(RQ-008), 어떤 key와 어떤 값도 라이브러리에게 뜻이 없다.
  애플리케이션이 어떤 key를 쓰는지, 그 값이 무엇을 뜻하는지, 그 값이 다른 값 타입(예: template 언어의
  데이터 모델)과 어떤 관계인지는 이 라이브러리의 규칙이 아니라 애플리케이션의 규칙이다.
  <a id="SE-1.3"></a> 모든 규칙은 `packages/core/rust`, `packages/file/rust`,
  `packages/redis/rust`의 Rust crate에 있다. 바인딩은 타입을 변환하고 규칙을 갖지 않는다. `packages/rust`의 crate
  `polyspec-session-store`가 라이브러리다: 이 crate는 세 crate의 항목을 다시 내보내고, 저장소는 기본으로 빌드하는
  feature `file`과 `redis` 아래에 두며, 그 package는 바인딩의 하나뿐인 인터페이스 계약을 담는다. 그래서 모든
  언어에 session-store의 package가 하나씩 있다. <a id="SE-1.4"></a> `polyspec-session-store-core`는 I/O를 하지 않는다:
  파일, 시계, 환경, 네트워크를 읽지 않는다. 이 crate의 유일한 외부 원천은 운영체제의 난수원이다
  (SE-5.3). 다른 쪽이 예측할 수 있는 식별자는 다른 브라우저의 세션을 가리키기 때문이다.
  <a id="SE-1.5"></a> 세 crate는 `polyspec-php-serialization`과 `polyspec-binding-core`에 의존하고, 다른 값 타입의
  라이브러리에는 의존하지 않는다.

## 저장소 인터페이스

- **SE-2** <a id="SE-2.1"></a> 저장소는 요청 하나를 위해 세션 하나의 값을 갖는다. 저장소에는 연산이 넷 있다: key의
  `get`은 그 key가 가진 값을 돌려주고, key에 값이 없으면 아무것도 돌려주지 않는다. key와 값의 `set`은
  그 key가 그 값을 갖게 한다. key의 `remove`는 그 key가 값을 갖지 않게 한다. `renew`는 값을 새 세션
  식별자로 옮긴다(SE-7). <a id="SE-2.2"></a> 값을 가진 key의 `set`은 그 값을 바꾸고, 값이 없는 key의 `set`은
  그 key를 더한다. <a id="SE-2.3"></a> 값이 없는 key의 `remove`는 성공하고 아무것도 바꾸지 않는다.
  <a id="SE-2.4"></a> 값은 최상위 값이 reference가 아닌 PHP 값이다(RQ-008). `get`은 호출자가 소유하는 값을
  주므로 그 값을 바꿔도 저장소는 바뀌지 않고, `set`은 복사본을 받는다. <a id="SE-2.5"></a> key는 텍스트다
  (UTF-8). PHP가 두 key를 하나의 배열 key로 저장하면 두 key는 한 값을 가리킨다: 64비트 정수의 canonical 형식인
  텍스트는 그 정수를 가리키므로(RQ-004), `"5"`와 레코드의 정수 key 5는 하나의 key다.
- **SE-3** <a id="SE-3.1"></a> 저장소 인터페이스는 public API이므로 애플리케이션은 자기 언어로 저장소를 구현할 수
  있다. 바인딩은 그런 저장소를 Rust trait `Store`의 구현으로 감싸고 그 member 넷을 호출한다
  (binding 설계 U-3). <a id="SE-3.2"></a> 그런 저장소가 일으키는 예외는 이 라이브러리의 오류가 아니다: 예외는
  연산을 멈추고, 코어를 바뀌지 않은 채 지나 같은 객체로 호출자에게 도달한다. 라이브러리는 예외를
  감싸지도 바꾸지도 않는다. Rust 코드가 구현한 저장소는 자신의 오류를 같은 방식으로 돌려주고
  (binding-core의 `Failure::Callback`), 연산은 그 오류를 바꾸지 않고 그 Rust 호출자에게 돌려준다. <a id="SE-3.3"></a> 그런 저장소는 SE-2를 따라야 한다. 그 저장소의 잠금(SE-8), 다른
  프로세스와의 공유, 다른 서버와의 상호 운용은 애플리케이션의 책임이며, 라이브러리는 그중 어느 것도
  검사하지 않는다. <a id="SE-3.4"></a> null이 값의 부재이면서 PHP 값 null이기도 한 언어(PHP,
  Python, Node)에서는 값이 없는 key의 `get`과 null을 가진 key의 `get`이 같은 언어 값을 준다. Rust
  API는 둘을 구분하고(`Option`), 레코드는 null을 가진 key를 유지한다(SE-6.6).
- **SE-4** <a id="SE-4.1"></a> 메모리 저장소는 세션 하나의 값을 프로세스 하나의 메모리에 둔다. 시험과, 세션이
  프로세스만큼 사는 애플리케이션을 위한 것이다. <a id="SE-4.2"></a> 메모리 저장소는 key의 순서를
  SE-2.2가 적는 대로 유지한다. <a id="SE-4.3"></a> 그 `renew`는 값을 바꾸지 않고 갱신 횟수를 세며, `renewals`가 그 횟수를
  돌려준다. 메모리 저장소에는 식별자가 없기 때문이다.

## 식별자

- **SE-5** <a id="SE-5.1"></a> 세션 식별자는 22자부터 256자까지의 텍스트이고, 각 문자는 `A`–`Z`, `a`–`z`, `0`–`9`,
  `,`, `-` 중 하나다. 이 문자와 길이는 PHP가 자신의 세션 handler를 위해 만드는 식별자의 것이므로
  (`session.sid_length` 22부터 256), PHP가 만든 모든 식별자가 받아들여진다. <a id="SE-5.2"></a>
  `accept`는 세션 쿠키 값의 바이트를 받아 식별자를 주고, 식별자가 아닌 바이트에는 아무것도 주지 않는다.
  식별자가 아니라는 것이 부재의 유일한 이유다. 식별자가 아닌 값은 어떤 세션도 가리키지 않고 아무것도
  실패시키지 않는다. 쿠키는 브라우저의 입력이기 때문이다. <a id="SE-5.3"></a> 새 식별자는 운영체제의
  난수원(`getrandom`)의 32바이트를 소문자 16진수 64자리로 적은 것이다. <a id="SE-5.4"></a> 난수원이
  실패하면 연산은 kind `random`으로 실패한다. 다른 원천이 난수원을 대신하지 않는다.

## 레코드

- **SE-6** <a id="SE-6.1"></a> 세션의 레코드는 그 값의 바이트다: key가 세션의 key이고 값이 그 key의 값인
  PHP 배열 하나의 serialize 형식이다(RQ-001). 이것은 PHP 세션 모듈의 `php_serialize`
  형식이다. <a id="SE-6.2"></a> 바이트가 없는 레코드는 값이 없는 세션이다.
  <a id="SE-6.3"></a> unserialize가 거부하는 바이트는 kind `record`로 실패하고, 그 source는 kind와 offset을 가진
  php-serialization의 오류다. <a id="SE-6.4"></a> 최상위 값이 배열이 아닌 레코드는
  kind `record_not_array`로 실패한다. 메시지는 식별자를 적는다: `the session of <id> holds no array of values`.
  <a id="SE-6.5"></a> element가 reference인 key의 `get`은 그 reference가 가진 값을 준다.
  저장소의 값은 reference가 아니기 때문이다(SE-2.4). 그 값이 레코드의 다른 element와 공유하는 객체나 reference는
  내준 값에서 복사본이다. <a id="SE-6.6"></a> 요청이 쓰는 레코드는 모든 key를 배열의 순서대로
  갖는다: 요청이 set하지 않은 key는 읽은 그대로의 값을, 그런 다른 key와 공유하는 객체와 reference와 함께
  갖고, null을 가진 key는 유지된다. 문자열이 UTF-8이 아닌 레코드의 key는 그대로
  유지되며, 어떤 텍스트도 그 key를 가리키지 않는다(SE-2.5). <a id="SE-6.7"></a> set한 key는 요청이 준 값을 갖고,
  그 값의 객체와 reference는 그 값 안에서만 공유된다. <a id="SE-6.8"></a> php-serialization이 쓰는 레코드가
  없는 값, 예를 들어 레코드가 RQ-006의 깊이를 넘을 만큼 깊게 중첩된 값은 세션의
  `finish`를 kind `record_write`로 실패시키고, 그 source는 php-serialization의 오류다.

## 요청의 세션

- **SE-7** <a id="SE-7.1"></a> 레코드 세션은 레코드를 두는 저장소(SE-6), 예를 들어 파일 저장소와 Redis 저장소의
  요청 하나의 저장소다. 레코드 세션은 I/O를 하지 않는다. 그 저장소는 자신이 찾은 것으로 레코드 세션을 연다: 기존
  세션의 식별자와 레코드, 또는 아무것도 없음이다. <a id="SE-7.2"></a> 세션은 첫 `get`, `set`,
  `remove` 또는 `renew`에서 시작하므로, 세션을 한 번도 쓰지 않는 요청은 세션을 만들지 않는다. 기존 세션을 시작하면 그
  레코드를 decode하고(SE-6), 실패하는 레코드는 그 연산과 그 뒤의 모든 연산을 같은 kind로 실패시킨다. 기존 세션
  없이 시작하면 새 식별자(SE-5.3)와 값이 없는 세션을 만든다. <a id="SE-7.3"></a> `renew`는 세션을
  시작하고 세션에 새 식별자를 준다. 세션이 찾아졌을 때 가졌던 식별자가 close가 삭제하는
  식별자다. 두 번째 `renew`는 식별자를 다시 바꾸고, 여전히 찾은 식별자만 삭제한다.
  <a id="SE-7.4"></a> `finish`는 레코드 세션을 끝내고 그 저장소가 쓸 것을 준다: 한 번도 시작하지 않은 세션에는
  `unused`를 준다. 값의 레코드가 읽은 레코드와 바이트 단위로 같고 식별자가 바뀌지 않은 기존
  세션에는 식별자와 함께 `unchanged`를 준다. 그 밖에는 식별자, 레코드, 식별자가 새것인지 여부,
  삭제할 식별자(갱신일 때)와 함께 `write`를 준다. <a id="SE-7.5"></a> `finish` 뒤에는
  레코드 세션이 없다. 바인딩은 그 뒤의 호출을 kind `consumed`로 거부한다(binding 설계 E-1a).
  <a id="SE-7.6"></a> 새 식별자를 쓰는 저장소는 그 식별자를 호출자에게 주고, 호출자는 그것을 세션 쿠키로 설정한다.
  그 쿠키의 이름과 속성은 애플리케이션의 규칙이다.
- **SE-8** <a id="SE-8.1"></a> 세션 하나의 요청은 하나씩 차례로 실행된다: 레코드를 두는 저장소는 요청의 세션을
  열 때부터 닫을 때까지 그 세션의 잠금을 가지므로(SF-3, SR-3), 한 요청이 쓴 것이
  다른 요청의 쓰기로 사라지지 않는다. <a id="SE-8.2"></a> 세션을 열지 않는 요청은 잠금을 갖지 않는다.

## 오류

- **SE-9** <a id="SE-9.1"></a> 라이브러리의 모든 실패는 kind를 가진, crate의 오류 enum의 오류다. 호출자는
  kind로 실패를 구분하고, 메시지는 규칙이 적는 곳에서만 규칙이다(SE-6.4). <a id="SE-9.2"></a>
  `polyspec-session-store-core`의 kind:

  | Kind | Status | Category | 규칙 |
  |---|---|---|---|
  | `random` | 500 | `io` | SE-5.4 |
  | `record` | 500 | `store` | SE-6.3 |
  | `record_not_array` | 500 | `store` | SE-6.4 |
  | `record_write` | 500 | `store` | SE-6.8 |
  | `internal` | 500 | `internal` | 구현의 불변식이 실패한다. 이것은 결함이다 |

  `record`와 `record_write`의 오류는 php-serialization 오류의 offset을 갖거나 갖지 않는다(`offset`).
  <a id="SE-9.3"></a> status는 세션이 실패하는 요청의 status다: 서버가 읽거나 쓸 수 없는
  세션은 그 요청을 서버의 오류로 실패시킨다. <a id="SE-9.4"></a> 저장소의 kind(SF-9, SR-9)는
  같은 오류 enum, 곧 `polyspec-session-store-core`의 `Error`의 variant다. 그래서 kind 인터페이스 하나와 클래스
  표 하나가 라이브러리의 모든 실패를 담고, 저장소가 만나는 레코드의 실패는 자신의 kind를 유지한다.
  <a id="SE-9.5"></a> category `internal`은 구현의 결함인 kind `internal`만 갖는다.
  프로세스의 환경이 결함 없이 일으키는 실패는 category `io`다: 운영체제의 난수원이
  실패하거나(`random`, SE-5.4), 다른 프로세스가 새 식별자의 파일을 만든 경우다
  (`identifier_taken`, SF-4.4). 그러면 오류의 클래스로 결함과 환경의 실패를 구분하는 호출자는
  두 경우 모두에서 옳다.

## 파일 저장소

- **SF-1** <a id="SF-1.1"></a> 파일 저장소는 각 세션을 디렉터리 하나의 파일에 두므로, 그 디렉터리를 공유하는
  서버 프로세스들이 그 세션을 공유한다. <a id="SF-1.2"></a> 디렉터리는 존재하는 디렉터리의 절대 경로다.
  그 밖의 경로는 저장소를 만드는 일을 kind `invalid_directory`로 실패시키고, 오류는 그 경로를
  적는다. `N;/path`나 `N;MODE;/path` 형식의 PHP `session.save_path`(식별자의 앞 문자로 나눈
  하위 디렉터리)는 절대 경로가 아니므로 그렇게 실패한다: 저장소는 모든 파일을 자신의 디렉터리에 둔다.
  <a id="SF-1.3"></a> 수명은 양의 초 수이고, 호출자가 적지 않으면 PHP `session.gc_maxlifetime`의
  기본값인 1440이다. 0은 kind `invalid_lifetime`으로 실패한다. <a id="SF-1.4"></a> 파일 저장소에는 WASM 바인딩이
  없다: 브라우저에는 서버 프로세스가 함께 쓰는 디렉터리가 없고, 저장소는 파일 I/O와 다른 프로세스의 lock을 기다리는데
  브라우저의 WebAssembly는 그것을 할 수 없다(binding Q-4). 클래스 `FileSessions`와 `FileSession`은 PHP, Python,
  Node, Go의 클래스다.
- **SF-2** <a id="SF-2.1"></a> 세션은 디렉터리의 파일 `sess_<id>`이고, 그 바이트는 세션의 레코드이며(SE-6),
  저장소가 파일을 만들 때 그 mode는 0600이다: PHP 세션 모듈 files handler의 파일 이름, 레코드,
  mode다. <a id="SF-2.2"></a> 바이트가 없는 파일은 값이 없는 세션이다(SE-6.2). <a id="SF-2.3"></a>
  파일이 마지막으로 수정된 때가 수명보다 오래 전인 세션은 끝났다: 그 파일은 어떤 세션도 가리키지 않는다.
  <a id="SF-2.4"></a> 저장소는 모든 파일을 `O_NOFOLLOW`와 `O_CLOEXEC`로 열므로, 세션 파일 자리에 있는
  심볼릭 링크는 open을 실패시키고, exec로 실행된 자식 프로세스는 그 파일과 잠금을 물려받지 않는다.
- **SF-3** <a id="SF-3.1"></a> `open`은 세션 쿠키의 바이트를 받아 요청의 레코드 세션을
  준다(SE-7). 식별자가 아닌 바이트(SE-5.2)는 기존 세션도 잠금도 없는
  레코드 세션을 준다. <a id="SF-3.2"></a> 한 프로세스 안에서 식별자 하나의 요청들은 잠금을 시도하기 전에 그 프로세스의
  queue에서 온 순서대로 기다린다. 그래서 프로세스 하나에서 thread 하나만 세션의 잠금을 기다린다.
  <a id="SF-3.3"></a> 요청은 자기 차례에 파일을 만들지 않고 연다. 파일이 없으면 기존 세션이 없는
  레코드 세션을 주고, 요청은 차례를 내놓는다. <a id="SF-3.4"></a> 요청은 파일에
  `LOCK_EX`로 `flock`을 걸고, 잠금을 가질 때까지 자신의 thread에서 block한다. 운영체제가 기다리는
  프로세스의 순서를 정하고, 라이브러리에는 재시도도 쉬는 시간도 없다. <a id="SF-3.5"></a> 잠금을 가지면 요청은
  잠근 파일과 경로가 가리키는 파일을 비교한다(device와 inode). 경로가 다른 파일을 가리키거나 아무 파일도 가리키지 않으면
  앞선 보유자가 파일을 삭제했거나 바꾼 것이므로(SF-4.4), 요청은 잠금을 풀고 SF-3.3부터 다시 연다.
  <a id="SF-3.6"></a> 수명보다 오래된 파일(SF-2.3)은 기존 세션이 없는 레코드 세션을 준다.
  요청은 잠금과 차례를 내놓는다. <a id="SF-3.7"></a> 그렇지 않으면 요청은 파일을 처음부터 끝까지 읽고
  `close`까지 잠금과 차례를 유지한다. <a id="SF-3.8"></a> 기다리기를 그만둔 호출자(그 `cancel`, 예를 들어
  연결이 닫힌 요청)는 kind `cancelled`로 실패하고 세션의 다음 요청이 진행하게 한다.
  queue에서 기다리는 요청은 취소로 즉시 깨어나 queue를 떠난다. `flock`에서 기다리는 요청은
  깨울 수 없다. 어떤 thread도 block하는 `flock`을 중단시킬 수 없기 때문이다: 그 요청은 잠금을 얻으면 취소를 보고,
  잠금과 차례를 즉시 내놓고 실패한다.
- **SF-4** <a id="SF-4.1"></a> `close`는 요청의 레코드 세션을 받아 `finish`가 주는 것을 쓴다(SE-7.4).
  <a id="SF-4.2"></a> `unused`는 아무것도 쓰지 않는다. `unchanged`는 바이트를 유지하고 파일의 수정 시각을
  지금으로 설정하므로, 세션은 마지막 요청부터 수명만큼 산다. <a id="SF-4.3"></a> 기존 식별자의 `write`는
  잠금 아래에서 레코드를 파일의 처음부터 제자리에 덮어쓰고 파일을 레코드의 길이로 자른다.
  PHP의 files handler가 하는 것과 같다. 쓰는 중에 끝나는 프로세스는 다음 요청이 읽을 수 없는 레코드를
  남길 수 있다(SE-6.3). <a id="SF-4.4"></a> 새 식별자의 `write`는 그 파일을 `O_CREAT | O_EXCL`과
  mode 0600으로 만들고 레코드를 쓴다. 다른 프로세스는 그 식별자를 모르므로 그 파일에는 잠금이 필요 없다. 갱신은
  옛 식별자의 파일을 삭제하기 전에 새 파일을 만든다. 그래서 실패해도 세션은 둘 중 하나의
  식별자 아래에 남는다. 새 식별자를 이미 가진 파일은 kind `identifier_taken`으로 실패한다. <a id="SF-4.5"></a>
  `close`는 쓰기가 실패할 때에도 잠금과 차례를 내놓고, 새 식별자를 주거나 아무것도 주지 않는다.
- **SF-5** <a id="SF-5.1"></a> `collect`는 끝난 세션(SF-2.3)의 파일을 삭제하고 삭제한 수를 준다.
  <a id="SF-5.2"></a> 디렉터리의 파일 `sess_<id>` 중 `<id>`가 식별자인 파일마다 `collect`는 기다리지 않고
  잠금을 시도한다(`LOCK_NB`). 요청이 잠금을 가진 파일은 건너뛰고, 잠근 뒤에 보니 아직 수명 안에 있는 파일과
  경로가 더는 가리키지 않는 파일은 그대로 둔다. <a id="SF-5.3"></a> 라이브러리는 스스로 `collect`를 호출하지 않는다:
  애플리케이션이 자신이 정한 때에, 예를 들어 예약된 작업에서 호출하고, 저장소는 요청이 세션을 열 때
  세션을 삭제하지 않는다.
- **SF-6** <a id="SF-6.1"></a> PHP files handler와 이 저장소는 파일 이름, 레코드, mode 0600,
  `flock(LOCK_EX)`를 공유한다. 그래서 `session.serialize_handler`를 `php_serialize`로 두고 native 세션 모듈을 쓰는
  PHP 프로세스와 이 저장소는 서로의 세션을 읽을 수 있다. 둘은 아래와 같이 다르며, 이 차이는 고치지 않는다:
  - native handler는 수정 시각을 수명과 비교하지 않고 파일을 읽으므로, 자신의 garbage collection이
    아직 삭제하지 않은 끝난 세션을 읽는다.
  - native handler는 파일을 `O_CREAT`로 열므로, 어떤 세션도 가리키지 않는 식별자의 파일을 만든다.
  - native handler의 garbage collection은 잠금 없이 `unlink`로 파일을 삭제하므로, 요청이 가진 세션의 파일을
    삭제할 수 있다.
  - native handler는 잠근 뒤에 파일이 삭제되었거나 바뀌었는지 검사하지 않는다.
  - `session_regenerate_id(true)`는 새 파일을 쓰기 전에 옛 파일을 삭제한다.
  - native handler는 소유자가 프로세스의 사용자가 아닌 파일을 거부하고, 이 저장소는 그것을 검사하지 않는다.
- **SF-7** <a id="SF-7.1"></a> `flock`은 열린 파일의 모든 descriptor가 닫힐 때까지 유지된다. 그래서 `exec` 없는 `fork`의
  자식이 잠긴 세션의 descriptor를 물려받으면, 그 자식은 끝나거나 descriptor를 닫을 때까지 세션을
  잡고 있다. `O_CLOEXEC`(SF-2.4)는 다른 프로그램을 실행하는 자식을 처리한다.
- **SF-8** <a id="SF-8.1"></a> 기다림(계획 B-5): `open`은 다른 프로세스를 기다리고(`ProcessWait`), `new`,
  `close`, `collect`는 짧은 블로킹 I/O를 한다(`Io`). block하는 Rust 함수는 자신의 토큰을 첫 인자로 받는다.
  <a id="SF-8.2"></a> 수명은 인터페이스 계약의 부호 없는 정수(`uint`)이므로, 바인딩은 0 이상의 정수가 아닌
  값을 저장소가 실행되기 전에 거부한다: 음수나 소수는 kind
  `invalid_number`로, 텍스트나 boolean은 kind `invalid_argument_type`으로 거부한다(binding 설계 T-1, E-1a). `contracts/file-sessions.json`의
  그런 case는 `binding`을 갖고, 바인딩이 그 case를 실행한다. Rust 시험은 그 case를 무시된 것으로 보여 준다.
- **SF-9** <a id="SF-9.1"></a> `polyspec-session-store-file`의 kind:

  | Kind | Status | Category | 규칙 |
  |---|---|---|---|
  | `invalid_directory` | 없음 | `value` | SF-1.2 |
  | `invalid_lifetime` | 없음 | `value` | SF-1.3 |
  | `cancelled` | 500 | `io` | SF-3.8 |
  | `identifier_taken` | 500 | `io` | SF-4.4 |
  | `not_found`, `permission_denied`, `cross_device`, `storage_full`, `io` | 500 | `io` | 파일 시스템의 연산이 실패한다. 오류는 `errno`를 갖는다 |
  | SE-9.2의 kind | | | 코어 |

  <a id="SF-9.2"></a> 파일 시스템 실패의 kind는 그 `errno`를 따른다: `ENOENT`는 `not_found`, `EACCES`와
  `EPERM`은 `permission_denied`, `EXDEV`는 `cross_device`, `ENOSPC`와 `EDQUOT`는 `storage_full`이고, 그 밖의
  모든 값은 `io`다. 오류는 연산과 경로를 적는다. <a id="SF-9.3"></a> `invalid_directory`와
  `invalid_lifetime`은 요청의 오류가 아니라 저장소를 만드는 호출의 오류이므로 status가 없다.

## Redis 저장소

- **SR-1** <a id="SR-1.1"></a> Redis 저장소는 각 세션을 Redis 서버의 key에 두고, phpredis 세션 handler의 key,
  잠금, 레코드를 쓴다. 그래서 그 Redis 서버를 공유하는 서버 프로세스들은 그 세션을
  서로 공유하고, SR-7이 적는 대로 phpredis를 쓰는 PHP 프로세스와도 공유한다. <a id="SR-1.2"></a> 옵션과
  그 기본값:

  | 옵션 | 뜻 | 기본값 | 거부되는 값 |
  |---|---|---|---|
  | `lifetime` | 세션의 마지막 요청부터 세션이 끝날 때까지의 초 수 | 1440 (`session.gc_maxlifetime`) | 0, kind `invalid_lifetime` |
  | `prefix` | key의 접두사, phpredis `session.save_path`의 `prefix` | `PHPREDIS_SESSION:` | 없음 |
  | `lock_expire` | 잠금이 만료될 때까지의 초 수, `redis.session.lock_expire` | 없음: 호출자가 적는다 | 0과, 밀리초로 바꾼 값이 2^64 − 1을 넘는 초 수, kind `invalid_lock_expire` |
  | `lock_retries` | 요청이 남이 가진 잠금을 몇 번 더 시도하는지, `redis.session.lock_retries` | 100 | 없음 |
  | `lock_wait` | 두 시도 사이의 마이크로초, `redis.session.lock_wait_time` | 20000 | 없음 |

  <a id="SR-1.3"></a> Redis 서버 하나의 모든 프로세스는 옵션에 같은 값을 주고, `lock_expire`는 어떤 요청보다도
  길다. `lock_expire`에는 기본값이 없다. 모든 애플리케이션의 모든 요청보다 긴 값은 없고
  phpredis에도 기본값이 없기 때문이다(phpredis의 0은 `max_execution_time`을 쓴다). 호출자가 그 값을 적는다.
- **SR-2** <a id="SR-2.1"></a> 세션은 key `<prefix><id>`이고, 그 바이트는 세션의 레코드이며(SE-6), 이 key는
  수명 뒤에 만료된다. 세션의 잠금은 key `<prefix><id>_LOCK`이다. <a id="SR-2.2"></a> 요청은 자신만의
  secret으로 잠금을 얻는다: 운영체제 난수원의 16바이트를 소문자 16진수 32자리로 적은 것이다. 그래서
  이 프로세스의 요청이든 다른 프로세스의 요청이든 다른 잠금의 secret을 갖지 않는다. <a id="SR-2.3"></a> 해제 스크립트는
  `if redis.call("get",KEYS[1]) == ARGV[1] then return redis.call("del",KEYS[1]) else return 0 end`이며,
  잠금을 그 보유자를 위해서만 삭제한다.
- **SR-3** <a id="SR-3.1"></a> 프로토콜은 I/O 없는 상태기계다: 보낼 명령이나 기다릴 시간을
  주고 응답을 받는다. 연결(SR-6)이 보내고 기다린다. 이 순서의 그 명령들이 phpredis와의
  인터페이스이며, `contracts/redis-sessions.json`의 명령 기록으로 검사한다. <a id="SR-3.2"></a> 열기:
  식별자가 아닌 바이트는 기존 세션이 없는 레코드 세션을 주고 아무것도 보내지 않는다. 그렇지 않으면
  한 프로세스 안에서 식별자 하나의 요청들은 그 프로세스의 queue에서 기다리고(SF-3.2와 같다), 요청은
  `EXISTS <key>`를 보낸다. 응답 0은 기존 세션이 없는 레코드 세션을 준다. <a id="SR-3.3"></a> 그다음 요청은
  `SET <key>_LOCK <secret> NX PX <lock_expire × 1000>`을 보낸다. 응답 `OK`는 잠금을 얻은 것이고, nil 응답은
  다른 쪽이 잠금을 가졌다는 뜻이다: 요청은 `lock_wait`만큼 기다리고 다시 시도하며, 모두 `lock_retries`번 더 시도한 뒤
  kind `lock_timeout`으로 실패한다. 요청은 가진 것이 없으므로 해제를 보내지 않는다. <a id="SR-3.4"></a> 잠금을 가진 요청은
  `GET <key>`를 보낸다: bulk string은 레코드이고 기존 세션의 레코드 세션을 주며, 요청은 닫을 때까지 그 잠금과
  차례를 유지한다. nil 응답, 곧 `EXISTS` 뒤에 만료되었거나 삭제된 key는 해제(SR-2.3) 뒤에 기존 세션이 없는
  레코드 세션을 준다.
- **SR-4** <a id="SR-4.1"></a> 닫기는 `finish`가 주는 것을 쓴다(SE-7.4). `unused`는 가진 잠금의
  해제만 보낸다. <a id="SR-4.2"></a> `unchanged`는 잠금을 비교하지 않고 `EXPIRE <key> <lifetime>`을 보낸다. 레코드를
  쓰지 않기 때문이다. 그래서 읽기만 하는 긴 요청은 잠금이 만료되어도 실패하지 않는다. 그다음 해제를 보낸다.
  <a id="SR-4.3"></a> 기존 식별자의 `write`는 먼저 `GET <key>_LOCK`을 보낸다: 요청의 secret이 아닌
  응답은 잠금이 만료되었고 다른 쪽이 가졌을 수 있다는 뜻이므로, 요청은 아무것도 쓰지 않고 해제를 보내고
  kind `lock_lost`로 실패한다. <a id="SR-4.4"></a> 그다음 요청은 `SETEX <key> <lifetime> <record>`를 보낸다. 갱신은
  새 key를 쓰고 그다음 `DEL <old key>`를 보낸다. 그다음 해제를 보낸다. 잠금을 가진 요청은 무엇이든 쓰기 전에
  잠금을 비교하며, 갱신의 새 식별자를 쓸 때에도 그렇다. 세션을 찾지 못한 요청은 잠금이 없고
  잠금의 `GET`을 보내지 않는다. <a id="SR-4.5"></a> 잠금을 얻은 뒤의 실패도 오류가 호출자에게 도달하기 전에 해제를
  보내고, 요청은 모든 경우에 차례를 내놓는다.
- **SR-5** <a id="SR-5.1"></a> 명령이 주는 타입(`EXISTS`에는 정수, `SET`에는 `OK`나 nil,
  `GET`에는 bulk string이나 nil, `EXPIRE`, `DEL`, 해제에는 정수, `SETEX`에는 `OK`)이 아닌 타입의 응답은
  가진 잠금을 해제한 뒤 kind `reply`로 실패한다. bulk string은 텍스트로 읽지 않으므로 레코드의 바이트는 바이트로
  남는다. <a id="SR-5.2"></a> Redis의 오류 응답은 명령을 적는 kind `redis`로 실패한다.
- **SR-6** <a id="SR-6.1"></a> 연결은 프로세스의 tokio runtime 하나에서 crate `redis`(redis-rs)를 쓴다.
  애플리케이션은 client가 아니라 연결 설정을 준다: 서버 하나, Sentinel 또는 Cluster이고, 각각 그 주소, TLS,
  ACL 사용자와 비밀번호를 갖는다. <a id="SR-6.2"></a> Sentinel에서는 명령을 보내는 요청이 첫 명령 전에 Sentinel에게
  자기 service의 primary를 묻는다. 이전 primary로의 연결이 열려 있을 때에도 묻는다.
  이전 primary는 승격 뒤 몇 초 동안 쓰기를 받아들이기 때문이다. <a id="SR-6.3"></a> Cluster에서는 프로토콜의 모든 명령에
  key가 하나이므로 세션과 그 잠금이 서로 다른 primary에 있을 수 있다. 해제 스크립트는 자신의 key 하나를
  지정한다. <a id="SR-6.4"></a> 만들 수 없거나 명령 중에 실패하는 연결은 kind
  `connection`으로 실패하고, 오류는 주소, 사용자, 비밀번호를 알리지 않는다. 실패한 요청은 자신의 잠금이 만료되도록 둔다.
  <a id="SR-6.5"></a> 프로세스는 연결을 만들 때의 process id를 기억한다. `fork` 뒤에 자식은
  물려받은 연결을 쓰지도 닫지도 않고(그 socket은 부모가 아직 쓴다), 자신의 연결을 만든다.
- **SR-7** <a id="SR-7.1"></a> phpredis를 쓰는 PHP 프로세스는 다음을 설정할 때 세션을 공유한다: `session.save_handler=redis`,
  `session.serialize_handler=php_serialize`, `redis.session.locking_enabled=1`, `redis.session.lock_expire`를
  저장소의 `lock_expire`로(0은 안 된다. phpredis는 그때 `max_execution_time`을 쓰고 그 값은 명령줄에서 0이다),
  `redis.session.lock_retries`를 저장소의 `lock_retries`로(-1은 안 된다. 그 값은 끝없이 시도한다),
  `redis.session.lock_wait_time`을 저장소의 `lock_wait`로(둘 다 마이크로초),
  `redis.session.lock_failure_readonly=0`, `redis.session.early_refresh=0`, `redis.session.compression=none`.
  그 밖의 설정은 상호 운용의 범위 밖이다. <a id="SR-7.2"></a> phpredis의 `rediscluster` 세션 handler는
  잠금을 얻지 않으므로, Redis Cluster에서는 phpredis를 쓰는 어떤 PHP 프로세스도 세션을 공유하지 않는다.
- **SR-8** <a id="SR-8.1"></a> 기다림(계획 B-5): `open`과 `close`는 tokio runtime에서 실행되고(`async`),
  잠금의 두 시도 사이의 기다림은 tokio sleep이다. 호출자는 future를 drop해 취소한다. drop된
  열기가 얻은 잠금은 `lock_expire` 뒤에 만료된다. <a id="SR-8.2"></a> `lifetime`, `lock_expire`, `lock_retries`,
  `lock_wait`는 인터페이스 계약의 부호 없는 정수이고, 바인딩은 SF-8.2가 적는 대로 그 값을 판정한다.
  `contracts/redis-sessions.json`의 case 중 `binding`을 가진 것은 바인딩에서 실행된다.
- **SR-9** <a id="SR-9.1"></a> `polyspec-session-store-redis`의 kind:

  | Kind | Status | Category | 규칙 |
  |---|---|---|---|
  | `invalid_lifetime` | 없음 | `value` | SR-1.2 |
  | `invalid_lock_expire` | 없음 | `value` | SR-1.2 |
  | `invalid_setting` | 없음 | `value` | SR-6.1: Redis의 URL이 아닌 주소 또는 빈 주소 목록 |
  | `lock_timeout` | 500 | `store` | SR-3.3 |
  | `lock_lost` | 500 | `store` | SR-4.3 |
  | `reply` | 500 | `store` | SR-5.1 |
  | `redis` | 500 | `store` | SR-5.2 |
  | `connection` | 500 | `io` | SR-6.4 |
  | SE-9.2의 kind | | | 코어 |

## 검증

- **SE-10** <a id="SE-10.1"></a> `contracts/sessions.json`은 코어의 case를 담는다: group `identifier`(SE-5.1,
  SE-5.2), group `created`(SE-5.3: 새 식별자의 `length`와 `alphabet`. 새 식별자는 식별자이고
  그 앞에 만든 식별자와 다르다), group `store`(SE-2, SE-4, 메모리 저장소와 레코드 세션에서 실행한다),
  group `record`(SE-6, SE-7). `contracts/file-sessions.json`은 파일 저장소의 요청을 담고(SF-1부터 SF-5),
  `contracts/redis-sessions.json`은 Redis 저장소의 요청을 각 요청이 보내는 명령과 함께 담는다(SR-1부터
  SR-5). <a id="SE-10.2"></a> case의 PHP 값은 그 serialize 형식이고, case의 바이트 문자열은 문자 U+0000부터
  U+00FF가 그 바이트인 JSON 텍스트다. 실패는 `kind`, `status`, `category`,
  `message`를 가진 key `raises`이고, 그런 case에는 결과 key가 없다. 실패하는 여러 요청의 case(`steps`)는 실패하는
  step과 연산을 `raises`에 적는다(`step`, `at`: `open`, `call` 또는 `close`). 그런 case에는 결과가 없고, 그 `records`나
  `stored`는 실패 뒤에 저장소가 가진 것을 담는다. case가 만든 식별자는 `#n`이고, 이것은 case가 만드는 n번째
  식별자이며 0부터 센다. 명령 기록에서 잠금의 secret은 `<secret>`이고 해제 스크립트는
  `<release>`다. 호출의 값은 `{"nested": n}`이기도 하다: 각각 key 0에 다음 배열을 가진 배열 n개이고, 가장 안쪽 배열은
  null을 갖는다. 이것은 serialize 형식으로 `a:1:{i:0;` n번, `N;`, `}` n번이다. case는 `found`에
  step마다 그 step의 열기가 찾은 식별자(SE-7.1)를 `#n`이나 null로 적는다. `found`가 없는 case는 그것을
  비교하지 않는다. 파일 저장소의 group `open`의 case는 만든 저장소의 디렉터리를
  `store_directory`에 적고, `setting`을 가진 Redis 저장소의 case는 그 연결 설정(`server`, `sentinel`,
  `cluster` 중 하나와 그 필드, SR-6.1)의 저장소를 만들며, 그 저장소는 어디에도 연결하지 않는다. <a id="SE-10.3"></a> case가 쓰는
  모든 레코드는 PHP 실행 파일이 주어지면(`PHP`) 코어의 시험에서 실제 PHP(PHP 8.4의 `unserialize`)가 다시
  읽으므로, 그 레코드는 PHP의 레코드다. <a id="SE-10.4"></a> Redis case의 명령은
  프로토콜이 보내는 Redis 명령의 모델에 대해 실행되고, 같은 요청이 실제 Redis 서버에 대해 실행된다.
  시험은 `redis-server` 실행 파일이 주어지면(`REDIS_SERVER`) 그 서버를 시작한다. `textReplies`를 가진 case(모델이
  각 레코드를 status 응답으로 준다, SR-5.1)나 `errorReplies`를 가진 case(모델이 한 이름의 모든 명령에 적힌 오류 응답을
  준다, SR-5.2)는 모델에 대해서만 실행된다. 실제 서버는 이 key에 대한 이 명령에 둘 중 어느 것도
  주지 않기 때문이다.
