# AUDIT: crates/playback-engine/tests/model_check.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-23 |
| Lines / functions | 407 / 16 (11 parser/test-support fns + 5 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only file proving the PRODUCTION state machine: `playback_engine::state::{TRANSITIONS, apply_input, rejection_reason, SessionState, SessionInput, Effect}` (src/state.rs) — the table `Engine::poll`'s command drain executes (session.rs:1164-1172) in the shipped CLI host. The two other opinions are the committed artifact tests/model/expected_table.toml (104 `[[pair]]` entries, rg-verified) and the independently authored tests/model/oracle.rs (376 loc, no TRANSITIONS/apply_input reference — rg-verified). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | Direct execution of prebuilt binary `./target/debug/deps/model_check-a0de6cd6c0e4a5ac` -> 5 passed / 0 failed (exit 0; cargo invocation blocked by disk-full — environment, not repo); baseline audit/runs/baseline/cargo-test-workspace.txt:4265-4277 (same 5 ok); rg/sed verifications. |

## 1. One-paragraph verdict
This is the real cure for the self-referential echo chamber it claims to kill: for all 104 (state, input) pairs, the engine's production data table (`apply_input`), a wildcard-free imperative oracle written from the docs (tests/model/oracle.rs — rg confirms zero references to TRANSITIONS), and the committed human-review TOML artifact must agree exactly on target state, effects, AND typed rejection reason; the suite additionally pins completeness (104 unique pairs, 33 rows / 71 rejections / TRANSITIONS.len()==33), row-set equality, and the 21-edge wire projection against `PlaybackState::can_transition_to`. All 5 tests pass in my direct run of the prebuilt binary. The oracle's no-wildcard discipline makes any new state/input a compile error until classified — a genuinely strong design. Defects are parser hygiene only: the artifact reader silently DROPS a malformed `[[pair]]` block (missing `input`) instead of erroring, contradicting its own "as strict as the writer" claim (downstream count/dup tests would catch the fallout, so the blast radius is contained); the multi-line-effects path is never exercised by the actual artifact and would panic on a `#` comment line inside an array; and one stale "over all 96" comment contradicts the file's own 104-pair arithmetic.

## 2. What actually works (verified)
— `./target/debug/deps/model_check-a0de6cd6c0e4a5ac` -> `test result: ok. 5 passed; 0 failed` (exit 0), matching baseline line-for-line. Verified by read: the three-way comparison asserts per-pair equality of full `ModelAnswer` values (target + effects slice + reason, 299-308); the oracle has no wildcard and no table access (oracle.rs header + rg); the artifact carries exactly 104 `^\[\[pair\]\]$` entries (rg count) vs the 2 extra comment mentions of the string in its header (the naive count of 106 is comments, not entries); the edge test compares against the WIRE edge function `can_transition_to` (369), a third independent surface. Mutation reasoning: change any row's target/effect in TRANSITIONS, oracle, or artifact → test 2 names the pair and all three opinions; add a duplicate pair → test 1; break the wire projection → test 4.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/model_check.rs:88-97
Code:
```
        if line == "[[pair]]" {
            if let (Some(state), Some(input)) = (state, input) {
                out.push(finish(state, input, to, reject.take(), &effects));
            }
```
What is wrong: a `[[pair]]` block that has `state` but is missing `input` (or vice versa) is silently swallowed — the `if let` fails and the partial entry is discarded without any error — directly contradicting the reader's own contract at 54-59 ("an unknown line is a parse failure, never skipped ... the reader is as strict as the writer"). The downstream completeness test (104 unique pairs) would catch the resulting missing pair, so a broken artifact still fails the suite — but it fails as "artifact is missing X + Y", masking the actual defect (malformed block), and a hypothetical malformed block compensated by an extra legal block would swap one failure mode for another rather than pointing at the garbage. What correct looks like: `unreachable!` on a block with only one of state/input, mirroring `finish`'s own exactly-one-of to/reject check at 146-148. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

F-002 [S4]
Where: crates/playback-engine/tests/model_check.rs:70-84, and the grammar claim at 56
Code:
```
        if in_effects {
            // Inside an effects array: consume items until `]`.
            let body = line.strip_suffix(']').unwrap_or(line);
            for item in body.split(',') {
```
What is wrong: the multi-line-effects path is never exercised — the committed artifact uses only single-line arrays (rg `"effects = \[$"` over expected_table.toml: zero matches), despite the grammar comment advertising "possibly empty/multi-line". And the path has a hole: inside an array, comment lines are NOT skipped (the `#` check at 85 is only on the non-effects path), so a `# comment` line inside a multi-line array reaches `unquote` and panics `not a quoted string`. Unreachable with the current artifact; a latent trap for the next human who edits the review surface and helpfully comments inside a long array. What correct looks like: skip `#`-prefixed lines inside arrays too, or drop the multi-line support the artifact does not use. Confidence: HIGH — Verification: read + rg on the artifact.
Confidence: HIGH — Verification: read

F-003 [S4]
Where: crates/playback-engine/tests/model_check.rs:315-318
Code:
```
/// The artifact's row set has no duplicates and is exactly the
/// engine's row set (the same rows, not merely the same count —
/// belt-and-braces for the agreement test above, which compares per
/// pair over all 96 anyway).
```
What is wrong: "over all 96" — the pair universe is 8 × 13 = 104 (the file's own header line 2, and the count assert at 254). 96 = 8 × 12 is a stale number from before the 13th input existed. A reader auditing the proof coverage from this comment would mis-derive the arithmetic. What correct looks like: "over all 104". Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mod oracle (#[path] include) | 33-34 | REAL | — | pulls the independent second author in |
| PairExpectation (struct) | 47-52 | REAL | — | pair + expected classification |
| parse_artifact | 60-131 | REAL | S4 | silent drop of malformed blocks (F-001); unexercised multi-line path (F-002) |
| finish | 133-155 | REAL | — | exactly-one-of to/reject enforced (unreachable!) |
| leak_effects | 162-164 | REAL | — | Box::leak, bounded + documented (157-161) |
| unquote | 166-171 | REAL | — | strict, panics on unquoted |
| parse_state | 173-185 | REAL | — | all 8 states, unknown panics |
| parse_input | 187-204 | REAL | — | all 13 inputs, unknown panics |
| parse_effect | 206-223 | REAL | — | all 13 effects |
| parse_reason | 225-237 | REAL | — | all 8 reasons |
| artifact | 239-241 | REAL | — | include_str! — committed artifact, not generated |
| the_artifact_covers_every_pair_exactly_once | 251-274 | REAL | — | 104 unique + full coverage over ALL×ALL |
| table_oracle_and_artifact_agree_on_every_pair | 279-313 | REAL | — | THE three-way proof; rows=33/rejections=71/TRANSITIONS=33 pinned |
| the_artifact_row_set_is_the_engine_row_set | 319-347 | REAL | S4 | stale "96" comment (F-003); set equality after canonical sort |
| the_artifact_edges_are_exactly_the_wire_edge_set | 353-389 | REAL | — | 21 edges vs can_transition_to; the 3.C1 completion edges pinned |
| the_artifact_reasons_match_the_total_rejection_function | 394-407 | REAL | — | pins the total rejection function directly |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_artifact_covers_every_pair_exactly_once | YES | exact count + HashSet dup check + full ALL×ALL coverage |
| table_oracle_and_artifact_agree_on_every_pair | YES | full-value three-way equality per pair + pinned 33/71/33 |
| the_artifact_row_set_is_the_engine_row_set | YES | sorted set equality (would also catch a per-pair test that skipped a pair) |
| the_artifact_edges_are_exactly_the_wire_edge_set | YES | compares against the wire's own can_transition_to (a third surface) + 21-edge count + 3 named edges |
| the_artifact_reasons_match_the_total_rejection_function | YES | direct comparison against rejection_reason |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "three independently authored opinions ... must agree exactly" | model_check.rs:1-12 | REAL | oracle has no table access (rg); artifact committed (include_str); my run ok |
| "kills the self-referential echo chamber ... table.rs proof #1 checked the table against itself" | model_check.rs:14-17 | REAL (design) | this file never compares TRANSITIONS to itself; the oracle and artifact are separate sources |
| "8 × 13 = 104" | model_check.rs:2 | REAL | assert at 254; artifact has exactly 104 entries (rg) |
| "33 legal rows / 71 typed rejections" | model_check.rs:26-27 | REAL | asserts at 310-312 |
| "the reader is as strict as the writer" | model_check.rs:58-59 | PARTLY FALSE | malformed blocks are silently dropped (F-001), though unknown keys/values do panic |
| "possibly empty/multi-line" effects | model_check.rs:56 | MISLEADING | multi-line path unexercised by the artifact and comment-hostile (F-002) |

## 7. Dead / duplicated / unreachable code
- Multi-line effects path (70-84) — unexercised by the shipped artifact (F-002).
- No #[ignore], no todo!/unimplemented! (rg verified; the unreachable!s are the parser's strictness mechanism, deliberate).

## 8. Dependencies & flags
playback_engine (state module) + testkit only; std-only artifact reader (C5 zero-dependency contract honored); `#[path]` module include for the tests/model/ directory. No feature gates, no I/O (artifact embedded via include_str!).

## 9. Open questions for the next agent
- state.rs's `apply_input` is the only mover per the header — confirm session.rs really funnels ALL command application through it (no second ad-hoc transition site), since this proof's value depends on that single-mover claim (session.rs is another agent's file; spot check suggested at session.rs apply_command).
- The artifact's header says "3.C1's four rows + 3.A3's three work-carriers" grew the table — if a 14th input lands, the oracle's no-wildcard discipline forces classification here: verify future cards keep all three sources in lockstep (process note, not a defect).
