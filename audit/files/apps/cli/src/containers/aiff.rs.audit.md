# AUDIT: apps/cli/src/containers/aiff.rs
| Field | Value |
|---|---|
| Auditor | W13-A08 |
| Date | 2026-11-21 |
| Lines / functions | 451 / 11 (2 production fns + 4 test helpers + 1 sink impl + 5 tests) |
| File verdict | REAL — the open/harvest lane is genuine, all 5 tests are skip-free, hard-asserting, committed-fixture-based and baseline-executed; only hygiene-level gaps |
| Reachable from shipped entry point? | YES — `lumen` → run/dispatch → `demux_host::open_source` → `containers::open` (mod.rs:53 `"aiff" => aiff::open(stream, opts)`) → this `open` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `Read` aiff.rs 1-451 (full); `ls tests/fixtures/generated/valid | grep -i aif` (exactly 4 valid fixtures: aifc_raw_u8, aifc_sowt_s16le, aiff_pcm_s16be, aiff_pcm_s24be — no fl32/fl64/s32) + malformed set (trunc0-4, flip1/4/16, garbage0 — `__trunc1` used by the test exists); `grep -n "containers::aiff" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (all 5 tests `ok`, lines 2781-2786); `rg "pcm_f32be\|fl32" crates/` (demuxer maps fl32/fl64→pcm_f32be/f64be, aiff/chunks.rs:100-101, unit-tested at aiff/tests.rs:223-224; PCM decoder supports the formats, codecs/src/audio/pcm/mod.rs:53-55,173-175; NO fixture or end-to-end test anywhere joins them); `sed -n '245,280p' crates/demux/src/aiff/chunks.rs` (COMM gates: rate finite/positive/integral ≤ u32::MAX, channels 1..=64 — the harvest's ungated pair is safe); `grep -n aiff apps/cli/src/containers/mod.rs` (dispatch); `sed -n` spot-checks of quoted ranges. |

## 1. One-paragraph verdict
This is the cleanest container host audited in my list: `open` follows the shared demux→start→harvest→clamp→seek-query→wrapper shape with no panics, no swallowed errors, no guessed values, and the demuxer beneath it refuses everything outside its vocabulary typed (channels 1..=64, normalized/positive/integral 80-bit rates, compression whitelist — verified in the demuxer source, not trusted from comments). All five tests run in the baseline against committed fixtures with hard assertions and no skip path — the ffmpeg-measured decode truths (8 820/9 600 frames) would fail on any harvest or decode regression, and the two refusal tests (hand-built `ima4` AIFC, classic 8-bit, committed truncation derivative) pin the typed error codes. The only defects are hygiene: the "differential" covers 4 of the 7 documented PCM forms — float (`fl32`/`fl64`) and 32-bit AIFF lanes have no fixtures and zero end-to-end evidence anywhere in the repo, despite the module doc advertising them — and the `--start-time` watermark test exempts `Stamp::Missing` packets from its assertion, the same hole as the mkv twin.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2781-2786` → `containers::aiff::tests::{all_aiff_fixture_flavors_open_and_decode, pcm_be_fixture_opens_with_expected_facts, start_time_clamps_and_skips, truncated_aiff_refuses_at_the_demux_stage, unsupported_aifc_compression_refuses_typed}` all `ok` (committed fixtures; no corpus gate, no skip path — these are real passes).
— Verified by read: the COMM validation backing the ungated `pcm` pair — `crates/demux/src/aiff/chunks.rs:258-266` rejects `channels == 0`, `channels > 64`, and non-normalized/non-positive/fractional rates before `start()` returns, so `harvest_facts` cannot mint an implausible geometry.
— Verified by read: the typed-refusal chain — module doc's compression vocabulary matches `resolve_compression` in the demuxer (`fl32`/`fl64`→float descriptors at chunks.rs:100-101, `ima4`/`ulaw`/`alaw` → typed `unsupported-compression`).

## 3. Findings

### F-001 [S4]
Where: apps/cli/src/containers/aiff.rs:247-276 (the truths table), 13-16 (module-doc vocabulary claim)
Code:
```rust
//! * **Every form the demuxer exposes is a PCM descriptor**: classic
//!   AIFF / AIFC `NONE` (big-endian `pcm_s16be`/`s24be`/`s32be`),
//!   AIFC `sowt` (little-endian `pcm_s16le`/`s24le`/`s32le`),
//!   `fl32`/`fl64` (big-endian float) and `raw ` (unsigned 8-bit,
```
What is wrong: The end-to-end differential (the test's own claim: "No corpus lanes exist for this family … these fixtures ARE the differential", 234-236) exercises 4 descriptors (`pcm_s16be`, `pcm_s24be`, `pcm_s16le`, `pcm_u8`). The documented vocabulary adds `pcm_s32be`, `pcm_s32le`, `pcm_f32be`, `pcm_f64be` — for these, no fixture exists in the tree (verified by ls), no CLI test, and no end-to-end coverage anywhere (`crates/demux/tests/wav_aiff_fixtures.rs` has no fl32/fl64/s32 truths; only the demuxer's descriptor-mapping unit test at aiff/tests.rs:223-224 and the PCM decoder's format table touch them).
Why it matters: The float and 32-bit AIFF lanes are shipped, advertised, and evidence-free — a regression in the f32be byte-order or the s32be width would pass every test in the repo.
What correct looks like: four more 200 ms fixtures (fl32, fl64, s32be, s32le) with ffmpeg-measured truths in the same table.
Confidence: HIGH — Verification: read + fixture ls + repo-wide grep.

### F-002 [S4]
Where: apps/cli/src/containers/aiff.rs:413-422 and 437-443
Code:
```rust
                Ok(DemuxEvent::Packet(packet)) => {
                    packets += 1;
                    if let Stamp::Present(pts) = packet.pts {
                        let ns = pts
                            .convert(&media_clock::NANO_BASE)
                            .unwrap_or_else(|e| unreachable!("{e:?}"));
                        assert!(
                            ns.ticks >= past.start_position_ns,
```
What is wrong: In the in-media-target half of `start_time_clamps_and_skips`, packets whose pts is `Stamp::Missing` bypass the ≥1 ms watermark (the `if let` skips the assert); only Present-stamped packets are checked. (The past-the-end half is immune — it asserts `packets == 0` by count.)
Why it matters: Same class as mkv.rs#part2 F-004: the one test guarding the skip watermark has a hole where a timestamps regression could hide. One test, one container family.
What correct looks like: assert zero `Missing`-pts packets among the survivors, or count them and assert a recorded bound.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` (production) | 51-94 | REAL | — | Shared container shape; `i64::MAX/4` fallback clamp only when duration missing (AIFF always declares); pure seek query gates `--start-time`; no panics, no swallowed errors |
| `harvest_facts` (production) | 102-128 | REAL | — | Ungated `(channels, rate)` pair is safe — demuxer gates 1..=64 channels and normalized positive integral rates (chunks.rs:258-266, verified); same pair stamped for every track (demuxer mints exactly one); `extra_data: None` per the PCM truth |
| `fixture` (test helper) | 144-148 | REAL | — | Committed-fixture resolver |
| `CountingSink::accept` | 153-163 | REAL | — | Counts sample frames via `frames_in` — the honest unit |
| `open` (test helper) | 167-172 | REAL | — | Real `open_source` path; panics on error, no silent pass |
| `decode_frames` | 177-204 | REAL | — | Real provider, real decode loop, `?` propagates decode errors |
| `pcm_be_fixture_opens_with_expected_facts` | 211-227 | REAL | — | Executed (baseline :2782); hard asserts on codec/pair/no-blob/span-end 8 820 |
| `all_aiff_fixture_flavors_open_and_decode` | 237-310 | REAL | F-001 | Executed (:2781); 4 flavors to exact ffmpeg frame counts; would fail on harvest/decode regression; missing 4 documented flavors |
| `unsupported_aifc_compression_refuses_typed` | 319-377 | REAL | — | Executed (:2786); hand-built `ima4` AIFC + classic s8 via demuxer builder; asserts stage + `demux.aiff.unsupported-compression`; self-checking (a build error would yield a different code and fail) |
| `truncated_aiff_refuses_at_the_demux_stage` | 382-396 | REAL | — | Executed (:2784); committed `__trunc1` derivative; asserts `demux.aiff.truncated` |
| `start_time_clamps_and_skips` | 402-450 | REAL | F-002 | Executed (:2783); past-end → clamped start > 0 + 0 packets; in-media watermark with the Missing-pts hole |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| pcm_be_fixture_opens_with_expected_facts | YES | Committed fixture, exact facts asserts, no skip path, baseline-executed |
| all_aiff_fixture_flavors_open_and_decode | YES | Exact ffmpeg frame truths per flavor; `assert!(path.is_file())` self-checks fixture presence |
| unsupported_aifc_compression_refuses_typed | YES | Asserts both stage and typed code for two distinct refusal shapes |
| truncated_aiff_refuses_at_the_demux_stage | YES | Asserts stage + `demux.aiff.truncated` |
| start_time_clamps_and_skips | YES (partial) | Past-end half is count-immune; in-media half has the Missing-pts hole (F-002) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every form the demuxer exposes is a PCM descriptor … fl32/fl64 (big-endian float)" (module doc 13-16) | aiff.rs | REAL at demux level / UNPROVEN end-to-end | Mapping unit-tested (aiff/tests.rs:223-224); no fixture or decode evidence (F-001) |
| "the COMM parse itself rejects compression types outside that vocabulary … with the typed demux.aiff.unsupported-compression" (23-28) | aiff.rs | REAL | Demuxer chunks.rs resolve_compression + tests :2786 baseline-ok |
| "these fixtures ARE the differential" (234-236) | aiff.rs | SHALLOW-by-scope | True for the 4 covered flavors; 4 documented flavors have no fixtures (F-001) |

## 7. Dead / duplicated / unreachable code
None. The file mirrors the wav twin's structure (intentional, documented duplication of the container shape).

## 8. Dependencies & flags
Workspace-only (demux::aiff, plugin_api, media_types, commands). No feature gates. Two `lumen-allow-literal` waivers on test scratch filenames (347, 364) — legitimate test-only literals.

## 9. Open questions for the next agent
- W13 wav.rs owner: does the WAV twin's differential cover its full documented PCM vocabulary, or does the same 4-of-7 gap exist there? (Symmetry check worth one grep.)
- Whoever owns `TrackFact::audio_geometry` (demux_host chunk): the mkv twin re-gates the pair (channels ≤ 64 etc.) while aiff trusts the demuxer — one layer owning the plausibility gate would remove the asymmetry.
