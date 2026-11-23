# AUDIT: crates/codecs/tests/av1_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Lines / functions | 597 / 6 tests + 6 helpers (Truth/FIXTURES/Sink/ivf_packets/decode_all/video_bytes + inline SplitMix64) |
| File verdict | REAL |
| Reachable from shipped entry point? | The test itself: NO (test binary). The decoder under test: YES — apps/cli/src/pipeline_host.rs:304-308 `"av1" \| "av01" => Av1Decoder::new()`; rav1d-backed (librav1d-*.rlib present in target/deps). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/av1_differential-745f0c9fef4e31ca` → **6 passed / 0 failed** (9.16s) — with ffmpeg ON PATH, so `differential_pixel_md5_against_ffmpeg_dump` exercised the LIVE external oracle, not the pinned constants. Independent `ffmpeg -i <fixture> -f rawvideo -pix_fmt yuv420p - \| md5sum` for all three 8-bit fixtures → `275082414fa880cc72a4c52f71875953` / `fbedc17b688665d981b917380e83e2c1` / `f153f1d34f1c8828a8c061fefc100d74` — **exactly the three pinned constants**. `ffprobe` nb_frames/width/height for all fixtures → matches FIXTURES (320,240,60; 160,128,18; 192,108,6; tenbit 128,96,yuv420p10le,5). |

## 1. One-paragraph verdict
This is what a real differential looks like: the AV1 decoder (rav1d-backed, wired into the shipped CLI) is decoded against ffmpeg's own rawvideo dump, and the comparison runs LIVE whenever ffmpeg is on PATH — which it was in my run, so the passing test is a genuine external-oracle bit-exactness proof, not a self-referential golden. I independently re-computed all three pinned MD5 constants with this sandbox's ffmpeg and they match byte-for-byte, so the fallback constants are honest too. Frame counts/dimensions/pts ladder, the 10-bit typed-refusal boundary, per-packet error typing, mid-stream corruption with exact resync accounting (1 typed error, 2 gate skips, 15/18 survivors, pts restoration), and a 6000-iteration fuzz-lite sweep all pass with exact assertions. Two hygiene findings only.

## 2. What actually works (verified)
- 6/6 tests pass (binary run pasted above), including the live-oracle MD5 test (9.16s — the ffmpeg subprocess decode ran).
- Pinned-constant honesty: independent ffmpeg md5sums equal the three `yuv_md5` constants in `FIXTURES`.
- ffprobe-derived expectations (`frames`, `size`) re-verified independently for all four fixtures.
- The `midstream_corruption_resyncs_and_recovers` test asserts exact outcome counts and survivor pts — would fail on any regression of the reference-state gate.

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/av1_differential.rs:144-146
Code:
```rust
            discontinuity: packet.discontinuity,
            generation,
        };
        one.generation = generation;
```
What is wrong: `one.generation = generation;` is a dead store — the struct literal two lines up already sets `generation`. Same pattern at L272 (`one.generation = generation;` after `packet.clone()` — that one is a genuine overwrite, the L145 one is not). Harmless duplication; a reader may wrongly conclude the first assignment was insufficient. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/codecs/tests/av1_differential.rs:220-249
Code:
```rust
        let expected = if ffmpeg {
            let out = std::process::Command::new("ffmpeg")
```
What is wrong: on the ffmpeg-present lane the pinned `yuv_md5` constant is never compared, and nothing pins the committed `.ivf` fixture bytes themselves (no sha256 of the fixture anywhere in this suite, unlike the MJPEG corpus's whole-file digests). A silently regenerated fixture would still pass the live lane (both sides decode the new bytes) and only fail on ffmpeg-less machines. Adding `assert_eq!(live, truth.yuv_md5)` when ffmpeg is present would double-pin fixture integrity. Confidence: HIGH — Verification: read (no sha of fixtures in file; live-lane behavior read from L231-248).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Truth struct + FIXTURES | 50-81 | REAL | — | constants independently re-verified (ffprobe + ffmpeg md5) |
| Sink (impl FrameSink) | 84-94 | REAL | — | collecting sink |
| ivf_packets | 98-117 | REAL | — | the LANDED demux::ivf, not a re-implementation; clean-fixture errors unreachable! |
| decode_all | 120-153 | REAL | S4 | F-001 dead store; collects error codes; flush drains reorder delay |
| video_bytes | 156-167 | REAL | — | asserts v0 output shape (CPU I420) |
| differential_frame_count_dimensions_and_pts_vs_ffprobe (test) | 169-210 | REAL | — | name's "vs ffprobe" is recorded-truth (constants), re-verified by me live |
| differential_pixel_md5_against_ffmpeg_dump (test) | 212-250 | REAL | S4 | F-002; the core live differential — verified |
| tenbit_fixture_is_a_typed_refusal_not_a_misdecode | 252-291 | REAL | — | exact code `av1.bitdepth`, 5/5 refusals, zero frames |
| error_paths_are_typed_and_per_packet | 293-402 | REAL | — | unconfigured/wrong-descriptor/forbidden-bit/truncated/empty + recovery; honest note that `av1.init.engine` is code-reviewed not fault-injected |
| midstream_corruption_resyncs_and_recovers | 404-489 | REAL | — | exact counts 1/2/15 + pts ladder |
| fuzz_lite_framing_and_decode_survival | 499-596 | REAL | — | 3000 random + 3000 structured mutations; payload_bytes ≤ len invariant; well-formed I420 invariant |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| differential_pixel_md5_against_ffmpeg_dump | YES | a single wrong pixel changes the MD5; live ffmpeg decode is external to the code under test |
| differential_frame_count_dimensions_and_pts_vs_ffprobe | YES | exact counts, per-frame size + I420 byte length + per-frame pts |
| tenbit_fixture_is_a_typed_refusal_not_a_misdecode | YES | wrong kind, wrong code, any fabricated frame, or any silent decode fails it |
| error_paths_are_typed_and_per_packet | YES | asserts specific context codes per damage class + post-damage clean decode |
| midstream_corruption_resyncs_and_recovers | YES | exact typed_errors/gate_skips/frame-count/pts vectors |
| fuzz_lite_framing_and_decode_survival | YES (panic class) | any panic fails the test; structural invariants asserted per input |
Self-referential? NO — the oracle is ffmpeg (external process), or constants independently derived from ffmpeg and re-verified here.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "our concatenated I420 output MD5 == ffmpeg's own dump MD5 (computed live when ffmpeg is on PATH...)" | header L16-20 | REAL | live run passed; independent md5sum matches |
| "the recorded constants are the same command's output" | header L19-20 | REAL | all 3 constants == this sandbox's ffmpeg output |
| rav1d-backed Av1Decoder | header L3 | REAL | librav1d rlib in deps; decoder wired at pipeline_host.rs:304 |
| "g6 variant forces an independent frame every 6 packets" | header L32-34 | REAL-UNPROVEN | resync test behavior consistent (gate reopens at packet 12); keyframe positions not directly inspected |

## 7. Dead / duplicated / unreachable code
- The dead store F-001. Nothing else.

## 8. Dependencies & flags
- Uses codecs, demux (ivf + test_support), plugin-api, testkit (.must), media-types, codec-api — all workspace crates; ffmpeg/ffprobe are external processes probed at runtime. No feature gates; not ignored; runs in the default `cargo test -p codecs` lane (baseline: 4744 passed includes it).

## 9. Open questions for the next agent
- crates/codecs/src/video/av1/{mod,decoder,obu}.rs are NOT in my assignment — the reference-state gate and pending-map pts design deserve their own function-level audit (the resync test only covers one damage pattern at one position).
- Does CI run with ffmpeg on PATH? If not, the differential silently degrades to the pinned constants (still honest, but weaker).
