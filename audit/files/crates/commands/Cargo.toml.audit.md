# AUDIT: crates/commands/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W02-A01 |
| Date | 2026-11-23 |
| Lines / functions | 45 / 0 (manifest: 1 package, 1 lints, 3 deps, 1 bin target, 5 config keys) |
| File verdict | REAL (with one stale, self-contradictory comment block) |
| Reachable from shipped entry point? | YES — apps/cli/Cargo.toml + apps/desktop/Cargo.toml depend on `commands`; apps/cli/src/run.rs:51 imports `commands::{...WireRequest...}` and builds wire requests on the open path (run.rs:161, 200, 274) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p commands` (exit 0; 57 + 2 + 2 + 4 = 65 passed, 0 failed); rg sweeps cited below; baseline audit/runs/baseline/cargo-test-workspace.txt:1421-1551 (exit 0) |

## 1. One-paragraph verdict
The manifest's architecture claims are all true and I verified each one by execution or
inspection: all five consumer hosts (desktop, web, CLI, control-server, playback-engine) plus
the contract-test crate (tests/compat) depend on this crate; the hosts import
`WireRequest`/`WireResponse` from it rather than redefining them; the `catalog-gen` bin exists
inside this crate, is spawned by the registered xtask `codegen` subcommand, and its three
committed artifacts exist and are guarded by a byte-for-byte in-memory drift test; the catalog
is exactly 65 commands, matching the "~65-command catalog" claim exactly. The one defect is
housekeeping: the header comments still tell the 1.D3/1.D4 story in which "media-types
(payload types) arrive with 1.D4" and "No dependencies in 1.D3" — while the file itself now
declares three dependencies and the catalog shipped without ever adding media-types (the
wire/engine type split is instead documented, deliberately, in catalog/types.rs).

## 2. What actually works (verified)
— `cargo test -p commands` -> 65 passed, 0 failed (57 unit + catalog_shape/wire_forms/
catalog_emission_ledger/catalog_drift integration). Baseline agrees (exit 0).
— "All consume the same catalog... defined here, re-exported (never re-implemented)":
rg over all Cargo.tomls -> consumers: apps/cli, apps/desktop, apps/web, apps/control-server,
crates/playback-engine, tests/compat (contract tests). Spot-checked imports:
apps/desktop/src/shell.rs:40, apps/cli/src/run.rs:51 both `use commands::{... WireRequest,
WireResponse ...}`. No host redefines the envelope.
— Codegen chain: [[bin]] catalog-gen -> crates/commands/src/bin/catalog_gen.rs exists (279
lines); xtask subcommand "codegen" registered (tools/xtask/src/cmd/mod.rs:95,224) and spawns
`run -p commands --bin catalog-gen` (tools/xtask/src/cmd/codegen.rs:67); artifacts exist:
schemas/commands.schema.json (139,694 B), ui/src/generated/commands.ts (14,023 B),
schemas/generated/commands.checksum.txt; drift guard: crates/commands/tests/catalog_drift.rs
regenerates all three in memory via the same `commands::catalog::codegen` API and compares
byte-for-byte (test passes).
— "~65-command catalog": `awk` over the CommandId enum -> exactly 65 `=> "id"` variants;
26 EventId variants.
— Dependency family unification (C5 claim): control-protocol pins the same serde 1.0.229 /
serde_json 1.0.154; theme-tokens pins serde_json 1.0.154; settings-schema uses loose `serde =
"1"` (unified by the workspace lockfile).

## 3. Findings
F-001 [S4]
Where: crates/commands/Cargo.toml:8-12 and 24 vs 38-41, and 18
Code:
```
# generic over the catalog payloads, which land with
# card 1.D4 (the ~65-command catalog + codegen). Std-only until then:
# serde (wire forms) and media-types (payload types) arrive with 1.D4.
```
and
```
# No dependencies in 1.D3 (std-only; the catalog card adds serde + media-types).
```
What is wrong: Both comments are stale and the second contradicts the section eight lines
below it: `[dependencies]` now contains serde, serde_json and schemars — and media-types never
arrived. 1.D4 landed (the catalog and codegen are present and tested), so the promised
"media-types (payload types)" dependency was silently dropped; the actual design — wire types
deliberately distinct from media-types' engine vocabulary (µs integer Timestamp vs ns
Nanoseconds, f32 Rate vs rational Rate, Duration with an `Infinite` variant) — is documented
only in crates/commands/src/catalog/types.rs:5-11, not here. The package description (line 18)
is likewise stale: it still bills the catalog as future work ("catalog in card 1.D4") though it
shipped. Why it matters: the manifest is the first file an auditor or Lead reads; it narrates a
dependency plan the crate no longer follows, and a reader checking "does the command layer
reuse media-types?" gets the wrong answer from the comments and the right one only by reading
the source. What correct looks like: delete the 1.D3 narrative, state the wire/engine type
split and point at types.rs.
Confidence: HIGH — Verification: read (manifest vs actual dep set vs types.rs doc).

## 4. Per-function table
Not required (45 LOC manifest, no functions). Config-key inventory: name=commands,
version=0.1.0, edition=2024, publish=false, [lints] workspace=true, deps {serde 1.0.229
(+derive), serde_json 1.0.154, schemars 1.2.3}, bin {catalog-gen -> src/bin/catalog_gen.rs}
— all present and load-bearing; no unused dependencies (all three are imported: serde/serde_json
throughout the wire forms, schemars for JsonSchema derives in catalog/types.rs:27-29).

## 5. Test honesty
No tests in this file. The crate's tests (ran, 65 pass) include catalog_drift.rs — a genuine
drift guard: it regenerates the committed artifacts in memory through the same API the bin
calls and diffs byte-for-byte, so a hand-edited schema or a stale generated TS file fails the
build. That test would fail if the catalog types changed without re-running codegen: honest and
load-bearing.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| desktop UI, web UI, CLI, contract tests, control server all consume the same catalog/envelope | Cargo.toml:3-6 | REAL | dep graph (6 consumers incl. tests/compat) + imports at apps/desktop/src/shell.rs:40, apps/cli/src/run.rs:51 |
| defined here, re-exported (never re-implemented) by every host | Cargo.toml:5-6 | REAL | hosts import commands::{WireRequest,...}; UI TS side is codegen-generated from this crate |
| serde/serde_json/schemars are the preaudit-pass family, same versions as settings-schema/theme-tokens | Cargo.toml:28-30 | REAL | serde_json 1.0.154 identical in theme-tokens; control-protocol pins same serde pair |
| `cargo xtask codegen` shells out to catalog-gen, which emits schema + TS + checksum | Cargo.toml:31-36 | REAL | xtask cmd/mod.rs:95,224; cmd/codegen.rs:67; artifacts exist; drift test passes |
| the bin is part of THIS crate so generated code can never drift | Cargo.toml:35-36 | REAL | src/bin/catalog_gen.rs + catalog_drift.rs byte-for-byte guard |
| "No dependencies in 1.D3... the catalog card adds serde + media-types" | Cargo.toml:24 | FALSE (stale) | F-001: 3 deps now, media-types never added |
| "media-types (payload types) arrive with 1.D4" | Cargo.toml:11-12 | FALSE (stale) | F-001 |
| "~65-command catalog" | Cargo.toml:11 | REAL | exactly 65 CommandId variants (awk count) |

## 7. Dead / duplicated / unreachable code
None in this manifest. (The wire/engine vocabulary duplication between commands::catalog::types
and media-types is a documented, deliberate boundary split per types.rs:5-11 — flagged there,
not here; see open questions.)

## 8. Dependencies & flags
serde 1.0.229 (+derive), serde_json 1.0.154, schemars 1.2.3 — all imported and load-bearing;
no dev-dependencies block (integration tests are std-only); no features; no optional deps;
`publish = false`; `[lints] workspace = true`.

## 9. Open questions for the next agent
- The wire/engine unit split is conversion-code territory: playback-engine/src/event.rs:770
  `ns_to_us(ns) = (ns + 999) / 1_000` — ceil division; for negative positions
  (SeekTarget::Relative) this rounds toward +inf (sub-µs), and `ns + 999` can overflow near
  i64::MAX ns (~292 y — unrealistic but unchecked). Whoever audits playback-engine should
  judge it; it is NOT this file's defect.
- Who actually EXECUTES the 65 catalog commands in the shipped CLI — run.rs dispatches
  WireRequests, but is every handler real or do some forward to nothing? (Next wave: audit
  crates/playback-engine session/command dispatch and apps/cli/src/run.rs handler table.)
- control-protocol's policy.rs `is_sensitive(CommandId)` — verify its sensitive list covers all
  mutating catalog ids when control-server ships (Phase 5).
