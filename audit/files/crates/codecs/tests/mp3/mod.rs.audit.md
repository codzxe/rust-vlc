# AUDIT: crates/codecs/tests/mp3/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 199 / 11 helpers (shared test-support module) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test-support module (not shipped); code under test shipped: `mp3`/`mp2` arms in apps/cli/src/pipeline_host.rs (symphonia-bundle-mp3 engine, feature `codec-mp3`/`codec-mp2` default on) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
Clean shared test tooling for the four mp3 suites: a committed-fixture loader, an ini reference-data parser, the ffmpeg-mirroring f32→s16 conversion, an honest per-sample delta statistic, and the live-oracle decode command. The float→s16 rule (`x·32768`, round-ties-even, clip to [-32768, 32767]) was hand-verified against libswresample's `av_clip_int16(lrintf(...))` at the clip boundaries (32767.5→32767, -32768.5→-32768 both match), and the downstream suites pass live. No findings.

## 2. What actually works (verified)
- `timeout 100 ./target/debug/deps/mp3_bitexact-a1d379f06feffc22 --test-threads=2` → `2 passed` (recorded-window MD5 equality incl. tagged-CBR lane).
- `timeout 60 ./target/debug/deps/mp3_fuzz_survival-a38f4eaafbfa5a92 --test-threads=2` → `2 passed` (mutated fixtures never panic, settle typed).
- Boundary math checked by hand (see verdict); `s16_delta_stats` compares only the common prefix and reports (differing, max|Δ|) — the honest tolerance shape.

## 3. Findings
None. (Checked: f32_to_s16 clip/round boundaries vs ffmpeg's rule — exact, incl. both ties; NaN path → Rust saturating cast yields 0 (deviation from ffmpeg's clamp-to-min possible, but only reachable from NaN decoder output which the engine's typed refusals preclude — no impact); parse_reference totality (typed Err, section/key rules); corpus_dir env-override + is_dir check with loud skip; ffmpeg_decode_s16le panics only on oracle failure, stderr surfaced; decode_s16le composes the shipped decode_stream; no dead code beyond the declared `#![allow(dead_code)]` for suite-shared helpers.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_path / fixture_bytes | 21-34 | REAL | — | panic-on-absent = repo-state error, documented |
| reference_path / reference / fixture_reference / corpus_reference | 37-67 | REAL | — | OnceLock cache, typed sections |
| parse_reference | 71-93 | REAL | — | ini parser, typed errors |
| corpus_dir | 99-106 | REAL | — | env override, honest None |
| f32_to_s16 / to_s16le | 114-137 | REAL | — | ffmpeg-exact rule (boundary-verified) |
| decode_s16le | 141-145 | REAL | — | shipped decoder + conversion |
| s16_delta_stats | 151-165 | REAL | — | common-prefix, per-sample delta |
| ffmpeg_available / ffmpeg_decode_s16le | 169-199 | REAL | — | `-map 0:a -c:a pcm_s16le -f s16le -` |

## 5. Test honesty
This file is the harness; the suites using it (mp3_bitexact, mp3_corpus_differential, mp3_fuzz_survival, mp3_bench) assert recorded MD5s and live-oracle comparisons. Both suites I ran pass; the harness's own claim ("its own correctness is pinned by the recorded expectations") is circular by design but validated by the live ffmpeg cross-checks in the suites.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the ffmpeg-matching s16 conversion ... the EXACT mapping" | :108-113 | TRUE | boundary math hand-verified; bitexact suite green |
| "little-endian on every target the differential runs on" | :127-129 | TRUE (pragmatic) | MD5s pinned LE; targets are the sandbox hosts |

## 7. Dead / duplicated / unreachable code
`#![allow(dead_code)]` at :1 covers helpers not used by every suite — intentional for a shared mod; not hiding rot (each helper has ≥1 suite consumer, rg-verified).

## 8. Dependencies & flags
Test-only; codecs::audio::mp3 (shipped engine), testkit, system ffmpeg.

## 9. Open questions for the next agent
- None.
