# AUDIT: crates/commands/src/catalog/mod.rs
| Field | Value |
|---|---|
| Auditor | W02-A09 |
| Date | 2025-06-27 |
| Lines / functions | 56 / 0 fns (6 `pub mod` + 13 `pub use` + module contract docs) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen` CLI: apps/cli/src/main.rs → run.rs → dispatch.rs:62 `lumen_dispatch(host, &WireRequest)` where `WireRequest = CommandRequest<Command>` is this module's `codegen::WireRequest` re-export; desktop wraps the same (apps/desktop/src/dispatch.rs:211-212); release lane builds clean (B: cargo-build-release-lumen-cli.txt exit 0) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p commands` → 57 unit + 15 integration, all pass, exit 0 (covers every submodule re-exported here); rg census of re-export consumption |

## 1. One-paragraph verdict

This is the hub module of the one command layer: it wires the six catalog
submodules (ids, types, command, event, spec, codegen) and re-exports the
public surface. Every contract statement in its module docs verified against
the tree and against tests I ran myself: the id list exists in exactly one
place (ids.rs macro), the spec table is data with class/target/gen-bumping
(pinned to doc 05 §1.5/§4.2 by tests that pass), and the codegen artifacts +
CI drift gate are real (ci.yml:280 `codegen --check`). The only defect is
naming-surface bloat: every item is reachable under three public paths
(`commands::X`, `commands::catalog::X`, `commands::catalog::<sub>::X`), and
the mod-level re-export path has zero external consumers for 7 of 13 names.

## 2. What actually works (verified)

— `cargo test -p commands` (ran): 57 unit + 2+2+4+7 integration tests, all
pass, exit 0 — including `catalog_sizes_match_doc_05` (65/26),
`gen_bumping_set_is_exactly_section_4_2`, `class_counts_match_the_doc_05_tables`,
`tables_cover_every_id_exactly_once_in_order`, and the wire-form goldens.
— Module-doc claims verified by direct inspection:
  - "Ids are stable forever within major 1 (§6)" — ids.rs:47 `#[non_exhaustive]`
    + additive growth 61→65 with documented provenance (ids.rs:90-92,176-179).
  - "committed schema/TS/checksum artifacts regenerate via `cargo xtask codegen`
    (CI fails on drift)" — ci.yml:279-280 `command-catalog artifacts drift
    (doc 05 §5.1 — card 1.D4)` / `cargo run -p xtask --locked -- codegen --check`;
    artifacts exist (schemas/commands.schema.json, ui/src/generated/commands.ts,
    schemas/generated/commands.checksum.txt, "counts: 65 commands / 26 events").
  - "every host … consumes THIS catalog" — rg: control-protocol (protocol.rs:28-30
    `use commands::WireRequest`), CLI (dispatch.rs), desktop (dispatch.rs:211),
    web (host.rs:1403+ matches on `commands::Command`), control-server
    (dispatch.rs:17-24).

## 3. Findings

F-001 [S4]
Where: crates/commands/src/catalog/mod.rs:38-56
Code:
```rust
pub use command::Command;
pub use command::CommandListing;
pub use command::CommandResult;
pub use event::EventKind;
pub use ids::CommandId;
pub use ids::EventId;
pub use ids::UnknownId;
pub use spec::CommandSpec;
pub use spec::CommandTarget;
pub use spec::CompletionClass;
pub use spec::EventSpec;
pub use spec::PayloadDesc;
pub use spec::catalog_data;
pub use spec::event_catalog_data;
```
What is wrong: three public paths now exist for most names
(`commands::Command` via lib.rs:52-55, `commands::catalog::Command` here,
`commands::catalog::command::Command` naturally). External consumers
exclusively use the lib-level or deep-spec paths: rg census shows 0 external
uses of `catalog::PayloadDesc`, `catalog::CommandSpec`, `catalog::EventSpec`,
`catalog::CommandTarget`, `catalog::UnknownId`, `catalog::event_catalog_data`,
`catalog::CommandListing` as paths (the types themselves are used via
`commands::catalog::spec::*` deep imports, e.g. apps/control-server/src/dispatch.rs:23).
Why it matters: API surface drift risk — a future rename touching one path
and not another compiles fine and ships two names for one thing; the lib.rs
prelude already is the sanctioned entry.
What correct looks like: keep the re-exports that the crate's own prelude
needs; demote the rest to `pub(crate)` or document the mod-level path as the
canonical one and shrink lib.rs's list.
Confidence: HIGH — Verification: rg census (14 symbols, 0-7 external files each).
Severity S4: hygiene; no behavioral risk (all paths resolve to the same items).

## 4. Per-function table (item-level; file is 56 LOC, table not mandatory)

| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (contract statement) | 1-29 | REAL | — | every claim verified (§2); "contract tests" phrase = the in-crate wire/drift tests, not doc-05 §7's absent suite |
| `pub mod` declarations (6) | 31-36 | REAL | — | all six submodules exist and are tested |
| `pub use codegen::{CatalogDocument, Wire*}` | 38-41 | REAL | — | WireRequest/WireResponse/WireEvent are the concrete envelope instantiations (codegen.rs:39-43) used by every host |
| `pub use command::{Command, CommandListing, CommandResult}` | 42-44 | REAL | S4 | CommandListing re-export path unused externally (F-001) |
| `pub use event::EventKind` / `ids::{CommandId, EventId, UnknownId}` | 45-48 | REAL | S4 | UnknownId re-export path unused externally |
| `pub use spec::{CommandSpec, CommandTarget, CompletionClass, EventSpec, PayloadDesc, catalog_data, event_catalog_data}` | 49-55 | REAL | S4 | 5 of 7 re-export paths unused externally (F-001); `catalog_data` has 2 external consumers |
| `pub use types::PlaybackState` | 56 | REAL | — | used via lib prelude (`commands::PlaybackState` in apps/cli/src/dispatch.rs:120) |

## 5. Test honesty

No tests in this file (it is a hub). The submodules and integration tests
that cover its entire surface were run by me: `cargo test -p commands` →
72 tests pass. Mutation reasoning for the hub itself: deleting any re-export
breaks lib.rs's own re-exports or downstream imports (compile-level proof);
the golden/drift tests fail on any catalog-data mutation
(`mutated_table_entry_changes_the_checksum` ran and passed).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One command layer (§1.1): every host — Tauri shell, web page, CLI, contract tests, control server — consumes THIS catalog through CommandRequest<Command>…" | mod.rs:22-26 | REAL | four hosts verified by rg (§2); "contract tests" = crates/commands/tests/* (ran), NOT doc-05 §7's suite (absent — see doc ledger C82) |
| "Ids are stable forever within major 1 (§6); adding an entry is additive (§6.2)" | mod.rs:26-28 | REAL | ids.rs non_exhaustive + additive 61→65 history; drift tests ran |
| "committed schema/TS/checksum artifacts regenerate via cargo xtask codegen (CI fails on drift)" | mod.rs:28-29 | REAL | ci.yml:279-280; artifacts present with matching counts (65/26); `cargo test -p commands` drift tests pass |

## 7. Dead / duplicated / unreachable code

- No dead code; duplicated NAMING surface only (F-001).
- Cross-ref: `commands::ErrorCode::NotSeekable` is dead vocabulary at the
  production level (see error.rs audit §7 and doc ledger C99) — not this
  file's fault, noted for the spec owner.

## 8. Dependencies & flags

- Submodules depend on: schemars (codegen/types), serde (everywhere) —
  declared in crates/commands/Cargo.toml; no feature gates on the catalog
  (always compiled into every host). Boundaries: `control-protocol->commands`
  is a Lead-ratified same-layer pair (boundaries.toml [rules]).

## 9. Open questions for the next agent

1. Who owns the decision to slim the triple-path export surface (F-001)?
   Cheap now, breaking after external consumers appear.
2. `catalog_emission_ledger.rs` (399 LOC) was not in my assignment — it
   appears to assert which events each command may emit; worth a dedicated
   audit against doc 05 §2 tables' completion-evidence claims.
3. If doc 05 §7's conformance suite ever lands, this hub is where a
   `HostHarness` abstraction would attach — consider reserving the name.
