# AUDIT: crates/playback-engine/tests/table.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-23 |
| Lines / functions | 846 / 27 (3 helpers + 24 items incl. 23 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test binary). It verifies `playback-engine/src/state.rs` (TRANSITIONS, apply_input, rejection_reason, translate_pipeline_signal, close_plan) which IS shipped: `session.rs:1550` runs `apply_input` in the live executor (`let row: &'static Transition = apply_input(self.state, input)?;`), so the audited model is the release lane's real state machine. Ran live: `cargo test -p playback-engine --test table` -> 23 passed, 0 failed (exit 0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p playback-engine --test table` -> 23 passed (exit 0); `rg`/`awk`/`sed` verifications |

## 1. One-paragraph verdict
A genuine exhaustive model-check suite over the session state machine: every one of the 104 (8×13) `(state, input)` pairs is classified against the table, every legal row proven wire-conformant, the engine's edge projection proven equal to the wire's 21-edge set, the pipeline-signal translation proven closed over illegal inputs, and every row walked from `Idle` through BFS-derived legal paths. Cross-checks against the generated command catalog (class A/S, gen-bumping) and doc-03 effect columns are real reconciliation, not tautology. The two defects are cosmetic: a stale "26 rows" comment (the table has 33) and the hand-written `ALL` vocabulary arrays that no compiler force to track enum variants.

## 2. What actually works (verified)
— verified by `cargo test -p playback-engine --test table` -> `test result: ok. 23 passed; 0 failed; 0 ignored` (exit 0). Includes the exhaustive-pair test (`legal == 33`, `illegal == 71`, `104 - TRANSITIONS.len()` arithmetic), the 21-edge projection equality, the 3.C1 row pins at full effects detail, and the 5 walked session lives (happy/stopped/failed/stop-while-opening/buffered-pause).
— cross-verified the load-bearing claim: `apply_input` is the live executor's only transition path (session.rs:1550), and the independent opinions the header cites exist and pass (tests/model_check.rs, 5 tests, baseline cargo-test-workspace.txt:4249-4261).

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/table.rs:516-517
Code:
```
fn every_row_is_walked_from_idle_through_legal_inputs_only() {
    // For each of the 26 rows: walk a legal path to `row.from`, then
```
What is wrong: The comment says "the 26 rows"; the loop iterates `TRANSITIONS.iter()` which the same file pins at 33 rows (`assert_eq!(legal, 33, "the pinned completed-machine row count")`, line 134). Stale count in the very suite whose job is counting.
Why it matters: The file's own header is arithmetically careful (104 = 33 + 71); a stale 26 in the walk test misleads a reader auditing coverage.
What correct looks like: "For each of the 33 rows".
Confidence: HIGH — Verification: read + ran (33-row pin passes).

F-002 [S4]
Where: crates/playback-engine/tests/table.rs:776-790 (against state.rs:111, 222)
Code:
```
fn the_state_and_input_vocabularies_are_unique_and_complete() {
    assert_eq!(SessionState::ALL.len(), 8);
    // Card 3.A3: `SwitchTrack` joins the vocabulary (13).
    assert_eq!(SessionInput::ALL.len(), 13);
```
What is wrong: The exhaustiveness of the whole suite rests on `SessionState::ALL` / `SessionInput::ALL`, which are hand-written const arrays in state.rs (no `const _: () = match ...` exhaustiveness hook). A new enum variant forces updates to `as_wire`'s exhaustive match (compile error) but nothing forces adding it to `ALL` — the arrays could silently omit it and this test stays green (it only checks the array against itself). Today the arrays are complete (verified: 8 and 13 enum variants, matching ALL).
Why it matters: Same blind-spot class as the events-contract engine-kind test: "exhaustive" claims enforced by hand-maintained lists.
What correct looks like: A `const`-context exhaustive match over the enums building ALL, or a macro-generated ALL.
Confidence: HIGH — Verification: read + awk variant counts (8/13 match).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| table_row | 37-41 | REAL | — | Linear find over the static table. |
| reachable_from | 45-65 | REAL | — | BFS over table rows; parent map keyed by target. |
| path_to | 69-94 | REAL | — | Backwards walk through the parent map; re-derives `from` by (input, to) lookup — a duplicate (input, to) pair could mis-walk, but the consumer re-applies every step via `apply_input` and asserts the landing (lines 521-536), so a wrong path fails loudly, never silently. |
| state_index | 180-185 | REAL | — | Index into ALL. |
| every_state_input_pair_is_classified_exactly_by_the_table | 101-136 | REAL | — | The core exhaustive check; 104/33/71 pinned. |
| the_table_has_no_duplicate_rows | 138-152 | REAL | — | O(n²) over 33 rows — fine. |
| transition_table_accessor_is_the_table | 154-160 | REAL | — | Public accessor equals the table. |
| every_legal_row_is_wire_conformant | 167-178 | REAL | — | `can_transition_to` check per row. |
| the_engine_edge_projection_is_exactly_the_wire_edge_set | 187-209 | REAL | — | Both directions enumerated; 21 pinned. |
| self_transitions_are_the_idempotent_confirmations_and_seek_work_rows | 211-238 | REAL | — | The 10 self-rows pinned with owners (C8/3.A3/3.C1). |
| rejection_reason_agrees_with_apply_input_everywhere | 244-258 | REAL | — | Total reason coverage. |
| the_interesting_rejections_carry_their_documented_reasons | 260-309 | REAL | — | 30 pinned reason cases. |
| open_on_a_live_session_is_source_replacement_everywhere | 311-325 | REAL | — | All 7 non-Idle opens refuse typed (3.A2). |
| the_card_3c1_completion_rows_are_pinned | 331-374 | REAL | — | Four rows with full effects vectors. |
| the_close_plan_composes_wire_conformant_trails_only | 380-412 | REAL | — | close_plan walked from every state; ≤2 steps; lands Idle. |
| rejections_render_with_state_input_and_reason | 414-424 | REAL | — | Display contains state/input/reason tokens. |
| translate_never_yields_an_illegal_input | 431-449 | REAL | — | 8 states × 5 signals closure check. |
| translate_mappings_are_pinned | 451-498 | REAL | — | 20 pinned mappings incl. absorbed facts. |
| every_state_is_reachable_from_idle | 504-513 | REAL | — | BFS witnesses. |
| every_row_is_walked_from_idle_through_legal_inputs_only | 515-538 | REAL | — | F-001 (stale comment only; the walk covers all 33 rows). |
| the_full_happy_path_and_exits_walk_cleanly | 540-601 | REAL | — | Five nominal lives, input-by-input. |
| session_commands_reconcile_with_the_wire_catalog | 607-672 | REAL | — | Cross-checked against generated `catalog_data()` — not a tautology. |
| gen_bumping_commands_carry_the_bump_effect_and_vice_versa | 674-708 | REAL | — | Catalog semantics ↔ table effects, both directions, Idle+Close exception documented. |
| effect_columns_match_the_doc_03_side_effects | 710-769 | REAL | — | Doc-anchored effect invariants per row class. |
| the_state_and_input_vocabularies_are_unique_and_complete | 775-790 | REAL | S4 | F-002. |
| engine_states_map_one_to_one_onto_the_wire_states | 792-822 | REAL | — | 1:1 wire mapping + display names + 7 command inputs. |
| command_kinds_map_onto_table_inputs | 824-846 | REAL | — | The class-S subtitle flavor's mapping documented as harmlessly unreachable. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| every_state_input_pair… / rejection_reason_agrees… | YES | Full 104-pair classification vs the table; any drift in either direction fails. |
| the_engine_edge_projection… | YES | Independent two-sided enumeration of engine edges vs wire edges. |
| translate_never_yields… / translate_mappings_are_pinned | YES | Closure over all (state, signal) + 20 pinned cases. |
| every_row_is_walked… / every_state_is_reachable… | YES | BFS-derived paths re-applied through apply_input with landing asserts. |
| session_commands_reconcile… / gen_bumping… | YES | Reads the generated catalog — a catalog/engine drift fails. |
| effect_columns… | YES | Doc-anchored invariants; an effect removed/misplaced fails its row. |
| the_close_plan… | YES | Every state's plan walked; wire-conformance per step. |
| Vocabulary tests (2) | PARTIALLY | Blind to NEW enum variants omitted from ALL (F-002). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "104 pairs: 33 rows, 71 typed rejections — 26/70 before 3.C1's four rows and 3.A3's three" | header, lines 10-12 | REAL | Arithmetic reconciles: 8×12=96 (26+70) before; +4 converted illegal→legal; +SwitchTrack adds 8 pairs (3 legal/5 illegal) → 104=33+71. Test passes. |
| "the engine's edge projection is exactly the wire's 21-edge set" | header, lines 19-21 | REAL | Test at 187-209 enumerates both sides, passes. |
| "the independent second and third opinions … live in tests/model_check.rs" | header, lines 13-15 | REAL | model_check.rs exists; 5 tests pass in baseline (cargo-test-workspace.txt:4249-4261). |
| "For each of the 26 rows" | line 517 | FALSE (stale) | 33 rows pinned by the same suite. F-001. |
| `apply_input` is the only way states move (lib.rs claim this suite enforces) | lib.rs:40 | REAL | session.rs:1550 is a single apply_input call site in the executor; this suite + model_check cover the table it consults. |

## 7. Dead / duplicated / unreachable code
- None. All three helpers are used by the walk tests; every test runs (23/23 live, 0 ignored).
- The `SessionInput::SelectSubtitle → SwitchTrack` mapping is documented as never reaching the table (executor's pre-table branch owns the class-S flavor) — an honest, pinned unreachable.

## 8. Dependencies & flags
- Depends on `playback_engine::state` (pure model), `playback_engine::{SessionCommandKind, transition_table, close_plan}`, `commands::{CommandId, PlaybackState, catalog::spec}` — all shipped; no feature gates; runs in the plain test lane (verified live).

## 9. Open questions for the next agent
- state.rs itself (TRANSITIONS data, translate, close_plan) is another agent's file: its row data is only as good as this suite says — this suite proves internal consistency and wire conformance, not product desirability of each edge (e.g. `Ended + Play → RestartNeedsSeek` is a product choice, Phase-3 per the pins).
- The committed expected-table artifact (`tests/model/expected_table.toml`) is the third opinion — not my file; confirm it is generated from (not hand-copied after) the table.
