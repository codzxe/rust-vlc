# AUDIT: crates/codecs/tests/adpcm_bench.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 239 / 1 test + 4 helpers |
| File verdict | REAL (methodologically sound measurement bench, honestly #[ignore]d) — cannot produce evidence in a corpus-less checkout (F-001) |
| Reachable from shipped entry point? | NO (test binary; `#[ignore]`d by design — baseline records it ignored) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read of full file; `ls /tmp/my-project/corpus-v4/media` (absent); `ls crates/codecs/tests/fixtures/` (committed ADPCM wavs exist); baseline citation audit/runs/baseline/cargo-test-workspace.txt:915 |

## 1. One-paragraph verdict
A real throughput bench with better hygiene than most: it drives the exact contract path the differential drives (FileAccess open → WavDemuxer → configure from the real fmt chunk → per-block decode → sink), asserts the exact inter-channel sample extent (168,822×2 / 128,524×2 — the differential's pinned window) on the warm-up pass AND on every repetition, reports the median of 7 reps as samples/s and ×realtime, and enforces an explicit floor (1 M/s release, 50 k/s debug). The `#[ignore]` is the documented flac_bench convention ("never the normal gate run") and the baseline records exactly that (1 ignored). The single defect is environmental: both lanes live in the private /tmp corpus, which is absent here, so even the `--ignored` invocation skips loudly and the 5.R1 row stays DNF — no committed-fixture fallback exists although mono ADPCM wavs are already committed in-tree for other tests.

## 2. What actually works (verified)
— Read: decode_pass asserts one audio track, the registry descriptor string (`adpcm_ima_wav`/`adpcm_ms`), channels/rate from the demuxer, `DecodeOutcome::Emitted { frames: 1 }` per block, and the exact inter-channel extent — the bench doubles as a correctness re-check of the differential's window (would fail on any decode regression).
— Arithmetic checked: median index `per_sec[REPS/2]` = 3 of 7 ✓; realtime = median/(channels·rate) ✓; `bytes_per_sample().max(1)` guards the sink's division ✓.
— Baseline: `test adpcm_decode_throughput_corpus_lanes ... ignored, 6.ADPCM-bench throughput measurement ...` (audit/runs/baseline/cargo-test-workspace.txt:915) — the ignore reason itself documents the intended invocation and where the number lands.

## 3. Findings
F-001 [S4]
Where: adpcm_bench.rs:46-61, 70-77
Code: (verbatim, adpcm_bench.rs:46-53)
```
const LANES: [Lane; 2] = [
    Lane {
        name: "0042-audio-only.wav",
        descriptor: "adpcm_ima_wav",
        channels: 2,
        rate: 44_056,
        frames: 168_822,
    },
```
What is wrong: Both measured lanes reference private corpus files; `corpus_media_dir()` falls back to `/tmp/my-project/corpus-v4/media`, which does not exist in this sandbox (verified). The bench therefore produces zero evidence from a fresh checkout without the private corpus, and the 5.R1 row it closes "stays DNF" — honestly documented (lines 16-18), but avoidable: committed mono ADPCM fixtures already exist (`crates/codecs/tests/fixtures/adpcm_ima_mono.wav`, `adpcm_ms_mono.wav`, verified present) and could carry at least a floor-checking lane. Blast radius: a worklog measurement, not a gate.
Confidence: HIGH — Verification: ls of corpus dir and fixtures dir.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Lane/LANES consts | 32-61 | REAL | — | extents match the differential's pinned windows (per doc; cross-checked by name) |
| corpus_media_dir | 70-77 | REAL | — | LUMEN_CORPUS override honoring both conventions; None → loud skip |
| open_via_input | 84-89 | REAL | — | the differential's exact open path; `.must()` on to_str is test-acceptable |
| CountingSink::accept | 98-107 | REAL | — | counts samples, no PCM copy — throughput shape honest |
| decode_pass | 112-172 | REAL | — | full contract path; Emitted{frames:1} per block; loud unreachable! on any damage |
| adpcm_decode_throughput_corpus_lanes | 174-239 | REAL | S4 | warm-up + 7 reps + median + floor; corpus-gated (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| adpcm_decode_throughput_corpus_lanes (when corpus present) | YES | exact extent asserted on warm-up and every rep (any decode regression fails it), plus the throughput floor |
| In this sandbox | N/A | skips loudly even when --ignored (corpus absent); baseline records it as ignored-by-design, not as a fake pass |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an ignored measurement test whose number goes into the card's worklog entry ... never the normal gate run" | adpcm_bench.rs:1-4 | REAL | baseline:915 shows 1 ignored |
| "Decode-only throughput is strictly higher than the reported number (the wav demux + file open ride along)" | adpcm_bench.rs:13-15 | REAL | read: timing wraps open+demux+decode |
| "when absent the test SKIPs with a message ... and the 5.R1 row it closes stays DNF" | adpcm_bench.rs:15-18 | REAL | skip path verified by reading; corpus absent |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
codec_api (DecoderConfig/ThreadBudget — default budget, consistent with config.rs F-001), demux::wav, input::FileAccess, testkit. No features. Env knob: LUMEN_CORPUS.

## 9. Open questions for the next agent
None.
