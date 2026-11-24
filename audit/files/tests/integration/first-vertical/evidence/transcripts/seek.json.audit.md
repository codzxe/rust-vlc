# AUDIT: tests/integration/first-vertical/evidence/transcripts/seek.json
| Field | Value |
|---|---|
| Auditor | W16-A10 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Kind | Committed evidence artifact (transcript/1 JSON) |
| File verdict | REAL-UNPROVEN (coherent recorded transcript; currency vs fresh run unproven) |
| Reachable from shipped entry point? | N/A — test evidence; read by tests/integration/first-vertical/tests/evidence.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 sha256/ledger cross-check; Read full file; rg consumers; baseline citation |

## 1. Verdict
A genuine recorded transcript of the `seek` scenario (stop→reopen-at-target shape, doc 10 §6.4):
4 commands (open/pause/seek/play), 19 events with a coherent Idle→Opening→Buffering→Playing→Paused
→seek(TimingSeekCompleted at 1_000_000us)→Playing→Ended state machine, final position 2_000_000us ==
duration 2_000_000us. Its ledger row is digest-verified (below). But the suite's currency test
failed in the orchestrator baseline, so byte-currency vs a fresh run is UNPROVEN for this file.

## 2. What actually works (verified)
- sha256=2f3d69b8…3dacdf, size 4883 — recorded in evidence/ledger.json; my python3 re-hash of the
  committed bytes finds both values present in the ledger (matches the passing baseline test
  `ledger_matches_the_artifacts`, cargo-test-workspace.txt:2765 "ok").
- Internal consistency: `duration_us: 2000000` (seq 5) == `position_us: 2000000` observables;
  staging_bytes 0 after Ended; final_png 320x240 == the AVI fixture's extent.
- Consumer: tests/evidence.rs:41-53 loops SCENARIO_IDS incl. `seek` and byte-compares normalized
  fresh vs committed.

## 3. Findings
F-001 [S3]
Where: tests/integration/first-vertical/tests/evidence.rs:37-68 (suite) + this artifact
Code: (baseline cargo-test-workspace.txt:2773-2774)
thread 'committed_transcripts_match_fresh_runs' (27977) panicked at tests/integration/first-vertical/tests/evidence.rs:47:9:
assertion `left == right` failed: the committed transcript for `open` drifted from a fresh run — regenerate with: cargo test -p first-vertical --test evidence -- --ignored regen
What is wrong: the currency gate FAILS at the first scenario (`open`), so this transcript was
never re-proven against a fresh run in the only recorded execution; the whole committed set is
stale-suspect until regenerated. Why it matters: doc 10 §7 evidence claims "byte-for-byte" currency.
What correct looks like: regenerate all transcripts, then the loop reaches and proves `seek`.
Confidence: HIGH — Verification: read baseline log (fail-fast at first id).
F-002 [S4]
Where: tests/integration/first-vertical/evidence/transcripts/seek.json:154-160
Code:
      "seq": 14,
      "session": 1,
      "gen": 1,
      "kind": "(unrecorded event kind)",
      "debug": "TimingSeekCompleted { position: Timestamp(1000000), seek_kind: Command }"
What is wrong: the one event that proves the seek landed is recorded under a catch-all
"(unrecorded event kind)" with a Rust Debug dump — the transcript schema has no typed slot for
TimingSeekCompleted. Evidence for the seek row therefore rides on a debug string, not a field.
What correct looks like: a named event kind in the harness event mapping.
Confidence: HIGH — Verification: read.

## 4. What was checked and how
Full JSON read; sha256/byte-count vs ledger (python3); state-machine walk of all 19 events;
gen/session/request_id monotonicity (note: seq 7 is absent — same gap exists in corrupt-noidx.json,
so it is a harness recording property, not file corruption); accounting arithmetic sanity
(154 accepted / 46 pushed / 29 presented / 87 repeats consistent with pause+resume at 50%).
No doc cites this file directly beyond the ledger/evidence suite.
