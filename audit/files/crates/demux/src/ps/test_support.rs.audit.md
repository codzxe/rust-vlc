# AUDIT: crates/demux/src/ps/test_support.rs
| Field | Value |
|---|---|
| Auditor | W05-A08 |
| Date | 2025-11-23 |
| Lines / functions | 153 / 10 fns |
| File verdict | REAL |
| Reachable from shipped entry point? | NO from runtime (by design, "not player surface" — honest); compiled into release via `pub mod test_support;` (ps/mod.rs:94, unconditional). Publicness is genuinely required here: the rustdoc example at ps/mod.rs:296-310 consumes `demo_stream`+`mem_stream` (doctests cannot see cfg(test) items) — baseline ran it (`cargo-test-workspace.txt:2474 ... ps::PsDemuxer (line 296) ... ok`). Other consumers: `ps/fuzz.rs:36` (mem_stream), `ps/tests.rs:17-19` (all builders), `apps/cli/src/containers/ps.rs:658` (mpeg1_pack/mpeg1_pes/program_end, inside `#[cfg(test)]` at :556). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `sed -n` quotes (exit 0); `./target/debug/deps/demux-036dd705535b2fb3 ps::` → 28 passed / 0 failed (exit 0); `ffmpeg -f lavfi -i testsrc -f mpeg/-f dvd` byte-generation + `od`/python hexdumps of the result and of `crates/demux/fixtures/ps/ps-{mp2-video-audio.mpg,vob-ac3.vob}` (exit 0) |

## 1. One-paragraph verdict
The honest one of the two test_support files I audited: every layout claim I could test against ground truth holds. `mpeg2_pack` is byte-exact against real ffmpeg output (generated `44 00 04 00 04 01 … f8` pack decodes under the builder's exact field/marker layout; SCR=1 second pack matches too); `ac3_substream_payload` matches the real ffmpeg-authored fixture's private-1 payload (`80 06 00 01` + `0b 77` AC-3 syncword); `stamp_bytes` round-trips with the real `parse_stamp` at 7 tick values including the 2^33−1 boundary (test-pinned, I ran it: 28/28 ps tests pass); and the public-module justification (doctests) is real, unlike mp4's. The only defects are two S4 documentation/masking nits.

## 2. What actually works (verified)
— verified by `./target/debug/deps/demux-036dd705535b2fb3 ps::` -> `test result: ok. 28 passed; 0 failed` (exit 0) and baseline doctest `cargo-test-workspace.txt:2474`.
— verified by `ffmpeg -f lavfi -i testsrc=duration=0.04:size=64x48:rate=25 -f mpeg/-f dvd` + od: real MPEG-2 VOB pack `44 00 04 00 04 01 86 66 cf f8` and second pack (`b8=0x0c` → SCR base 1) both decode consistently under `mpeg2_pack`'s layout ('01'+[32:30]+M+[29:28] / [27:20] / [19:15]+M+[14:13] / [12:5] / [4:0]+M, ext, `|0x03` trailing markers, `0xF8|stuff`); the demuxer's own pin (ps/mod.rs:440-443 "pinned against ffmpeg's `44 00 04 00 04 01` zero-SCR packs") agrees.
— verified by hexdump of `crates/demux/fixtures/ps/ps-vob-ac3.vob`: first 0xBD PES payload = `80 06 00 01 0b 77 …` — substream byte + 3-byte filler + AC-3 frame, exactly `ac3_substream_payload`'s shape.
— verified by hexdump of `crates/demux/fixtures/ps/ps-mp2-video-audio.mpg`: pack `21 00 01 00 01 a1 9b 1d` — `mpeg1_pack`'s structure with trailing `01`.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/ps/test_support.rs:51-53
Code:
```
    // The trailing two bits: the spec says markers `11`, ffmpeg
    // writes `01` — the builder forges ffmpeg's (see tests).
    out.push((((m & 0x3F) as u8) << 2) | 0x01);
```
What is wrong: "ffmpeg writes `01`" is version-dependent, not a fact about ffmpeg. Verified by me: the repo's pinned fixture `ps-mp2-video-audio.mpg` carries trailing `01` (byte 11 = 0x1d), but a freshly generated `ffmpeg 7.1.5 -f mpeg` file carries trailing `11` (byte 11 = 0x67, low bits `11`). The demuxer parses the bits as rate data and enforces neither flavor (ps/mod.rs:474-477: "parsed, never enforced"), so there is no functional defect — but the comment states an overgeneralized falsehood about ffmpeg that a future maintainer could "fix" against. What correct looks like: "the repo's ffmpeg-authored fixtures carry `01` (older ffmpeg); 7.1.5 writes `11`; parsed, never enforced". Confidence: HIGH — Verification: ran (ffmpeg 7.1.5 generation + od of both files).
F-002 [S4]
Where: crates/demux/src/ps/test_support.rs:27,41,48
Code:
```
    let t = ticks & 0x1_FFFF_FFFF;
```
(and `let s = scr & 0x1_FFFF_FFFF;` :41, `let m = mux_rate & 0x3F_FFFF;` :48). What is wrong: out-of-range inputs are silently masked — a caller passing a 34-bit tick value or a mux_rate > 4,194,303 gets a different stream than asked, with no assert, no doc note. `stamp_bytes`' round-trip test pins 2^33−1 as the max, but the masking itself is undocumented. Test-scale harmless. What correct looks like: `debug_assert!` on range or a doc line. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mem_stream` | 17-21 | REAL | — | delegates to the avi seam; fuzz.rs:36 + doctest + CLI tests |
| `stamp_bytes` | 26-35 | REAL | S4 | 3/15/15 split correct (markers at b0.0/b2.0/b4.0); round-trip pinned incl. 2^33−1; F-002 masking |
| `mpeg1_pack` | 40-55 | REAL | S4 | 12-byte shape ✓ vs real fixture; F-001 comment |
| `mpeg2_pack` | 60-77 | REAL | — | byte-exact vs real ffmpeg VOB (empirically verified); ext hardcoded 0 (comment discloses); `0xF8\|stuffing` ✓ |
| `mpeg2_pes` | 81-97 | REAL | — | PES_packet_length counts post-length bytes ✓; 0x80/0x81 flags ✓; PTS-first ✓; no-stamp = 0xFF + hdrlen 1 ✓ |
| `mpeg1_pes` | 102-118 | REAL | — | 0x0F no-stamp convention pinned by test (tests.rs:126-133) |
| `program_end` | 122-124 | REAL | — | `00 00 01 B9` ✓ ISO 11172 end code |
| `ac3_substream_payload` | 129-134 | REAL | — | shape-exact vs real fixture payload (`80 06 00 01` + `0b 77`) |
| `demo_stream` | 139-146 | REAL | — | doctest consumer (baseline :2474 ok) |
| `ramp` | 151-153 | REAL | — | honest "not a real frame" disclosure |

## 5. Test honesty
No tests in this file; it IS test support. Honesty = builders vs ground truth: verified against real ffmpeg output and the repo's ffmpeg-authored fixtures (§2). The consumer unit tests are mutation-sensitive (`stamp_bytes_roundtrip_with_the_parser` asserts exact tick equality for 7 values; pack-shape tests assert exact bytes, e.g. tests.rs:115 `assert_eq!(bytes[13], 0xF8)`); malformed-shape coverage exists (`fuzz_lite_mutations_of_the_committed_fixtures_never_panic`, passed).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "rustdoc examples … need deterministic container shapes" (why public) | :3-5 | REAL | doctest ps/mod.rs:296-310 consumes it; baseline :2474 ok |
| "the fuzz/conformance suites need deterministic shapes" | :4-5 | REAL | fuzz.rs:36; CLI ps.rs:658 (builders, cfg(test)); ps/tests.rs:17-19 (incl. malformed mutations) |
| "Test-support surface, not player surface" | :6 | REAL | rg: zero production callers |
| "the spec says markers `11`, ffmpeg writes `01`" | :51-52 | SHALLOW (version-dependent) | F-001: repo fixture `01` ✓, ffmpeg 7.1.5 writes `11` (my run) |
| stamp_bytes "round-trips with its parse" | :23-24 | REAL | tests.rs:118-125, ran (28/28) |
| ramp "not a real frame — bytes the demuxer must pass through" | :148-149 | REAL | honest disclosure, read |

## 7. Dead / duplicated / unreachable code
None. All 10 functions have consumers (tests.rs imports 9 of 10 at :17-19; demo_stream via doctest; mem_stream via fuzz/CLI). No duplication with mp4/mkv builders (different formats).

## 8. Dependencies & flags
No feature gates; std-only. Deps: `plugin_api::SourceStream`, `crate::avi::test_support` (mem_stream). No unsafe/unwrap/expect/panic (house rules upheld; `u16::try_from(...).unwrap_or(u16::MAX)` in both PES builders is a clamp, not a panic).

## 9. Open questions for the next agent
- ps/mod.rs owner: the real VOB fixture's first 0xBD PES carries `header_data_length = 9` (PTS + PES_extension/P-STD bytes `10 60 04 ff`) before the substream payload; the builder writes hdrlen 5. Both parse — but confirm the demuxer's PES-extension skip is tested against the fixture (it is exercised by the ps_fixtures lanes, all 4 passed in baseline — `cargo-test-workspace.txt:2389-2398`).
- Whoever owns the fixture generator: record the ffmpeg version that produced the `01`-trailing MPEG-1 fixtures (F-001) so the pinned divergence is auditable.
