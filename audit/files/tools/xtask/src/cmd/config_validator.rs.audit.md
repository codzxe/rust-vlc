# AUDIT: tools/xtask/src/cmd/config_validator.rs

| Field | Value |
|---|---|
| Auditor | W01-A01 |
| Date | 2026-11-16 |
| Lines / functions | 1008 / ~14 entry+helpers (schema walk S, cross-file X, JSON mini-parser `json::parse`, find_root, tests) — audited head+run; full per-function walk done for the entry path, S/X check bodies spot-verified against the run output |
| File verdict | REAL (v0 with honestly-registered limits) |
| Reachable from shipped entry point? | NO — dev tooling, entry `cargo xtask config-validator` |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | `cargo xtask config-validator` → exit 1, 4 violations (`audit/runs/baseline/xtask-config-validator.txt`) |

## 1. One-paragraph verdict
A real validator: it parses product.toml with toml_lite, parses product.schema.json with a bundled mini-JSON parser, walks a JSON-Schema subset (required/additionalProperties/type/enum/ranges/items/$defs refs) and does cross-file joins (brand assets exist, locale/theme/profile listings, msgid existence in en-US.toml, title template variables, telemetry⇒privacy, schema_version vs migrations, reverse-dns app ids). Executed: FAILURE with exactly the 4 brand-asset violations the docs predict — the docs' claim "RED with 4 violations" is TRUE at HEAD. Its v0 limits are printed on every run rather than hidden: pattern constraints are NOT enforced here (owned by the product-config crate's tests), themes/profiles enum injection skipped because those planes don't exist on disk yet.

## 2. What actually works (verified)
- The full run: `violation: brand.icon_ico/icon_icns/icon_png_512/logo_svg: asset path … does not exist under the root` ×4, `result: FAILURE (4 violation(s))`, exit 1 — matches the docs' "RED with 4 violations" claim exactly.
- Pattern-constraint limits disclosed at lines 14-21 and in every run's `note:` lines — the tool does not silently pass what it cannot check (with one caveat, F-001).

## 3. Findings

### F-001 [S3] Pattern-gated fields pass this validator without any pattern check
- **Where:** `tools/xtask/src/cmd/config_validator.rs:16-18`
- **Code:**
  ```rust
  //! * `pattern` constraints are NOT enforced — std has no regex. The
  //!   `product-config` crate's committed CI suite owns them (msgid/bcp47/
  //!   https/hex shapes); this tool owns the existence/join checks above.
  ```
- **What is wrong:** The product.toml fields whose validity is entirely a `pattern` (msgid/bcp47/https/hex shapes) are validated by *another crate's test suite*, not by the validator named in product.toml's own header ("Validated by product.schema.json via `cargo xtask config-validator`"). If product.toml's pattern fields regress, this tool stays green and the failure surfaces only if the product-config tests run. The disclosure is honest; the division of labor means "validated" is weaker than the name implies.
- **Why it matters:** The v0.1.2-era paperwork leaned on "config-validator RED with 4 violations" as evidence of a working gate; a green run of this tool would NOT prove pattern validity.
- **What correct looks like:** A `--patterns` mode that shells the regex checks (or the crate's checker) so one command validates everything product.toml claims.
- **Confidence:** HIGH — **Verification:** read + ran (tool green on structure while patterns unchecked).

## 4. Per-function table (entry-path functions; helper bodies spot-verified)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 52-130+ | REAL | — | args, reads product+schema, prints report header, exit 1 on parse fail |
| schema walk (S) | ~131-600 | REAL | — | required/unknown/type/enum/ranges/items/$ref($defs) subset |
| cross-file (X) | ~600-900 | REAL | S3 | asset/locale/theme/profile/msgid/title/telemetry checks; F-001 pattern gap disclosed |
| `json::parse` (bundled) | within file | REAL | — | mini JSON parser for the schema |
| `find_root` | tail | REAL | — | duplicated walker (S4, third copy in xtask) |
| tests | tail | REAL | — | ran within the 1001-pass workspace suite |

## 5. Test honesty
Tests exist for the walk and the joins (ran in the workspace suite, all passing). They assert concrete violations on crafted products — they would fail if checks regressed. Verified by suite run, not re-run in isolation.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "config-validator is RED with 4 violations" | docs (premise §0; KNOWN-LIMITS/README to be audited W19) | REAL | run: FAILURE (4 violations) — the four brand assets missing |
| product.toml "Validated by product.schema.json via cargo xtask config-validator" | product.toml header | SHALLOW | structure+joins yes; patterns excluded (F-001) |

## 7. Dead / duplicated / unreachable code
`find_root` triplicated across xtask cmds (S4).

## 8. Dependencies & flags
std + toml_lite only; no feature flags.

## 9. Open questions for the next agent
- W11: does the product-config crate's suite really cover every pattern field product.schema.json declares? (Claimed at lines 16-17; unverified here.)
