# AUDIT: tests/rebrand/nova.toml
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Data fixture (complete alternate product definition, card 1.E4) |
| Size / sha256-prefix | 3606 bytes / fc5cd1033d858a1d (verified locally) |
| File verdict | REAL — loaded through the real product-config pipeline |
| Reachable from shipped entry point? | N/A — test asset (rebrand acceptance + xtask rebrand) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 sha256, read file, rg consumers, compare product.toml [features] |

## 1. Verdict
Real, complete product fixture carrying the "Nova Player" identity. It is
NOT trusted: the acceptance test loads it through the real runtime path —
`include_str!("nova.toml")` + `load_str(..., &schema)` with the committed
product.schema.json (rebrand_acceptance.rs:63-67) — and asserts every
identity surface resolves from the loaded config. Its 4 brand assets
(tests/rebrand/assets/{icon.ico,icon.icns,icon-512.png,logo.svg}) all exist
(verified by ls). The rebrand acceptance suite passes in the baseline
(cargo-test-workspace.txt: product-config rebrand_host — passing suite; the
config-validator FAILURE in xtask-feature-audit concerns product.toml's own
`assets/brand/*` paths, not nova's).

## 2. What actually works (verified)
- Consumer (verbatim, tests/rebrand/rebrand_acceptance.rs:63-67):
```
    let schema: Schema = include_str!("../../product.schema.json")
    ...
    let src = include_str!("nova.toml");
    load_str("tests/rebrand/nova.toml", src, &schema)
```
- Identity derivation asserted (nova.toml:35-49 → "org.novaplayer.NovaPlayer" on all four OS ids).
- Assets present: `ls tests/rebrand/assets` -> icon-512.png icon.icns icon.ico logo.svg.

## 3. Findings
F-001 [S3]
Where: tests/rebrand/nova.toml:79-101 ([features] block) vs product.toml:147-168
What is wrong: The header comment claims "Feature set mirrors the committed
product.toml" — it does not: product.toml now has `streaming-http = true`
(w31.3), nova.toml still says `streaming-http = false`. One-flag drift,
undetected by any gate (the rebrand test only asserts identity surfaces).
Code (nova.toml:90):
```
streaming-http = false
```
vs product.toml:157: `streaming-http = true            # w31.3: the CLI/GUI open path serves http(s)...`
Why it matters: a stale "mirrors" claim rots silently; if rebrand ever
copied the features block, it would ship a configuration that silently
disables a landed capability.
What correct looks like: drop the features block from the fixture (rebrand
ignores it) or assert equality with product.toml in the acceptance test.
Confidence: HIGH — Verification: read + diff of both files.
F-002 [S4]
Where: tests/rebrand/nova.toml:30
`publisher = "Nova Player Contributors"   # [PROPOSAL] placeholder entity`
What is wrong: placeholder publisher entity — fine for a test fixture, but
it is the value asserted as the About surface; if nova.toml were ever used
as a template, the placeholder ships. Cosmetic, documented in-tree.
