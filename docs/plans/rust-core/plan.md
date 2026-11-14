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
6. The structure of a release: the GitHub workflows `ci` and `release` and the script test of their rules (T1.10), the
   classification of every manifest for the release (T1.11), the owners of every tracked path (T1.12), the text of the
   license (T1.13) and the release 0.0.1 (T1.14), which waits for the branch `main` of origin.

## Binding runtime prerequisites

The generated APIs unblock runner development; a passing full binding job is its result, not its prerequisite.
T1.7 and T1.9 therefore resume when the tagged dependency graph of T1.32 resolves and their APIs generate, without
waiting for T1.30 to finish. T1.30-9 retains its full binding-job verification, which can finish only after the runners
and their native installation inputs exist. This avoids a cycle in which missing runners prevent the job that would
permit their development.

1. Resolve one released provider graph in T1.32. The direct dependency is php-serialization, whose tagged crate must
   use the same released binding-core as session-store. A json release alone does not replace that prerequisite.
   Pin the generator asset and the crate tags, scaffold the binding manifests and update the native locks before
   generation and runtime verification. Do not commit an intermediate graph with two binding-core identities.
2. Generate each target API and complete the applicable generation checks of T1.30-4 and T1.30-5. The pinning and
   service preparation of T1.30-9 can proceed before its full-job result exists. Generation proves neither native
   runtime installation nor execution of a case.
3. Write the missing contract runners of T1.7 and T1.9. Prepare their independent installation inputs in T1.34-1
   (Python provider wheels), T1.34-2 (Node provider archives) and T1.34-3 (the tagged Go provider module and sums).
   A fresh Python environment must receive its provider wheel before an installation without an index, and a fresh
   Node installation must name its provider archives; neither may rely on packages left in a developer environment.
   Go must record the resolved module checksums before its offline test.
4. Prove the applicable service fault cases in T1.34-4. Static inspection shows that the current Go build and test
   flags omit fault-test even though its manifest declares it; this is not an observed runtime failure. Align each
   target with its declared test features and prove that its applicable case identities actually run without skips.
   Keep the contract's exclusion of socket service cases from WASM and run its applicable cases too.
5. Run the native targets with their real peers, including bay and the pinned Redis image for service cases, then
   obtain the full binding-job evidence for T1.34 and T1.30-9. T1.26 retains its separate bay runtime verification.
   Only after the runtime and asset requirements of T1.33 pass does T1.14 prepare the version and branch for release.
