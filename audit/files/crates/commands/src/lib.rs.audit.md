# AUDIT: crates/commands/src/lib.rs (full file, 72 lines)

| Field | Value |
|---|---|
| Auditor | W02-A07 |
| Date | 2026-11-21 |
| Lines / functions | 72 / 5 units (4 `pub mod` + re-export block + prelude module) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (this is the crate root of `commands`, a direct dependency of the shipped `lumen` CLI — apps/cli/Cargo.toml:52; every CLI dispatch path flows through `commands::{WireRequest, WireResponse, …}` re-exported here; also consumed by control-protocol (server.rs:43-44), apps/control-server (dispatch.rs, 14 use sites), apps/desktop, apps/web, playback-engine) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `sed -n` verbatim verification (27-36, 52-72); `ls crates/commands/src{,/catalog}` + `wc -l` (module inventory); repo-wide `rg "commands::prelude"` → ZERO consumers (contrast: `media_types::prelude` has one — media-types/tests/testkit_wiring.rs:8); `rg "use commands::"` consumer census (12 files, top: apps/control-server/src/dispatch.rs ×14); assignments.tsv cross-check (correlation.rs→A08, envelope.rs→A10, error.rs→A06 — no coverage gap behind this facade); baseline build #1 (clean workspace build proves all re-exports resolve). |

## 1. One-paragraph verdict
A facade file: four module declarations, three re-export blocks, one prelude — no logic, no tests, nothing to execute. Everything it promises exists (the crate compiled clean in baseline run #1, so every re-export path resolves), and the consumer census confirms the "one command layer, consumed by every host" claim is real (CLI, control-protocol, control-server, desktop, web, playback-engine all import from this crate). Two defects: the module doc still sells "the ~61-command / 23-event catalog" when the shipped table is 65 commands / 26 events (same stale-count drift as the spec.rs header — the additions were wired into the table, tests and schema, but not into either doc comment), and the `prelude` module is dead API — zero importers repo-wide — whose doc claims to be "the full 1.D3 + 1.D4 surface" while omitting the three `Wire*` types that are the layer's most-used exports.

## 2. What actually works (verified)
— All re-exports resolve: baseline run #1 `cargo build --release --locked --workspace` exit 0 (INDEX.md row 1) — a facade with a broken `pub use` cannot compile.
— Consumer census (`rg "use commands::"`): apps/control-server/src/dispatch.rs (14 sites), crates/control-protocol/src/{server,protocol,policy}.rs, apps/{cli,desktop,web}, crates/playback-engine — the doc's "One command layer … consumed by every host" (lines 3-7) is verifiably true in-tree.
— Module inventory matches the doc's card split: `error.rs` (424 LOC), `envelope.rs` (288), `correlation.rs` (407), `catalog/` (mod, ids, command, event, types, spec, codegen + `bin/catalog_gen.rs`) — every module named in the header exists; all are wave-2-assigned (correlation→A08, envelope→A10, error→A06, catalog parts→A01–A07).

## 3. Findings

### F-001 [S4]
Where: crates/commands/src/lib.rs:30-31
Code:
```
//! - [`catalog`]   — the ~61-command / 23-event catalog: stable ids
//!   (§2.1/§3.2), payload wire types (§2.2), [`catalog::Command`]/
```
What is wrong: Stale counts. The shipped catalog is **65 commands / 26 events** (`grep -c 'id: CommandId::' spec.rs` → 65; `id: EventId::` → 26; pinned by `catalog_shape.rs` schema counts and `CommandId::ALL.len()==65`). The w31.1 frame-channel bind and the issue #32/#42 additions were propagated to the table, tests and schema but not to this header (nor to spec.rs's own header — see my spec.rs#part1 audit F-002).
Why it matters: This is the crate's front-door description; "~61" hedges the command count but "23-event" is exact and wrong. Readers sizing the surface (keymap authors, control-client authors) under-count by 4 commands and 3 events.
What correct looks like: "the 65-command / 26-event catalog (§2.1's 61 + issue #32 trio + w31.1 bind; §3.2's 23 + issue #42 trio)" — one sentence, fixed once in both headers.
Confidence: HIGH — Verification: read + counts re-verified this session (spec.rs audit) + catalog_shape.rs assertions (executed in baseline run #3, test-workspace.log.txt:1388).

### F-002 [S3]
Where: crates/commands/src/lib.rs:62-72 (vs the root re-export at :52-55)
Code:
```
/// Prelude: the full 1.D3 + 1.D4 surface in one `use`.
pub mod prelude {
    pub use crate::catalog::{
        Command, CommandId, CommandResult, EventId, EventKind, PlaybackState,
    };
```
(and, for contrast, the crate root at :52-55:)
```
pub use catalog::{
    Command, CommandId, CommandResult, EventId, EventKind, PlaybackState, WireEvent, WireRequest,
    WireResponse,
};
```
What is wrong: Two-part defect. (a) Dead API: `commands::prelude` has **zero importers** repo-wide (`rg "commands::prelude"` over apps/, crates/, tests/, tools/ — no hits; the only in-tree prelude consumers are `codec_api::prelude` and `media_types::prelude`, different crates). (b) False doc claim: "the full 1.D3 + 1.D4 surface in one `use`" — the prelude omits `WireEvent`, `WireRequest`, `WireResponse`, the wire forms that are this crate's most-imported items (control-server's dispatch alone has 14 `use commands::` sites built on them). A user who trusted the doc comment would import the prelude and still be missing the types they need.
Why it matters: Dead, misleading public API in the crate every host consumes; it invites cargo-cult imports and documents the surface inaccurately at the exact place newcomers look first.
What correct looks like: Either delete the prelude (preferred — nobody uses it), or make it actually the full root surface (add the `Wire*` trio) and get one consumer (e.g. a doc example) to prove it.
Confidence: HIGH — Verification: read + exhaustive repo-wide rg (two patterns, four directories).

## 4. Per-function table
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module doc | 1-46 | REAL | S4 | Card-split narrative accurate (modules exist, contents verified); count claim stale (F-001). "Accepted ≠ Completed" truthful-UI summary matches envelope's CommandOutcome split (envelope.rs — A10 to verify the split itself). |
| `pub mod` declarations | 47-50 | REAL | — | catalog, correlation, envelope, error — all exist with substance (LOC above). |
| Root re-exports | 52-60 | REAL | — | 9 + 3 + 7 + 3 items; compile-proven (baseline build #1 clean); consumed across 12+ files. |
| `pub mod prelude` | 62-72 | REAL (compiles) / DEAD (unused) | S3 | F-002: zero importers; "full surface" claim false (Wire* trio missing). |

## 5. Test honesty
No tests in this file (none possible — declarations only). The crate's tests live in the submodules and `tests/` (catalog_shape.rs executed in baseline run #3: test-workspace.log.txt:1388; catalog::* unit tests :1306-1333). N/A here.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "One command layer … the same envelope … consumed by every host" | lib.rs:3-7 (doc 05 §1.1) | REAL | Consumer census: 12+ files across cli/control-protocol/control-server/desktop/web/playback-engine. |
| Card 1.D3/1.D4 split: error/envelope/correlation/catalog contents | lib.rs:11-36 | REAL | Module inventory + LOC; details are A06/A08/A10/A01-A05's audits. |
| "the ~61-command / 23-event catalog" | lib.rs:30-31 | FALSE (stale) | 65/26 actual (F-001). |
| "Prelude: the full 1.D3 + 1.D4 surface in one use" | lib.rs:62 | FALSE | Omits Wire* trio; zero users (F-002). |
| Committed codegen artifacts (schemas/commands.schema.json, ui/src/generated/commands.ts, checksum) | lib.rs:34-36 | REAL | catalog_shape.rs reads the committed schema + checksum and recomputes both — passed in baseline run #3. |

## 7. Dead / duplicated / unreachable code
- `pub mod prelude` — dead (F-002). Nothing else: every root re-export has in-tree consumers.
- No duplication: this facade exists precisely to prevent re-implementation (and does — hosts import, they don't redefine).

## 8. Dependencies & flags
- No external deps used in this file; crate-level deps (serde/schemars via submodules) are the catalog files' concern. No feature gates, no unsafe, no panics. Workspace lints inherited (crate Cargo.toml).

## 9. Open questions for the next agent
- A06/A08/A10 (error/correlation/envelope): the header's claims about YOUR modules ("redaction rule enforced at construction", "§4.2 host rule unit-tested", "delivery-log validator checks §4.3 invariants") are verified here only as "the named items exist and the crate's tests ran in baseline" — verify each claim against the module bodies.
- Whoever owns the catalog doc comments: fix the 61/23 counts in BOTH lib.rs:30 and spec.rs:5-8 in one pass (F-001 here, F-002 in my spec.rs#part1 audit).
