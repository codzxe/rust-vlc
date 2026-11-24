# AUDIT: crates/plugin-api/src/ui.rs
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-30 |
| Lines / functions | 180 / 5 items (1 struct, 1 trait, 1 fn, 1 error type) + 2 tests |
| File verdict | SHALLOW scaffolding — trait has ZERO implementors/consumers outside its own test; the seam's two sides already diverged |
| Reachable from shipped entry point? | NO — nothing references `UiPanel`/`PanelDescriptor`/`surface_contract_violation` outside this file |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p plugin-api --lib ui` → 2 passed, exit 0; rg consumer sweeps |

## 1. One-paragraph verdict
A tidy, well-documented host contract for extension UI panels — with no host. `UiPanel` is implemented only by the test's `LyricsPanel`; `PanelDescriptor` is constructed only in this file's tests; `surface_contract_violation` has no non-test caller. Worse, the one crate that should implement this trait (plugin-host-wasm, whose `ui_register_panel` is the guest-side wire) does not reference it at all — it stores an unvalidated raw `String` (`PanelRecord`), so the typed descriptor vocabulary and the actual guest representation have already diverged before a single panel ever rendered. The "layout system" the doc leans on does not exist in any shipped app.

## 2. What actually works (verified)
— verified by `cargo test -p plugin-api --lib ui` → 2 passed, exit 0 (descriptor shape: message key not literal copy; typed violation error with source chain; idempotent teardown).
— Baseline: cargo-test-workspace.txt:4808 plugin-api section, exit 0.

## 3. Findings
F-001 [S3]
Where: crates/plugin-api/src/ui.rs:49-52 vs crates/plugin-host-wasm/src/state.rs:222-226
Code (ui.rs:49-52):
```
pub trait UiPanel: Send + Sync {
    /// The panel's registration data (surfaced to the layout system
    /// under the `ui-surface` permission).
    fn descriptor(&self) -> PanelDescriptor;
```
Code (state.rs:222-226):
```
#[derive(Debug, Clone)]
pub struct PanelRecord {
    /// The descriptor text the guest submitted (bounded).
```
What is wrong: the seam's two halves disagree on the data model. plugin-api defines a typed `PanelDescriptor {panel_id, title_key, surface}`; plugin-host-wasm's `ui_register_panel` accepts any ≤64 KiB text and stores it verbatim as `PanelRecord.descriptor` (String) — no parse, no `PanelDescriptor` construction anywhere (rg). The trait designed as "the seam the host uses to represent a registered panel to the layout system" is not the representation any host code uses. Why it matters: contract drift between the two crates that meet at this boundary, undetectable by any test (no test ties them). What correct looks like: plugin-host-wasm parses the guest descriptor into `PanelDescriptor` at the seam, or the trait carries the raw form. Confidence: HIGH — Verification: read + rg both crates.

F-002 [S4]
Where: crates/plugin-api/src/ui.rs:13-14
Code:
```
//! Its `ui-surface` permission is the gate, and escalation attempts
//! trap the instance (§5.3).
```
What is wrong: doc-vs-implementation drift on the security semantics: the implemented behavior (imports.rs) is a typed `E_PERM` per attempt with the instance faulted only at the threshold (3) — imports.rs:8-18 itself records the §2.2-vs-§5.3 trap-vs-status [PROPOSAL]. This file asserts the trapping form as settled fact. Confidence: HIGH — Verification: read (both files).

F-003 [S4]
Where: crates/plugin-api/src/ui.rs:75-87
Code:
```
pub fn surface_contract_violation(panel: &str, what: &str) -> EngineError {
```
What is wrong: dead helper (no non-test caller); `panel`/`what` are cloned unbounded into the error source (no length cap — the crate's own bounded-descriptor discipline caps at 64 KiB elsewhere). Confidence: HIGH — Verification: rg + read.

F-004 [S4]
Where: crates/plugin-api/src/ui.rs:54-61
Code:
```
/// Deliver one command-layer event to the panel (serialized by
/// the host; the panel's guest protocol defines the encoding).
```
What is wrong: "the panel's guest protocol defines the encoding" — there is no guest protocol: plugin-host-wasm's v1 catalog has exactly one UI import (`ui_register_panel`, api.rs V1_IMPORTS) and no event-delivery export path; nothing can ever call `on_event` from a real guest. The contract references machinery that does not exist in either crate. Confidence: HIGH — Verification: read + api.rs catalog.

## 4. Per-function table (>100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PanelDescriptor (type) | 22-34 | REAL / dead | S3 | never constructed outside tests (F-001) |
| UiPanel::descriptor | 49-52 | REAL (contract) | S3 | no implementors (F-001) |
| UiPanel::on_event | 54-61 | REAL (contract) | S4 | no event path exists (F-004) |
| UiPanel::teardown | 63-69 | REAL (contract) | — | idempotence contract; toy-impl tested only |
| surface_contract_violation | 75-87 | REAL / dead | S4 | uncalled, unbounded clones (F-003) |
| Violation Display/Error | 89-104 | REAL | — | fine |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| descriptor_is_data_not_ui_copy | YES (shape only) | asserts surface/title_key prefixes on the toy's own data — proves nothing about any host (ran: ok) |
| violations_are_typed_and_teardown_idempotent | YES (shape only) | stage + source-chain + double-teardown on the toy (ran: ok) — no real panel can ever reach this path |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a panel renders through the layout system and issues commands" | ui.rs:10-12 | FALSE in shipped builds | no layout-system consumer of PanelDescriptor exists (rg); panels render nowhere (imports.rs F-003: write-only sink) |
| "The host … registers the descriptor with the layout system, routes command envelopes" | ui.rs:38-40 | FALSE/unimplemented | zero UiPanel implementors; no command routing to panels exists |
| "escalation attempts trap the instance (§5.3)" | ui.rs:13-14 | PARTLY — typed status below threshold | imports.rs escalation_or_status + the [PROPOSAL] the other file records (F-002) |
| "Extension-only in v1: no builtin is ever a ui-panel" | ui.rs:4-5 | REAL | consistent with kind.rs (UiPanel is an extension kind); trivially true since no extension can load either |

## 7. Dead / duplicated / unreachable code
The whole contract is dead repo-wide (F-001/F-003). Duplicate representation risk vs plugin-host-wasm `PanelRecord`.

## 8. Dependencies & flags
media-types (L0). No features. No unsafe. plugin-api ships in the release lane (via demux), so this dead contract compiles into every engine binary.

## 9. Open questions for the next agent
- doc 08 (layout/UI) agents: is there ANY doc-promised "layout system" registration API for extension panels? If a user-facing doc promises extension panels, the whole chain (ui.rs trait → host impl → layout registration → render) is PAPER-ONLY end to end.
- If the Lead resolves the [PROPOSAL: placement], one of `PanelDescriptor`/`PanelRecord` dies — today both live.
