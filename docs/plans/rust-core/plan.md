<!-- doc-id: plan-rust-core -->
# Rust core

[한국어](plan.ko.md)

session-store keeps the sessions of PHP servers by the rules of [the specification](../../spec/sessions.md): a store in
memory, a store of files and a store of Redis, each of which reads and writes the records of PHP sessions. Every rule is
written once, in the Rust crates of `packages/core/rust`, `packages/file/rust`, `packages/redis/rust` and
`packages/rust`, and tested once, by the cases of `contracts/`. The packages of PHP, Python, Node, the browser (WASM) and
Go hold no code of their own: binding generates each of them from the interface contract `packages/rust/interface.json`,
and binding scaffold writes their binding crates, so a rule never has a second copy that can differ from the first.

The checklist follows the order in which each step needs the one before it:

1. The specification and its cases (T1.1), then the core that passes them (T1.2), whose store of Redis is tested
   against real servers in Apple container (T1.3).
2. The interface contract of the core for the five targets of binding, with a case for every kind and a fault injection
   point for every failure that no input reaches (T1.4).
3. One Cargo workspace with the core and the binding crates that binding scaffold writes, pinned to one commit of
   binding (T1.5).
4. The check of the documents and of this checklist (T1.6).
5. The bindings that binding generates, each tested by the tests that binding generates and by the contract cases on
   the standard runner of its language (T1.7), the end-to-end check of binding on this repository (T1.8) and the Go
   binding (T1.9). They wait for binding, which refuses to generate this contract for every target.
