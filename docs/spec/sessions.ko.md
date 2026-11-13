<!-- doc-id: sessions -->
<!-- source-sha256: 02e9fa28b7183a87e1cf770d47fdac73171b93ce211469394ce5d42950375426 -->
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
  <a id="SE-9.6"></a> 환경의 실패 몇 가지와 kind `internal`에는 어떤 입력도 닿지 않으므로, 실패 주입점이 그것을
  흉내 낸다(binding 설계 G-5, X-1). `polyspec-session-store`가 다시 내보내는 `polyspec-session-store-core`의
  모듈 `fault`가 주입점을 `session_store.<id>`로 이름하고, 저장소의 crate는 자기 자리의 주입점을 그 모듈로
  부르며, 인터페이스 계약은 같은 주입점을 `faults`에 적는다. 주입점은 `polyspec-session-store-core`의 feature
  `fault-test`가 켜져 있지 않으면 아무것도 하지 않는다: `polyspec-session-store`는 자기 feature `fault-test`로 그것을
  켜고, 바인딩 crate는 자기 시험에서만 그 feature를 켜며, crate의 시험은 dev-dependency `polyspec-binding-core`로
  그것을 켜고, 어떤 release 빌드에도 그것은 없다. `open`, `create`, `read_dir`의 `io` 주입은 그 시스템 호출이 fault의
  `errno`로 실패하는 것이고 그 kind는 SF-9.2를 따르며, fault가 `errno`를 적지 않으면 `errno` 없는 kind `io`의
  실패다. `errors.kinds`의 variant `Io` 행이 `kinds`를 적지 않고 주입점은 그 표의 kind만 채우므로, 계약의
  `fills`는 SF-9.2의 kind 중 `io`만 적는다. `create`는 그 호출의 `EEXIST`가 가리키는 kind `identifier_taken`도 채운다(SF-4.4). `random`의 `io` 주입은 난수원의 실패이며 `errno`가 없다. 코어는 task도
  thread도 시작하지 않으므로, 각 주입점은 그 점에 닿는 호출의 장전을 받는다. record session의 `get`, `set`,
  `remove`(`file_session`과 `redis_session`의 호출)가 닿는 kind `internal`의 모든 불변식 분기는 바로 앞에 주입점을
  가지며, 그 주입점은 실패한 불변식을 `broken`으로 흉내 낸다. 복사의 주입점은 같은 복사로 레코드를 쓰는 `close`도
  닿는다. `MemoryStore`의 `get`, `set`, `remove`와 `random_hex`에는 kind `internal`의 분기가 없다. 그 member `memory_store.get`, `memory_store.set`,
  `memory_store.remove`는 kind를 내지 않지만, 코어 경로는 `Result`를 돌려주는 trait `Store`의 메서드이므로 인터페이스
  계약은 그것들에 `core.result: true`를 적는다(binding CK-21).

  | Id | 자리(함수) | 주입 | 채우는 kind | 조건의 규칙 | 장전을 받는가 |
  |---|---|---|---|---|---|
  | `random` | `new_identifier`, 새 세션의 시작, 갱신, Redis 저장소의 열기가 부르는 `random_hex` | `io` | `random` | SE-5.4 | 예 |
  | `internal` | 두 분기: `MemoryStore`의 `renew`에서 갱신 횟수가 u64를 넘는 것으로 다루고(SE-4.3), `get`, `set`, `remove`, `renew`에서 record session의 시작(`RecordSession::start`)에서 시작 뒤의 상태를 기다리는 상태로 다룬다(SE-7.2). 둘째는 `file_session`과 `redis_session`의 모든 호출자의 경로에 있으므로, 이 라이브러리의 kind를 넘겨주는 계약은 자기 member에서 지점 `internal`을 장전한다(binding X-1) | `broken` | `internal` | SE-9.6 | 예 |
  | `record_root` | record session의 시작에서 찾은 레코드의 decode(`Values::decode`): root가 배열을 가리키지 않는 것으로 다룬다 | `broken` | `internal` | SE-6.3 | 예 |
  | `record_index` | record session의 `get`(`Values::get`): index가 항목을 가리키지 않는 것으로 다룬다 | `broken` | `internal` | SE-6.5 | 예 |
  | `value_reference` | 참조인 읽은 원소에 대한 record session의 `get`(`value_of`): 원본이 참조를 가리키지 않는 것으로 다룬다 | `broken` | `internal` | SE-6.5 | 예 |
  | `value_root` | 읽은 원소에 대한 record session의 `get`(`value_of`): 복사본이 최상위 값을 거부하는 것으로 다룬다 | `broken` | `internal` | SE-6.5 | 예 |
  | `copy_new_reference` | `get`과 `close`에서 원본의 참조의 복사(`Copier::shallow`): 새 참조가 거부되는 것으로 다룬다 | `broken` | `internal` | SE-6.6 | 예 |
  | `copy_array` | `get`과 `close`에서 배열의 복사(`Copier::fill`): 원본이 배열을 가리키지 않는 것으로 다룬다 | `broken` | `internal` | SE-6.5 | 예 |
  | `copy_element` | `get`과 `close`에서 배열의 원소의 복사(`Copier::fill`): 원소가 거부되는 것으로 다룬다 | `broken` | `internal` | SE-6.5 | 예 |
  | `copy_object` | `get`과 `close`에서 객체의 복사(`Copier::fill`): 원본이 객체를 가리키지 않는 것으로 다룬다 | `broken` | `internal` | SE-6.5 | 예 |
  | `copy_property` | `get`과 `close`에서 객체의 속성의 복사(`Copier::fill`): 속성이 거부되는 것으로 다룬다 | `broken` | `internal` | SE-6.5 | 예 |
  | `copy_reference` | `get`과 `close`에서 참조의 복사(`Copier::fill`): 원본이 참조를 가리키지 않는 것으로 다룬다 | `broken` | `internal` | SE-6.6 | 예 |
  | `copy_set_reference` | `get`과 `close`에서 참조의 값의 복사(`Copier::fill`): 값이 거부되는 것으로 다룬다 | `broken` | `internal` | SE-6.6 | 예 |
  | `open` | `FileSessions::open`에서 세션 파일의 open | `io` | `io` | SF-3.1, SF-9.2 | 예 |
  | `create` | `file_session`의 `close`에서 새 세션 파일의 배타적 생성 | `io` | `io`, `identifier_taken` | SF-4.4, SF-9.2 | 예 |
  | `read_dir` | `FileSessions::collect`에서 디렉터리의 읽기 | `io` | `io` | SF-5.1, SF-9.2 | 예 |

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
  <a id="SF-1.5"></a> 저장소는 Rust의 static 함수 `FileSessions::in_directory`와 인터페이스 계약의 static member
  `file_sessions.in_directory`가 만든다. 이 member는 디렉터리와 수명을 받아 저장소를 돌려주며, 클래스 `FileSessions`에는
  생성자가 없다. 저장소를 만드는 일은 디렉터리를 읽으므로(SF-1.2) 파일 I/O를 기다리는데, 생성자를 await할 수 없는
  언어에는 기다리는 생성자의 꼴이 없고 binding은 그것을 모든 target에서 거부한다(binding GN-6m).
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
  그 앞에 만든 식별자와 다르다. 그리고 SE-5.4), group `store`(SE-2, SE-4, 메모리 저장소와 레코드 세션에서
  실행한다), group `record`(SE-6, SE-7), group `internal`(SE-9.6), group `recordFaults`(SE-9.6). `contracts/file-sessions.json`은 파일 저장소의 요청을 담고(SF-1부터 SF-5),
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
  모든 레코드는 코어의 시험에서 실제 PHP(PHP 8.4의 `unserialize`)가 다시 읽고, PHP 실행 파일이 주어지지
  않으면(`PHP`) 그 시험은 실패하므로, 그 레코드는 PHP의 레코드다. <a id="SE-10.4"></a> Redis case의 명령은
  프로토콜이 보내는 Redis 명령의 모델에 대해 실행되고, 같은 요청이 실제 Redis 서버에 대해 실행된다.
  시험의 모든 실제 서버(case의 서버, 그리고 SR-6의 ACL 사용자, TLS, Sentinel, Cluster)는 Makefile이 digest로 고정한
  image(`REDIS_IMAGE`, `make redis-image`가 받는다)의 container 안에서 도는 Redis 8.10.3의 `redis-server`이고, 어떤
  시험도 장비에 설치된 서버를 실행하지 않는다. 서버는 bay가 시작하고 지운다. bay는 Compose 프로젝트를 Darwin에서는
  Apple `container`로, Linux에서는 Docker로 실행한다. 그래서 image와 그 digest는 하나이고, 두 플랫폼의 시험이 같은
  bytes의 서버에 연결하며, 이 저장소의 어떤 파일도 engine의 명령을 갖지 않는다. `scripts/servers.sh`가 bay의
  명령(`check`, `pull`, `plan`, `up`, `images`, `exec`, `logs`, `output`, `down`)을 갖고, harness와 run은 이 script만 시작한다.
  - 시험마다 프로젝트 하나. 시험은 자기가 필요한 서버의 Compose 파일을 run의 디렉터리(`SESSION_STORE_PROJECTS`) 아래
    자기 디렉터리에 쓰고, 한 번의 `up`으로 프로젝트 하나로 시작한다. 서버마다 service 하나를 두고, image는 digest로
    적고, 서버의 인자는 `command`로, 서버의 port는 `expose`로, 서버가 시험의 파일을 읽는 곳은 프로젝트의 디렉터리를
    volume으로 적으며, port를 host에 공개하지 않는다. 서버의 인자는 시험이 적는 내용의 일부이고(ACL 사용자, TLS,
    Sentinel, cluster 방식), 한 시험의 key와 사용자와 cluster는 다른 시험에 닿으면 안 된다. 그래서 시험끼리 서버를
    함께 쓰지 않고, run은 자기가 실행하는 시험의 서버만 시작한다. `tests/contract.rs`의 case들은 이 뜻에서 시험
    하나다. 서버 하나를 함께 쓰고, case마다 자기 database를 쓴다.
  - 프로젝트의 이름은 그 디렉터리의 이름이다. 파일이 이름을 적지 않으면 bay가 디렉터리의 이름을 쓴다. 그 이름은
    run의 디렉터리 이름, 시험 process의 id, 순번으로 이루어진다. run의 디렉터리는 run의 process를 따라 이름 지으므로,
    한 장비의 두 run이 같은 이름을 갖지 않는다. bay는 이름에 글자, 숫자, `-`, `_` 밖의 문자가 있는 프로젝트를
    거부한다. 그래서 harness나 `scripts/servers.sh`가 만드는 프로젝트의 디렉터리는 모두 그 문자들로만 이름 짓는다.
    template 없이 `mktemp -d`가 주는 이름에는 점이 있다.
  - 모든 이름은 bay가 준다. 시작하기 전에 harness는 Compose 파일의 `bay plan --json`을 읽는다. 거기에 각 service의
    host 이름과, bay가 그 이름에 발급하는 certificate의 경로가 있다. 다른 서버의 이름을 적는 서버는 그 이름을
    plan에서 받는다. Sentinel의 설정이 primary의 이름을 이렇게 적는다. 그래서 한 번의 `up`이 한 시험의 모든 서버를
    시작한다. 시작한 뒤 harness는 `bay up --json`의 출력을 읽고(`address`, 곧 이름과 port, 그리고 `ipv4`), 시험은 그
    주소에 연결한다. 이 저장소의 어떤 파일도 이름의 domain이나 certificate의 경로를 적지 않고, 어떤 port도 host에
    공개하지 않는다. harness는 `schemaVersion`이 1이 아닌 plan이나 `up`의 출력을 실패시킨다. 1이 harness가 읽는
    계약의 version이다.
  - TLS 서버는 bay가 자기 이름에 발급한 certificate를 plan이 주는 container 안의 경로에서 읽고, 시험은 plan이 그
    파일을 적는 bay의 authority를 신뢰한다. 어떤 시험도 certificate를 만들지 않는다.
    저장소는 서버를 자기 process의 trust store로 검증한다. 연결 설정이 authority를 적지 않기 때문이고(SR-6.1),
    rustls는 `SSL_CERT_FILE`이 적는 파일을 읽는다. process의 환경은 그 모든 thread의 것이고, 한 thread가 환경을 읽는
    동안 다른 thread가 그것을 바꾸는 일은 안전하지 않다. 그래서 어떤 시험도 trial이 병렬로 도는 process의 환경을
    바꾸지 않는다. TLS의 시험은 trial 하나를 가진 자기 test binary(`tests/tls.rs`)이고, 두 번째 thread를 시작하기
    전에 authority 파일을 `SSL_CERT_FILE`에 적는다.
  - `up`의 반환이 시작의 event다. bay는 모든 service가 자기 port에서 연결을 받은 뒤 반환한다. harness는 시간 제한도,
    sleep도, 반복해서 묻는 일도 갖지 않는다. cluster의 시험은 image의 `redis-cli --cluster create`로 cluster를
    만든다. `bay exec`가 그 명령을 첫 node의 container에서 세 node의 IPv4 주소와 함께 실행한다. 그 명령이 Redis가
    cluster를 만드는 일을 적는 방법이며, 프로젝트의 service가 아니다. 그 명령은 node의 주소를 받는데 bay는 주소를
    시작한 뒤에만 주고(service의 이름은 주소를 둘 이상 주고, node는 그 명령이 준 주소를 client에게 알린다),
    `bay exec`가 그 status를 시험에 돌려주기 때문이다. cluster가 요청을 받는다는 사실은 node가 state ok를 알리는
    줄이다. 각 node는 자기 state가 바뀔 때 그 줄을 한 번 쓴다. 시험은 각 node의 그 줄을 `bay logs -f`가 첫 줄부터
    주는 출력에서 기다린다. 시험은 `CLUSTER INFO`를 읽지 않는다. 그것은 한 순간의 state를 적으므로, 바뀌기 전에
    읽으면 다시 읽어야 하기 때문이다.
  - 어떤 시험도 image를 받지 않는다. harness는 프로젝트를 `up -no-pull`로 시작하고, 이 명령은 image가 설치되지
    않았으면 아무것도 만들기 전에 실패한다. 그렇게 실패한 시험은 `make redis-image`를 적는다. 그런 시험은 무시된
    시험으로 보고되지 않는다. `make redis-image`는 `bay pull`로 받으며, 받는 step은 이것뿐이다.
    `scripts/servers.sh pull`은 image를 `TMPDIR` 아래 자기 디렉터리 `session-store-pull-<글자>`에 프로젝트의 service
    하나로 적고, 받고, 그 디렉터리를 지운다.
  - bay는 make처럼 장비가 주는 도구다. run은 PATH의 `bay`를 시작하고, 그 version을 고정하지도 비교하지도 않는다.
    harness의 결과를 바꾸는 것은 bay 릴리스의 번호가 아니라 harness가 읽는 출력의 계약이다. 그래서 harness는
    `schemaVersion`이 1이 아닌 plan이나 `up`의 출력을 실패시키고, 자주 새 빌드가 되는 개발 장비의 bay는 그대로 시험을
    실행한다. run은 `bay version`의 출력을 첫 줄로 쓴다. 무엇이 서버를 시작했는지의 사실이다. run은 시험을 빌드하기
    전, 아무것도 만들기 전인 시작할 때 다음 경우에 실패한다. bay를 찾지 못하거나 bay가 version을 주지 않으면 설치
    방법을 적고 실패하고, `bay check -scope run`이 프로젝트에 필요한 장비 설정의 일부가 없다고 보고하면 그 설정을
    적용하는 `bay install`을 적고 실패한다. 어떤 run도 장비의 설정을 바꾸지 않고, `up`도 바꾸지 않는다. 시작할 때의
    검사는 빌드 뒤 모든 시험에서 한 번씩 실패하는 대신 빌드 전에 한 번 실패하게 한다.
  - bay의 릴리스는 CI를 위해서만 고정한다. CI의 runner에는 bay가 없으므로 workflow가 bay를 받아야 하고, 받은 것은
    추적 파일이 적은 것일 때만 쓴다. `config/toolchain.json`이 CI가 설치하는 릴리스를 플랫폼별 archive의 sha256과
    함께 적는다. `make install-bay`는 그 릴리스의 그 플랫폼 archive를 받고, sha256이 고정한 값과 다르면 실패하며,
    `bay`와 `bayd`를 `var/tools/bay`에 둔다. run의 어떤 경로도 그 고정 값을 읽지 않는다.
  - bay를 시작하는 방법은 값 하나, Makefile의 `BAY`이고, `scripts/servers.sh`만 그 값을 읽는다. 다른 어떤 파일도
    bay를 시작하지 않는다. 기본값은 PATH의 `bay`다. bay의 설정은 `bay install`이 그것을 실행한 사용자를 위해 쓰는
    장비의 상태(resolver, DNS agent, authority)다. 그래서 개발 장비의 bay는 그 장비의 주인이 설치하고 설정한 것이다.
    workflow `ci`는 bay를 실행하는 step마다 `BAY_SUDO=1`을 주고, 그러면 `BAY`는 `sudo -n` 뒤의 `var/tools/bay`의
    bay가 된다. Linux에서는 bay의 설정과 명령이 root로 실행되고 bay는 자기 상태를 실행한 사용자의 home 아래에
    두므로, 설치와 검사와 받기와 시험의 모든 명령이 한 사용자로 실행되어야 하기 때문이다. `make setup-bay`는
    `bay install`을 실행한다. 장비의 설정을 바꾸는 target은 이것 하나이며, workflow가 자기 runner에서 실행하고
    개발자는 자기 선택으로 실행한다.
  - `make test`는 시험을 `scripts/test-run.sh`로 실행하고, 이 script가 run의 디렉터리를 소유한다. Compose 파일은
    `up`보다 먼저 쓰이므로, 시작된 모든 프로젝트에는 파일이 있다. run이 끝날 때, 실패할 때, SIGINT나 SIGTERM으로 멈출
    때 script는 시험을 기다리고, 디렉터리에 남은 모든 Compose 파일에 `down`을 실행하고, 디렉터리를 지운다. 신호가
    멈춘 `up`은 자기가 만든 것을 지운다. `down`이 지우지 못한 프로젝트가 있으면 run은 실패하고, 디렉터리를 남기고 그
    파일을 적는다.

  `text_replies`를 가진 case(모델이 각 레코드를 status 응답으로 준다, SR-5.1)나
  `error_replies`를 가진 case(모델이 한 이름의 모든 명령에 적힌 오류 응답을 준다, SR-5.2)는 모델에 대해서만 실행되고
  서버에 대한 trial이 없다. 실제 서버는 이 key에 대한 이 명령에 둘 중 어느 것도 주지 않기 때문이다.
  <a id="SE-10.5"></a> `faults`를 가진 case는 생성된 바인딩 시험처럼 SE-9.6의 실패 주입점을 자기 한 호출에
  장전한다(binding 설계 G-5): Rust 시험은 그 목록을 member에 장전하고, member의 entry point가 받는 장전으로 호출을
  실행하며, 터지지 않은 fault가 있으면 case를 실패시킨다. 그런 case는 자기 member만 읽는 group의 한 호출 case이며,
  그래서 다른 member의 시험은 그것을 장전하지 않는다: `contracts/sessions.json`의 group `created`(`new_identifier`)와
  `internal`(`memory_store.renew`), `contracts/sessions.json`의 group `recordFaults`(record session의 주입점마다
  record session의 한 호출이며, Rust 시험만 읽고, 같은 호출을 먼저 장전 없이 실행해 입력만으로는 실패가 없음을
  확인한다), `contracts/file-sessions.json`의 group `openFaults`(`file_sessions.open`)와
  `collectFaults`(`file_sessions.collect`), `contracts/redis-sessions.json`의 group
  `openFaults`(`redis_sessions.open`)다. 한 호출 case는 자기 unit의 생성자로, 파일 저장소는 `file_sessions.in_directory`로(SF-1.5) case의 key에서 receiver를
  만든다(binding T-5): 라이브러리의 모든 플랫폼에 있고 그 읽기를 장전된 주입점이 호출이 읽기 전에 실패시키는
  디렉터리 `/`의 파일 저장소, 그리고 어떤 파일도 될 수 없는 `/dev/null` 아래 socket의 Redis 저장소다.
  `contracts/redis-sessions.json`의 group `opening`은 그 설정으로 연결이 실패하는 한 호출의 입력 case를
  담는다(SR-6.4). 이 입력이 닿는 실패를 흉내 내는 주입은 없다. <a id="SE-10.6"></a> 요청의 세션(`file_session`, `redis_session`)은 다른 unit의 메서드인 그 저장소의
  `open`이 만든다. 그래서 그 호출 하나의 case는 앞선 호출이 만드는 instance 값으로 키 `receiver` 아래에 receiver를
  준다(binding GN-13f). 그 값은 생성자나 `file_sessions.in_directory`(SF-1.5)로 만든 저장소의 쿠키 없는 `open`과 `then`의 호출이다. 저장소는 디렉터리 `/`의
  파일 저장소(발동하지 않는 취소와 함께)와 `/dev/null` 아래 socket의 Redis 저장소다. `get`, `set`, `remove`,
  `renew`의 group `file_session.<member>.made`와 `redis_session.<member>.made`는 새 세션의 시작이 닿는
  `session_store.random`을 장전한다(SE-5.4, SE-7.2). `redis_session.close.made`는 값을 set한 세션을 닫는다. 그 close는
  그 socket에 연결하므로 `connection`으로 실패한다(SR-6.4). `file_sessions.open.cancel`은 호출 전에 발동한
  취소(`{"cancel": "fired"}`)로 `file_sessions.open`의 `cancelled`(SF-3.8)를 준다. 계약에는 `file_session.close`의
  `io`와 `identifier_taken`의 case가 없고, 계약 검사는 둘을 지적한다(binding CK-22). 세션 파일의 쓰기에는 결함 주입
  지점이 없고, 라이브러리의 모든 플랫폼에서 두 kind에 닿는 입력이 없기 때문이다. 검사는 group `requests`의 case를
  그 group을 읽는 모든 member에 센다. 그러나 바인딩 시험은 case의 key를 한 호출의 인자로 받으며 여러 요청의
  case(`steps`)는 하나도 실행하지 않는다. 그래서 그 group의 어떤 case도 주입점을 장전하지 않는다. 그런 case는
  어떤 바인딩 시험도 실행하지 않는데 검사는 통과시키기 때문이다.
