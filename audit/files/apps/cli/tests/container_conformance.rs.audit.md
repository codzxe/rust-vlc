# AUDIT: apps/cli/tests/container_conformance.rs
| Field | Value |
|---|---|
| Auditor | W13-A09 |
| Date | 2025-07-14 |
| Lines / functions | 763 / 7 helpers + 19 tests |
| File verdict | SHALLOW at the recorded-evidence level (11/19 lanes real and green; 8/19 corpus-gated and vacuous here; 2 wired families have no lane at all) |
| Reachable from shipped entry point? | N/A (integration test) — drives the REAL shipped binary via `CARGO_BIN_EXE_lumen` (strongest harness form) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 1 |
| Commands run | `cargo test -p lumen-cli --test container_conformance -- --nocapture` -> 19 passed, 0 failed, 8 SKIP lines printed; corpus dir check: `/tmp/my-project/corpus-v4/media` ABSENT |

## 1. One-paragraph verdict
The committed-fixture half of this conformance matrix is excellent: real binary, piped session scripts, exact µs/percentage pins, state-transition pins, absence pins, and a byte-identical double-run determinism test — I ran them, they pass. The corpus-gated half (8 of 19 lanes, including the ONLY mpegts-family lane) silently self-skips when `/tmp/my-project/corpus-v4/media` is absent — and it is absent in this sandbox and almost certainly was during the recorded baseline (identical 2.6s runtime profile, no corpus anywhere in the tree), so the baseline's "19 passed" carries 8 vacuous greens. Worse, the header claims "one systematic lane per wired container family" while the wired mpegps family (containers/mod.rs:71) and the raw-ES h264/hevc family (mod.rs:63-64) have no lane at all.

## 2. What actually works (verified)
— verified by `cargo test -p lumen-cli --test container_conformance -- --nocapture` -> `test result: ok. 19 passed` with 8 `SKIP ... corpus dir absent` lines; the 11 committed-fixture lanes (avi, wav, aiff, mp4, mp4-h264, webm, ivf-av1, ivf-vp9, es-mp2, es-flac-refusal, determinism) executed the real binary and asserted exact outputs.

## 3. Findings
F-001 [S2]
Where: apps/cli/tests/container_conformance.rs:84-91 (+ all corpus-gated lanes: 227, 261, 291, 336, 366, 464, 569, 690)
Code:
```
fn corpus_file(name: &str) -> Option<String> {
    let dir = match std::env::var("LUMEN_CORPUS") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from("/tmp/my-project/corpus-v4/media"),
    };
```
What is wrong: 8 of 19 lanes (adts, ogg-vorbis, ogg-opus, mov, m4a, mkv, mpegts, es-mp3) resolve their media through this default, print `SKIP ... corpus dir absent` to stderr, and return Ok — counted as "ok" by cargo. The corpus dir does not exist in this sandbox (checked: DIR_ABSENT), and the recorded baseline (audit/runs/baseline/cargo-test-workspace.txt:3149-3172: `19 passed ... finished in 2.61s`) matches my corpus-absent run (`19 passed ... 2.67s`) lane-for-lane, so those 8 baseline greens are vacuous. Why it matters: the only mpegts-family CLI lane (hevc refusal shape, `--no-video` aac limb, `demux.ts.seek.unsupported`) never ran in the recorded evidence; the conformance matrix's TS coverage is UNPROVEN. What correct looks like: corpus lanes should fail loudly in CI (env var required) or the corpus should be committed/present in the lane that records the baseline.
Confidence: HIGH — Verification: ran (nocapture output above); baseline line cited.

F-002 [S2]
Where: apps/cli/tests/container_conformance.rs:1-6
Code:
```
//! matrix, through the REAL binary (session scripts over piped stdio,
//! the `session_script.rs`/`unsupported_path.rs` pattern): one
//! systematic lane per wired container family — avi, wav, aiff,
//! adts(aac), ogg(vorbis), ogg(opus), mp4/mov/m4a, mkv/webm,
//! mpegts, ivf(av1/vp9), raw es (mp3/mp2/flac).
```
What is wrong: The claim "one systematic lane per wired container family" is false: the wired `mpegps` family (`"mpegps" => ps::open(stream, opts)`, containers/mod.rs:71 — described there as "the FINAL §5 unit: every registered probe id now has an arm") has NO lane in this file (rg for `mpegps|\.mpg|\.vob` in the file: zero matches), and neither does the raw-ES annexb family (`h264`/`hevc` arms, mod.rs:63-64 — the header's own list stops at "raw es (mp3/mp2/flac)"). Why it matters: two of the 15 wired families ship with zero CLI-level conformance evidence, while the header asserts per-family completeness. What correct looks like: add ps and raw-ES h264/hevc lanes, or correct the header.
Confidence: HIGH — Verification: read + rg (zero matches).

F-003 [S3]
Where: apps/cli/tests/container_conformance.rs:89
Code:
```
        Err(_) => PathBuf::from("/tmp/my-project/corpus-v4/media"),
```
What is wrong: A machine/session-specific absolute path (`/tmp/my-project/...`) baked in as the default corpus location — portable only to the original author's session layout. It is the direct enabler of F-001's silent skips.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: apps/cli/tests/container_conformance.rs:532-540
Code:
```
    let mut script = String::new();
    for _ in 0..40 {
        script.push_str("advance 512\n");
    }
```
What is wrong: The webm lane drives playback by issuing 40 wall-clock-paced `advance` commands and expecting the 2.008 s media to play out — a load-sensitive timing dependency (on a starved 2-core machine the audio-master clock may not reach eom within the fixed pump count).
Confidence: MEDIUM — Verification: read (passed in my run and baseline).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run | 42-62 | REAL | — | real binary, piped stdin, BrokenPipe tolerated honestly |
| fixture | 66-70 | REAL | — | committed fixtures path |
| codec_fixture | 75-80 | REAL | — | codec-crate fixture home |
| corpus_file | 84-91 | REAL | S2/S3 | the skip gate (F-001/F-003) |
| assert_accepted | 97-103 | REAL | — | exit 0 + accepted receipt |
| assert_no_decodable_track | 111-124 | REAL | — | exit 65 + typed refusal + container named |
| ivf_row | 617-651 | REAL | — | shared av1/vp9 body |
| avi_family_lane | 136-161 | REAL | — | exact 2.000s/50.0% pins; RAN |
| wav_family_lane | 165-187 | REAL | — | RAN |
| aiff_family_lane | 191-208 | REAL | — | RAN |
| adts_aac_family_lane | 222-254 | UNVERIFIED (skipped) | — | corpus-gated; limb-less event pins |
| ogg_vorbis_family_lane | 258-286 | UNVERIFIED (skipped) | — | corpus-gated |
| ogg_opus_family_lane | 290-318 | UNVERIFIED (skipped) | — | corpus-gated |
| mp4_family_lane | 322-347 | REAL | — | RAN |
| mov_family_lane | 351-395 | UNVERIFIED (skipped) | — | corpus-gated; 6ch ring-alignment regression pin |
| m4a_family_lane | 399-428 | UNVERIFIED (skipped) | — | corpus-gated |
| mp4_h264_family_lane | 432-455 | REAL | — | RAN; software h264 + converter absence pins |
| mkv_family_lane | 459-514 | UNVERIFIED (skipped) | — | corpus-gated |
| webm_family_lane | 518-576 | REAL | S4(F-004) | RAN; wall-clock pump loop |
| mpegts_family_lane | 580-612 | UNVERIFIED (skipped) | — | corpus-gated; the ONLY ts-family lane |
| audio_less_start_time_is_deterministic_across_runs | 655-684 | REAL | — | RAN; byte-identical double run |
| ivf_av1_family_lane | 687-689 | REAL | — | RAN |
| ivf_vp9_family_lane | 693-695 | REAL | — | RAN |
| es_mp3_family_lane | 699-726 | UNVERIFIED (skipped) | — | corpus-gated |
| es_mp2_family_lane | 730-744 | REAL | — | RAN (committed codec fixture) |
| es_flac_family_lane_is_the_honest_refusal | 747-763 | REAL | — | RAN; pins the honest es.rs:258-271 typed refusal |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 11 committed-fixture lanes | YES | exact time strings, exit codes, state lines, absence pins — executed green |
| audio_less determinism | YES | byte-equality of two full runs |
| 8 corpus lanes (in this env) | NO — vacuous | self-skip before any assertion; counted "ok" (F-001) |
| es_flac refusal lane | YES for the refusal, but pins a STUB | asserts the arm's honest typed refusal (es.rs:258-271 discards stream/opts, always Err) — the test is honest, the arm is a refusal stub by design |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one systematic lane per wired container family" | header L4-6 | FALSE | mpegps + raw-ES h264/hevc have no lanes (F-002) |
| "Committed-fixture lanes always run" | header L20-21 | REAL | my run: 11 committed lanes executed |
| "skipped (not failed) when the corpus is absent" | header L18-20 | REAL | 8 SKIP lines in my nocapture run |
| FLAC "plays through the wired containers ... just not as raw ES" | L750-752 | REAL-UNPROVEN | mkv flac limb is corpus-gated (unverified here); es.rs stub verified |

## 7. Dead / duplicated / unreachable code
None — no dead helpers; `ivf_row` deduplicates the twin lanes properly.

## 8. Dependencies & flags
testkit::Must for spawns; `CARGO_BIN_EXE_lumen` (the built CLI binary); `LUMEN_CORPUS` env override. No feature gates.

## 9. Open questions for the next agent
- Does a corpus-enabled environment exist anywhere (tools/corpus)? If the corpus can be regenerated cheaply, re-run this suite to activate the 8 skipped lanes — that would also give the mpegts lane its first real evidence.
- Who owns the mpegps conformance lane gap (F-002) — the ps.rs wiring unit never shipped its lane?
