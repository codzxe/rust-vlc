# AUDIT: crates/codecs/src/audio/mp2/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-21 |
| Lines / functions | 105 / 6 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. Chain: `lumen open file.mp2` → apps/cli/src/containers/mod.rs:65 `"mp2" => es::open_mpeg_audio(...)` (ES container lane; also PS demuxer lanes, ps.rs:204) → track codec `mp2` → apps/cli/src/pipeline_host.rs:266-270 `"mp2" => { Mp2Decoder::new() … }` with `use codecs::audio::mp2::Mp2Decoder` (pipeline_host.rs:59) — the doc's "R3 registration card routes the `mp2` descriptor HERE" claim is TRUE in the shipped wiring. Engine present: `codecs` default features include `codec-mp2` (= `codec-mp3` = symphonia-bundle-mp3 0.6.1, crates/codecs/Cargo.toml:55-78), and apps/cli consumes `codecs` with default features (apps/cli/Cargo.toml:51, no feature list). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg '"mp2"' crates/demux/src apps/cli/src (minters: ES + PS lanes; also es.rs test tables); sed -n '261,270p' pipeline_host.rs (descriptor routing verified); sed -n '30,78p' crates/codecs/Cargo.toml (default = full feature set incl. codec-mp2); rg "mp2" audit/runs/baseline/test-workspace.log.txt:383-407 (25 unit tests ok) + :1134-1142 (corpus differential 3 ok incl. `corpus_and_fixture_differential_vs_ffmpeg`); ls fuzz/fuzz_targets/fuzz_codec_mp2.rs (EXISTS — for F-001); hand-verification of the test's constants against the MPEG-1/2 Layer II tables (384 kbps @ V1 idx 14; LSF idx 13 = 160; 0xFFFDE000 = MPEG1 LII 44.1 kHz no-CRC). |

## 1. One-paragraph verdict
A facade module (docs + module declarations + re-exports + one pinning test) for a genuinely reachable, genuinely tested Layer II decoder: the shipped CLI opens `.mp2` through the ES container lane, routes the `mp2` descriptor to `Mp2Decoder` exactly as the doc claims, the symphonia engine is on by default (no feature-gating trap), and 25 unit tests plus an ffmpeg differential (whole-decode PCM MD5s) are green in the committed baseline. The doc's "honest overlap note" (the family `Mp3Decoder` also accepts `mp1`/`mp2`/`mp3`) is accurate — the provider routes `mp1` to the family decoder and `mp2` here. The one defect is inherited-by-reference: mod.rs points at the fuzz module's wiring status as documented "honestly", but that status is stale — the "proposed" libFuzzer target and scaffolding both exist in the tree (same staleness class I proved on aac/fuzz.rs).

## 2. What actually works (verified)
— verified by audit/runs/baseline/test-workspace.log.txt:383-407 → all 25 `audio::mp2::*` unit tests ok (descriptor gate exactly `mp2`, Layer III refusal, truncation typed, CBR seek arithmetic, bitrate tables, mode extension, padding geometry, mid-stream layer/bitrate switch refusal); :1134-1142 → `mp2_corpus_differential` 3 ok, incl. `corpus_and_fixture_differential_vs_ffmpeg` — the doc's bit-exactness posture is executed, not asserted.
— verified by read + hand-arithmetic: the single in-file test's constants are the real MPEG tables — `layer_ii_bitrate(V1, 14) = 384`, `LAYER_II_BITRATES_MPEG2_LSF[13] = 160`, `is_mp2_stream(&[0xFF,0xFD,0xE0,0x00])` (0xFFFD = sync 11111111 111, MPEG1, Layer II bits `10`, no CRC), `intensity_bound(Mono, _) = None`, `from_bits(2).bound_subband() = 12` (mode-extension `10` → subbands 12–31 intensity bound).
— verified by read: `"mp2"` arm in the shipped provider (pipeline_host.rs:266-270) + import from `codecs::audio::mp2` (:59) — the routing claim in the module docs is true of the shipped binary.

## 3. Findings

### F-001 [S4] — "wiring status documented there, honestly" points at a stale status doc
Where: crates/codecs/src/audio/mp2/mod.rs:32-33 (pointer) and crates/codecs/src/audio/mp2/fuzz.rs:20-24 (stale target)
Code:
```rust
//! - [`fuzz`] — the `fuzz_codec_mp2` harness (`+FB`; wiring status
//!   documented there, honestly).
```
What is wrong: The referenced "honest" status says the libFuzzer target is "proposed at fuzz/fuzz_targets/fuzz_codec_mp2.rs … until they land", but `fuzz/fuzz_targets/fuzz_codec_mp2.rs` exists in the tree today and the fuzz-registry gate is CLEAN ("35 targets, diff empty, corpora present" — audit/runs/baseline/full-gate.log.txt:46-48). Only the narrower claim (campaign not run in CI; stable-pinned toolchain) remains true.
Why it matters: mod.rs vouches for the honesty of a doc that has drifted; auditors following the pointer are told the wiring does not exist when it does. Identical staleness proven on aac/fuzz.rs:19-31 this wave (my audit of that file, F-001) — a copy-paste pattern across the codec fuzz modules.
What correct looks like: Update the fuzz modules' wiring-status sections (target + scaffold committed; campaign awaiting nightly wiring) — then mod.rs's pointer becomes true as written.
Confidence: HIGH — Verification: read (file listing + registry gate cite).

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-62 | REAL | S4 | format-slice, overlap, conventions, bit-exactness claims all verified or record-backed; one stale pointer (F-001) |
| module decls (`decoder`,`error`,`header`,`seek` pub(crate); `fuzz` pub) | 64-71 | REAL | — | fuzz is pub for the out-of-crate target/survival tests (documented) |
| re-exports (decoder/header/seek surfaces) | 73-79 | REAL | — | compile-pinned by the in-file test + baseline build |
| test `the_public_surface_reexports_resolve` | 85-104 | REAL | — | compile-level pin + five hand-verifiable value assertions (MPEG tables); honest comment that behavior lives in the submodule suites |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| the_public_surface_reexports_resolve | YES | a missing/renamed re-export fails compilation; each asserted value (384/160/12/None/true) is a real table constant — a corrupted table or gate flips it |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Evidence |
|---|---|---|---|
| "The R3 registration card routes the `mp2` descriptor HERE" | mod.rs:42-44 | REAL | pipeline_host.rs:59,266-270 |
| "no new dependency (the 5.A3 admission covers the whole family)" | mod.rs:15-19 | REAL | codec-mp2 = ["codec-mp3"] → symphonia-bundle-mp3 already in graph (codecs/Cargo.toml:77-78); record file exists (docs/dependencies/records/symphonia-mp3-0.6.1.md) |
| "Card 5.A3's Mp3Decoder accepts mp1/mp2/mp3 … family's multi-layer acceptance stays" | mod.rs:37-44 | REAL | provider routes `mp1` to Mp3Decoder (pipeline_host.rs:261-265) — both decoders shipped |
| "no panic on any input — every decode/parse/seek path is bounds-checked" | mod.rs:50-53 | REAL (submodule suites) | 25 unit tests + fuzz survival green (baseline :383-407, :1143+) — submodule internals owned by other agents |
| "differential … compares whole-decode PCM MD5s … (tests/mp2_corpus_differential.rs)" | mod.rs:55-62 | REAL | baseline :1134-1142 `corpus_and_fixture_differential_vs_ffmpeg ... ok` |
| "fuzz — wiring status documented there, honestly" | mod.rs:32-33 | FALSE (by reference) | target + scaffold exist; registry gate CLEAN (F-001) |

## 7. Dead / duplicated / unreachable code
None in this file. The deliberate Mp2Decoder/Mp3Decoder overlap is documented honestly and both are reachable (different descriptors).

## 8. Dependencies & flags
- External engine `symphonia-bundle-mp3` =0.6.1 (via `codec-mp2 = ["codec-mp3"]`) — **enabled by default**, so the shipped CLI build carries it (no "feature gates gating real code that shipped builds never enable" trap here — verified apps/cli consumes default features). Purity record cited in-tree. No unsafe in this file.

## 9. Open questions for the next agent
- mp2/{decoder,header,seek,fuzz,error}.rs owners: (a) the stale wiring-status text in fuzz.rs:20-24 (F-001 root); (b) verify the seek arithmetic's padding refine against a VBR-pretending stream (doc claims CBR-only geometry gate — the refusal path for a bitrate switch is tested, :396, good).
- Wave hostile-question note: MP2 (like AAC/FLAC/Vorbis/ALAC/Opus/MP3) is REACHABLE and functional in the shipped build via the ES/PS container lanes; only `decode-bench` (hard-coded MJPEG+PCM fall-through) and `engine-info` (hard-coded literal) misreport it.
