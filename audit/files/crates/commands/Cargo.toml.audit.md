# AUDIT: crates/commands/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A08 |
| Date | 2026-11-21 |
| Lines / functions | 45 / 4 config blocks ([package], [lints], [dependencies], [[bin]]) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: apps/cli/Cargo.toml, apps/desktop/Cargo.toml, apps/web/Cargo.toml, apps/desktop-tauri/Cargo.toml all depend on `commands` — rg '^commands = ' over Cargo.tomls; baseline #1 workspace build clean) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "catalog-gen|catalog_gen" tools/xtask/src/cmd/codegen.rs` (bin wiring); `python3` Cargo.lock parse (dep versions); `rg -l '^commands = '` (dependents); `rg -l serde_json/schemars/media_types crates/commands/src` (dep usage); sed -n quotes. |

## 1. One-paragraph verdict
A clean, consistent manifest. Every claim I could test holds: the three declared deps are
the exact single versions in Cargo.lock (serde 1.0.229, serde_json 1.0.154, schemars
1.2.3 — no duplicate versions), all three are used in the crate's sources, the
`catalog-gen` bin exists at the declared path and `cargo xtask codegen` really shells
out to it (codegen.rs:53 "Run the `catalog-gen` binary through cargo and forward its
status"), the generated artifacts named in the comment exist (schemas/commands.schema.json,
ui/src/generated/commands.ts, schemas/generated/commands.checksum.txt), and CI gates
their drift (ci.yml:280 `cargo run -p xtask --locked -- codegen --check`). The only
defect is a stale comment predicting a `media-types` dependency that never arrived.

## 2. What actually works (verified)
- Lock consistency: Cargo.lock `commands 0.1.0` block lists exactly schemars, serde,
  serde_json; each resolves to ONE version repo-wide (python3 parse) — no version
  fragmentation.
- `[[bin]] catalog-gen` exists (`ls crates/commands/src/bin/` → catalog_gen.rs) and is
  invoked by xtask codegen (tools/xtask/src/cmd/codegen.rs:67 `"catalog-gen"`, :73
  context "spawning `{program} run -p commands --bin catalog-gen`").
- catalog_gen.rs:95-107 implements a real drift check ("output drift — run `cargo
  xtask codegen` and commit the refreshed files", exit 1) — not a fake PASS printer.
- Dependency usage: serde_json → error.rs/catalog/event.rs/catalog/command.rs;
  schemars → envelope.rs/bin/catalog_gen.rs/error.rs/catalog/types.rs. No unused deps.
- Consumers match the header's claim (CLI, web, desktop, desktop-tauri, control
  server, tests): rg found `commands =` in apps/{cli,web,desktop,desktop-tauri,
  control-server}/Cargo.toml, crates/{control-protocol,playback-engine}/Cargo.toml,
  tests/{compat,integration/first-vertical}/Cargo.toml.

## 3. Findings

### F-001 [S4] Stale comment: promised `media-types` dependency never arrived
Where: crates/commands/Cargo.toml:11-12, 24 (cross-evidence: [dependencies] 38-41; rg
media_types in crates/commands/src → no matches)
Code:
```
# Card 1.D3 (this commit) delivers the envelope, the typed error taxonomy
# with redaction (§1.2 V1–V8, §2.13), and the correlation/ordering
# machinery (§4) — generic over the catalog payloads, which land with
# card 1.D4 (the ~65-command catalog + codegen). Std-only until then:
# serde (wire forms) and media-types (payload types) arrive with 1.D4.
```
What is wrong: the 1.D4 dependency block (lines 38-41) added serde/serde_json/schemars
but no `media-types`; `rg -l "media_types|media-types" crates/commands/src` finds zero
usage. Line 24's "the catalog card adds serde + media-types" is equally stale — serde
arrived, media-types did not (payload types live in-crate under src/catalog/).
Why it matters: a reader auditing dependency boundaries (boundaries.toml, check-
architecture) is told to expect a media-types edge that does not exist; comments that
misdescribe the dep graph rot trust in the rest of the header.
What correct looks like: delete the media-types prediction or note "payload types
defined in-crate (src/catalog/types.rs); media-types not needed".
Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| [package] name/version/edition | 14-17 | REAL | — | "commands" 0.1.0 matches Cargo.lock block; edition 2024 OK under Rust 1.99.1 pin (baseline header) |
| [package] description | 18 | REAL | — | accurate ("catalog in card 1.D4" — catalog exists, 65 commands per codegen output) |
| [package] publish | 19 | REAL | — | false — correct for an internal crate |
| [lints] workspace = true | 21-22 | REAL | — | inherits workspace lints (baseline hygiene: 39/40 crates inherit; clippy -D warnings clean, baseline #5) |
| header comments | 1-12, 24, 26-36 | SHALLOW | S4 | media-types prediction stale (F-001); codegen claim itself verified real |
| [dependencies] serde | 39 | REAL | — | 1.0.229, sole lock version, derive feature, used |
| [dependencies] serde_json | 40 | REAL | — | 1.0.154, sole lock version, used |
| [dependencies] schemars | 41 | REAL | — | 1.2.3, sole lock version, used |
| [[bin]] catalog-gen | 43-45 | REAL | — | path exists; invoked by xtask codegen; drift check real; CI-gated (ci.yml:280) — artifact freshness itself REAL-UNPROVEN (no baseline run of `codegen --check`; not in INDEX.md table) |

## 5. Test honesty
No tests in this manifest; the crate's test suite ran in baseline #3/#4
(`cargo test --workspace`: 1681 passed / 1 failed — the failure was
apps/desktop zero-alloc, not this crate). No test asserts the committed
generated artifacts match the catalog (rg over crates/ found only
catalog_gen.rs itself doing the comparison) — that job belongs to
`codegen --check`, which no baseline run executed: REQUEST-ORCH-RUN:
`cargo run -p xtask --locked -- codegen --check`.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "all consume the same catalog ... defined here, re-exported (never re-implemented)" | Cargo.toml:3-6 | REAL | 9 dependent manifests (rg); single crate in lock |
| "the generated code can never drift from the types that produce it" | Cargo.toml:34-36 | REAL-UNPROVEN | catalog_gen.rs:95-107 drift check exists + ci.yml:280 runs it; not executed in baseline |
| "cargo xtask codegen shells out to the catalog-gen bin" | Cargo.toml:31-32 | REAL | codegen.rs:53,67,73 |
| "serde ... and media-types ... arrive with 1.D4" | Cargo.toml:11-12 | FALSE (comment) | no media-types dep (F-001) |

## 7. Dead / duplicated / unreachable code
None: no dead deps, no version duplication, bin is wired.

## 8. Dependencies & flags
serde 1.0.229 +derive, serde_json 1.0.154, schemars 1.2.3 — all workspace-consistent
(single lock versions, matching the "same versions settings-schema/theme-tokens use"
claim's family; verified single-version for all three). No feature flags. No dev-deps
(tests use only the above).

## 9. Open questions for the next agent
- Does `xtask codegen --check` pass today? (No baseline run covers it; the committed
  ui/src/generated/commands.ts could be stale without any current signal outside CI.)
- schemars 1.2.3 on crates.io — confirm it is the genuine crate (SBOM baseline #25
  says licenses ok; drift noted there is a separate crate's issue).
