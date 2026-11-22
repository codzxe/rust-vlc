# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_02_play.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 98 / 1 `#[test]` |
| File verdict | **REAL** — an exact-count end-to-end play scenario over the real engine, real demux/codecs and a real fixture file |
| Reachable from shipped entry point? | The *subject chain* is production-real: ScenarioHost composes the real playback-engine + real input/demux(AVI)/codecs(MJPEG/PCM) over the fixture `generated/valid/avi_mjpeg_pcm.avi` (host.rs:137); the harness itself is a local command-layer shape (the ARCH-5 note in host.rs:5-27 honestly documents why the real `lumen_dispatch` is unreachable from this crate — dev-edge cycle — and names every fake). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read of the full file; read of scenarios_b/host.rs:1-60 (harness honesty note) + :137 (PARENT fixture); no cargo (static). |

## 1. One-paragraph verdict

This is what a scenario test should be: open the real fixture through the command catalog, assert `Playing`, pin the first frame's pts to 0 (the fixture's declared `[first_frame]` stamp), run to the honest terminal state with a bounded pump budget (512), assert exact delivered counts (120 frames total, 60 audio), exactly one `playback.end-of-media`, zero error events, A/V end-stamp distance ≤ one 30 fps frame duration, and a clean close. Every assertion is an exact value or a named bound; a broken engine (dropped frame, duplicated EoM, stamp drift beyond tolerance, spurious error) fails it. The harness caveat is not hidden: host.rs's ARCH-5 note states plainly that this suite pins the command-layer *contract* against the real engine while the real CLI dispatch remains covered only by the first-vertical crate.

## 2. What was checked

- Not a mock-of-the-subject: engine, pipeline, demux (AVI), codecs (MJPEG/PCM) are the production crates; only the consumer seam (ObservingSink) and access/demux fakes for *other* scenarios are local, and host.rs names them (host.rs:44-60).
- Exact-value asserts: `frames == 120`, `audio_accepted == 60`, `count_events("playback.end-of-media") == 1`, `first.2 == 0` (51, 62-68).
- The A/V check is the documented end-of-media checkpoint (doc 03 §4.10 tolerance) — last-audio vs last-video stamps, ≤ 33 333 400 ns (21, 90-94); it does not claim continuous sync (scenario 07 owns that).
- Terminal-state assertion with the pump count in the failure message (54-59) — debuggable failure, not a bare assert.
- The fixture is a generated real file (`generated/valid/avi_mjpeg_pcm.avi`), consistent with the W00 baseline's mjpeg+pcm decodes-fully result.
- `serialize_tests()` (25): the host's sinks use process-global state; the guard prevents cross-test interference.

## 3. Findings

None. (Checked: tautologies — none; ignored tests — none; `#[ignore]` absent; assert-only-ok smoke — none; fixture circularity — the fixture is generated test data, not produced by the code under test.)

## 4. Per-function table

| Item | Lines | Verdict | Notes |
|---|---|---|---|
| `play_flows_first_frame_samples_and_honest_stamps` | 24-98 | REAL | §1; exact counts + tolerance bound + clean close |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| play_flows_first_frame_samples_and_honest_stamps | YES | frame counts, EoM uniqueness, error emptiness, stamp tolerance, terminal state — five independent failure classes |

UNVERIFIED by run (no cargo); W00 baseline (1751 pass, failures only in first-vertical) is consistent.

## 6-9

- Doc claim (header): "first-frame event + samples flow; A/V stamps within tolerance" — the body checks exactly that; the "first-frame event" is checked via the sink ring's first pts (not the engine-private FirstFrame event) — a faithful proxy, noted.
- No dead code. Deps via dev-dependencies — consistent.
- Open question for wave 16: the suite's 16 scenarios share the host; scenario_02's quality is representative (spot-checked against the host's honesty note).
