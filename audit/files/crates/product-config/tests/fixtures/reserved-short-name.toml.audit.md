# AUDIT: crates/product-config/tests/fixtures/reserved-short-name.toml
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines | 226 (stale base + 1 mutation) |
| File verdict | REAL (fixture; serves its rule row) |
| Reachable from shipped entry point? | N/A — consumed by `tests/semantic.rs::reserved_short_name_is_semantic` (baseline: ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 (shared stale base; reserved-list location) |
| Commands run | `diff` vs base (exactly 1 mutation); `sed -n '22p' | od -c` (byte-verified); `rg RESERVED_NAMES` semantic.rs:85 |

## 1. One-paragraph verdict
Valid fixture for doc 07 §3.3(a): `short_name` mutated to `"vlc"` — a reserved clean-room name. The test asserts exactly one semantic diagnostic for `identity.short_name` with rule containing "reserved" and found `"vlc"`. The reserved list is a 3-entry hardcoded const in semantic.rs (`["VLC", "VideoLAN", "vlc"]`), enforced at the SEMANTIC stage, not in the JSON schema.

## 2. What actually works (verified)
— `sed -n '22p' | od -c` -> `short_name = "vlc"\n`; baseline `reserved_short_name_is_semantic ... ok`; `rg` shows `pub const RESERVED_NAMES: &[&str] = &["VLC", "VideoLAN", "vlc"];` (semantic.rs:85) and the short_name check at semantic.rs:163-173.

## 3. Findings
F-001 [S4] Stale base — identical to unknown-top-level.toml F-001 (diff-verified).
F-002 [S4]
Where: crates/product-config/src/semantic.rs:85 (consumes this fixture)
Code:
```
pub const RESERVED_NAMES: &[&str] = &["VLC", "VideoLAN", "vlc"];
```
What is wrong: the reserved-name list is a Rust const, so adding a reserved name requires a code change, while the parallel structural mechanism (`not: { enum: [...] }` in product.schema.json, handled by schema.rs:204-223) is data-driven. Two mechanisms for the same rule class (fixture map lists `reserved-name.toml` for `name = "VLC"` as the structural(?) row and this file as "semantic pass"). Also the const checks only `short_name` — `name`/`publisher` go through the other path. Not a hole (both paths tested), but a split-brain rule implementation. Fixture itself is fine.
Confidence: HIGH — Verification: rg + read of the check site; reserved-name.toml path not audited (other agent's fixture).
Confidence (F-001): HIGH — Verification: diff.

## 4. Per-function table
Data file — N/A. Mutation (byte-verified): L22 only.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| semantic::reserved_short_name_is_semantic | YES | `one()` pins exactly 1 diagnostic + "reserved" rule + found `"vlc"`; deleting the RESERVED_NAMES short_name check fails it |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`short_name = "vlc"` (semantic pass)" (fixture map) | tests/fixtures/README.md:49 | REAL | od-verified + baseline pass |
| clean-room rule "must not be a reserved name" (MP §1) | semantic.rs:168 | REAL | enforced + tested via this fixture |

## 7. Dead / duplicated / unreachable code
Duplicated stale base (by design).

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
Is the `name = "VLC"` reserved check (reserved-name.toml, other agent) enforced via schema `not.enum` or via the same const? If schema, the two reserved mechanisms should be unified.
