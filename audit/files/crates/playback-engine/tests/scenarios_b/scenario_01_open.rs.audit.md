# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_01_open.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 145 / 1 `#[test]` fn (three scenarios) |
| File verdict | REAL-UNPROVEN (behavioral, wire-level, honest about the build's refusal surface) |
| Reachable from shipped entry point? | N/A (test file); it drives the same command layer the CLI uses (`dispatch` over `Command::SessionOpen`) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | full-file read; cross-check of the MP4-refusal expectation against the matrix's MP4-plays claim (docs/feature-matrix.toml:152) and the W00 baseline (h264+aac MP4 → 0 frames decoded) |

## 1. One-paragraph verdict

A genuinely behavioral scenario test through the command layer: (1) the valid AVI parent — `Accepted` with session+generation, settles to `Playing`, the track list read from the wire (2 tracks, kinds, both selected), the duration exactly the fixture-declared 2.000 s, exactly one `timing.duration-known` event; (2) the MP4 parent — asserts a typed Probe-stage refusal naming the AVI vertical ("this build's first vertical serves AVI only, D-018"); (3) an absent path — typed Input-stage `Inaccessible` with the basename surviving (V8 redaction check). Every assertion checks real wire outcomes. Notably, this test *pins the MP4 refusal as expected behavior* — which is consistent with the W00 baseline (MP4 does not play) but sits in tension with the feature-matrix's newer "end-to-end VIDEO PLAYBACK proven on the h264+aac lane" claim (different fixture, different lane; the contradiction is flagged for the claim-ledger waves). The only defect is one either-or session assertion.

## 2. What actually works (verified)

- All three scenarios assert typed outcomes with stage, code, and message content — no success-with-garbage accepted.
- The duration/tracks assertions are read from the wire (`TracksList`, `MediaGetDuration`), not from the fixture's TOML directly — the test says so and does so (66-87).
- W00 baseline: this suite ran green — meaning the MP4 refusal it pins is the *actual current behavior* of the build (the mjpeg+pcm MP4 fixture refuses at probe). UNVERIFIED by my own run.

## 3. Findings

### F-001 [S4] — The session assertion is an either-or disjunction, weakening the accept contract

Where: crates/playback-engine/tests/scenarios_b/scenario_01_open.rs:53-56.

Code (53-56, verbatim):
```rust
    assert!(
        session.is_some() || host.session_id().is_some(),
        "the accept carries a session"
    );
```

What is wrong: the comment claims "the accept carries a session" but the assert passes if *either* the receipt's session field or the host's session id is present — a receipt missing its session field would pass as long as the host has one. The strict form is `assert!(session.is_some() && host.session_id().is_some())` (or assert the two agree). One-line weakening in an otherwise exact test.

Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open_answers_honest_facts_or_typed_refusals` | 29-144 | REAL-UNPROVEN | S4 | three scenarios; F-001; MP4 refusal pinned as expected (D-018) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `open_answers_honest_facts_or_typed_refusals` | YES | exact track kinds/selection, exact duration, event count, typed refusals with stage+code+message content |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the MP4 parent … refuses at the PROBE stage, typed — never a garbage success" | module docs 12-13 | REAL (current build) | test green in W00 run; consistent with W00 MP4 smoke (0 frames) |
| Matrix: "2026-11-15: end-to-end VIDEO PLAYBACK proven on the h264+aac lane — mp4_h264_aac.mp4 plays to Ended" | docs/feature-matrix.toml:152 | CONTRADICTION (different lane/fixture) | this test's fixture (`mp4_mjpeg_pcm.mp4`) refuses; whether the h264 lane plays is the MP4-trace waves' question; the two claims coexist only if the lanes differ — flag to wave 20 A05/A09 |

## 7. Dead / duplicated / unreachable code

None.

## 8. Dependencies & flags

`commands`, `testkit` (fixture_path, serialize_tests); the scenarios_b host (`crate::host` — another agent's file). Always runs.

## 9. Open questions for the next agent

- The scenarios_b `host.rs` (`PARENT`, `ScenarioHost`, `dispatch`, `count_events`) — the harness that makes these tests wire-level; another agent's file.
- If the h264 MP4 lane truly plays (matrix claim), this suite needs an MP4-plays sibling scenario; if it doesn't, the matrix row is false — wave 18/19 must adjudicate with the fixture names in hand.
