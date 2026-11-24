# AUDIT: crates/product-config/tests/fixtures/README.md
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 68 / 0 (fixture-map doc) |
| File verdict | REAL (map is complete and accurate; the regeneration instruction is stale) |
| Reachable from shipped entry point? | N/A (documentation). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 completeness check (42 fixture files vs README table -> bidirectional 1:1); diff of 5 sampled fixtures vs current product.toml; file/od on fixtures/assets/assets/brand/* (magic bytes); rg suite cross-references (fixtures_matrix.rs, golden_s35.rs:14, semantic.rs) |

## 1. One-paragraph verdict
The fixture↔rule map is mechanically complete (python3: all 42 .toml files listed, all listed files exist — zero orphans in either direction) and every suite assignment I checked is true (fixtures_matrix consumes the structural/parse rows, golden_s35.rs:14 consumes range-probe-max-bytes.toml, semantic.rs consumes the cross-field/injected/asset rows; the assets/ claims are byte-true: icon.ico starts with "NOT-ICO", icon-512.png carries a real PNG signature). But the README's core maintenance instruction is stale: the fixtures are NOT copies of the current product.toml — they share a pre-w28/pre-ADR-001 snapshot, so following the documented regeneration procedure would rewrite ~20 lines per fixture beyond the mutation.

## 2. What actually works (verified)
— python3: `files not in README table: []` and only `product.toml` (prose mention of the base) listed-but-not-a-fixture — the map is exact.
— Suite wiring: rg confirms every suite named in README L10-17 references the fixtures it claims (fixtures_matrix.rs tests shown in my fixture audits; golden_s35.rs:14 `let name = "range-probe-max-bytes.toml"`; semantic.rs:137/265 for injected-enum-locale/assets-missing).
— assets/ claims: `od -c` — icon.ico = `NOT-ICO\0...` (deliberately wrong magic), icon-512.png = `\211 P N G \r \n 032 \n` (valid PNG signature), logo.svg = real SVG, icon.icns = "Mac OS X icon" — README L66-68 is byte-accurate.

## 3. Findings
F-001 [S4]
Where: crates/product-config/tests/fixtures/README.md:5-8
Code: (verbatim)
```
Each fixture is the repo-root `product.toml` (doc 07 §2 draft) with exactly
one documented mutation — regenerate a fixture by re-applying its mutation
to a fresh copy of `product.toml` (the generation script was a one-off;
mutations are listed below).
```
What is wrong: the fixtures are based on an OLD product.toml snapshot, not the current one. Diff (5 fixtures sampled, all identical base drift): `display_version = "0.1.0-dev"` vs current `"0.1.3"`, `version_code = 1` vs `4`, `min_width/min_height = 480/320` vs `940/600` (w28 fix 5), `streaming-http = false` vs `true` (w31.3), pre-ADR-001 hw-decode prose vs the accepted-ADR-001 prose, and no `[web.size_budgets]` section vs the committed one. Following the README's own regeneration instruction produces files that differ from the committed fixtures well beyond the documented mutation. Worse, the stale base's comments make claims that are now false (e.g. "`software-only` is the ONLY legal value until the ADR lands" — contradicted by the same directory's post-ADR-001 enum-hw-decode-policy fixture and its test asserting the three-value vocabulary). Why it matters: maintainers regenerating a fixture will either commit unrelated churn or (more likely) stop trusting the map; the stale prose actively misstates current product decisions. What correct looks like: regenerate all fixtures from the current product.toml once, or reword to "based on the product.toml snapshot of card 1.B1". Confidence: HIGH — Verification: diff + read.
F-002 [S4]
Where: crates/product-config/tests/fixtures/README.md:5 (intro) vs table rows 26 and 36
Code: (verbatim, L26)
```
| `missing-window-fields.toml` | Required fields | `window.title_template` + `window.title_idle_key` removed (two diagnostics: every violation is reported) |
```
What is wrong: the intro promises "exactly one documented mutation" per fixture, but two fixtures mutate multiple fields (missing-window-fields: 2 keys; range-limits-multi: 4 keys). The table documents both honestly — the intro wording is what overstates. What correct looks like: "exactly one documented mutation (multi-field where the rule requires it)". Confidence: HIGH — Verification: read + diff.

## 4-9. (short doc audit)
Claim ledger: map completeness REAL (python3); suite assignments REAL (rg); assets/ claims REAL (od); regeneration instruction FALSE-as-of-today (F-001); "exactly one mutation" PARTLY FALSE (F-002). No dead code. Dependencies: none. Open questions: none — F-001's fix (bulk regeneration) will touch all 42 fixtures and should be coordinated with the golden_s35 range-probe-max-bytes fixture, whose §3.5 golden diagnostic is pinned to the fixture's line numbers (a regeneration shifts lines; golden_s35 must be regenerated in the same commit).
