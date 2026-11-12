# 변경 기록

[English](CHANGELOG.md)

## 미출시

- 명세 `docs/spec/sessions.md`를 더했다: 키와 값의 의미를 모르는 브라우저 세션 값 저장(SE-1~SE-10), 파일 저장소(SF-1~SF-9),
  Redis 저장소(SR-1~SR-9).
- case `contracts/sessions.json`, `contracts/file-sessions.json`, `contracts/redis-sessions.json`을 더했다. Redis
  프로토콜의 명령 기록을 담는다.
- crate `polyspec-sessions-core`를 더했다: trait `Store`, 메모리 저장소, 식별자, `php_serialize` 형식의 세션 레코드, 요청의
  레코드 세션. kind는 `random`, `record`, `record_not_array`, `record_write`, `internal`이다.
- crate `polyspec-sessions-file`을 더했다: `FileSessions`의 `open`(프로세스 queue, `flock(LOCK_EX)`, 잠금 뒤 파일 확인, 끝난
  세션), `FileSession::close`, `collect`. kind는 `invalid_directory`, `invalid_lifetime`, `cancelled`,
  `identifier_taken`과 파일 시스템의 kind다.
- crate `polyspec-sessions-redis`를 더했다: 상태기계인 phpredis 프로토콜과 redis-rs로 서버 하나, Sentinel, Cluster에 TLS와
  ACL 사용자로 연결하는 `RedisSessions`. kind는 `invalid_lifetime`, `invalid_lock_expire`, `invalid_setting`,
  `lock_timeout`, `lock_lost`, `reply`, `redis`, `connection`이다.
- 클래스 `Sessions<Category>Error`를 쓰는 인터페이스 계약 `packages/*/rust/interface.json`을 더했다. 아직 생성한 바인딩은
  없다.
- Windows는 지원하지 않는다.
