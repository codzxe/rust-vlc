# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_04_seek.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-23 |
| Lines / functions | 153 / 5 (1 const + 3 helpers + 1 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-only scenario driving the production stack: real Engine + real AVI demux + real MJPEG/PCM decode over the pinned parent fixture (PARENT = "generated/valid/avi_mjpeg_pcm.avi", host.rs:137), through the host's local command-layer shape whose `dispatch_seek` (host.rs:1287-1339) mirrors the CLI's seek dispatch — wire target → engine `SeekTarget` (µs→ns), class-A receipt, then PUMPS TO THE SEEK TERMINAL (bounded, typed error if it never reports). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Direct run `./target/debug/deps/scenarios_b-bee4b59599013918 scenario_04` -> 1 passed / 0 failed (exit 0); baseline audit/runs/baseline/cargo-test-workspace.txt:4393 (ok); cat of both expectation TOMLs (proof of F-001); sed/rg verifications. |

## 1. One-paragraph verdict
A real seek scenario over the real pipeline: three seeks in the three wire forms (absolute 1 s, relative +500 ms, percent 0.5), each asserted to accept with a class-A receipt, produce exactly one NEW `timing.seek-completed` and zero `timing.seek-failed`, land within tolerance, and leave a post-completion sink ring that is forward-only from the landing (a stale pre-seek frame would show as a stamp far behind the landing); the media then drains to `Ended` honestly after all three. The host's dispatch genuinely pumps to the seek terminal before the asserts run, so the "exactly one new terminal" count is not racing the pipeline. One dishonest constant: `TOLERANCE_US = 100_000` is commented as "The parent's recorded seek tolerance (its expectation TOML)" — the parent fixture's expectation file records `tolerance_us = 33_333` (one 30 fps frame); 100 000 is the tolerance of a DIFFERENT fixture (the 10 fps `avi_mjpeg_2pcm.avi`), so the test enforces a bound 3× looser than the recorded contract while claiming provenance it does not have. Plus a dead `let _ = generation;` and a magic 40 ms "one quantum" that is not this fixture's quantum.

## 2. What actually works (verified)
— `./target/debug/deps/scenarios_b-bee4b59599013918 scenario_04` -> `test result: ok. 1 passed; 0 failed` (exit 0). Verified by read: (1) the terminal is pumped-to inside `dispatch_seek` (host.rs:1325-1336, bounded with a typed Internal error if absent) — the `completed_before + 1` count at 45-49 is therefore not a race; (2) the no-stale-frames check (69-75) filters the sink ring slice from `ring_before` (the pre-seek watermark) for stamps `< landed − 40 ms`, so a leaked pre-seek frame (e.g. 500 ms behind) fails; (3) `run_until_terminal(512)` asserts `Ended` after three seeks — a seek that broke end-of-media would fail; (4) `dispatch_seek`'s µs→ns mapping uses `saturating_mul` (host.rs:1294, 1297) — no overflow; (5) the relative row's landing math tolerates the small inter-row advance because dispatch stops at the first terminal (host.rs:1325-1329 stops pumping once the terminal is seen).

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/scenarios_b/scenario_04_seek.rs:24-25; proof at tests/fixtures/expectations/valid/avi_mjpeg_pcm.avi.toml:23 and avi_mjpeg_2pcm.avi.toml:23
Code:
```
/// The parent's recorded seek tolerance (its expectation TOML).
const TOLERANCE_US: i64 = 100_000;
```
What is wrong: the parent fixture is `generated/valid/avi_mjpeg_pcm.avi` (host.rs:137, PARENT), and its expectation TOML records `[seek] tolerance_us = 33_333` — one frame at the fixture's 30 fps. The value 100_000 is the recorded tolerance of a different fixture (`avi_mjpeg_2pcm.avi`, 10 fps video, tolerance_us = 100_000 in its own TOML). The comment claims provenance from "the expectation TOML" that the file does not contain; the assert is 3× looser than the recorded contract, so a seek landing up to 3 frames off at 30 fps passes a test claiming to enforce the fixture's recorded tolerance. Why it matters: this is the doc-10 row-4 acceptance scenario — the tolerance is the acceptance criterion, and it is silently loosened with a false citation. What correct looks like: `const TOLERANCE_US: i64 = 33_333;` (or read it from the expectation TOML so it cannot drift), and tighten the relative-row expectation accordingly. Confidence: HIGH — Verification: read (both TOMLs cat'ed; PARENT rg-verified) + ran (the test passes under the loose bound; whether it passes at 33 333 is UNVERIFIED — would need an edit, forbidden).
Confidence: HIGH — Verification: read / ran / UNVERIFIED (pass-at-recorded-tolerance)

F-002 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_04_seek.rs:37-44
Code:
```
    let CommandOutcome::Accepted { generation, .. } = accepted else {
        unreachable!()
    };
    let _ = generation;
    assert!(
        host.generation().is_some(),
        "{name}: the accept carries the post-bump generation"
    );
```
What is wrong: the receipt's generation is extracted and explicitly discarded (`let _ = generation;`), then replaced by a weaker assert that SOME generation exists. The message even claims "the accept carries the post-bump generation" — but the post-bump generation in the receipt is never compared against anything (e.g., the pre-seek generation + 1, or the host's current generation). A regression that stamped the OLD generation into the Accepted receipt would pass. What correct looks like: `assert_eq!(Some(generation), host.generation())` or compare against the pre-seek generation. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

F-003 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_04_seek.rs:66-74
Code:
```
    // No stale frames: every frame delivered AFTER the completion
    // carries a stamp at or beyond the landing point minus one
    // quantum (the frame already in flight at the boundary).
    let ring = host.sink().must().frame_ring();
    let post = &ring[ring_before.min(ring.len())..];
    let stale = post
        .iter()
        .filter(|(_, _, pts)| *pts < landed * 1_000 - 40_000_000)
        .count();
```
What is wrong: the 40 ms slack is a magic number commented as "one quantum", but the parent's frame period is 33.33 ms (30 fps per its expectation TOML) and no named constant in the host ties 40 ms to any quantum of this fixture (rg for a 40_000_000/quantum constant in host.rs finds none). If the parent ever changes fps, the comment rots silently. The threshold itself is sound for today's fixture (covers one in-flight frame, catches anything ≥ ~1 frame + 7 ms behind). What correct looks like: derive the slack from the fixture's fps (`1_000_000_000 / fps + margin`) or name the constant. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TOLERANCE_US (const) | 24-25 | REAL | S3 | false provenance + 3× looser than recorded (F-001) |
| seek_and_assert | 29-76 | REAL | S4 | receipt/terminal/tolerance/stale checks; dead generation discard (F-002), magic 40 ms (F-003) |
| seek_completed_position | 79-87 | REAL | — | last terminal's position, µs |
| name_target_us | 90-97 | REAL | — | the three rows' expected targets |
| seek_lands_three_targets_within_tolerance_without_stale_frames | 100-153 | REAL | — | opens parent, primes ≥8 audio frames, three rows, ends honestly; ran ok |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seek_lands_three_targets... | YES, but at a LOOSER bound than recorded | missing terminal, extra terminal, any failure event, landing >100 ms off, stale post-completion frame, or a broken end-of-media each fail a named assert; a landing 33-100 ms off passes (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The parent's recorded seek tolerance (its expectation TOML)" | scenario_04_seek.rs:24 | FALSE | parent's TOML records 33_333 (avi_mjpeg_pcm.avi.toml:23); 100_000 belongs to avi_mjpeg_2pcm.avi.toml:23 (F-001) |
| "exactly one `timing.seek-completed` whose position is within the fixture's recorded 100 ms tolerance" | scenario_04_seek.rs:8-9 | PARTLY FALSE | "exactly one" is real (assert 45-49); "the fixture's recorded ... tolerance" is not 100 ms (F-001) |
| "the frames that follow the completion belong to the post-seek timeline" | scenario_04_seek.rs:10-13 | REAL | ring-slice filter with landing-relative floor (69-75); my run ok |
| "The media still ends honestly after all three" | scenario_04_seek.rs:14 | REAL | run_until_terminal(512) asserts Ended (145-150) |

## 7. Dead / duplicated / unreachable code
- `let _ = generation;` (40) — dead assignment (F-002).
- No #[ignore], no todo!/unimplemented! (rg verified).

## 8. Dependencies & flags
commands catalog (wire SeekTarget/Timestamp vocabulary), testkit (Must, serialize_tests, fixture_path), the local host over the real engine/demux/codecs. `serialize_tests` held. No feature gates; no platform-conditional behavior in this file.

## 9. Open questions for the next agent
- Does the seek landing actually satisfy the RECORDED 33 333 µs tolerance (one frame)? The test passes at 100 000; a stricter run is the obvious follow-up (needs an edit — forbidden here). If it fails at 33 333, the loosened constant is masking a real landing-accuracy defect, which would upgrade F-001 to S2.
- scenario_06_repeated_seeks.rs and scenario_13/14 (not my files): do they use the same 100_000-with-false-provenance pattern, or the recorded value?
