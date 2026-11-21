# AUDIT: apps/cli/src/containers/wav.rs
| Field | Value |
|---|---|
| Auditor | W13-A06 |
| Date | 2026-11-21 |
| Lines / functions | 590 / 14 (2 production fns + 12 test items) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI open → demux_host::open_source → probe "wav" → containers/mod.rs:73 `"wav" => wav::open(stream, opts)`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "wav\|open\|pub" apps/cli/src/containers/mod.rs` (dispatch wiring :73); `rg corpus\|wav_pcm\|adpcm_extra… audit/runs/baseline/test-workspace.log.txt` (no lumen_cli target in run #3 — it stopped at the desktop failure); `sed -n` on orchrun-test-workspace-nofailfast.log.txt:2769-2910 (all 6 wav container tests ran and passed in the no-fail-fast rerun); `ls /tmp/my-project/corpus-v4/media` (absent in sandbox — corpus gate evaluation); `rg duration_us apps/cli/src/demux_host.rs` (:239) + `rg sentinel crates/demux/src/wav/mod.rs` (extent resolution for the clamp-edge finding); `sed -n` quote verification |

## 1. One-paragraph verdict
The WAV container host is a real, thin, correctly-shaped wiring unit: construct the demuxer, run the pre-pass, harvest per-family decoder-config truth (PCM pair / ADPCM WAVEFORMATEX blob / FLAC STREAMINFO), clamp `--start-time` through the demuxer's own seek query, wrap in the selection demuxer. Its six tests are honest where they run — the ADPCM test builds a real IMA WAV and proves 505 decoded frames through the REAL provider, the clamp test proves a past-end target admits zero packets — and all six passed in the orchestrator's no-fail-fast rerun. The one structural weakness: the 12-lane ffmpeg corpus differential is gated on `/tmp/my-project/corpus-v4/media` (or `LUMEN_CORPUS`), which does not exist in this sandbox, so the heaviest evidence row "passed" without provably executing — cargo captures the skip's eprintln, so the baseline log cannot distinguish ran from skipped. Three smaller debts: dead truth data on the FLAC row, a per-track loop that assumes the demuxer's single-track invariant without asserting it, and a duration fallback that silently disables clamping when the extent is unknown.

## 2. What actually works (verified)
- All 6 container tests passed in the orchestrator rerun: `containers::wav::tests::{pcm_fixture_opens_with_expected_facts, all_wav_fixture_flavors_open, adpcm_extra_data_flows_into_the_decoder, broken_fmt_chunk_refuses_at_the_demux_stage, corpus_wav_lanes_decode_through_the_open_path, start_time_clamps_and_skips}` — "129 passed; 0 failed" for the lumen_cli lib target (orchrun-test-workspace-nofailfast.log.txt:2769-2903, relative lines 70-75, 134). — verified by reading the cited baseline log.
- ADPCM blob seeding works end-to-end: hand-built IMA WAV → 20-byte `fmt ` body in `extra_data` → real `HostDecoderProvider` decode → 505 frames (test asserts the count; wav.rs:312-320). — verified by reading the test + its passing result.
- `--start-time` clamp + skip: past-end target clamps to the declared span and admits 0 packets; in-media target's survivors all land ≥ target (wav.rs:328-374). — verified by reading + passing result.
- Dispatch reachability: containers/mod.rs:73. — verified by rg.

## 3. Findings

### F-001 [S3]
Where: apps/cli/src/containers/wav.rs:509-528
Code:
```rust
        let default = PathBuf::from("/tmp/my-project/corpus-v4/media"); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
        default.is_dir().then_some(default)
    }

    #[test]
    fn corpus_wav_lanes_decode_through_the_open_path() {
        let Some(dir) = corpus_dir() else {
```
What is wrong: The 12-lane ffmpeg differential — the only test that compares decoded frame counts against external truth for this container — silently degrades to a no-op when the corpus directory is absent. `/tmp/my-project/corpus-v4/media` does not exist in this sandbox (verified: `ls` fails), and cargo captures the skip's `eprintln`, so the "ok" result in every baseline run cannot be distinguished from an executed differential. The primary baseline run (#3, test-workspace.log.txt) does not even contain the lumen_cli target (it aborted at the desktop zero_alloc failure); only the orchestrator's no-fail-fast rerun shows the test "ok".
Why it matters: The test's name promises a corpus differential; in any environment without the corpus it proves nothing while reporting success. The reference table (2026-10-24 instance) lives only in the worklog, not in the repo's test evidence. The always-on fixture tests partially compensate (six flavors + one ADPCM decode), but 11 of 12 truth rows (PCM s24/s32/u8 chaos rates, ADPCM ms, multichannel) are unexercised in committed evidence.
What correct looks like: Emit the skip through a mechanism CI can see (e.g., `#[ignore = "needs LUMEN_CORPUS"]` so it shows in the ignored count, like the codec-crate bench tests at test-workspace.log.txt:818), or commit a minimal subset of the corpus lanes as fixtures.
Confidence: HIGH — Verification: read + `ls /tmp/my-project/corpus-v4/media` (absent) + baseline log inspection (no distinguishable output).

### F-002 [S4]
Where: apps/cli/src/containers/wav.rs:443-447
Code:
```rust
        CorpusTruth {
            file: "0016-audio-only.wav",
            codec: "flac",
            rate: 96_000,
            channels: 4,
            truth_frames: 384_000,
        },
```
What is wrong: `truth_frames: 384_000` is dead data: the flac arm (561-575) asserts the pinned `flac.frame.*` refusal and `continue`s before any frame comparison. The field reads as an ffmpeg-measured expectation that is never checked.
Why it matters: Suggests a verified count that does not exist; if the FLAC frame-alignment gap is ever fixed, the stale number either gets compared (fine) or keeps lying (worse). Minor.
What correct looks like: `truth_frames: 0` with a comment, or an `Option<u64>` that the flac arm sets to `None`.
Confidence: HIGH — Verification: read (561-575 shows the continue).
### F-003 [S4]
Where: apps/cli/src/containers/wav.rs:94-131
Code:
```rust
fn harvest_facts(demuxer: &WavDemuxer, tracks: &[Track]) -> MediaFacts {
    let mut facts = Vec::with_capacity(tracks.len());
    for track in tracks {
        let (codec, pcm, extra_data) = match demuxer.format() {
```
What is wrong: The loop iterates all tracks but reads the demuxer's single `format()` for each — every track would receive identical codec/pcm/extra_data facts. WAV's one-audio-track invariant makes this benign today, but nothing here asserts `tracks.len() == 1`; a future multi-track demuxer change clones wrong facts silently.
Why it matters: Latent wiring hazard, zero current blast radius.
What correct looks like: `assert`/debug-assert the single-track invariant at open, or key facts by the demuxer's actual per-track data.
Confidence: HIGH — Verification: read.
### F-004 [S4]
Where: apps/cli/src/containers/wav.rs:53-55
Code:
```rust
    let duration_us = facts.duration_us().unwrap_or(i64::MAX / 4);
    // 2. Clamp the start position (doc 05 §2.4: clamping is truth).
    let start_us = opts.start_time_us.unwrap_or(0).clamp(0, duration_us);
```
What is wrong: When the declared duration is unknown (`duration_us()` → None), the clamp bound becomes i64::MAX/4 — clamping is silently disabled, and a user `--start-time` past the real extent then hits the seek query's typed refusal instead of the documented clamp-to-end behavior. For file sources the demuxer resolves even sentinel sizes to the stream length (demux/src/wav/mod.rs:571-579), so this arm is reachable only for sources whose length is genuinely unknown (e.g., unseekable streaming input).
Why it matters: "Clamping is truth" (the cited doc 05 §2.4) is violated on the unknown-extent edge: error instead of clamp.
What correct looks like: When duration is unknown, validate the target through the seek query and clamp to whatever it accepts, or refuse `--start-time` on unknown-extent sources with a dedicated code.
Confidence: MEDIUM — Verification: read; the unknown-extent WAV source shape itself is UNVERIFIED (no fixture exercises an unseekable WAV).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-22 | REAL | — | Family truths accurate; FLAC-in-WAV "frames ride the packets" — decode lane honestly pinned as a gap by the test (see claim table) |
| `open` | 36-88 | REAL | S4 | F-004 clamp edge; seek query before session; wrapper gets per-track bases |
| `harvest_facts` | 90-137 | REAL | S4 | F-003 single-format loop; `None => "unknown"` honest fallback; `fmt_chunk_bytes().to_vec()` copy is cold-path |
| `fixture` (test) | 153-157 | REAL | — | manifest-dir relative; fixtures committed (gate #12 PASS) |
| `CountingSink` | 159-172 | REAL | — | counts sample frames via `frames_in` — honest unit; None silently uncounted (test-only, fine) |
| `open` (test helper) | 174-181 | REAL | — | panics via `unreachable!` on open failure — acceptable in-test |
| `decode_frames` (test) | 183-215 | REAL | — | real provider + real demuxer; typed refusal RETURNED (corpus test pins flac gap by name) |
| `pcm_fixture_opens_with_expected_facts` | 217-235 | REAL | — | would fail if harvest broke (codec/pair/None assertions) |
| `all_wav_fixture_flavors_open` | 237-260 | REAL | — | 6 flavors incl. extensible + RF64; would fail on flavor regression |
| `adpcm_extra_data_flows_into_the_decoder` | 262-321 | REAL | — | strongest test in file: builds real WAV, proves blob + 505-frame decode through real provider |
| `start_time_clamps_and_skips` | 323-375 | REAL | — | past-end → 0 packets; mid → survivors ≥ target; would fail if clamp/skip broke |
| `broken_fmt_chunk_refuses_at_the_demux_stage` | 377-398 | REAL | — | would fail if broken fmt stopped refusing |
| `CorpusTruth`/`TRUTHS` | 410-504 | REAL | S4 | F-002 flac dead truth value |
| `corpus_dir` | 506-516 | REAL | S3 | F-001 gate |
| `corpus_wav_lanes_decode_through_the_open_path` | 518-589 | REAL | S3 | F-001; asserts per-family fact shape before decode; flac gap asserted by error code substring |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| pcm_fixture_opens_with_expected_facts | YES | asserts codec string, (2,44100) pair, extra_data None, span > 0 |
| all_wav_fixture_flavors_open | YES | asserts per-flavor container/track/pair; fixture presence asserted |
| adpcm_extra_data_flows_into_the_decoder | YES | 505-frame decode count through the real provider; blob length+tag asserted |
| start_time_clamps_and_skips | YES | 0-packet and ≥target assertions directly encode the clamp+skip contract |
| broken_fmt_chunk_refuses_at_the_demux_stage | YES | expects Err with stage Demux |
| corpus_wav_lanes_decode_through_the_open_path | ONLY WITH CORPUS | F-001: without it the body returns after the skip eprintln |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "FLAC-in-WAV: the 34-byte STREAMINFO … frames ride the packets" | module docs 20-22 | SHALLOW | seeding verified (corpus test asserts 34-byte blob) but decode refuses typed `flac.frame.*` — the pinned gap the test itself documents (wav.rs:555-560) |
| "clamp `--start-time` against the declared sample-frame span" | module docs 4-5 | REAL | test start_time_clamps_and_skips + passing run |
| "the demuxer's own accessors are the only source — no local copy of the tag tables" | module docs 9-11 | REAL | harvest reads `demuxer.format()`/`fmt_chunk_bytes()` only |

## 7. Dead / duplicated / unreachable code
- `truth_frames` on the flac row (F-002).
- No other dead code; the production half is 92 lines and all of it reachable.

## 8. Dependencies & flags
Depends on demux (wav), media-types, plugin-api, commands — all default-on; no feature gates. No new deps.

## 9. Open questions for the next agent
1. Did the 2026-10-24 corpus instance include the CLI wav lanes, or only crates/demux/tests/corpus_wav.rs? (worklog check; affects how much of F-001 is mitigated by historical evidence)
2. demux_host agent: `MediaFacts::duration_us` (demux_host.rs:239) — confirm None only when span end is Missing, and which container/host combos can produce that on the shipped file-open path.
