# AUDIT: crates/codecs/tests/fixtures/alac/alac_stereo24.m4a
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 112,791 bytes (binary) / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (test fixture; exercised by 3 test binaries, all green in orchestrator baseline) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> ISO Media, Apple iTunes ALAC/AAC-LC (.M4A); python3 atom walk (below); `rg alac_stereo24` -> 3 test files; baseline citation |

## 1. One-paragraph verdict
Genuine ALAC fixture. Direct binary parse: `ftyp M4A / isomiso2`, single `alac` sample entry — 2 channels, 24-bit sample size, 48,000 Hz — with an in-band `alac` config box (frameLength 4096, bitDepth 24, channels 2, maxFrameBytes 24,580, avgBitRate 2,304,000 = 48k·2·24 uncompressed-rate marker, rate 48,000) and an 112,029-byte `mdat`. The name `alac_stereo24` accurately describes the contents. Consumed by the bit-exact differential (pinned ffmpeg 7.1.5 s24le MD5 `04ca6310cf048ddca4f12374b1c3849e`), the fuzz-survival suite (cookie+packet mutation), and the (default-ignored) throughput bench — the first two passed in the baseline.

## 2. What actually works (verified)
— python3 atom walk -> ftyp(M4A,isomiso2) / free / mdat 112,029 / moov(mvhd,trak(tkhd,edts,mdia(mdhd,hdlr,minf(smhd,dinf,stbl(stsd))))) — well-formed box tree, no truncation at top level.
— python3 stsd parse -> sample entry `alac`, channels 2, samplesize 24, samplerate 48000, ALACSpecificConfig frameLength=4096 bitDepth=24 numChannels=2 rate=48000.
— Baseline: `test fixtures_bitexact_vs_ffmpeg ... ok` (audit/runs/baseline/cargo-test-workspace.txt:960-964, ran in 0.12s — not skipped) and `alac_fuzz_survival` 3/3 ok (baseline:967-971), both consume this fixture (alac_bitexact.rs:89, alac_fuzz_survival.rs:102).

## 3. Findings
None. Checked: box tree integrity, stsd entry consistency with the filename, ALAC magic config sanity, size on disk 112,791 bytes vs mdat+moov+headers (arithmetic consistent), and that at least one non-ignored baseline test actually exercises it with a pinned external truth.

## 4. Per-function table
N/A (binary asset).

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| alac_bitexact::fixtures_bitexact_vs_ffmpeg (this file's lane) | YES | pinned MD5 of decoded s24le PCM vs ffmpeg 7.1.5 dump; any decode deviation changes the digest; ran (not skipped) in baseline |
| alac_fuzz_survival (cookie24/packets24 from this file) | YES | mutated packets must be typed-or-ok, never panic; bite verified by `errs` accounting in that suite |
| alac_bench::alac_decode_throughput_fixtures | N/A | `#[ignore]`d throughput measurement (baseline:952-954 shows 1 ignored) — by design, not evidence |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "| `alac_stereo24.m4a` | 2 | 48_000 | s24le | `04ca6310...` |" | crates/codecs/tests/alac_bitexact.rs:25 | REAL | my atom parse confirms 2ch/24-bit/48kHz container facts; the MD5 itself re-proven only by the passing baseline test (decoder not re-run here — UNVERIFIED directly) |

## 7. Dead / duplicated / unreachable code
N/A. Fixture referenced by 3 test files — not orphaned.

## 8. Dependencies & flags
Data-only asset.

## 9. Open questions for the next agent
None.
