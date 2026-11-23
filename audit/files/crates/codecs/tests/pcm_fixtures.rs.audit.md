# AUDIT: crates/codecs/tests/pcm_fixtures.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 475 / 8 fns + 7 tests |
| File verdict | REAL (baseline 7/7 pass; ground truth independently re-verified with ffmpeg) |
| Reachable from shipped entry point? | N/A (integration test; `cargo test -p codecs --test pcm_fixtures`; baseline cargo-test-workspace.txt:1317-1330, 7/7 ok in 0.03s) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 hashlib MD5 RFC-1321 vector check (4/4 True); ffmpeg -map 0:a extraction of both parents -> sha256sum; rg/ls fixture+expectation existence; sed quote checks |

## 1. One-paragraph verdict
A genuine acceptance suite that closes the loop it claims: extract the PCM stream from the committed AVI and MP4 parents with test-side walkers, pin those walkers against the writer-recorded sha256, then decode the extracted chunks through the real codec-api `PcmDecoder` path and assert byte-identical output plus per-chunk outcomes, format, pts, and derived duration. Every layer is independently verified: the four RFC 1321 MD5 vectors in the pinning test are correct (recomputed with python hashlib, 4/4 match), the baseline ran all 7 tests green, and — the strongest check — my own independent ffmpeg extraction of BOTH parents produces sha256 `1d9f5019c9b5181f0910528dce360f46e74f9c4df8d9bc9a20c2c68fd218b95b`, exactly the `audio_stream_sha256` recorded in both expectation TOMLs, and identical across containers as the suite asserts. No findings.

## 2. What actually works (verified)
- `python3 hashlib` -> md5 of "", "abc", "message digest", "abcdefghijklmnopqrstuvwxyz" all equal the constants at pcm_fixtures.rs:144-153 (True ×4).
- `ffmpeg -i tests/fixtures/generated/valid/avi_mjpeg_pcm.avi -map 0:a -f s16le -c:a pcm_s16le - | sha256sum` -> `1d9f5019…8b95b` == expectation avi_mjpeg_pcm.avi.toml:108; same command on mp4_mjpeg_pcm.mp4 -> SAME digest == mp4_mjpeg_pcm.mp4.toml:108 — the "identical audio essence" claim (test :229) independently confirmed, and the test-side walkers' pinned truth matches an external tool.
- Baseline: cargo-test-workspace.txt:1321-1330 — all 7 tests `ok`, `7 passed; 0 failed` in 0.03s.
- 352_800 audio_stream_bytes = 60 chunks × 1470 instants × 4 B (CHUNK_COUNT/TICK_INSTANTS at :38-42) — arithmetic consistent.

## 3. Findings
None. What was checked and how: MD5 pin constants (recomputed externally); expectation digests (re-extracted with ffmpeg, not the repo's own walker); fixture and expectation files exist at workspace tests/fixtures (fixtures_root resolution common/mod.rs:30-31 verified: `CARGO_MANIFEST_DIR/../../tests/fixtures`); pts arithmetic (index×1470 @ 1/44100, matches dwSampleSize stamping doc); duration floor 1470/44100 s = 33_333_333 ns (doc says "floored" — correct); big-endian normalization branch (identity on LE CI, byte-swap on BE — honest, documented); outcome assertion shape (`Emitted { frames: 1 }` per chunk — would fail on concealment); flush must return `Drained` (:123-126); synthesized WAVEFORMATEX layout (16 bytes, checked_mul on avg-bytes). Copy-paste `match … unreachable!` style is uniform, no `.ok()` swallows, no ignored tests.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CaptureSink/impl | 47-59 | REAL | — | audio-only, unreachable! on video |
| audio_packet | 64-82 | REAL | — | pts at sample-tick base; whole chunk as data |
| configured | 85-105 | REAL | — | configures from container strf / synthesized WAVEFORMATEX (two real shapes) |
| decode_stream | 109-128 | REAL | — | contract path + flush-must-drain |
| concat | 131-137 | REAL | — | |
| md5_reader_pinned… (test) | 139-154 | REAL | — | RFC 1321 §A.5 vectors — externally recomputed 4/4 |
| avi_audio_extraction… (test) | 156-178 | REAL | — | walker pinned by sha256; verified vs ffmpeg |
| mp4_audio_extraction… (test) | 180-202 | REAL | — | same for sowt/stbl |
| concat_chunk_bodies | 206-212 | REAL | — | |
| both_containers… (test) | 214-230 | REAL | — | avi==mp4 chunks; independently confirmed (same ffmpeg digest) |
| decoded_avi… (test) | 232-256 | REAL | — | strf len 16 assert; real strf config |
| decoded_mp4… (test) | 258-286 | REAL | — | synthesized config; facts asserted (2ch/44100/16bit) |
| assert_bit_exact | 299-370 | REAL | — | 60 frames, per-chunk Emitted{1}, format/pts/duration/size, decoded==extracted, LE sha256 vs writer |
| audio_probe_matches_the_expectation | 372-444 | REAL | — | 4800-instant f32 probe MD5 recomputed from decoder output; both parents agree |
| ConfigureFrom/synthesized_waveformat | 447-475 | REAL | — | checked_mul; debug_assert len 16 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| md5_reader_pinned… | YES | hard-coded external truth (RFC 1321) |
| avi/mp4 extraction tests | YES | sha256 vs writer-recorded digest — a wrong walker fails |
| both_containers… | YES | avi==mp4 equality; independent ffmpeg re-check confirms reality |
| decoded_avi/mp4… | YES | byte equality + sha256 + per-chunk outcomes/pts/duration — a broken PcmDecoder cannot pass |
| audio_probe_matches… | YES | pinned f32-conversion MD5 from expectation TOML |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the concatenated decoded samples equal the container's stream, and … hash-match the writer-recorded sha256" | pcm_fixtures.rs:7-11 | REAL | baseline 7/7; my ffmpeg digest == both expectation sha256s |
| "test-side walkers whose correctness is pinned by the expectation's audio_stream_sha256" | :5-6 | REAL | the pin is real and matches an external extractor |
| "on the little-endian hosts this repo's CI targets" hash-match direct | :9-11 | REAL-UNPROVEN (LE host here: REAL) | cfg branch verified by read; this host is LE so the sha256 assert executed in baseline |

## 7. Dead / duplicated / unreachable code
None. (The expectation TOMLs' `[seek]`/`[first_frame]` sections are consumed by other suites, not this one — not this file's dead code.)

## 8. Dependencies & flags
Test-only. Depends on committed fixtures at workspace tests/fixtures (generated/valid + expectations/valid, CC0-licensed per TOML header) and on tests/common/mod.rs walkers. No external tooling required (unlike av1_conformance).

## 9. Open questions for the next agent
- tests/common/mod.rs (walkers + md5/sha256 helpers) is shared by many suites and is NOT in my assignment — its own audit should confirm the walkers' error paths (this suite only exercises the happy extraction).
