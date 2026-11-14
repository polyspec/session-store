<!-- doc-id: plan-rust-core -->
<!-- source-sha256: 02d08ce36f225061de4587fb0b68c738c2147668dd34008519eb4cf75b103a3b -->
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
6. 릴리스의 구조. GitHub workflow `ci`와 `release`, 그 규칙의 스크립트 시험(T1.10), 모든 manifest의 릴리스 분류(T1.11),
   추적되는 모든 경로의 소유자(T1.12), 라이선스 본문(T1.13), 그리고 origin의 branch `main`을 기다리는 0.0.1
   릴리스(T1.14)다.

## Binding runtime 선행 조건

생성된 API가 runner 개발을 시작할 수 있게 한다. 전체 binding job 통과는 그 결과이며 선행 조건이 아니다.
따라서 T1.7과 T1.9는 T1.32의 tag로 고정된 의존 graph가 해석되고 API가 생성되면 T1.30 완료를 기다리지 않고
재개한다. T1.30-9의 전체 binding job 검증은 유지하며 runner와 native 설치 입력이 존재한 뒤에만 끝날 수 있다.
이렇게 하면 없는 runner 때문에 그 개발을 허용할 job이 끝나지 못하는 순환을 없앤다.

1. T1.32에서 release된 하나의 provider graph를 해석한다. 직접 의존성은 php-serialization이며 그 tag의 crate는
   session-store와 같은 release의 binding-core를 사용해야 한다. json release만으로는 이 선행 조건을 대신하지
   못한다. Generator asset과 crate tag를 고정하고 binding manifest를 scaffold하며 native lock을 갱신한 뒤 생성과
   runtime을 검증한다. binding-core identity가 둘인 중간 graph를 commit하지 않는다.
2. 각 target API를 생성하고 T1.30-4와 T1.30-5의 해당 생성 검사를 끝낸다. T1.30-9의 pin과 service 준비는 전체 job
   결과가 있기 전에 진행할 수 있다. 생성은 native runtime 설치나 case 실행을 증명하지 않는다.
3. T1.7과 T1.9의 없는 contract runner를 작성한다. 독립적인 설치 입력은 T1.34-1(Python provider wheel),
   T1.34-2(Node provider archive), T1.34-3(tag가 있는 Go provider module과 sum)에서 준비한다. 새 Python 환경은
   index 없는 설치 전에 provider wheel을 받아야 하고 새 Node 설치는 provider archive를 명시해야 한다. 둘 다
   개발 환경에 남은 package에 의존하지 않는다. Go는 offline 시험 전에 해석한 module checksum을 기록해야 한다.
4. T1.34-4에서 적용되는 service fault case를 증명한다. 정적 확인에서 현재 Go build와 test flag는 manifest가
   선언한 fault-test를 빠뜨리지만 runtime 실패를 관찰한 것은 아니다. 각 target을 선언된 시험 feature에 맞추고
   적용되는 case ID가 skip 없이 실제 실행됐음을 증명한다. 계약에서 WASM의 socket service case를 제외한 것은
   유지하며 적용되는 case도 실행한다.
5. 실제 peer로 native target을 실행한다. Service case에는 bay와 고정한 Redis image를 포함한다. 그 뒤 T1.34와
   T1.30-9의 전체 binding job 증거를 얻는다. T1.26의 별도 bay runtime 검증은 유지한다. T1.33의 runtime과 asset
   요건이 통과한 뒤에만 T1.14에서 release 버전과 branch를 준비한다.
