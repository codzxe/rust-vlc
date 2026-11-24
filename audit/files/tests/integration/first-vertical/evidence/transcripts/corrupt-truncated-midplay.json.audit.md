# AUDIT: tests/integration/first-vertical/evidence/transcripts/corrupt-truncated-midplay.json
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| Lines / functions | 174 / evidence artifact (15 events, 1 command, observables) |
| File verdict | SHALLOW — coherent corrupt-input story, currency gate not reached |
| Reachable from shipped entry point? | NO — test evidence; consumed by evidence.rs SCENARIO_IDS loop (:47-53) — this id is 9th of 12 (scenarios.rs:52-65), AFTER the `open` scenario where the loop panicked |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | python3 (in run-to-end audit run): committed final-frame.png sha256 698af4ee… == this transcript's :169 digest; scenarios.rs:40-65 SCENARIO_IDS order read; baseline cargo-test-workspace.txt:2764-2791 cited |

## 1. One-paragraph verdict
Corrupt-input transcript for the truncated AVI (avi_mjpeg_pcm__trunc3.avi): walks Idle->Opening->Buffering->Playing, plays to position 1,000,000 µs of a 2,000,000 µs duration, then end-of-media followed by a NON-fatal demux `playback.error` ("recovered 424669 damaged unit(s) while resynchronizing") and Ended — the recover-and-continue story the malformed lane exists to prove. Two defects: the `final_png` digest is the same stale artifact run-to-end carries (baseline-proven stale), and the event stream has an unexplained seq 7-8 gap. Its own freshness comparison never executed: `committed_transcripts_match_fresh_runs` panicked on the first scenario (`open`) and this id is ninth — event-stream currency UNVERIFIED.

## 2. What actually works (verified)
— verified by python3: this transcript's final_png.sha256 (:169) byte-matches the committed final-frame.png (698af4ee…), satisfying the evidence.rs:168-style naming chain for its run-to-end sibling; ledger consistency for all transcripts passes per baseline :2765 (`ledger_matches_the_artifacts ... ok`). — coherence: error stage demux/code corrupt/fatal false (:131-136) matches the truncated fixture story; final_state Ended == last transition (:142-144).

## 3. Findings
F-001 [S2]
Where: tests/integration/first-vertical/evidence/transcripts/corrupt-truncated-midplay.json:167-172
Code:
```
    "final_png": {
      "bytes": 307541,
      "sha256": "698af4ee249d8b31925ad958ba49ca4dcf1a530aa949c6aab30a8ed93cb553db",
      "width": 320,
      "height": 240
```
What is wrong: identical digest to run-to-end.json's final_png — and that digest names the committed final-frame.png which the baseline proves STALE (`final_frame_is_current` FAILED, cargo-test-workspace.txt:2781-2782 "the committed final-frame.png drifted from a fresh run"). For a midplay truncation that halts at 50% position (position_us 1,000,000 of 2,000,000), ending on the byte-identical final frame as the full 2s run additionally presupposes the whole 16-frame video limb finished before the truncation error — plausible (error comes AFTER end-of-media, :125-137) but unproven. Why it matters: the corrupt-lane evidence cites a frame artifact the current engine demonstrably does not produce. What correct looks like: regen the family (`-- --ignored regen`) and recommit; then the digest chain is current. Confidence: HIGH for staleness (baseline), MEDIUM for the same-frame story. Verification: ran (digest comparison) + baseline citation; fresh-run comparison UNVERIFIED (loop aborted at `open`, scenarios.rs order).
F-002 [S4]
Where: tests/integration/first-vertical/evidence/transcripts/corrupt-truncated-midplay.json:74-76
Code:
```
    },
    {
      "seq": 9,
```
What is wrong: seqs 7 and 8 are missing (events jump 6 -> 9) in a transcript schema that promises the exhaustive event record; no doc explains which kinds are unrecorded. (run-to-end.json shows a 7-only gap — the gap width varies per run, consistent with unrecorded/transient events, but nothing documents it.) Why it matters: silent holes in an "exhaustive" ledger. What correct looks like: record the filtered kinds in the schema header. Confidence: MEDIUM — Verification: read (gap is fact; cause unverified).

## 4. Per-function table
N/A (data artifact). Sections: labels (:5-13), commands (:14-26), events (:27-146), observables (:147-173).

## 5. Test honesty
Same machinery as run-to-end: the freshness gate is honest and red family-wide (baseline :2764/:2767); ledger-vs-artifact consistency green (:2765). This transcript's own fresh-run check never ran (order).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| corrupt/truncation scenarios prove recovery-not-crash | doc 10 §6.4 family | SHALLOW-as-evidence | this transcript shows non-fatal error + Ended, but currency gate unverified and final_png stale |

## 7. Dead / duplicated / unreachable code
N/A.

## 8. Dependencies & flags
Fixture generated/malformed/avi_mjpeg_pcm__trunc3.avi — W16-A02's manifest work: trunc2 is registered in fixtures MANIFEST.toml; trunc3 registration status UNVERIFIED here.

## 9. Open questions for the next agent
(1) Accounting tension: position_us 1,000,000 (half the media) with video_presented 16 (the full set run-to-end presents) and frames_accepted 61 vs scheduler_pushed 31 — which counters mean unique vs repeated? Resolve against harness.rs before trusting the accounting block. (2) Whether the trunc3 fixture's video limb is fully intact (making the identical final PNG honest) — verify against the expectation TOML.
