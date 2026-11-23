# AUDIT: crates/commands/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W02-A02 |
| Date | 2025-06-27 |
| Lines / functions | 72 / 4 modules + 4 re-export groups + prelude |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (crate root; lumen-cli release lane depends on `commands` — apps/cli/Cargo.toml:52; every re-export imported by real hosts: apps/web/src/protocol.rs:46, apps/desktop/src/dispatch.rs:62, apps/cli/src/dispatch.rs:30, apps/control-server) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | full Read (72 lines); `rg` for prelude users repo-wide (exit 0, zero hits for `commands::prelude`); `rg` for `use commands::{` consumers (15+ files); `sed -n` doc 05 §1.4 (C1–C8) |

## 1. One-paragraph verdict
A re-export facade that does its job: the four modules (catalog, correlation, envelope, error) are real, and every root re-export is imported by shipped hosts — nothing here forwards to nothing. The defects are documentation debt and one dead module: the crate doc still advertises "the ~61-command / 23-event catalog" when the shipped table and committed checksum say 65/26; it claims the correlation module hosts "the host-side half of rules C1–C8" when correlation implements only C6's staleness half plus seq/echo ordering (C1/C3/C4/C5/C7/C8's host halves live in the hosts/UI per doc 05 §1.4); and `pub mod prelude` has zero users in the entire repo while claiming to be "the full 1.D3 + 1.D4 surface" — which it is not, since it omits `WireRequest`/`WireResponse`/`WireEvent` that the root re-export includes.

## 2. What actually works (verified)
- `rg "use commands::\{"` -> 15+ real import sites across apps/web (protocol.rs:46, host.rs:87, open.rs:24), apps/desktop (dispatch.rs:62, shell.rs:40, host.rs:20), apps/cli (dispatch.rs:30, run.rs:51, containers/*, host.rs:86, demux_host.rs:55, bench.rs:62) — the facade is live in the shipped CLI and desktop hosts.
- The re-exported names resolve: baseline `cargo test --workspace` compiles and passes all `commands` package tests (spec 8/8 at :1459-1466, correlation 11/11 at :1472-1482, catalog_shape at :1521-1525), which exercise these exact paths.
- Module docs' architecture claim ("defined once here, re-exported (never re-implemented) by every host") spot-verified: hosts import `commands::{...}` types rather than redefining them (import list above).

## 3. Findings
F-001 [S3]
Where: crates/commands/src/lib.rs:62-72
Code:
```
/// Prelude: the full 1.D3 + 1.D4 surface in one `use`.
pub mod prelude {
```
What is wrong: Two defects. (a) Dead code: zero users — `rg "prelude" crates/ apps/ tools/ --type rust` shows the preludes of media_types, codec_api and plugin_api all have importers (`use media_types::prelude::*;` etc.) but `commands::prelude` has none. (b) False claim: "the full 1.D3 + 1.D4 surface" omits `WireRequest`/`WireResponse`/`WireEvent`, which the root re-export (L52-55) includes — the prelude is a strict subset, not the full surface. Contrast: plugin-api ships a test pinning its prelude (`fn prelude_reexports_the_whole_contract`, plugin-api/src/lib.rs:132); this crate ships neither users nor a pin. What correct looks like: delete the prelude or add the Wire types and a pinning test. Confidence: HIGH — Verification: rg repo-wide (exit 0, no matches for `commands::prelude`).

F-002 [S4]
Where: crates/commands/src/lib.rs:30
Code:
```
//! - [`catalog`]   — the ~61-command / 23-event catalog: stable ids
```
What is wrong: Stale counts. The shipped table holds 65 commands and 26 events (spec.rs tests at 1026-1040 assert 65; committed artifact schemas/generated/commands.checksum.txt says "counts: 65 commands / 26 events"). The "~" does not cover a 4-command/3-event shortfall (issue #32 trio + w31.1 bind + issue #42 trio landed after this comment). Confidence: HIGH — Verification: read + committed checksum + spec.rs tests.

F-003 [S4]
Where: crates/commands/src/lib.rs:41-43
Code:
```
//! [`CommandOutcome::Accepted`] and complete via events; class-S commands
//! answer [`CommandOutcome::Completed`]. The correlation module is where
//! the host-side half of rules C1–C8 lives (the staleness filter and the
```
What is wrong: Overbroad. Doc 05 §1.4 defines C1–C8 as UI/host truth rules; correlation.rs implements the host-side half of C6 (staleness) plus §4.3 seq/echo checks. The host halves of C1 (pending affordances), C3 (await completion events), C4 (state only from events), C5 (error surfacing), C7 (scrub ghost), C8 (self-transition handling) live in the hosts/UI, not in this crate's correlation module. The parenthetical ("the staleness filter and the ordering checks") gives partial cover, but the sentence asserts C1–C8 residency. Confidence: HIGH — Verification: read of correlation.rs + doc 05 §1.4 (sed).

F-004 [S4]
Where: crates/commands/src/lib.rs:24-25
Code:
```
//! `seq` ordering, and the delivery-log validator that
//! checks the §4.3 invariants on an observed stream.
```
What is wrong: Echoes correlation.rs's overstated invariant list: the delivery-log validator checks seq monotonicity and echo validity but NOT the origin/causality invariant its own doc claims (see correlation.rs audit F-001 — `answered` written, never read). This crate-level summary propagates the unimplemented claim. Confidence: HIGH — Verification: read + correlation.rs audit evidence.

## 4. Per-function table (file < 100 LOC; inventory for completeness)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crate docs | 1-45 | REAL | S4 | stale counts (F-002), C1–C8 overclaim (F-003), §4.3 echo (F-004) |
| module decls (catalog/correlation/envelope/error) | 47-50 | REAL | — | all four real and tested (baseline) |
| root re-exports | 52-60 | REAL | — | all names imported by shipped hosts (rg) |
| `pub mod prelude` | 62-72 | UNREACHABLE | S3 | zero users; not the "full surface" (F-001) |

## 5. Test honesty
No tests in this file. The re-export surface is exercised indirectly by every consumer crate's tests (baseline: commands package 19+ tests green; consumer hosts' tests green).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One command layer … consumed by the same envelope … by every host" | lib.rs:3-7 | REAL | rg import sites in web/desktop/cli/control-server |
| "the ~61-command / 23-event catalog" | lib.rs:30 | FALSE | 65/26 (spec.rs tests 1026-1040; committed checksum counts line) |
| "the delivery-log validator that checks the §4.3 invariants" | lib.rs:24-25 | SHALLOW | only 2 of its own 3 claimed invariants implemented (correlation.rs F-001) |
| "The correlation module is where the host-side half of rules C1–C8 lives" | lib.rs:42-43 | SHALLOW | only C6 + ordering there; C1/C3/C4/C5/C7/C8 host halves are in hosts/UI (doc 05 §1.4) |
| "Prelude: the full 1.D3 + 1.D4 surface in one `use`" | lib.rs:62 | FALSE | omits Wire* types; zero users (F-001) |

## 7. Dead / duplicated / unreachable code
- `pub mod prelude` (62-72): dead, zero importers repo-wide (F-001).

## 8. Dependencies & flags
Crate `commands` v0.1.0, edition 2024, publish=false. Deps: serde, serde_json, schemars (catalog wire forms + codegen). No feature gates, no unsafe, no `#[allow]`. Bin target catalog-gen declared separately (audited in its own file).

## 9. Open questions for the next agent
- `envelope.rs` and `error.rs` are the two modules of this crate NOT in my assignment — the V8 redaction rule (error construction) and the envelope serde forms deserve their own hostile pass; my audits cite them only as dependencies.
- If the prelude is kept, should it re-export the Wire aliases? Currently only the root does — pick one surface.
