# AUDIT: crates/product-config/src/suggest.rs
| Field | Value |
|---|---|
| Auditor | W11-A07 |
| Date | 2025-06-14 |
| Lines / functions | 107 / 2 prod fns + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO for the release lane (lumen-cli + desktop do not depend on product-config). Reachable from apps/control-server (Cargo.toml dep) and the xtask config-validator lane. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | baseline citation (cargo-test-workspace.txt:5066-5071: all 5 suggest tests ok); rg for did_you_mean callers; sed quote verification. Own re-run blocked: disk full (cargo-build-budget). |

## 1. One-paragraph verdict
A correct, tested Levenshtein "did you mean" module. The DP is right (classic kitten/sitting=3 and empty-string cases pinned by tests), tie-breaking is deterministic (first candidate wins), exact matches are excluded, and the end-to-end consumer path is real: schema.rs's unknown-key diagnostic calls did_you_mean, proven by the fixtures_matrix typo test (baseline PASS). Two nits: the threshold comment disagrees with the code's clamp, and the threshold uses byte length while distance is char-based.

## 2. What actually works (verified)
— baseline audit/runs/baseline/cargo-test-workspace.txt:5066-5071: `suggest::tests::{distance_zero_for_identical, distance_one_for_single_substitution, distance_counts_insert_delete_substitute, doc_07_s35_example_suggests_the_right_key, no_suggestion_for_unrelated_keys}` all ok; and :5097 `unknown_key_typo_gets_a_did_you_mean_hint ... ok` (the end-to-end path through schema.rs).

## 3. Findings
F-001 [S4]
Where: crates/product-config/src/suggest.rs:45-47 vs :51
Code (doc, :45-47):
```
/// Threshold: edit distance <= 2, or <= one third of the typo's length for
/// longer keys (doc 07 §3.5 gives `cache_demux_mb` vs `cache_demux_mib`,
/// distance 1) `[PROPOSAL: exact threshold]`.
```
vs code (:51):
```
    let max_dist = (typo.len() / 3).clamp(2, 4);
```
What is wrong: the comment promises "one third of the typo's length" for longer keys, but the clamp caps it at 4 — a 30-char typo still gets max_dist 4, not 10. The comment describes the unclamped formula. Cosmetic doc/code drift; the [PROPOSAL] marker already concedes the threshold is provisional. Also `typo.len()` is BYTES while levenshtein counts CHARS — mixed units for non-ASCII input (schema keys are ASCII in practice). Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/product-config/src/suggest.rs:12-43
Code (defensive dead branches):
```
            let up = match prev.get(j + 1) {
                Some(v) => *v + 1,
                None => usize::MAX / 4,
```
What is wrong: the None arms are unreachable (prev/curr always have b.len()+1 slots; the loop bounds guarantee j+1 in range) — four vestigial branches + the trailing `match prev.last() { None => 0 }` that cannot fire since b is non-empty by the early return. Not a correctness bug; dead defensive code that obscures the invariant. Confidence: HIGH — Verification: read (loop bounds vs vec lengths).

## 4. Per-function table (107 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| levenshtein | 6-43 | REAL | S4 | Correct char-level DP; unreachable defensive arms |
| did_you_mean | 49-67 | REAL | S4 | Deterministic first-wins ties; threshold doc drift |
| tests (5) | 70-107 | REAL | — | Honest: exact values + the doc-07 example + negative case |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| distance_zero_for_identical | YES | 0 |
| distance_one_for_single_substitution | YES | The doc-07 example pair = 1 |
| distance_counts_insert_delete_substitute | YES | kitten/sitting=3 (known-good oracle) + empty-string edges |
| doc_07_s35_example_suggests_the_right_key | YES | Distinguishes cache_demux_mib from cache_network_mib (distance-sensitive) |
| no_suggestion_for_unrelated_keys | YES | zebra vs queue_packets_audio: distance > threshold -> None |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "near-miss keys get a Levenshtein-based hint" (doc 07 §3.5 rule 4) | suggest.rs:1-2 | REAL | schema.rs caller + baseline fixtures_matrix typo test |
| threshold "one third of the typo's length" | suggest.rs:45 | PARTLY FALSE | F-001: clamp caps at 4 |

## 7. Dead / duplicated / unreachable code
The four `None =>` arms and the final `None => 0` (F-002). Note: settings-schema/src/runtime.rs:121-137 has a SECOND, byte-level edit_distance implementation (separate crate, same algorithm) — duplication across L1 crates by layering design (no internal deps), acknowledged cost.

## 8. Dependencies & flags
None (std only). No unsafe/unwrap/expect.

## 9. Open questions for the next agent
- Is apps/control-server shipped anywhere (it is the only product-config consumer)? If dev-only, the whole product-config lane is dev-lane.
