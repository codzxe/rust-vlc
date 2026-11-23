# AUDIT: crates/codecs/tests/flac/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 342 / 14 (fixture loaders, ini reference parser, corpus_dir, streaminfo_body, walk_frames, CaptureSink, decode_all drivers, 3 recode helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test helper module); the decoder it drives (FlacDecoder) IS shipped — pipeline_host.rs:253 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | read + cross-verified consumers (flac_fuzz_survival executed earlier this session used walk_frames/CaptureSink paths; flac_bitexact is my file 11); ls/head of tests/data/flac_bitexact_reference.txt (5501 bytes, ffmpeg 7.1.5 provenance header) |

## 1. One-paragraph verdict
A real, clean shared test-surface module: committed-fixture loader, an ini-style reference-data parser (the file exists, 5501 bytes, with an honest provenance header naming the ffmpeg version that authored the fixtures), a corpus-dir resolver with typed skip, a spec-correct STREAMINFO extractor, a CRC-8-valid frame walker whose split is externally cross-pinned (ffprobe frame counts + byte-exact ffmpeg PCM digests via the consumers), and a contract-path decode-all driver that asserts one-Emitted-per-packet and Drained-flush. The endianness recode helpers carry big-endian branches that cannot be exercised on this LE host. No fakeness; two hygiene notes.

## 2. What actually works (verified)
— verified by execution earlier this session: flac_fuzz_survival's gate run (784 mutants, 3451 isolation byte-matches) exercises walk_frames + parse_stream + decode_frame through these helpers; flac/mod.rs's decode_all is the driver for flac_bitexact (file 11, executed below in this session).
— verified `ls crates/codecs/tests/data/flac_bitexact_reference.txt` -> exists (5501 bytes); header states ffmpeg 7.1.5-0+deb13u1 authorship — matches the sandbox's installed ffmpeg version exactly.
— verified walk_frames consumer contract: my python3 metadata parse of flac_mono_s16_44100.flac (frames_start=8286, 3 syncs) is consistent with the walker's scan-from-frames_start design.

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/flac/mod.rs:209-211
Code: (verbatim)
```
        let first_sample = match audio.pts {
            media_types::Stamp::Present(ts) => u64::try_from(ts.ticks).unwrap_or(u64::MAX),
            media_types::Stamp::Missing => u64::MAX,
```
What is wrong: a negative pts tick and a missing stamp both collapse to the u64::MAX sentinel — a comparison against first_samples cannot distinguish them. Test-side only; no current consumer feeds negative pts. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/codecs/tests/flac/mod.rs:298-342
Code: (verbatim, L302-306)
```
        let value = if cfg!(target_endian = "big") {
            (i32::from(triple[0]) << 16) | (i32::from(triple[1]) << 8) | i32::from(triple[2])
        } else {
            (i32::from(triple[2]) << 16) | (i32::from(triple[1]) << 8) | i32::from(triple[0])
```
What is wrong: the big-endian branches of lift_s24_to_s32le / to_native_le are dead on every host this repo tests (LE sandbox; CI lanes x86_64/aarch64 LE); the documented BE correctness is UNVERIFIABLE here. Confidence: HIGH — Verification: read; no BE execution possible.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_path / fixture_bytes | 31-44 | REAL | - | committed-bytes contract, unreachable! on absence (documented) |
| reference_path / reference / fixture_reference / parse_reference | 47-99 | REAL | - | ini parser with typed errors; file verified present |
| reference_list | 102-113 | REAL | - | |
| corpus_dir | 119-126 | REAL | - | env override + /tmp default + None-skip (consistent with the repo's corpus lanes) |
| streaminfo_body | 131-147 | REAL | - | spec walk; STREAMINFO-first enforced |
| walk_frames | 167-190 | REAL | - | CRC-8-valid header scan; externally pinned (ffprobe counts + ffmpeg digests) |
| CaptureSink | 193-217 | REAL | S4 | sentinel conflation (F-001) |
| decode_all / decode_all_from / decode_all_from_scan | 222-288 | REAL | - | contract driver, strict outcome asserts |
| concatenated / lift_s24_to_s32le / to_native_le | 291-342 | REAL | S4 | BE branches unexercised (F-002) |

## 5. Test honesty
No tests in this file (helper); honesty is inherited: every helper's output is cross-checked by the consuming suites (fuzz survival isolation oracle — executed; bitexact digests — file 11).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "its own correctness is pinned by the recorded expectations it is checked against ... never trusted blind" | L14-17 | REAL | flac_bitexact compares decode_all output vs recorded ffmpeg digests; walker mis-split fails ffprobe count pins |
| "130 316 bytes total — the card's < 200 KB mandate" | L29-30 | UNVERIFIED (not re-summed; plausible given ls of fixtures dir) | |

## 7. Dead / duplicated / unreachable code
BE-only branches (F-002). #![allow(dead_code)] is module-scoped with a documented per-binary rationale — acceptable.

## 8. Dependencies & flags
Test-only; testkit::Must used for infallible-by-contract unwraps. No feature gates.

## 9. Open questions for the next agent
- None blocking. The flac decoder itself (src/audio/flac/*) is not in my assignment.
