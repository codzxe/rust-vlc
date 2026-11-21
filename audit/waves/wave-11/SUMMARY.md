# WAVE 11 — SUMMARY (config system)

Date: 2026-11-21 (session 2) · Agents W11-A01…A10 (A05 re-spawned per §7.4) ·
Domain: product-config, settings-schema, localization, keymaps, schemas/.
Gate: **PASS — 33/33 assignments audited, 0 missing, 0 thin, citation
spot-checks clean.**

## Verdicts (file level)
REAL 13 · UNREACHABLE 19 · SHALLOW 2 (composite verdict strings canonicalized)

## Findings
**S0:0 S1:0 S2:22 S3:58 S4:71 — total 151**

## The wave's hostile questions
- **Config declared vs consumed: massive gap.** The entire `localization`
  crate (message-format engine, fallback chain, five-rule auditor) has ZERO
  dependents — the UI hardcodes `["en-US"]`. The config-enforcement layer
  (rule 4/F1/F2 validators) has no shipped caller. All 7 runtime identity
  resolvers have zero production callers (doc 07 §8.2 "every surface reads
  product-config at run time" is FALSE — the CLI prints compile-time
  `CARGO_PKG_VERSION`). `config.set-setting` validates then stores into a
  dead in-memory map — **no preference ever persists** (doc 07 §4.1
  "persisted settings" is FALSE).
- **Settings don't round-trip:** the G4 validator rejects the seed's own
  empty-string defaults; get-setting returns fabricated placeholder strings
  for 28/121 `default_ref` entries that set-setting then rejects.
- **The locale "reference catalog — complete, source of truth" holds 0 of the
  121 `settings.*` msgids** the schema references; real resolution lives in
  the parallel TS catalog (ui/src/i18n/messages.ts) — two sources of truth.
- **`cargo xtask config-validator` (the docs' RED-with-4-violations gate) runs
  in no CI** — doc 05:533 falsely names it as the D3 gate; it has zero D-rule
  code; `l10n-audit` (doc 07 §6.4) does not exist (ci.yml:468 admits it).
- Keymap: the shipped 9-binding TS table contradicts keymaps/default.toml on
  B/G while the settings screen shows the wrong action.

## Worst findings
1. [S2] Settings never persist; `SettingApplied`+`restart_required` returned
   for writes guaranteed lost (A02, A09 — found independently).
2. [S2] G4 validator rejects the seed's own defaults; fabricated
   get-setting placeholders for 28/121 entries (A03).
3. [S2] Reference catalog holds 0/121 referenced msgids; doc-promised CI
   error cannot fire (A06).
4. [S2] fallback.rs:292 test fails under `cargo test --release`
   (baseline-reproduced) (A07).
5. [S2] D1 literal-dup heuristic: one product.toml boolean flip sprays
   false build-breaking errors at four unrelated settings while real
   cross-area duplication goes unflagged (A01).

## Blocked / UNVERIFIED
- product-config/settings-schema/localization test suites never ran in any
  primary baseline run (aborted early) — REQUEST-ORCH-RUN filed.
- CLDR fr/it plural-category check (offline; MEDIUM confidence).
