<!-- doc-id: plan-rust-core -->
<!-- source-sha256: 118f48572caee0eb041b6c7c7acff228451a2caf8c8a4b5b3dfed621c668cae6 -->
# Rust 코어

[English](plan.md)

session-store는 [명세](../../spec/sessions.ko.md)의 규칙으로 PHP 서버의 세션을 보관한다. 메모리 저장소, 파일 저장소,
Redis 저장소가 있으며, 각각 PHP 세션의 레코드를 읽고 쓴다. 모든 규칙은 `packages/core/rust`, `packages/file/rust`,
`packages/redis/rust`, `packages/rust`의 Rust crate에 한 번 쓰이고, `contracts/`의 case로 한 번 시험된다. PHP, Python,
Node, 브라우저(WASM), Go 패키지는 자기 코드를 갖지 않는다. binding이 인터페이스 계약 `packages/rust/interface.json`에서
각 패키지를 생성하고, binding scaffold가 그 바인딩 crate를 쓴다. 그래서 규칙에는 처음 것과 달라질 수 있는 두 번째
사본이 없다.

checklist는 각 단계가 앞 단계를 필요로 하는 순서를 따른다.

1. 명세와 그 case(T1.1), 그다음 그것을 통과하는 코어(T1.2). 코어의 Redis 저장소는 Apple container의 실제 서버로
   시험된다(T1.3).
2. binding의 다섯 target을 위한 코어의 인터페이스 계약. kind마다 case가 있고, 어떤 입력도 닿지 않는 실패마다 fault
   주입점이 있다(T1.4).
3. 코어와 binding scaffold가 쓰는 바인딩 crate를 담고 binding의 한 commit에 고정된 하나의 Cargo workspace(T1.5).
4. 문서와 이 checklist의 검사(T1.6).
5. binding이 생성하는 바인딩. 각각 binding이 생성하는 시험과 contract case로 그 언어의 표준 runner에서 시험되며(T1.7),
   이 저장소에 대한 binding의 end-to-end 검사(T1.8)와 Go 바인딩(T1.9)이 있다. binding이 모든 target에서 이 계약의
   생성을 거부하므로 셋은 binding을 기다린다.
