# AUDIT: crates/playback-engine/tests/table.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 847 / 17 test fns + 3 pure helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test) — pure-model suite over `playback_engine::state`'s public items; plain `#[test]`s; part of W00 baseline (execution UNVERIFIED, no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read in full (3 chunks); cross-check against model_check.rs (my file #15) for the D-5 echo-chamber claim; sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The exhaustive transition-table suite: all 104 `(state, input)` pairs classified exactly by the table (33 rows / 71 rejections pinned), no duplicate rows, every row wire-conformant, the engine's edge projection exactly the wire's 21-edge set, rejection reasons total and pinned for the interesting cases, the card-3.C1 completion rows pinned at full effect detail, `close_plan` compositions proven wire-legal end-to-end, the pipeline-signal guard proven closed (translate never yields an illegal input, with the full mapping pinned), BFS reachability plus every row walked from Idle through legal inputs only, five named lives walked step-by-step, catalog reconciliation (command kinds ↔ wire ids/classes/gen-bumping), effect columns matched against doc 03's side-effect rules field by field, and vocabulary pins. The suite's inherent limitation — proof #1 (`apply_input` ≡ `TRANSITIONS`) checks the crate's data against the same crate's data — is disclosed in the header (10-15) and mitigated by model_check.rs's oracle and artifact opinions (my file #15, verified consistent). Two stale/weak spots only.

## 2. What actually works (verified)

- Exhaustive classification (102-136): every pair's Ok/Err matches a table row's presence exactly, with field-identity checks on rows and count pins (33/71, 104 total).
- The edge-projection proof (188-209): engine edges (dedup'd, sorted) == wire edges derived independently from `can_transition_to` over all pairs, 21 pinned. This is the strongest single check in the file — a bidirectional equality between two independently computed sets.
- The guard proof (432-449): `translate_pipeline_signal` over ALL states × 5 signals must yield only table-legal inputs — the pipeline side cannot reach an illegal transition; the mapping itself is pinned case-by-case (452-498), including the absorbed-fact `None` arms.
- Walks (504-601): BFS from Idle reaches all 8 states; every row walked via a legal path (path_to + apply_input chain, asserting landing state AND effects); five named lives (happy, stopped, failed, stop-while-opening, buffered pause) with exact per-step landing states.
- Catalog reconciliation (608-672): each `SessionCommandKind` maps to its wire `CommandId`, whose catalog spec carries the pinned completion class and gen-bumping flag — engine vocabulary and wire catalog cannot drift silently.
- Effect-column rules (711-769): fifteen implication rules over all rows (fatal-first into Error, teardown on Error, episode brackets, freeze/resume clock pairs, end-of-media + StopClock into Ended, DestroySession on close, 3.C1 bookends) — the doc 03 side-effect contract as executable rules.
- Gen-bumping biconditional (675-708): the mint effect appears exactly on the gen-bumping inputs (with the documented Idle+Close exception), nowhere else.

## 3. Findings

### F-001 [S4] Stale row-count comment ("26 rows") on the every-row walk
Where: crates/playback-engine/tests/table.rs:516-518
Code:
```rust
fn every_row_is_walked_from_idle_through_legal_inputs_only() {
    // For each of the 26 rows: walk a legal path to `row.from`, then
    // apply `row.input`, and confirm the machine lands on `row.to`.
```
What is wrong: The loop iterates `TRANSITIONS` (33 rows — pinned at line 134 and in the module header at 11). The "26" is the pre-3.C1/3.A3 count, same era as the header's own "26/70 before" note. The code is correct; the comment lies about the number it walks.
Why it matters: A reader auditing coverage ("does the walk really cover all rows?") gets the wrong arithmetic from the comment. Hygiene.
What correct looks like: "For each of the 33 rows" or "for each table row".
Confidence: HIGH — Verification: read.
### F-002 [S4] The rejection-Display test is three substring checks
Where: crates/playback-engine/tests/table.rs:415-424
Code:
```rust
    let text = rejection.to_string();
    assert!(text.contains("play"), "{text}");
    assert!(text.contains("ended"), "{text}");
    assert!(text.contains("seek"), "{text}");
```
What is wrong: The test name promises state, input, and reason rendering; the body checks three substrings of ONE rejection's message (Ended+Play → RestartNeedsSeek). Other rejection shapes' rendering (e.g., SettleFirst, SignalNotRaisedHere) are untested, and a message that concatenates the vocabulary in the wrong grammar passes.
Why it matters: Weak assertion breadth for a rendering contract; hygiene.
What correct looks like: A few exact-string pins across the reason families (the error.rs suite — my file #4 — shows the pattern).
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Test | Lines | Verdict | Notes |
|---|---|---|---|
| `table_row`/`reachable_from`/`path_to` (helpers) | 37-94 | REAL | BFS + backward walk; `unreachable!`s justified by ALL-coverage invariants. |
| every_state_input_pair_is_classified_exactly_by_the_table | 102-136 | REAL | The exhaustive classification + count pins. |
| the_table_has_no_duplicate_rows | 139-152 | REAL | O(n²) but n=33. |
| transition_table_accessor_is_the_table | 155-160 | REAL | Accessor honesty. |
| every_legal_row_is_wire_conformant | 168-178 | REAL | Direction 1. |
| the_engine_edge_projection_is_exactly_the_wire_edge_set | 188-209 | REAL | Bidirectional set equality; 21 pinned. |
| self_transitions_are_the_idempotent_confirmations_and_seek_work_rows | 212-238 | REAL | The 11 self-rows pinned with per-row rationale. |
| rejection_reason_agrees_with_apply_input_everywhere | 245-258 | REAL | Total-function agreement. |
| the_interesting_rejections_carry_their_documented_reasons | 261-309 | REAL | 25 case pins. |
| open_on_a_live_session_is_source_replacement_everywhere | 312-325 | REAL | 7-state sweep. |
| the_card_3c1_completion_rows_are_pinned | 332-374 | REAL | Full effect detail. |
| the_close_plan_composes_wire_conformant_trails_only | 381-412 | REAL | Per-state composition + step-count pins. |
| rejections_render_with_state_input_and_reason | 415-424 | SHALLOW | F-002. |
| translate_never_yields_an_illegal_input | 432-449 | REAL | The guard-closure proof. |
| translate_mappings_are_pinned | 452-498 | REAL | Full mapping table incl. None arms. |
| every_state_is_reachable_from_idle | 504-513 | REAL | BFS result. |
| every_row_is_walked_from_idle_through_legal_inputs_only | 516-538 | REAL | F-001 stale comment; walk itself exact. |
| the_full_happy_path_and_exits_walk_cleanly | 541-601 | REAL | Five named lives. |
| session_commands_reconcile_with_the_wire_catalog | 608-672 | REAL | Kind↔id↔class↔gen-bumping. |
| gen_bumping_commands_carry_the_bump_effect_and_vice_versa | 675-708 | REAL | Biconditional + play/pause row count. |
| effect_columns_match_the_doc_03_side_effects | 711-769 | REAL | Fifteen implication rules. |
| the_state_and_input_vocabularies_are_unique_and_complete | 776-790 | REAL | 8/13 pins + uniqueness. |
| engine_states_map_one_to_one_onto_the_wire_states | 793-822 | REAL | 1:1 wire mapping + Display names + 7 command inputs. |
| command_kinds_map_onto_table_inputs | 825-846 | REAL | Incl. the documented harmless SelectSubtitle arm. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 16 of 17 | YES | Mutation examples: add an illegal wire edge → projection equality; mint on a play row → biconditional; translate yielding an illegal input → guard proof; unreachable state → BFS; wrong effect on a 3.C1 row → pin; catalog drift → reconciliation. |
| rejections_render_with_state_input_and_reason | PARTIAL | Three substrings of one message (F-002). |

The suite's known structural weakness (self-reference of proof #1) is disclosed in the header and externally patched by model_check.rs — verified consistent across my two audits (same 33/71/104/21 numbers).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`apply_input` agrees with `TRANSITIONS` exactly (104 pairs: 33 rows, 71 typed rejections — 26/70 before 3.C1's four rows and 3.A3's three SwitchTrack work-carriers)" (10-12) | module doc | REAL | Test 102-136 + historical counts flagged as history. |
| "The independent second and third opinions … live in `tests/model_check.rs`" (13-15) | module doc | REAL | model_check.rs verified (my file #15): oracle + artifact, same counts. |
| "the engine's edge projection is exactly the wire's 21-edge set (18 before 3.C1's …)" (19-21) | module doc | REAL | 188-209. |
| "`translate_pipeline_signal` only ever yields table-legal inputs" (22-24) | module doc | REAL | 432-449. |

## 7. Dead / duplicated / unreachable code

None. (The `SelectSubtitle → SwitchTrack` mapping is documented as "harmlessly unreachable" — a deliberate vocabulary decision, disclosed at 833-835.)

## 8. Dependencies & flags

playback-engine + commands only. Pure model tests — no engine instance, no fakes, no I/O, no allocator. No feature gates.

## 9. Open questions for the next agent

- W03 colleague with state.rs: the `ALL` arrays' completeness is the linchpin of every sweep here (a missing state silently shrinks "exhaustive") — the vocabulary pins (776-790) catch size drift but not silent removal+shrink; model_check's artifact check is the backstop.
