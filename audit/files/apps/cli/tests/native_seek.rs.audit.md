# AUDIT: apps/cli/tests/native_seek.rs

| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-22 |
| Lines / functions | 606 / 9 helpers + 9 tests |
| File verdict | REAL-UNPROVEN (no corpus gating, real engine + real open path; every pin runtime-UNPROVEN — the binary never ran in the W00 baseline) |
| Reachable from shipped entry point? | N/A (test). Tests the REAL dispatch → host → engine → open path in-process (`lumen_dispatch` + `CliHost`, the same entry run.rs drives). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `sed -n` quote verification; `rg` verification that every host API used exists (host.rs:288-497); `ls` fixtures. NO cargo (adaptation). |

## 1. One-paragraph verdict

The best test file in my assignment: nine in-process integration tests driving the real `lumen_dispatch` → `CliHost` → engine → open path, with exact landing positions, generation arithmetic, state-history counter-proofs (the reopen-composition killer: `teardown_edges == 0` and `open_edges == 1`), boundary accounting pins, and honest negative tests (seek-from-Ended refusal, no-session refusal). No corpus gating — every test runs on committed fixtures. The single systemic problem is inherited, not self-inflicted: the W00 baseline `cargo test --workspace` never executed this binary (fail-fast stop at `first-vertical`), so every exact pin — including the five-landing storm vector and the µs-exact play-out closures — is runtime-UNVERIFIED against the current tree. Secondary nits: the opus seek lane's landing pin is `Timestamp(0)` (a single-data-page fixture where ANY seek lands at 0 — it cannot distinguish a correct seek from a broken always-rewind), and the accounting pins are coupled to pump-count implementation details.

## 2. What actually works (verified)

- Every host API the tests call exists: `settle` (host.rs:288), `session_id` (:308), `generation` (:314), `state_history` (:326), `position_us` (:346), `audio_lane_counters` (:387), `headless_accounting` (:402), `advance` (:497), `RUN_PUMPS = 512` (:146) — verified by `rg`.
- Fixtures exist: `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi`, `tests/fixtures/generated/valid/ogg_opus_48k_mono.opus`, `crates/codecs/tests/fixtures/vorbis_sine_44100_stereo.ogg` (verified `ls` in earlier audits of this session).
- Test structure is genuinely adversarial: the reopen counter-proof (one `Idle → Opening`, zero teardown edges across five seeks), the actual-vs-requested position pin (250 000 requested → 266 667 landed, clock at 266 666 with the ns→µs truncation named), and the `--start-time` watermark-death regression (backward seek below the start time must still feed pre-roll — 266-297).

## 3. Findings

### F-001 [S2] — None of these tests has a recorded run

Where: whole file; evidence: audit/runs/baseline/cargo-test.log (116 test binaries, `native_seek` absent)

Code (native_seek.rs:143-148, verbatim):
```rust
#[test]
fn native_seek_keeps_the_epoch_and_closes_the_accounting() {
    let mut host = opened();
    let session_before = host.session_id();
    let generation_before = host.generation();
    seek(&mut host, 2, SeekTarget::Percent { fraction: 0.5 });
```

What is wrong: not a code defect — an evidentiary one. The W00 baseline stopped at `first-vertical`'s failing evidence.rs (cargo-test.log:2499: `error: test failed, to rerun pass \`-p first-vertical --test evidence\``, EXIT=101) without `--no-fail-fast`, so this entire suite — the repo's ONLY in-session seek acceptance evidence, pinned by name in the module doc as "THE C8 acceptance" — has never produced a recorded green run on this tree. Its pins are exact to the microsecond (e.g. `Timestamp(1_766_667)` at native_seek.rs:431, `position_us() == 400_000` at :526); if any drifted, nobody has seen the failure.
Why it matters: §3.4 — a test suite that would detect real regressions but has never been run is evidence-shaped, not evidence; matrix rows citing "native seek" point here.
What correct looks like: baseline re-run with `--no-fail-fast`; this suite belongs in the merge gate.
Confidence: HIGH — Verification: read + baseline binary inventory.

### F-002 [S4] — The opus seek lane cannot detect a broken seek (landing always 0)

Where: apps/cli/tests/native_seek.rs:501-535

Code (native_seek.rs:508-512, verbatim):
```rust
    assert_eq!(
        seek_completed_position(&host),
        Some(Timestamp(0)),
        "the page-granular landing (the single data page ends past the half)"
    );
```

What is wrong: the fixture has ONE data page whose granule (19 512) exceeds every sub-half target, so the honest SlowLinear landing for the 50% seek is the stream head — 0. The pin therefore passes for a correct seek AND for any seek bug that always rewinds to 0. The lane's remaining value (decode continues, play-out closes at 400 000 µs, zero refusals) is real, but the seek-precision coverage for Ogg/Opus is effectively nil; the vorbis twin has the same shape (landing pinned at 0, native_seek.rs:561-565, with the same single-data-page cause).
What correct looks like: a multi-page opus/vorbis fixture (the corpus lanes have them — 201 packets) so a mid-media landing is nonzero.
Confidence: HIGH — Verification: read (page-granularity claim is the test's own comment).
Severity: S4 — the lane still proves header replay + play-out, the actual regression it was written for (6.FIX-ogg-reanchor).

### F-003 [S4] — Accounting pins coupled to pump/settle implementation constants

Where: apps/cli/tests/native_seek.rs:185-199, 455

Code (native_seek.rs:185-186, verbatim):
```rust
    assert_eq!(accounting.scheduler_dropped_stale, 15, "{accounting:?}");
    assert_eq!(accounting.queue_published, 1, "{accounting:?}");
```

What is wrong: pins like `scheduler_dropped_stale == 15` (first test) vs `== 16` (storm test, :455) and `frames_accepted == 90` encode the settle pump's decode-ahead window size; any change to `RUN_PUMPS`, the ring park threshold, or the scheduler window cap breaks these tests without indicating a user-visible regression. Honest brittleness — the comments derive each number — but it is maintenance debt that will teach maintainers to bulk-update magic numbers rather than reason about regressions.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parent` | 36-44 | trivial-correct | — | Committed fixture path. |
| `dispatch` | 48-60 | trivial-correct | — | Asserts success; failure tests bypass it deliberately. |
| `opened` | 63-75 | trivial-correct | — | Open + settle. |
| `seek` | 78-80 | trivial-correct | — | |
| `seek_completed_position(s)` | 84-103 | trivial-correct | — | Event-stream filters. |
| `teardown_edges` / `open_edges` | 110-132 | REAL-UNPROVEN | — | The reopen counter-proof machinery — good design. |
| `native_seek_keeps_the_epoch_and_closes_the_accounting` | 143-200 | REAL-UNPROVEN | S2 (F-001) | THE C8 acceptance: epoch/generation/landing/counters/play-out all pinned. |
| `native_seek_reports_the_actual_not_the_requested` | 206-232 | REAL-UNPROVEN | S2 (F-001) | 250 000 → 266 667 landing + truncation-named clock pin. |
| `paused_seek_completes_and_resumes_from_the_landing` | 237-258 | REAL-UNPROVEN | S2 (F-001) | Paused carve-out + resume. |
| `backward_seek_below_start_time_lands_and_plays_out` | 265-297 | REAL-UNPROVEN | S2 (F-001) | The watermark-death regression pin — directly tests `SelectDemuxer::reanchor`'s `skip_until_ns = None` (demux_host.rs:606-618). |
| `seek_from_ended_refuses_typed_at_the_old_position` | 303-337 | REAL-UNPROVEN | S2 (F-001) | Typed refusal + nothing-moved/no-fabricated-terminal. |
| `seek_without_a_session_refuses_not_ready` | 340-359 | REAL-UNPROVEN | S2 (F-001) | |
| `rapid_repeated_seeks_compose_in_pipeline` | 384-457 | REAL-UNPROVEN | S2 (F-001) | Five-landing vector pinned exactly; superseded-seek case honestly deferred to the engine's suite (comment 370-380). |
| `ogg_opened` | 473-491 | trivial-correct | — | |
| `ogg_opus_in_session_seek_replays_the_codec_headers_and_plays_out` | 501-535 | REAL-UNPROVEN | S4 (F-002) | Header-replay regression + play-out; landing pin weak. |
| `ogg_vorbis_in_session_seek_replays_the_codec_headers_and_lands` | 542-606 | REAL-UNPROVEN | S4 (F-002) | Same; `stale_dropped < 3` loose bound with honest comment; end-position derivation documented against the committed oracle (`vorbis_reference.txt`). |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 9 | YES (modulo F-002's landing-pin weakness on the two ogg lanes) | Exact positions/generations/counters/state edges; negative paths asserted. All runtime-UNPROVEN (F-001 — the binary never ran in the baseline). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the host performs the boundary duties behind its own seams (ring-flush prefix, window-full hold clear, presentation-queue publish)" | :4-6 | REAL-UNPROVEN | `headless_accounting`/`audio_lane_counters` counters exist (host.rs:387-402); the duties themselves live in host.rs (another agent's file). |
| "exactly one terminal per completed seek (doc 05 §4.3 invariant 4)" | :91-94 | REAL-UNPROVEN | Storm test pins five terminals (424-434). |
| Ogg seek = SlowLinear page-granular | :493-500 | REAL-UNPROVEN | Landing 0 pins; consistent with ogg.rs module doc (SlowLinear) and demuxer. |

## 7. Dead / duplicated / unreachable code

- None. No corpus gating in this file (the only test file in my assignment with none).

## 8. Dependencies & flags

- Uses `lumen_cli` as a library (host + dispatch) — integration test of the crate's own API; no feature gates; fixtures committed.

## 9. Open questions for the next agent

- The settle pump's decode-ahead constants (15/16 stale, 90 accepted) are pinned here but defined in host.rs/engine (outside my files) — if Wave 13's other agents find pump-count changes, these pins are the canary.
- Needs a run: `cargo test -p lumen-cli --test native_seek` (blocked — no cargo). Wave-20 A05 should include it.
