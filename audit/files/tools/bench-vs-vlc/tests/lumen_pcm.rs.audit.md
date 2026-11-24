# AUDIT: tools/bench-vs-vlc/tests/lumen_pcm.rs
| Field | Value |
|---|---|
| Auditor | W19-A07 |
| Date | 2026-11-24 |
| Lines / functions | 560 / 19 (9 test fns, 10 helpers/struct impl) |
| File verdict | REAL-UNPROVEN (tests not executed here; pins independently corroborated by corpus raw rows) |
| Reachable from shipped entry point? | NO — dev/test harness; tools/bench-vs-vlc is NOT a workspace member (Cargo.toml:41 `members = ["crates/*", "apps/*", "tools/xtask", "tests/integration/first-vertical", "tools/mem-audit", "tools/sweep"]` — sed-verified). Reached only by explicit `cargo test --manifest-path tools/bench-vs-vlc/Cargo.toml` or run.py. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg census of baseline (no PACKAGE bench-vs-vlc section — not run in baseline); ls fixtures (exist); rg pins -> corroborated in docs/corpus/{vlc-diff-report-20261023.md:313, compat-matrix-20261110.raw-rows.jsonl:3,4,6,7}; NO cargo test run (reason: cargo-build-budget — non-workspace crate, cold build on 2-core/3GB). |

## 1. One-paragraph verdict
This is what honest differential test code looks like: pinned digests of committed
fixtures, synthetic AVIs built byte-by-byte to prove the digest equals track-1 PCM alone,
typed-refusal garbage test, a TSV parser test with negative cases, and an #[ignore]d
orchestrator-driven batch entry that asserts one row per fixture with no silent skips.
The two pins are not self-asserted: the same MD5 (`08db7ccc163dc478709627c1db9044b5`,
352,800 B, 60 packets) appears as runtime output in
docs/corpus/compat-matrix-20261110.raw-rows.jsonl (a separate corpus-decode pipeline) and
as an independent Python RIFF-walk cross-check in docs/corpus/vlc-diff-report-20261023.md:313.
I could not execute the suite (non-workspace crate, cold build; not in orchestrator
baseline) — hence REAL-UNPROVEN, with the pin corroboration noted.

## 2. What actually works (verified)
— Fixtures exist: `ls tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` (also 2pcm) -> present.
— Pin corroboration: `rg -n 08db7ccc163dc478709627c1db9044b5 docs/` -> vlc-diff-report-20261023.md:313 (Python hashlib cross-verified) + compat-matrix raw-rows (4 rows with identical pcm_md5 from real decode runs).
— Not run: `cargo test --manifest-path tools/bench-vs-vlc/Cargo.toml --test lumen_pcm` — UNVERIFIED (cargo-build-budget; no prebuilt binary in target/debug/deps, `ls | rg bench` -> 0).

## 3. Findings
F-001 [S4]
Where: tools/bench-vs-vlc/tests/lumen_pcm.rs:471
Code: `std::fs::write(&garbage, [0x37u8, 0x13, 0x00, 0x42, 0xff, 0xee].repeat(64)).unwrap_or_default();`
What is wrong: write failure is silently swallowed (unwrap_or_default -> ()); on a read-only
temp dir the test would then feed a nonexistent path to the digest fn, which returns a typed
[input] error and the test still passes — the "refusal" it proves is then about a missing
file, not garbage bytes. Why it matters: test can pass for the wrong reason on exotic
environments. What correct looks like: `.unwrap()` (the fixture write is part of the test's
preconditions). Confidence: HIGH — Verification: read.
F-002 [S4]
Where: tools/bench-vs-vlc/tests/lumen_pcm.rs:62,81,298,332,372,417,454,486,540
Code: `.unwrap_or_else(|err| unreachable!("class-A fixture must digest: {err}"))` (representative, :62)
What is wrong: unreachable! as the error path makes any real error a panic (not a failure
with message). Functionally equivalent to a failed test; style deviation from expect().
Not fakeness. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture() | 32-36 | REAL | — | env!("CARGO_MANIFEST_DIR") join; fixtures verified present. |
| committed_fixture_digest_is_pinned | 59-76 | REAL (test) | — | 13 assertions; would fail on any PCM-affecting decode change; pin corroborated externally. |
| two_pcm_fixture_digests_the_first_track_only | 78-90 | REAL (test) | — | track-rule (first-of-kind) pinned incl. track_handle 1. |
| put_u16le/put_u32le | 97-102 | REAL | — | LE emitters, trivially correct. |
| chunk | 105-114 | REAL | — | RIFF chunk with odd-byte pad — correct. |
| begin_sized/end_sized | 118-130 | REAL | — | size includes form FourCC — correct RIFF semantics; :260-264 comment documents the idx1 off-by-four hazard and the demuxer's probe_offset_base. |
| SynthAudio::frame_bytes | 140-142 | REAL | — | bits/8. |
| build_synth_avi | 150-293 | REAL | — | full RIFF/AVI 1.0 writer: avih flags HASINDEX|ISINTERLEAVED, strl for vids+auds, PCMWAVEFORMAT strf, idx1 with offsets relative to 'movi' fourcc. Mirrors the xtask fixture writer (:148-149 comment). |
| synth_stream | 296-300 | REAL | — | input::SyntheticStream in-memory seam, 4096 window. |
| synthetic_avi_digest_is_exactly_first_track_pcm | 302-347 | REAL (test) | — | Strongest test here: digest == MD5(track-1 bytes alone); any video/track-2/container leakage fails it. |
| filtered_video_stream_does_not_touch_the_audio_digest | 349-398 | REAL (test) | — | FMP4 handler refused; demux_recovered==1 asserted WITH post-W5-14 explanation citing the engine's own pin (crates/demux/tests/idx1_stream_scope.rs::filtered_stream_idx1_entries_are_silent) — cross-referenced, not hand-waved. |
| synthetic_avi_u8_family_digests | 400-424 | REAL (test) | — | pcm_u8 family pinned. |
| synthetic_frames_feed_the_sink_in_order | 426-462 | REAL (test) | — | DigestSink order+completeness == md5 of concatenation. |
| garbage_file_is_a_typed_refusal | 468-481 | REAL (test) | S4 (F-001) | asserts error.starts_with("[probe]")||("[input]") — stage-tagged refusal enforced. |
| fixture_tsv_parses_and_rejects_shape_errors | 483-494 | REAL (test) | — | negative cases: no-tab, duplicate rel refused. |
| batch_json_envelope_carries_rows_and_host_endian | 496-518 | REAL (test) | — | schema/envelope/endian/md5-length assertions; stringly but adequate. |
| corpus_batch_via_env | 530-560 | REAL-UNPROVEN (#[ignore]) | — | ignore reason names run.py driver (exists, 1001 lines); asserts non-empty batch + one row per rel (no silent skips). Never runs in plain cargo test — by design, disclosed. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| committed_fixture_digest_is_pinned | YES | Pinned 32-hex MD5 + byte count; any decode/demux change to PCM path breaks it (pin cross-verified in two external artifacts). |
| two_pcm_fixture_digests_the_first_track_only | YES | Track-selection rule pinned (would catch first-of-kind -> last-of-kind regressions). |
| synthetic_avi_digest_is_exactly_first_track_pcm | YES | Exact digest equality against locally computed md5 of ONLY track-1 bytes — catches leakage of video payload/track-2/container bytes into the digest. |
| filtered_video_stream_does_not_touch_the_audio_digest | YES | FMP4 refusal + recovered==1 + digest equality; would catch filter-into-digest contamination. |
| synthetic_avi_u8_family_digests | YES | u8 conversion path pinned. |
| synthetic_frames_feed_the_sink_in_order | YES | Order/completeness at sink layer. |
| garbage_file_is_a_typed_refusal | YES (with F-001 caveat) | Garbage 6-byte pattern x64 refused with stage-tagged prefix; caveat: swallowed write error can vacuously pass via [input] missing-file error. |
| fixture_tsv_parses_and_rejects_shape_errors | YES | Duplicate-rel and shape errors refused. |
| batch_json_envelope_carries_rows_and_host_endian | PARTIALLY | String-contains assertions; a corrupted-but-present envelope could pass some checks; md5 length check is structural. |
| corpus_batch_via_env | YES when driven | Row-per-fixture assertion; ignored unless env-driven — honest design, discloses the driver. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the committed class-A fixtures' digests are PINNED" | :6-8 | REAL | Pins at :49/:56; corroborated by vlc-diff-report-20261023.md:313 + compat raw-rows. |
| "MEASURED 2026-10-23 two ways that agree ... Python RIFF walk ... hashlib.md5" | :44-47 | REAL | vlc-diff-report-20261023.md:313 states exactly this cross-verification. |
| "the orchestrator runs it explicitly and it asserts ... a row for every requested fixture" | :15-17 | REAL-UNPROVEN | run.py exists (1001 lines, --oracle vlc/mock/pending modes with mock-labeling invariants); an actual driven run not re-executed here. |

## 7. Dead / duplicated / unreachable code
None. All helpers are used by the tests. corpus_batch_via_env is #[ignore]d — disclosed.

## 8. Dependencies & flags
Crate graph (from imports): bench_vs_vlc (lib), input (SyntheticStream), media-types
(SharedBytes, AudioFrame...), plugin_api (SourceStream), codec_api (FrameSink — inside
test :432). Not a workspace member (Cargo.toml:41) — standalone manifest
tools/bench-vs-vlc/Cargo.toml with its own Cargo.lock (ls verified). No feature flags.

## 9. Open questions for the next agent
- Execute `cargo test --manifest-path tools/bench-vs-vlc/Cargo.toml --test lumen_pcm` on a
  machine with build budget; expect 9 pass / 0 fail / 1 ignored.
- W19-A03 audits src/lib.rs; my md5.rs audit (separate file) covers the hash primitive.
- corpus_batch_via_env: confirm run.py's LUMEN_DIFF_OUT consumer (docs/corpus/*.jsonl) matches render_batch_json schema `bench-vs-vlc/lumen-pcm-digests/1`.
