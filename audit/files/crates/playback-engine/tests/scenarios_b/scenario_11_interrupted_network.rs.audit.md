# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_11_interrupted_network.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 292 / 3 tests + 2 open helpers |
| File verdict | REAL-UNPROVEN (with a claim-ledger note: the scenario row's literal pass criteria are re-mapped, honestly) |
| Reachable from shipped entry point? | N/A — scenario test (doc 10 §6.4 row 11) over the command layer. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | `head -40` + inventory (exit 0). No cargo; run UNVERIFIED (W00: 1751/2/25). |

## 1. One-paragraph verdict

Three tests with an unusually candid mapping document: (1) real URL sources answer the production typed refusal `input.unsupported-for-host` (streaming is Phase 6) — asserted so the wire truth is pinned; (2) the **truncate** script (the source's visible length drops mid-stream) plays the delivered prefix and ends CLEANLY — `Playing → Ended`, exactly one `end-of-media`, exactly one non-fatal coalesced `playback.error`, bounded pumps; (3) the **reset** script (next read fails typed) surfaces the typed FATAL `Playing → Error` (stage `input`, code `inaccessible`), bounded. The doc 10 row's literal pass criteria ("`Buffering` → resume or typed error") are re-mapped in v0: a local-source cut is final, so there is no rebuffer arm here — the rebuffer/resume arm is proven in recovery_env.rs instead (my audit of it). The mapping is disclosed in the header ("the honest v0 mapping (recorded for the Lead)") — this is the right way to reinterpret a scenario row, but the claim ledger must still record that row 11's criteria are met by a different test file than the one named for the row.

## 2. What actually works (verified)

- `url_sources_refuse_the_phase_named` (133-171): the production refusal asserted — the shipped build's URL story is a typed refusal, pinned as wire truth (consistent with the streaming crate being absent from the shipped path).
- `truncated_midstream_source_ends_cleanly_within_deadline` (172-229): exact terminal state, exactly-one-event counts, bounded pump budget.
- `reset_midstream_source_fails_typed_within_deadline` (230-292): typed fatal with stage/code asserted, bounded.

## 3. Findings

### F-001 [S3] The scenario row's pass criteria are satisfied by substitution
Where: scenario_11_interrupted_network.rs:26-40 (the mapping)
Code (26-33, verbatim):
```rust
//! - **truncate** — the open lands, the session plays the delivered
//!   prefix, then ends CLEANLY: `Playing → Ended`, exactly one
//!   `playback.end-of-media`, exactly one NON-FATAL
//!   `playback.error` (the coalesced damage accounting). No hang:
//!   the terminal arrives within the pump bound. This is the
//!   "resume"-shaped arm in v0: there is no rebuffer-and-wait
//!   because a local-source cut is final — the honest equivalent of
//!   the network row's "resume or typed error" is "bounded play-out
//!   to a clean end".
```
What is wrong: nothing in the code — the substitution is disclosed and defensible — but a reader of doc 10 §6.4 row 11 ("interrupted network: Buffering → resume or typed error") would expect a rebuffer cycle in THIS scenario; it exists only in recovery_env.rs's engine-level test against local fakes. Neither file exercises a real network stack (there is none in the shipped build). Matrix/row adjudicators (waves 18/19) must record row 11 as: typed-refusal + bounded-end + engine-level rebuffer-over-fakes, not literal network recovery.
Confidence: HIGH — Verification: read (this file + recovery_env.rs cross-ref).

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `open_scripted_parked`/`open_scripted` | 72-131 | REAL | scripted access source over the parent fixture's bytes |
| `url_sources_refuse_the_phase_named` | 133-171 | REAL | production refusal pinned |
| `truncated_midstream_source_ends_cleanly_within_deadline` | 172-229 | REAL | exact counts, bounded |
| `reset_midstream_source_fails_typed_within_deadline` | 230-292 | REAL | typed fatal, bounded |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all three | YES (reasoned) | exact terminal states, exactly-one event counts, typed stage/code, pump bounds with context messages — a hang, a duplicate event, or an untyped failure fails |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Real URL sources answer the production typed refusal" | header 10-13 | REAL-UNPROVEN | asserted; run UNVERIFIED; consistent with streaming=Phase 6 |
| ED-FIX-3 containment semantics (truncate → clean end; reset → fatal) | header 30-45 | REAL-UNPROVEN | both asserted exactly |

## 7-9. Dead code / Dependencies / Open questions

None. commands/testkit/shared host. Open question: does doc 10's row-11 evidence string point at this file or recovery_env.rs? (waves 18/19).
