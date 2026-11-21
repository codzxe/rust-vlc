# AUDIT: crates/product-config/src/suggest.rs
| Field | Value |
|---|---|
| Auditor | W11-A01 |
| Date | 2026-11-21 |
| Lines / functions | 107 / 2 (levenshtein, did_you_mean) + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO from `lumen` CLI / `lumen-tauri` (no dependency edge to product-config; see diagnostic.rs audit). YES from workspace-built `control-server` binary (baseline #1, build log :234): main.rs:131-154 → `load_file` → schema.rs:362 unknown-key path calls `did_you_mean`; end-to-end tested by tests/fixtures_matrix.rs:347 (passed in baseline #3) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "did_you_mean|suggest::|levenshtein" -g '*.rs'` (callers: schema.rs:22,362 only; settings-schema has a separate byte-based `edit_distance` in runtime.rs:121) · `sed -n '350,372p' crates/product-config/src/schema.rs` (caller context verified) · `sed -n '340,365p' crates/product-config/tests/fixtures_matrix.rs` (e2e typo test verified) · `sed -n '45,52p'` (verbatim quote check) |

## 1. One-paragraph verdict
A small, correct, genuinely wired "did you mean" module: char-level Levenshtein with the standard two-row DP, a thresholded closest-match picker, and both functions exercised — the unit tests cover the doc 07 §3.5 rule-4 example, and the end-to-end fixture test (`unknown_key_typo_gets_a_did_you_mean_hint`) proves the suggestion actually reaches a rendered diagnostic through the real schema-validation pipeline (baseline #3: passed). No fakeness, no panics, no swallowed errors. The only defects are hygiene-level: the threshold doc comment ("one third of the typo's length") disagrees with the implemented `clamp(2, 4)` cap for keys ≥ 15 chars and measures bytes while the distance is char-level (S4), and `levenshtein` carries three unreachable defensive `None` arms with a magic `usize::MAX / 4` sentinel (S4).

## 2. What actually works (verified)
— `levenshtein` is correct DP: verified by reasoning over the two-row recurrence and the classic checks (`kitten`/`sitting` = 3, empty-string cases = 3) in tests that passed in baseline #3 (`cargo test --workspace --locked` → 1681 passed / 1 failed, failure unrelated: apps/desktop zero_alloc).
— `did_you_mean` produces doc 07 §3.5 rule 4's exact example: unit test `doc_07_s35_example_suggests_the_right_key` (typo `cache_demux_mb` → `cache_demux_mib`), plus the e2e fixture test asserting the rendered hint contains "did you mean \`cache_demux_mib\`" with the right line number (tests/fixtures_matrix.rs:347-363) — same baseline run.
— Wired into the production path: `schema.rs:22` imports it; `schema.rs:362` calls it in the `additionalProperties: false` unknown-key branch, prefixing the hint with "did you mean \`X\`?" before the legal-keys list — reachable from the control-server binary's product.toml load.

## 3. Findings

### F-001 [S4]
Where: crates/product-config/src/suggest.rs:46-51
Code:
```rust
/// Threshold: edit distance <= 2, or <= one third of the typo's length for
/// longer keys (doc 07 §3.5 gives `cache_demux_mb` vs `cache_demux_mib`,
/// distance 1) `[PROPOSAL: exact threshold]`.
pub fn did_you_mean<'a>(typo: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let max_dist = (typo.len() / 3).clamp(2, 4);
```
What is wrong: Two comment/code mismatches. (1) The comment promises "one third of the typo's length for longer keys" with no ceiling; the code caps at 4 (`clamp(2, 4)`), so a typo of ≥ 15 bytes with edit distance 5-6 to its intended key gets NO suggestion where the comment says it should. (2) `typo.len()` is bytes while `levenshtein` counts chars — for a non-ASCII typo the threshold is inflated (a 6-CJK-char typo: 18 bytes → threshold 4 chars instead of 2). Both directions are cosmetically wrong only (a missed or over-eager hint), and the exact threshold is marked `[PROPOSAL]` — but the comment describes a different formula than the one shipped.
Why it matters: The comment is the spec for anyone tuning the threshold; it describes behavior the code does not have. Tests only cover the len-13/distance-1 sweet spot, so the cap and the byte/char split are both untested behavior.
What correct looks like: Comment states the real formula — "max(2, min(len/3, 4)) over bytes" — or the code matches the comment (`(typo.chars().count() / 3).max(2)` with an explicit documented cap); add one test with a ≥ 15-char typo at distance 5 (asserts the cap either way).
Confidence: HIGH — Verification: read; arithmetic checked by hand; no test exercises len ≥ 15.

### F-002 [S4]
Where: crates/product-config/src/suggest.rs:21-35
Code:
```rust
            let up = match prev.get(j + 1) {
                Some(v) => *v + 1,
                None => usize::MAX / 4,
            };
            let left = match curr.get(j) {
                Some(v) => *v + 1,
                None => usize::MAX / 4,
            };
```
What is wrong: All three `None` arms are unreachable: `prev` and `curr` always have `b.len() + 1` elements and the loop indices satisfy `j + 1 <= b.len()` and `j < b.len()`, so every `.get()` is `Some`. The `usize::MAX / 4` magic sentinel can never flow into the min(). The early `if a.is_empty() / if b.is_empty()` returns make the `prev.last()` `None` arm (line 41) dead too (`prev` has ≥ 1 element once `b` is non-empty, and empty-`b` returned earlier).
Why it matters: Dead defensive arms with an arbitrary magic value are noise that suggests the invariants are unknown; a future refactor that breaks an invariant would silently compute against `usize::MAX / 4` instead of panicking loudly.
What correct looks like: Direct indexing with a debug_assert on lengths (the file's crate bans panics in non-test code — a debug_assert is the compliant loud option), or a comment stating why the arms exist.
Confidence: HIGH — Verification: read; index bounds traced by hand (`j in 0..b.len()`, vectors of length `b.len()+1`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `levenshtein` | 6-43 | REAL | S4 | Correct two-row DP, char-level (Unicode-safe), O(a·b) time / O(b) space — fine for schema key sets (tens of keys). Early empty-shortcuts correct incl. both-empty → 0. Dead defensive arms → F-002. |
| `did_you_mean` | 50-66 | REAL | S4 | Thresholded argmin; `d == 0` skip is correct for the unknown-key caller (key cannot be in candidates there — defensive); tie-break keeps the FIRST candidate (deterministic, candidate order = schema property order); threshold comment drift → F-001. |
| test `distance_zero_for_identical` | 72-75 | REAL | — | Trivial, real. |
| test `distance_one_for_single_substitution` | 78-80 | REAL | — | The doc 07 §3.5 example pair. |
| test `distance_counts_insert_delete_substitute` | 82-87 | REAL | — | Classic kitten/sitting + both empty-string directions. |
| test `doc_07_s35_example_suggests_the_right_key` | 89-101 | REAL | — | doc-claim pin: rule 4's example works through `did_you_mean`, not just the raw distance. |
| test `no_suggestion_for_unrelated_keys` | 103-106 | REAL | — | Negative path ("zebra" vs one candidate, distance 17 > threshold → None). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `doc_07_s35_example_suggests_the_right_key` | YES | If `levenshtein` returned a constant 0, the `d == 0` skip yields None → assert fails. If the threshold collapsed to 0 (`clamp` broken), distance 1 > 0 → None → fails. |
| `distance_counts_insert_delete_substitute` | YES | kitten/sitting=3 pins substitution+insertion; empty cases pin the shortcuts. A broken recurrence (e.g. swapped rows after `core::mem::swap`) fails it. |
| `no_suggestion_for_unrelated_keys` | WEAK | Only fails if everything suddenly suggests; a threshold that never fires passes trivially — but the positive test covers that direction. |
| e2e `unknown_key_typo_gets_a_did_you_mean_hint` (fixtures_matrix.rs:347) | YES | Drives fixture file → real `load_file` → schema walk → this module → asserts the rendered hint text and line number; catches wiring regressions no unit test can. |
| Untested behaviors | — | Tie-breaking (two equidistant candidates) and the len ≥ 15 clamp cap (F-001) have no tests. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "(4) where a near-miss key exists (`cache_demux_mb` vs a typo'd `cache_demux_mb`)… the hint says 'did you mean `…`' (Levenshtein over the known key set)" | docs/07 §3.5 rule 4 (:458-460) | REAL | schema.rs:362 wiring + unit test (file:89-101) + e2e fixture test (fixtures_matrix.rs:347), all green in baseline #3. Doc's own example pair works verbatim. |
| "[PROPOSAL: exact threshold]" disclosure | suggest.rs:49 | REAL | Honest flag; but the prose above it still misdescribes the implemented formula → F-001. |

## 7. Dead / duplicated / unreachable code
- Unreachable: the three `None` sentinel arms + `prev.last()` None arm in `levenshtein` (F-002).
- Duplication (cross-crate, by design but drifting): `settings-schema/src/runtime.rs:121 edit_distance` is a SECOND Levenshtein implementation — byte-based ("keys are ASCII" per its comment) — while this one is char-based. Both crates are L1 "no internal deps", so no shared util crate is allowed by the layering rule; the cost is two implementations with different Unicode semantics and different thresholds (settings-schema: ≤ 3 fixed; product-config: clamp(2,4) scaled). Not filed as a finding (layering rule forces it), but the semantic drift is worth a Lead note.
- Nothing else dead; both functions have production callers (schema.rs:362) or are the called utility.

## 8. Dependencies & flags
- std-only (plus `core::mem::swap`); zero external/internal deps — L1 compliant.
- No feature flags, no cfg-gating, no `unsafe`, no `unwrap`/`expect`/`panic` in non-test code. Clippy-clean per baseline #5/#6.

## 9. Open questions for the next agent
- A02 (settings-schema/runtime.rs): your `edit_distance` is byte-based with a fixed ≤ 3 threshold; this crate's is char-based with clamp(2,4). If the two hint surfaces are ever unified, pick one semantics deliberately (see §7).
- A04 (product-config/schema.rs): the suggestion only fires under `additionalProperties: false` tables — confirm every product.toml table that carries user-typoable keys actually sets it (the fixture proves `limits` does; a table without the flag silently gets no typo detection).
