# AUDIT: crates/commands/src/catalog/mod.rs
| Field | Value |
|---|---|
| Auditor | W02-A05 |
| Date | 2026-11-21 |
| Lines / functions | 56 / 0 functions (6 module declarations + 19 re-exports) |
| File verdict | REAL (the module hub of the catalog; every host consumes it) |
| Reachable from shipped entry point? | YES — chain: `lumen` CLI → `apps/cli/src/dispatch.rs:30` (`use commands::{Command, CommandId, CommandResult, WireRequest, WireResponse}`) → this module's re-exports; same for desktop (`apps/desktop/src/dispatch.rs:62`), web (`apps/web/src/host.rs`, 62 `commands::` uses), control-server (via `control-protocol`, Cargo.toml:32). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg `catalog::(CatalogDocument|UnknownId|CommandListing|CommandSpec|EventSpec|PayloadDesc|CommandTarget|CompletionClass|catalog_data|event_catalog_data|WireRequest|WireResponse|WireEvent)` worktree → only `catalog_data` used (apps/cli/src/host.rs:1666, apps/web/src/host.rs:747); rg re-export target definitions (command.rs:675, spec.rs:41-151 — all exist); rg `commands::` in apps/desktop, apps/web, apps/cli (all consume); sed verification of quotes |

## 1. One-paragraph verdict
A pure structure file: six module declarations and nineteen re-exports, every target verified to exist (baseline build #1 + clippy #5 both CLEAN would fail otherwise), every declared submodule audited or cross-referenced in this wave. The hub claim in the module doc — every host consumes THIS catalog — is verified for all five named hosts. Nothing is fake here; the only defects are cosmetic: nine re-exports with zero consumers anywhere in the worktree, and the crate-root doc still advertising "~61-command / 23-event" when the pinned, checksummed, test-enforced count is 65/26.

## 2. What actually works (verified)
— verified by `audit/runs/baseline/build-release-workspace.log.txt` (exit 0) + `clippy-workspace.log.txt` (exit 0, `-D warnings`): every re-export target exists and is used or at least compiles; no broken paths.
— verified by rg over `apps/`: CLI (`apps/cli/src/dispatch.rs:30`), desktop (`apps/desktop/src/dispatch.rs:62`, host.rs:19-20, files.rs:31), web (`apps/web/src/host.rs:747` etc.), control-server (via `crates/control-protocol/Cargo.toml:32`) — the "every host" claim (mod.rs:22-23) holds for all five consumers named.
— verified by `audit/runs/baseline/test-workspace.log.txt:1303-1410`: the `commands` package's whole test battery (unit + catalog_drift + catalog_emission_ledger + catalog_shape + wire_forms) ran **ok** — this hub's six submodules are all exercised.
— verified by `.github/workflows/ci.yml:280` (`cargo run -p xtask --locked -- codegen --check`): the "CI fails on drift" claim (mod.rs:28-29) is real wiring.

## 3. Findings
### F-001 [S4]
Where: crates/commands/src/catalog/mod.rs:38,43,48-53,55
Code:
```rust
pub use codegen::CatalogDocument;
pub use command::Command;
pub use command::CommandListing;
pub use ids::CommandId;
pub use ids::EventId;
pub use ids::UnknownId;
pub use spec::CommandSpec;
```
What is wrong: Nine of the nineteen re-exports have zero consumers via the `catalog::` path anywhere in the worktree (rg over all of `apps/`, `crates/`, `tools/`): `CatalogDocument`, `CommandListing`, `UnknownId`, `CommandSpec`, `CommandTarget`, `CompletionClass`, `EventSpec`, `PayloadDesc`, `event_catalog_data`. (All other re-exports are load-bearing: the nine duplicated at `lib.rs:52-55` / prelude feed the crate's top-level API; `catalog_data` is used by both the CLI and web hosts; the codegen aliases feed lib.rs.) Note consumers reach the spec items via the LONGER path `commands::catalog::spec::X` (e.g. `catalog_shape.rs:20-22`), so the short re-exports buy nothing.
Why it matters: Unused public API surface is untested surface — nothing fails if these drift or disappear, and a reader auditing "the catalog API" must check two paths for every name.
What correct looks like: Either delete the nine unused re-exports, or make the spec/codegen consumers actually use them (one consistent path per name).
Confidence: HIGH — Verification: rg (worktree-wide, both path forms checked).
### F-002 [S4]
Where: crates/commands/src/lib.rs:30 (the crate-root doc for this module family; mod.rs itself states no counts)
Code:
```rust
//! - [`catalog`]   — the ~61-command / 23-event catalog: stable ids
```
What is wrong: The count is stale. The enforced truth is 65 commands / 26 events — pinned by `catalog_shape.rs:99-108` (asserts 65/26, naming the additions: issue #32 bookmark trio, w31.1 frame-channel bind, issue #42 event trio) and by the committed checksum line `counts: 65 commands / 26 events` (`schemas/generated/commands.checksum.txt:5`).
Why it matters: The crate root is the first doc a reader sees for the catalog; "~" hedges the drift but 61→65 / 23→26 is four commands and three events of unexplained difference to anyone cross-checking against the schema or `system.list-commands` output.
What correct looks like: Either the count updated to 65/26 (with the addition references), or the doc states no count and points at the checksum file.
Confidence: HIGH — Verification: read + test assertions (baseline ok, log 1388-1396) + checksum file.

## 4. Per-function table (structure items — no functions in this file)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pub mod codegen` | 31 | REAL | — | Audited this wave (W02-A05 file 1). |
| `pub mod command` | 32 | REAL | — | Payload enums; round-trips in wire_forms.rs (baseline ok). |
| `pub mod event` | 33 | REAL | — | Round-trips + emission ledger (baseline ok). |
| `pub mod ids` | 34 | REAL | — | `CommandId`/`EventId`/`UnknownId` + FromStr; grammar-pinned by catalog_shape.rs:137-183. |
| `pub mod spec` | 35 | REAL | — | Metadata tables + FNV-1a checksum; consumed by codegen + both hosts. |
| `pub mod types` | 36 | REAL | — | Audited this wave (part1 by W02-A02, part2 by W02-A05 file 2). |
| re-exports ×19 | 38-56 | REAL | F-001 | 10 load-bearing (lib.rs/prelude chain + `catalog_data`), 9 unused (F-001). |

## 5. Test honesty
No tests in this file (pure structure). Its correctness is compile-time; the submodule batteries (unit, catalog_drift, catalog_shape, wire_forms, catalog_emission_ledger) all ran ok in baseline (log 1303-1410) and are the executable proxy for this hub.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every host — Tauri shell, web page, CLI, contract tests, control server — consumes THIS catalog" | mod.rs:22-25 | REAL | rg `commands::` in apps/desktop (dispatch.rs:62), apps/web (host.rs, 62 hits), apps/cli (dispatch.rs:30), control-protocol (Cargo.toml:32); contract tests: testkit/first-vertical use the wire types (baseline first-vertical evidence tests ran, log 2433+). |
| "Ids are stable forever within major 1 (§6); adding an entry is additive (§6.2)" | mod.rs:26-27 | REAL-UNPROVEN (contract statement; the current id sets are pinned by catalog_shape tests, but "forever" is a promise, not code) | catalog_shape.rs:162-182 pins ALL/ALL_IDS round-trip + counts. |
| "committed schema/TS/checksum artifacts regenerate via `cargo xtask codegen` (CI fails on drift)" | mod.rs:28-29 | REAL | ci.yml:280 (`codegen --check`); catalog_drift tests ok in baseline (log 1375-1376). |
| "the ~61-command / 23-event catalog" | lib.rs:30 | **FALSE** (stale counts; actual 65/26) | See F-002. |
| Module layout list (ids/types/command/event/spec/codegen with their contents) | mod.rs:5-18 | REAL | All six modules exist with the described contents (verified across this wave's audits + greps). |

## 7. Dead / duplicated / unreachable code
- Nine unused re-exports (F-001).
- No dead modules; nothing unreachable — this hub is the entry to the catalog for every host.

## 8. Dependencies & flags
- No imports, no unsafe, no logic. Submodule dependency directions are internal and acyclic (ids ← types? no: types/ids/command/event/spec are siblings; codegen imports command/event/envelope — all within-crate, no cycles; build clean).

## 9. Open questions for the next agent
- `spec.rs` and `ids.rs` are audited by other agents (W02 file list) — my findings here rely on their existence and the passing shape tests, not a line-by-line read of those two files.
- If the orchestrator wants the F-001 re-export cleanup, it touches mod.rs + possibly consumers' import paths — a one-card hygiene change.
