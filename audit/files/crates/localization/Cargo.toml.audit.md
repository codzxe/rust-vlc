# AUDIT: crates/localization/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines | 25 (manifest + header comment) |
| File verdict | REAL |
| Reachable from shipped entry point? | The crate it builds is NOT reachable from lumen-cli/desktop (zero reverse deps — see catalog.rs audit). Manifest itself: workspace member, built by test lanes. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cat` manifest; `rg` preaudit-foundation.md records (exit 0); `rg` Cargo.lock pinned versions (toml 1.1.8+spec-1.1.0, thiserror 2.0.21) |

## 1. One-paragraph verdict
A minimal, honest manifest: exactly two dependencies (toml 1.1, thiserror 2.0), no dev-dependencies needed (integration tests consume only the crate's own API + std), no features, no internal path deps — matching the header's "layer L1, no internal deps" registry claim. The "both pre-audited … verdict pass" claim verifies against docs/dependencies/preaudit-foundation.md:26-28, with one omission: the toml row carries an open `[VERIFY-P1]` marker (license endpoint returned null; re-check at pin) that the manifest's summary omits.

## 2. What actually works (verified)
— `rg` preaudit-foundation.md: `| toml | 1.1.8+spec-1.1.0 | [VERIFY-P1: …] | … | **pass** (license re-check at pin) |` and `| thiserror | 2.0.21 | MIT OR Apache-2.0 | … | **pass** |`; Cargo.lock pins match (toml 1.1.8+spec-1.1.0 at lock L1150-1151, thiserror 2.0.21 at L1124-1125). Baseline localization package builds + 45 tests pass.

## 3. Findings
F-001 [S4]
Where: crates/localization/Cargo.toml:7-11 vs docs/dependencies/preaudit-foundation.md:26
Code (manifest):
```
# Dependencies (both pre-audited in docs/dependencies/preaudit-foundation.md,
# verdict `pass`; no NEW crates were introduced by this card):
```
Code (record):
```
| toml | 1.1.8+spec-1.1.0 | [VERIFY-P1: version endpoint returned null; crate docs state MIT OR Apache-2.0 — confirm at pin time] | 2026-10-09 | Pure Rust | **pass** (license re-check at pin) |
```
What is wrong: the manifest summarizes toml's verdict as plain `pass`, eliding the record's open VERIFY-P1 condition (license confirmation at pin time). Not a false claim — the record's verdict column does say pass — but the summary drops a verification debt that the SBOM/license gate relies on being visible.
Confidence: HIGH — Verification: read of both files.

## 4. Per-function table
Manifest — sections: [package] (name/version/edition 2024/description/publish=false) ✓; [lints] workspace=true ✓ (root workspace lints exist); [dependencies] toml 1.1 + thiserror 2.0 (both used by src — `use toml`/`#[derive(thiserror::Error)]` confirmed in catalog.rs) ✓. No [dev-dependencies]/[features]/[[bin]] — consistent with the crate's shape.

## 5. Test honesty
N/A (manifest). The crate's 45 unit + integration tests pass in baseline without dev-deps, confirming the dev-deps-free claim implicitly.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "both pre-audited … verdict `pass`" | Cargo.toml:7-8 | REAL (with F-001 elision) | preaudit-foundation.md:26-28 |
| "no NEW crates were introduced by this card" | Cargo.toml:8-9 | REAL | toml + thiserror predate: both in the foundation preaudit list (L45, L95) |
| "layer L1, no internal deps" | Cargo.toml:3 | REAL | [dependencies] has zero path deps |

## 7. Dead / duplicated / unreachable code
None. (Unused-dependency check: both deps used in src.)

## 8. Dependencies & flags
toml = "1.1" → locked 1.1.8+spec-1.1.0; thiserror = "2.0" → locked 2.0.21. No feature gates anywhere in the crate.

## 9. Open questions for the next agent
- Who closes the toml VERIFY-P1 (license re-check at pin)? If the SBOM gate (full_gate step) reads the preaudit table, the open marker should surface there — the baseline full_gate shows the sbom section RED; connection UNVERIFIED by me.
