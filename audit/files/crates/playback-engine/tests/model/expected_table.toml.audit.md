# AUDIT: crates/playback-engine/tests/model/expected_table.toml
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-23 |
| Lines / functions | 606 / 104 `[[pair]]` entries (33 rows + 71 rejections) — data artifact, no functions |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test artifact) — consumed by `tests/model_check.rs:240` via `include_str!("model/expected_table.toml")`; proven live in the workspace test lane |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
This artifact is what it claims to be and the claim is enforced: a complete,
duplicate-free 104-entry classification of every (state, input) pair, verified
three ways (engine table / independent hand-written oracle / this file) by
`tests/model_check.rs`, all 5 tests green in the baseline. I recounted the file
independently: 104 `[[pair]]` headers, 33 `to =` rows, 71 `reject =` rejections —
exactly what the checker pins (model_check.rs:310-312). Spot-checks of rows
against `state.rs::TRANSITIONS` (buffering+stop, playing+seek, paused+primed,
ended+seek) agree on targets, effect order, and rejection reasons. The only
defects are in the artifact's own header comment, whose row-count arithmetic is
stale and contradicts the file's contents and its own first line.

## 2. What actually works (verified)
- Baseline (audit/runs/baseline/cargo-test-workspace.txt:4265-4273):
  `Running tests/model_check.rs ... test result: ok. 5 passed; 0 failed` —
  `table_oracle_and_artifact_agree_on_every_pair`,
  `the_artifact_covers_every_pair_exactly_once`,
  `the_artifact_edges_are_exactly_the_wire_edge_set` (21 edges, doc 05 §3.3),
  `the_artifact_reasons_match_the_total_rejection_function`,
  `the_artifact_row_set_is_the_engine_row_set`.
- Independent recount (rg): `^\[\[pair\]\]` = 104, `^to = ` = 33,
  `^reject = ` = 71 — matches the checker's pins and my state.rs audit.
- Oracle independence verified by reading tests/model/oracle.rs:47-120:
  straight-line match arms with doc citations, no `TRANSITIONS` lookup, no
  wildcard (compile-forced totality).

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/model/expected_table.toml:31-32
Code:
```
# Row count after card 3.C1: 30 legal rows, 66 typed rejections.
# Card 3.C1's four new rows (vs the 2.C2 baseline of 26/70):
```
What is wrong: the file contains 33 legal rows and 71 rejections (104 total);
the checker pins `assert_eq!(rows, 33)` / `assert_eq!(rejections, 71)`
(model_check.rs:310-311), and the artifact's own line 2 says "104 entries,
exactly". The header's "30 legal rows, 66 typed rejections" (= 96, the
pre-3.A3 world) contradicts line 2 of the same file and the file's body. This
file's entire raison d'être is to be the human review surface where "adding a
row is a deliberate act" — a header that miscounts the rows undercuts that
review. Why it matters: a reviewer diffing a new row against this header
cannot trust the arithmetic baseline. What correct looks like: "33 legal rows,
71 typed rejections (3.C1's four + 3.A3's three over the 2.C2 baseline of 26)".
Confidence: HIGH — Verification: rg counts + model_check.rs pins + baseline.

F-002 [S4]
Where: crates/playback-engine/tests/model/expected_table.toml:2-3
Code:
```
# one entry per (state, input) pair — 8 states x 13 inputs = 104
# entries, exactly (3.C1's four rows + 3.A3's three work-carriers). This file is the HUMAN review surface for every
```
What is wrong: the parenthetical "(3.C1's four rows + 3.A3's three
work-carriers)" is a dangling fragment — 4 + 3 = 7 explains nothing about 104
(it presumably means "4+3 rows beyond the 2.C2 baseline"), and the line runs
long un-wrapped. Cosmetic rot in the review surface's front door. Confidence:
HIGH — Verification: read.

## 4. Per-function table
Not applicable (data artifact). Structure inventory: 104 `[[pair]]` entries in
wire-catalog state order (idle 13, opening 13, buffering 13, playing 13,
paused 13, stopped 13, ended 13, error 13), each `to`+`effects` or `reject`,
kebab-case vocabulary matching `parse_state`/`parse_input`/`parse_effect`/
`parse_reason` in model_check.rs exactly (verified for all spellings by the
passing checker — an unknown token is an `unreachable!` parse failure).

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| table_oracle_and_artifact_agree_on_every_pair | YES | three independent opinions must match per pair; any row/reason drift names the pair |
| the_artifact_covers_every_pair_exactly_once | YES | 104 entries, no dupes, no missing pair (HashSet over ALL × ALL) |
| the_artifact_edges_are_exactly_the_wire_edge_set | YES | artifact edge projection == commands::PlaybackState::can_transition_to, 21 edges |
| the_artifact_reasons_match_the_total_rejection_function | YES | pins rejection_reason directly |
The artifact reader is strict (unknown line/key = unreachable! parse failure),
so the artifact cannot silently drift from its grammar. This is the strongest
test pattern found in my assignment: genuinely independent authorship
(oracle.rs is hand-written match arms citing doc 03 §4.6 / doc 05 §3.3).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one entry per (state, input) pair — 104 entries, exactly" | expected_table.toml:2 | REAL | rg count 104; checker test |
| "30 legal rows, 66 typed rejections" | expected_table.toml:31 | FALSE | 33/71 actual; model_check.rs:310-311 pins 33/71 |
| "the close-from-active trio is typed settle-first" | expected_table.toml:35-37 | REAL | :207/:282/:357 match state.rs:826/:831/:835 |
| "tests/model_check.rs loads this file and proves the three-way agreement" | expected_table.toml:8-12 | REAL | model_check.rs:280-313; baseline 5/5 green |

## 7. Dead / duplicated / unreachable code
None — every entry is loaded and asserted by the checker; vocabulary is total.

## 8. Dependencies & flags
None (TOML data). Parsed by the std-only hand-rolled reader inside the test —
consistent with playback-engine's zero-external-dependency contract.

## 9. Open questions for the next agent
- model_check.rs:318 comment says "compares per pair over all 96 anyway" —
  another stale 96 (should be 104); same family as state.rs:776 and this
  file's :31. Worth a single sweep for the stale-count family.
- The 21-edge wire set is pinned twice (table.rs:188 and model_check.rs:354) —
  intentional dual authorship or duplication debt?
