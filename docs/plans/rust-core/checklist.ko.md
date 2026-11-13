<!-- doc-id: checklist-rust-core -->
<!-- source-sha256: d08c65491f2bbed4002f0be160b519ded582d8fe6a79c3ebb1734771d4b1a87a -->
# Rust 코어 체크리스트

| ID | Task | Deliverables | Verification | State |
|---|---|---|---|---|
| T1.1 | 저장소의 규칙과 그 case를 정의한다. 저장소 인터페이스, PHP 세션의 레코드, 파일 저장소, Redis 저장소다(96da077, 5d17159, f3948a5, 851faf6, 88348b5, cde837e) | `docs/spec/sessions.md`, `contracts/` | `make test` | [o] |
| T1.2 | 규칙을 코어, 파일 저장소, Redis 저장소, 라이브러리의 Rust crate로 쓴다. 하나의 error enum이 모든 실패에 kind를 준다(82de061, 1ef5217, 98982bb, 25a86d3, cf97bac, 598a5b4, 49bbb85, f60156a, c9e30e1) | `packages/core/rust`, `packages/file/rust`, `packages/redis/rust`, `packages/rust` | `make test`, `make lint-rust` | [o] |
| T1.3 | Redis 저장소를 Apple container가 고정된 image로 돌리는 실제 Redis 서버로 시험한다(02f874a, 0199e81) | `scripts/test-run.sh`, `packages/redis/rust/tests` | `make test` | [o] |
| T1.4 | binding의 다섯 target을 위한 인터페이스 계약을 crate에 담아 쓴다. kind마다 case를 두고, 어떤 입력도 닿지 않는 실패마다 fault 주입점을 둔다(e2c3189, 0db652a, 6971e8a, 99e777f, 8fb66de, d736bae, ce680ac, 53c2c7d, 88348b5, 3202fe0, 5df4559) | `packages/rust/interface.json`, `packages/core/rust/src/fault.rs` | `make binding-check` | [o] |
| T1.5 | 코어와 binding scaffold가 쓰는 바인딩 crate를 binding의 한 commit에 고정된 하나의 workspace에 둔다(a0adbab, a6b4e66, 6ca4ab7, 225076b, c169b54, ab6e7a0, 0e08c7a, f31a8d9, 53c2c7d) | `Cargo.toml`, `Cargo.lock`, `packages/php-ext`, `packages/python`, `packages/ts`, `packages/ts/wasm`, `packages/go`, `scripts/binding.mts` | `make binding-check` | [o] |
| T1.6 | 계획과 checklist를 한국어 쌍과 함께 두고, 모든 문서와 이 checklist를 `make docs-check`로 읽는다 | `docs/plans/rust-core`, `config/documents.json`, `config/checklist.json`, `package.json`, `package-lock.json`, `Makefile` | `make docs-check` | [~] |
| T1.7 | PHP, Python, Node, WASM 바인딩을 binding이 생성하는 시험과 contract case로 각 언어의 표준 runner에서 시험한다. 각 target은 자기 `make bindings-<target>`이 생성한다 | `packages/php-ext/tests`, `packages/python/tests`, `packages/ts/tests`, `Makefile` | `make test-php`, `make test-python`, `make test-node`, `make test-wasm` | [!] 원인: binding이 모든 target에서 계약의 생성을 거부한다(G-4). `memory_store`의 implements와 abstract, `file_sessions`와 `file_session`의 only와 native, variant unit `setting`, `redis_sessions.open`의 wait async_driver, `redis_session`의 cleanup을 아직 생성하지 않는다; 재시도: binding이 php, python, node, wasm에서 packages/rust/interface.json을 생성한다 |
| T1.8 | 이 저장소에 대한 binding의 end-to-end 검사를 php, python, node, wasm에서 통과한다 | binding의 추적되지 않는 `var/report/e2e/session-store.json` | binding의 `make e2e` | [!] 원인: end-to-end 검사는 binding이 거부하는 T1.7의 생성에 닿는다; 재시도: T1.7이 끝난다 |
| T1.9 | Go 바인딩을 binding이 생성하는 시험과 `packages/go/tests`의 contract case로 시험한다 | `packages/go` | `make test-go` | [!] 원인: binding이 계약의 Go 생성을 거부한다(G-4). T1.7의 항목 밖에도 다른 저장소의 value alias(php-serialization의 `php_value`)와 callback unit(`store`)을 Go에서 아직 생성하지 않는다; 재시도: binding이 packages/rust/interface.json의 Go를 생성한다 |
