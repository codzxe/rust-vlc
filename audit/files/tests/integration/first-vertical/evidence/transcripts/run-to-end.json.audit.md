# AUDIT: tests/integration/first-vertical/evidence/transcripts/run-to-end.json
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A10) |
| Date | 2026-06-14 |
| Lines / functions | 195 / evidence artifact (18 events, 1 command, observables) |
| File verdict | SHALLOW — internally coherent committed evidence whose freshness gate is red family-wide |
| Reachable from shipped entry point? | NO — test evidence; consumed by tests/integration/first-vertical/tests/evidence.rs (SCENARIO_IDS loop :47-53, final_png chain :154-172) and cited by docs/plans/open-path-wiring.md, docs/worklog/archive-2026-10.md |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | python3 digest: run-to-end.json 3,819 B sha256 7afd348312e0b86165934f437fb62a1d76d2e90f1ef442ba0678725d880d22b4; final-frame.png 307,541 B sha256 698af4ee249d8b31925ad958ba49ca4dcf1a530aa949c6aab30a8ed93cb553db (== transcript :190 digest — python); baseline cargo-test-workspace.txt:2764-2791 cited |

## 1. One-paragraph verdict
Committed run-to-end transcript for the first vertical (AVI/MJPEG/PCM). The event stream is coherent: Idle->Opening->Buffering->Playing->Ended with 2,000,000 µs duration, six time-ticks up to 0.933, end-of-media then Ended; the accounting block is dimensionally consistent (16 staging frames × 307,200 B = 4,915,200 B staging_bytes; 320×240 RGBA; final PNG 320×240). But this is evidence whose currency is provably broken in the baseline: the digest it records names the committed final-frame.png, and `final_frame_is_current` FAILED (baseline :2781-2782: "the committed final-frame.png drifted from a fresh run") — so the `final_png` block documents bytes the current engine no longer produces. The transcript-vs-fresh-run comparison for THIS scenario never executed (the loop panicked earlier on `open`, baseline :2773-2774), so event-stream currency is UNVERIFIED, and the family-wide drift signal (sibling `open` transcript drifted; W16-A02 log) makes staleness likely.

## 2. What actually works (verified)
— verified by python3 digest run: transcript's final_png.sha256 (:190) == actual committed final-frame.png sha256 (byte-exact match to the named artifact; satisfies evidence.rs:168's chain). — verified by baseline cargo-test-workspace.txt:2765 `ledger_matches_the_artifacts ... ok`: the ledger's digest/byte-count for this transcript are current (3,819 B / 7afd3483…). — coherence checks: final_state "Ended" == last event (Playing->Ended :163-165); position_us 2,000,000 == duration_us 2,000,000 (:63); scheduler_pushed == scheduler_presented == 60.

## 3. Findings
F-001 [S2]
Where: tests/integration/first-vertical/evidence/transcripts/run-to-end.json:188-193
Code:
```
    "final_png": {
      "bytes": 307541,
      "sha256": "698af4ee249d8b31925ad958ba49ca4dcf1a530aa949c6aab30a8ed93cb553db",
      "width": 320,
      "height": 240
```
What is wrong: this digest names the committed final-frame.png, which the baseline proves is STALE — `final_frame_is_current` FAILED at evidence.rs:162 ("the committed final-frame.png drifted from a fresh run", baseline :2781-2784, byte arrays visibly diverge). Why it matters: the transcript's final-frame claim is the cited proof artifact for the first-vertical journey; a reader trusting it believes the current engine produces this frame when the proving test is red. What correct looks like: run the documented regen (`cargo test -p first-vertical --test evidence -- --ignored regen`) and recommit, turning the gate green. Confidence: HIGH — Verification: ran (digest match) + baseline citation. Event-stream portion: UNVERIFIED (comparison loop aborted at `open`, baseline :2773-2774).
F-002 [S4]
Where: tests/integration/first-vertical/evidence/transcripts/run-to-end.json:74-88
Code:
```
    {
      "seq": 6,
      "gen": 0,
      "kind": "media.info-ready",
```
…then `seq: 8` follows at :76 — seq 7 is absent from the 18-event stream (python: seqs [1..6, 8..19]).
What is wrong: an unexplained sequence gap in a transcript whose purpose is to be the exhaustive event record; nothing documents whether the missing seq 7 was an unrecorded event kind or a capture defect. Why it matters: an exhaustive ledger with silent holes invites "the transcript is complete" misreadings. What correct looks like: capture all events or document the filtered kinds in the schema header. Confidence: MEDIUM — Verification: read + python seq listing (the gap is a fact; its cause is unverified).

## 4. Per-function table
N/A (data artifact). Sections: schema/scenario/fixture labels (1-13), commands (14-26), events (27-167), observables (168-194).

## 5. Test honesty
The guarding tests are themselves honest and RED where it matters: committed_transcripts_match_fresh_runs (FAILED, baseline :2764), ledger_matches_the_artifacts (ok, :2765 — ledger↔artifact consistency only), final_frame_is_current (FAILED, :2767). The evidence does not lie about its own machinery; it is simply out of date.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| (any doc citing run-to-end as proof of the journey) | docs/plans/open-path-wiring.md, docs/worklog/archive-2026-10.md | SHALLOW-as-evidence | gate red per baseline :2767/:2781 |

## 7. Dead / duplicated / unreachable code
N/A.

## 8. Dependencies & flags
Artifact of crate tests/integration/first-vertical (evidence.rs machinery); no feature gates.

## 9. Open questions for the next agent
Did the fresh-vs-committed comparison for `run-to-end` (as opposed to `open`) ever run green before the loop aborts? Re-run with a filter or reorder to isolate; mark the event-stream portion REAL or STALE accordingly.
