# AUDIT: crates/playback-engine/tests/model_check.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 407 / 5 test fns + 9 parser helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test) — plain `#[test]`s, no ignores; part of W00 baseline (execution UNVERIFIED, no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read; peek at tests/model/oracle.rs (376 LOC, no-wildcard match verified in header + structure) and model/expected_table.toml (606 LOC, exists); sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The three-way exhaustive model check of the session state machine: for all 104 `(state, input)` pairs, the engine's `TRANSITIONS`/`apply_input` must agree with an imperative oracle (`tests/model/oracle.rs`, a no-wildcard total match) and a committed TOML artifact (`model/expected_table.toml`, strict-parsed by a hand-rolled reader) on target state, effects, and typed rejection reason — plus shape invariants: 104 entries exactly once, 33 rows / 71 rejections, no duplicate rows, wire-conformance of every row, the artifact's edge projection equal to the wire's 21-edge set, and the three 3.C1 completion edges present. This is the anti-echo-chamber pattern (directive D-5) done properly: three structurally distinct representations (data table / straight-line match / parsed artifact) that must agree pairwise, with loud per-pair failure messages naming all opinions. The single caveat: "independently authored" is an authorship claim I cannot verify without git history — the structural independence is real, the process claim is not auditable from the tree.

## 2. What actually works (verified)

- Exhaustiveness is enforced from both ends: the artifact must contain every pair in `SessionState::ALL × SessionInput::ALL` (264-273) and have no duplicates (256-263); a new state or input breaks the test until all three authors classify it.
- The agreement test (280-313) compares engine-vs-oracle AND oracle-vs-artifact per pair, with failure messages naming the pair and both opinions; the row/rejection counts (33/71) and `TRANSITIONS.len()` are pinned.
- The row-set test (320-347) is a stronger belt-and-braces than count equality: sorted row tuples compared for equality (same rows, not same count).
- The edge-projection test (354-389) derives the wire edge set from `can_transition_to` over all state pairs — a genuinely independent derivation — and demands the artifact's projection equals it, with the 21-edge count and the three 3.C1 completion edges asserted present.
- The rejection-reason test (395-407) pins the total `rejection_reason` function against the artifact directly.
- The artifact reader is strict by design: unknown keys/lines/state/input/effect/reason names panic with line numbers (101, 124, 183, 202, 221, 235) — the artifact cannot drift from the grammar silently.
- `leak_effects` (162-164): `Box::leak` of ≤104 small slices, test-side, documented and bounded.

## 3. Findings

### F-001 [S4] "Three independently authored opinions" — authorship independence is unverifiable from the tree
Where: crates/playback-engine/tests/model_check.rs:2-5 vs oracle.rs:4-10
Code (model_check.rs:2-5):
```rust
//! The three-way exhaustive proof (card 3.C1's model-based AC): for
//! every one of the `8 × 13 = 104` `(state, input)` pairs, three
//! **independently authored** opinions of the documented machine must
//! agree exactly — target state, side effects, and typed rejection
//! reason:
```
What is wrong: Nothing in the code — the claim of independent *authorship* cannot be audited without git history (adaptation: no git). What IS verified: structural independence (data table vs no-wildcard imperative match vs strict-parsed text artifact — a single systematic error would have to be replicated identically in three different representations to evade the check). The oracle's own header makes the same unverifiable claim ("deliberately NOT generated from … the engine's TRANSITIONS").
Why it matters: The suite's evidentiary weight rests partly on the independence claim; an auditor or claim-ledger agent should record it as REAL-structure / UNVERIFIED-process.
What correct looks like: The claim is already scoped in the headers; the final report should carry the caveat.
Confidence: HIGH (on what is and is not verifiable) — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `parse_artifact` | 60-131 | REAL | Strict fixed-grammar reader; fail-loud on drift. |
| `finish` | 133-155 | REAL | Exactly-one-of to/reject enforced (146-148). |
| `leak_effects` | 162-164 | REAL | Documented bounded test-side leak. |
| `unquote`/`parse_state`/`parse_input`/`parse_effect`/`parse_reason` | 166-237 | REAL | Total vocabularies; unknown → panic with line. |
| `artifact` | 239-241 | REAL | include_str! — the artifact ships in-tree. |
| the_artifact_covers_every_pair_exactly_once | 252-274 | REAL | Both-ends exhaustiveness. |
| table_oracle_and_artifact_agree_on_every_pair | 280-313 | REAL | The three-way proof; counts pinned. |
| the_artifact_row_set_is_the_engine_row_set | 320-347 | REAL | Row-tuple equality. |
| the_artifact_edges_are_exactly_the_wire_edge_set | 354-389 | REAL | Independent wire derivation; 21 edges + 3 completions. |
| the_artifact_reasons_match_the_total_rejection_function | 395-407 | REAL | Total-function pin. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 5 | YES | Mutation examples: change any TRANSITIONS row → agreement test names the pair; drop an artifact entry → completeness test; add an illegal wire edge (or remove a legal one) → edge test; change a rejection reason → reasons test; malformed artifact line → parser panic with line number. |

This suite is the antidote to the self-referential `table.rs` proof #1 it replaced (14-16) — table.rs is my file #26; consistency between the two files' claims will be checked there.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "8 × 13 = 104 (state, input) pairs" (2) | module doc | REAL | Test asserts 104 over ALL×ALL. |
| "33 legal rows / 71 typed rejections" (26-27) | module doc | REAL | Pinned at 310-312. |
| "no `toml` crate; the grammar is … exactly as the artifact's header documents" (19-22) | module doc | REAL-UNPROVEN | Parser verified against its stated grammar; the artifact's own header conformance spot-checked (606-line TOML exists); full read of the artifact is another agent's file. |
| "independently authored" (3) | module doc | UNVERIFIED | F-001 — authorship not auditable without git. |
| "the 3.C1 completions … were the deliberately-absent debt this card paid" (377-379) | comment | UNVERIFIED | Historical claim; the edges' presence IS verified (379-388). |

## 7. Dead / duplicated / unreachable code

None. (The sort-key closures in 328-334 and 339-345 are duplicated once — belt-and-braces test code, not filed.)

## 8. Dependencies & flags

dev-deps: playback-engine, testkit. std-only reader per C5. No feature gates. The oracle is path-included (33-34) — correct handling of cargo's tests/ auto-discovery limitation, documented.

## 9. Open questions for the next agent

- Whoever has tests/model/oracle.rs and tests/model/expected_table.toml: confirm the oracle's match is truly wildcard-free and the artifact's header grammar matches the parser (I spot-checked; full audit is yours).
- W03 colleague with state.rs: `SessionState::ALL`/`SessionInput::ALL` completeness is the linchpin — confirm the arrays are hand-maintained and complete (this suite fails loudly if not, so the risk is contained).
