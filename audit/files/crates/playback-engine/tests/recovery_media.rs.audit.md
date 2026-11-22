# AUDIT: crates/playback-engine/tests/recovery_media.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 689 / 7 tests + local fakes/builders |
| File verdict | REAL — named per-defect regressions (G1 NoDecodableTrack enrichment, G3 audio per-frame damage) with exact frame/pts/damage-count accounting and the budget's exact 1000/1001 arithmetic |
| Reachable from shipped entry point? | N/A (test binary). Subjects: error enrichment (error.rs) and the decode damage lane (pipeline.rs) — both shipped |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (quotes verified with unfiltered tools) |

## 1. One-paragraph verdict

Seven regression tests that pin behavior, not vibes. The G1 trio proves
the `session.no-decodable-track` enrichment: container + codec list
(:414-466), container-only on an empty track list (:467-501), and the
honest un-enriched shape when no container was declared (:502-537).
The G3 quartet proves the audio skip lane: two damaged packets skipped
and counted with the delivered 38 frames each carrying the NEXT GOOD
packet's pts — gaps, never synthesized silence (:554-592, the pts-gap
arithmetic at :581-591 is checked frame by frame); interleaved damage
resets the run (3+2 rides out, total 5, 35 delivered, :599-620); a
stream of nothing but per-frame damage hits the typed terminal with
`attempted: 1001` and exactly `VIDEO_CONCEALMENT_BUDGET` (1000)
recovered units (:629-659) — the off-by-one shape of the pipeline's
`conceal_failures > BUDGET` check is pinned; and the control test
proves a non-per-frame failure (`Unsupported`) stays fatal with zero
recovery accounting (:666-689). No tautologies anywhere.

## 2. What actually works (verified)

— `audio_per_frame_damage_is_skipped_and_reported`: the pts assertion
(:581-591) walks every delivered frame and derives its expected pts
from the two skipped indices — a fabricated-silence or wrong-slot
regression fails exactly.
— `audio_damage_budget_exhaustion_is_a_typed_terminal`: pins
`ResourceLimit { RecoveryAttempts, budget: 1000, attempted: 1001 }`,
stage Decode, code `decode.concealment.budget`, and the source-chain
root — matching pipeline.rs:2381-2389 and :683-694 (verified in the
pipeline audit).
— The G1 tests assert the exact error payloads the error.rs audit
traced (from_tracks → FormatInfo + source leaf).

## 3. Findings

### F-001 [S4] Stale "planned hoist" comment; the local fakes are a deliberate third copy
Where: recovery_media.rs:21-23

Code:
```
//! that suite's; 3.C1's planned hoist to `tests/common` will unify
//! them — this file keeps its own so the two waves cannot collide).
```
What is wrong: the hoist has happened (`tests/common/mod.rs` exists
and says "hoisted verbatim from tests/session_flow.rs"); this file
still carries its own fault-armed copies with a comment describing the
hoist as future work. The duplication is defensible (these fakes are
fault-armed; common's are not), but the comment is now false and the
real reason (fault arming) is not stated.
What correct looks like: "local fault-armed variants of the common
fakes (the common hoist landed; these keep per-packet failure
scripting)".
Confidence: HIGH — Verification: read + common/mod.rs existence.

## 4. Per-function table

| Test/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fakes/builders (SharedNow, faulty decoder/provider, `open_faulty_audio`) | 53-360 | REAL | — | per-index + fail-all fault arming |
| `poll_until`/`take_events`/`first_fatal`/`flushed_decode_damage` | 362-412 | REAL | — | helpers with loud failures |
| `no_decodable_track_names_container_and_codecs` | 413-466 | REAL | — | G1 full enrichment |
| `no_decodable_track_with_empty_track_list_names_container_only` | 467-501 | REAL | — | honest degradation |
| `no_decodable_track_without_container_stays_unenriched` | 502-537 | REAL | — | |
| `audio_per_frame_damage_is_skipped_and_reported` | 554-592 | REAL | — | pts-gap honesty per frame |
| `audio_damage_runs_reset_on_success` | 599-620 | REAL | — | run reset, total 5 |
| `audio_damage_budget_exhaustion_is_a_typed_terminal` | 628-659 | REAL | — | 1000/1001 arithmetic pinned |
| `audio_non_per_frame_damage_stays_fatal` | 666-689 | REAL | — | the control: no general swallowing |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 7 | YES | exact counts (38/35), per-frame pts derivation, typed error payloads, 1000/1001 budget figures |

Not executed by me (no cargo); W00 ran the suite green (this suite's
`session.no-decodable-track` assertions also appear in W00's evidence).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one named test per defect in the worklog's W2-19 §C2.0 gap list" | :1-2 | REAL | G1/G3 mapped to tests (UNVERIFIED against the worklog section itself — worklog.md is another agent's file) |
| "NEVER synthesize silence" | :17-18 | REAL | 38-delivered + pts-gap assertions |
| "the budget shared with the video concealment lane" | :16-17 | MISLEADING | the budget is per-limb in code (pipeline audit F-005); this test exercises one lane only, so it cannot detect the difference |

## 7. Dead / duplicated / unreachable code

Local fakes duplicated from the common harness (documented rationale
now stale — F-001).

## 8. Dependencies & flags

`playback_engine`, `codec_api`, `media_clock`, `media_types`,
`packet_queue`, `plugin_api`, `testkit`. No feature gates.

## 9. Open questions for the next agent

- Wave 3 A02 (pipeline damage lane owner): no test here (or in
  pipeline.rs's unit tests) exercises BOTH lanes failing
  concurrently — the "one shared budget" doc claim is untestable in
  the current suites because it is false in code (per-limb counters).
