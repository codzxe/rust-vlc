# AUDIT: crates/plugin-api/src/ui.rs
| Field | Value |
|---|---|
| Auditor | W12-A02 |
| Date | 2026-11-21 |
| Lines / functions | 180 / 8 (1 struct + 1 trait with 3 methods + 1 helper fn + 1 private type with 2 impls + 1 fixture impl + 2 tests) |
| File verdict | UNREACHABLE (a typed host contract the host itself never touches — the real panel representation is a raw string) |
| Reachable from shipped entry point? | NO. Repo-wide: `UiPanel` implemented only by the in-file test fixture `LyricsPanel` (ui.rs:114); `PanelDescriptor` constructed only by that fixture; `surface_contract_violation` called only by ui.rs's own tests. The plugin host's actual panel surface is `state.rs:224-227 PanelRecord { descriptor: String }` ("The descriptor text the guest submitted"), filled verbatim by `imports.rs:848-852` — never parsed, never namespaced, never mapped onto `PanelDescriptor`. The shipped binaries that link plugin-api (apps/cli, apps/web) reference no ui symbols. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg `UiPanel\|PanelDescriptor\|surface_contract_violation\|PanelRecord` over crates/ apps/ tests/ tools/ → kind/manifest/limits references only + the state.rs/imports.rs string record; read tests/plugin_examples_conformance.rs:317-380 (the real panel flow: JSON descriptor string, `on_command` export); read state.rs:223-250; baseline: `ui::tests::*` 2/2 ok (orchrun-test-workspace-nofailfast.log.txt:4470-4471); sed quote verification. |

## 1. One-paragraph verdict
The ui-panel contract is typed paperwork beside an untyped reality: `PanelDescriptor`/`UiPanel` define a clean seam (descriptor data, event delivery, idempotent teardown, typed violations) that no host shim implements, no layout system consumes, and no shipped binary can reach — while the actual plugin host stores whatever descriptor string a guest pushes as an opaque `PanelRecord { descriptor: String }` and the repo's flagship ui-panel example registers `"panel_id":"main"`, bare and un-namespaced, directly contradicting this file's documented "namespaced under the plugin id by the host" invariant. The trait's own header admits the seam's placement is [PROPOSAL]; the rest of the file's prose asserts behavior (host namespacing, security-event logging) that exists nowhere in code.

## 2. What actually works (verified)
— Baseline `ui::tests::{descriptor_is_data_and_not_ui_copy → descriptor_is_data_not_ui_copy, violations_are_typed_and_teardown_idempotent} ... ok` (orchrun-test-workspace-nofailfast.log.txt:4470-4471) — the trait is usable, the violation error carries its `Violation` source chain, teardown idempotence pattern demonstrated.
— The typed error helper produces a well-formed `EngineError` (stage Plugin) with a `Display`-able source — asserted at ui.rs:165-170.
— The real panel lane works WITHOUT this file: the ui-panel example extension registers its panel through the `ui_register_panel` import and answers `on_command` events end to end (conformance test green, baseline :4575) — proof the subsystem this contract claims to model runs on a different, stringly-typed foundation.

## 3. Findings

### F-001 [S3] The "seam the host uses" is not used by the host: zero implementers, zero consumers, and the host's real representation bypasses the typed contract
Where: crates/plugin-api/src/ui.rs:4-8 (the claim); crates/plugin-host-wasm/src/state.rs:224-227 and imports.rs:848-852 (the reality)
Code:
```rust
//! **Extension-only in v1** (§1.2 row 15): no builtin is ever a
//! ui-panel. The guest-side wire protocol lives in
//! `plugin-host-wasm` (L6, doc 06 §5.5); this host trait is the seam
//! the host uses to represent a registered panel to the layout system
```
What is wrong: No host shim implements `UiPanel`; nothing constructs `PanelDescriptor` from guest data; no "layout system" exists to receive it. The host's panel state is `PanelRecord { descriptor: String }` (state.rs:224-227), pushed verbatim from the guest's bytes at imports.rs:848-852 with no parsing or validation, and read back only by `instance.panels()` (instance.rs:150) for tests. The one real registered panel in the repository (the example extension, conformance test :337-353) is asserted by string containment (`descriptor.contains("\"panel_id\":\"main\"")`) — the test comment itself admits "v1 has no JSON parser in this crate's dependency closure … honest containment, not a fabricated parse".
Why it matters: Doc 06 §1.2 row 15's host-side story ("the seam the host uses to represent a registered panel to the layout system") is fiction: the typed contract and the working lane share no code, so any future wiring must either build the shim from scratch or keep two divergent panel models. Every consumer of the shipped `lumen` binary is unaffected — none of this compiles into a reachable path.
What correct looks like: A host-side adapter that parses the bounded descriptor string into `PanelDescriptor` (namespacing `panel_id` under the plugin id per the doc) and implements `UiPanel` around the instance — or the module is marked unwired alongside its [PROPOSAL] placement marker.
Confidence: HIGH — Verification: read + repo-wide rg (commands above).
### F-002 [S4] `surface_contract_violation`'s doc promises security-event logging that no code performs, and its error kind mislabels a protocol violation as access denial
Where: crates/plugin-api/src/ui.rs:72-82
Code:
```rust
/// Typed panel error: the guest protocol violated the surface
/// contract (e.g. attempted an engine call outside the command
/// envelope). Logged as a security event per doc 06 §5.3.
pub fn surface_contract_violation(panel: &str, what: &str) -> EngineError {
```
What is wrong: The helper only builds an `EngineError` — it logs nothing, and it has no caller that logs either (the §5.3 ledger path, `deny()`/`record_escalation` in plugin-host-wasm, is never invoked from here). Additionally the error kind is `ErrorKind::Inaccessible { reason: AccessDenied::PermissionDenied }` — a malformed guest event or out-of-envelope call is a protocol violation, not an access-denied condition; the security-ledger taxonomy in the host treats those as distinct (escalation events vs typed statuses).
Why it matters: A future implementer relying on this doc would believe violations are already ledged; they are not. The kind mismatch would surface as wrong error-classification in any UI/error surface that distinguishes permission errors from contract faults.
What correct looks like: The helper takes (or documents) the ledger-recalling responsibility explicitly, or the doc says "the CALLER must record this via the §5.3 ledger"; and the kind is a contract/protocol category, not Inaccessible.
Confidence: HIGH — Verification: read; cross-checked the host's real escalation path (imports.rs:199-215, my part1 audit).
### F-003 [S4] `PanelDescriptor.panel_id`'s documented host-namespacing invariant is contradicted by the repo's only real registered panel
Where: crates/plugin-api/src/ui.rs:23-26 (the invariant); tests/plugin_examples_conformance.rs:344-348 (the counterexample)
Code:
```rust
pub struct PanelDescriptor {
    /// Stable panel id (namespaced under the plugin id by the host,
    /// e.g. `ext.acme.lyrics-panel:main`).
    pub panel_id: String,
```
What is wrong: The doc says the host namespaces panel ids under the plugin id; the actual example extension's registered descriptor contains `"panel_id":"main"` — bare, no `ext.example.panel:` prefix — and the host (which never post-processes the string, F-001) enforces nothing. The invariant is violated by the flagship artifact the conformance suite celebrates.
Why it matters: Panel-id collision policy (two extensions both registering `main`) is exactly what namespacing exists to prevent; the documented rule and the demonstrated behavior disagree, and no code arbitrates.
What correct looks like: The host prefixes the stored panel id (or the descriptor contract requires the guest to namespace and the host validates), and the example + conformance test assert the namespaced form.
Confidence: HIGH — Verification: read of both sites (conformance test quote verified above).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PanelDescriptor` (struct) | 22-34 | REAL | F-001, F-003 | typed fields never built from real data; namespacing invariant unenforced/violated. |
| `UiPanel::descriptor` | 50-52 | REAL | F-001 | contract only; zero implementers. |
| `UiPanel::on_event` | 54-61 | REAL | F-001 | "serialized by the host" — no host serializer exists; the example's `on_command(ptr,len)` guest export is invoked directly by tests instead. |
| `UiPanel::teardown` | 63-69 | REAL | F-001 | idempotence demonstrated only by the fixture. |
| `surface_contract_violation` | 72-87 | REAL | F-002 | well-formed error, dead helper, logging claim unimplemented. |
| `Violation` + `Display`/`Error` | 89-104 | REAL | — | correct source-chain plumbing (tested). |
| `tests::LyricsPanel` (fixture) | 110-138 | REAL | — | the repo's sole `UiPanel` impl. |
| `tests::descriptor_is_data_not_ui_copy` / `violations_are_typed_and_teardown_idempotent` | 140-179 | REAL | — | see §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| descriptor_is_data_not_ui_copy | YES | exact field asserts through the trait (trivial but real). |
| violations_are_typed_and_teardown_idempotent | YES | stage assert, source-chain assert, double-teardown survival. |
| (both) | — | Caveat: fixture-only — with zero external implementers/consumers, no test anywhere fails if the real panel lane diverges from this contract (which it already has, F-001/F-003). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "this host trait is the seam the host uses to represent a registered panel to the layout system" | module doc 4-8; doc 06 §1.2 row 15 | FALSE | host uses `PanelRecord { descriptor: String }` (state.rs:224-227); no UiPanel impl, no layout consumer (F-001). |
| "panel_id (namespaced under the plugin id by the host …)" | PanelDescriptor doc 24-25 | FALSE | example registers `"panel_id":"main"` bare; host does no namespacing (F-003). |
| "Logged as a security event per doc 06 §5.3" | helper doc 72-74 | FALSE | no logging anywhere in the call graph (F-002). |
| "Extension-only in v1: no builtin is ever a ui-panel" | module doc 4 | REAL | no builtin ui-panel exists anywhere (rg); consistent. |
| "escalation attempts trap the instance (§5.3)" | module doc 13-14 | REAL (mechanism) / UNREACHABLE (this path) | the trap machinery exists in the host (my imports.rs#part1 audit, escalation_or_status) but nothing connects ui-contract violations to it. |
| matrix `plugin-wasm-extensions` = not_started | feature-matrix.toml:2329 | REAL (shipping view) | consistent with the whole lane being unshipped. |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from every shipped entry point; exported via `lib.rs:100` and the prelude (`lib.rs:116`) for consumers that do not exist.
- No duplication in-tree: nothing else models panels typedly — which is precisely the problem (the stringly-typed `PanelRecord` is the working duplicate of this contract's role).

## 8. Dependencies & flags
- `media-types` (EngineError/PipelineStage/ErrorKind). No feature gates, no unsafe, no unwrap/expect/panic in production code. Test fixture uses atomics for idempotence proof — appropriate.

## 9. Open questions for the next agent
- A03 (state.rs owner): `PanelRecord` is a bare string with only an 8-panel cap (imports.rs:839-847) and a 64 KiB size bound — no structural validation; if the panel lane is ever wired, decide descriptor schema + namespacing there (F-001/F-003).
- A01 (example manifests): does the ui-panel example's manifest `surfaces = ["side-panel"]` claim get checked against the registered descriptor's `"surface":"side-panel"` anywhere? The conformance test asserts containment as a courtesy, not as a capability-honesty gate (contrast `provides`-subset rule enforcement, which is also absent).
- UI wave: does ui/ have any panel-hosting surface (a layout region that could consume `PanelDescriptor`)? If not, the "layout system" referent of this contract does not exist at either end.
