# AUDIT: crates/plugin-api/src/discovery.rs
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-02-14 |
| Lines / functions | 199 / 5 (1 trait, 2 types, 1 helper + Display; 3 tests around 1 test double) |
| File verdict | UNREACHABLE (real contract code, zero non-test implementations, zero consumers) |
| Reachable from shipped entry point? | NO — no crate imports `plugin_api::discovery`; the only `impl ServiceDiscovery` is the in-file test double; no wasm import module exposes it (v1 module set is host/log/fs/http/ui/devices — plugin-host-wasm/src/api.rs:30-37); the parallel real domain trait lives in crates/device-discovery/src/device.rs with a DIFFERENT shape. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg (ServiceDiscovery/DiscoveryChange/claim_mismatch consumers tree-wide; manifest `discovers`); sed quote check; baseline citation |

## 1. One-paragraph verdict
A staged contract that nothing stages into: the `ServiceDiscovery` trait here (snapshot/changes polling model) has exactly one implementation — the in-file test double — and zero importers anywhere in the tree. Meanwhile the real domain trait with the same name lives in crates/device-discovery (a different shape: one `discover()` pass, one real impl at volume.rs:228), so the tree carries TWO divergent `ServiceDiscovery` traits. The module doc admits the placement is `[PROPOSAL: staged here]`, and the code itself is clean (typed error with a source chain, sensible Send+Sync bounds, honest "absence is data" contract), but as committed this is scaffolding with a mock-only test suite: the two behavior tests exercise the test double's own logic, not any production code path. Only `claim_mismatch_error_shape` tests real shipped code (the helper).

## 2. What actually works (verified)
— baseline cargo-test-workspace.txt: `discovery::tests::absence_is_data`, `claim_mismatch_error_shape`, `snapshot_and_change_drain` all `ok` (plugin-api unittests block).
— `claim_mismatch` is real, consistent code: stage `Discovery`, kind `Corrupt{InvalidStructure}` + source chain carrying the claimed protocol — the same manifest-lie convention as `codec_claim_mismatch` (codec.rs:62-75).
— The manifest side is real: `discovers: Option<Vec<String>>` is a parsed, validated, kind-gated permission field (manifest.rs:498, 519, 547, 571, 633) — so the capability vocabulary this trait's doc references exists.

## 3. Findings
F-001 [S3]
Where: crates/plugin-api/src/discovery.rs:47-70 vs crates/device-discovery/src/device.rs:32-40
Code:
```
pub trait ServiceDiscovery: Send + Sync {
    fn discovers(&self) -> Vec<&'static str>;
    fn snapshot(&self) -> Result<Vec<Device>, EngineError>;
    fn changes(&self) -> Result<Option<Vec<DiscoveryChange>>, EngineError>;
```
What is wrong: this trait has zero non-test implementations and zero consumers (rg: `plugin_api::discovery` imported nowhere; the only impl is `UpnpMock` at L124). The tree's OTHER `ServiceDiscovery` (device-discovery crate, `discover()` single-pass shape) is the one with a real implementation (volume.rs:228 — per W10-A02's audit of that crate) — and that crate is itself linked by nothing. Two same-named traits with divergent shapes is a layering trap: a future wiring pass must pick one and delete or adapt the other.
Why it matters: the doc comment says "Domain trait: `device_discovery::ServiceDiscovery` (L2); this minimal host surface is staged here" — implying one domain trait mirrored here, but the shapes are NOT mirrors (snapshot/changes polling vs discover pass), so neither implements the other's contract.
What correct looks like: one trait — either re-export the device-discovery one from plugin-api or delete this staging until the plugin lane (issue #42) wires discovery.
Confidence: HIGH — Verification: read (rg consumers; device.rs read).

F-002 [S4]
Where: crates/plugin-api/src/discovery.rs:143-187
Code:
```
    #[test]
    fn snapshot_and_change_drain() {
        let provider = UpnpMock {
```
What is wrong: `snapshot_and_change_drain` and `absence_is_data` test the in-file mock's own clone/drain logic — a test double of the thing itself (checklist 6.4). They cannot fail against any production defect because no production code participates. Only `claim_mismatch_error_shape` (L189-198) exercises real code.
Confidence: HIGH — Verification: read.
What correct looks like: once a real impl exists, point the tests at it; today, note the tautology honestly (this audit does).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Device (struct) | 25-36 | REAL | — | Plain data; uri is a String not SourceUri (unvalidated here — the Access layer parses it later; weaker than device-discovery's validated SourceUri Device). |
| ServiceDiscovery (trait) | 47-70 | UNREACHABLE | S3 | Zero real impls (F-001). |
| DiscoveryChange | 74-82 | UNREACHABLE | — | Added/Removed vocabulary; unused outside the mock. |
| claim_mismatch | 86-97 | REAL | — | Typed, source-chained; consistent with codec.rs precedent. |
| Mismatch + Display + Error | 99-113 | REAL | — | Carries the claimed protocol in the sentence. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| snapshot_and_change_drain | NO (nothing production participates) | Mock-driven; verifies the mock. |
| absence_is_data | NO (same) | Asserts the mock returns empty vec. |
| claim_mismatch_error_shape | YES | Real helper: stage, source chain, claim text. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Domain trait: `device_discovery::ServiceDiscovery` (L2); this minimal host surface is staged here" | L3-5 | SHALLOW | The two traits are divergent shapes, not a staging of one domain trait (F-001). |
| "Allowed as a WASM extension in v1 (§1.4)" | L5 | FALSE-in-practice | No v1 wasm import module serves discovery (api.rs module table L30-37); a v1 extension cannot implement this trait through the sandbox. |
| "Network protocols a discovery impl speaks (UPnP, mDNS) are manifest capabilities (`discovers`), enforced by the sandbox for extensions" | L11-13 | SHALLOW | `discovers` is a real manifest field (manifest.rs:498+), but no sandbox enforcement exists for it — the plugin-host permission set (fs/http/ui/devices, api.rs:113-124) has no discovery permission. |

## 7. Dead / duplicated / unreachable code
The entire trait + DiscoveryChange are unreachable (F-001). `Device.uri` as String duplicates (weaker than) device-discovery's validated Device. No other duplication.

## 8. Dependencies & flags
media-types only. No unsafe, no features, std-only.

## 9. Open questions for the next agent
- Issue #42 (device-discovery wiring) — when it lands, which trait survives? Flag for the Lead: deleting one of the two `ServiceDiscovery` definitions.
- Should `claim_mismatch` share a constructor with `codec_claim_mismatch` (identical shape modulo stage/string)?
