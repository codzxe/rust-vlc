# AUDIT: crates/codecs/src/audio/mp2/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-21 |
| Lines / functions | 187 / 9 (enum + 3 impls + 2 constructors + Detail carrier + 2 tests) |
| File verdict | REAL (correct, tested, consumed by the reachable MP2 decoder; one architecture-level duplication finding) |
| Reachable from shipped entry point? | YES — call chain: `lumen open` (MP2 lane: MPEG-TS/MP2-in-WAV etc.) → pipeline_host.rs:266-270 `"mp2"` arm → `Mp2Decoder::new` → decoder.rs:36 `use super::error::{Mp2Error, corrupt, unsupported}` — every decode/configure failure of an MP2 stream flows through this taxonomy. Tests green in baseline. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read error.rs 1-187 · `rg -n "Mp2Error\|unsupported\(\|corrupt\(" crates/codecs/src/audio/mp2` (consumers: decoder.rs 12+ sites, header.rs 3 sites — all variants used, none dead) · `rg -n "mp2" audit/runs/baseline/test-workspace.log.txt` (error tests green at :390-391; 8 mp2 decoder tests + 3 header/fuzz green :383-394) · `diff` of identifier-renamed mp2/error.rs vs mp3/error.rs (67 diff lines) and vs aac/error.rs (84 diff lines) — quantifies the 8-way duplication · `rg -n "unwrap\|expect\|panic"` on the file (production code clean; one `unwrap_or_default` in test code) |

## 1. One-paragraph verdict
A small, honest, fully-tested error taxonomy: three variants (`Unsupported`/`Corrupt`/`BadState`) mapping one-to-one onto `ErrorKind::{Unsupported, Corrupt, Decoder}` at stage `Decode`, exactly as the doc table promises (I cross-checked table vs code vs the asserted Display strings); stable dotted codes are `&'static str` (compile-time, no dynamic code injection); the human-readable reason rides in `source` behind a private `Detail` carrier, keeping `Display`'s surface secrets-free — and the test pins the rendered string, the stage, AND the source text for all three variants, so any kind/code/format mutation fails. The "no panics, every constructor total, no unwrap" claim is verified by read (the single `unwrap_or_default` is in the test module). All three variants have real consumers in decoder.rs/header.rs — nothing dead. The one finding is architecture: this file is one of EIGHT structurally near-identical per-codec error taxonomies (~1,534 lines total across `audio/*/error.rs`; after identifier renaming, mp2-vs-mp3 differ by 67 diff lines, mp2-vs-aac by 84 — ~85% shared skeleton), a deliberate, self-documented pattern ("the 5.A3 MP3 pattern verbatim") whose per-copy doc tables and `Detail` carriers must now be maintained in lockstep by hand.

## 2. What actually works (verified)
— Mapping table (29-33) cross-checked against `into_engine_error` (62-94) arm by arm: Unsupported→`ErrorKind::Unsupported{format:"mp2"}`, Corrupt→`ErrorKind::Corrupt{detail}`, BadState→`ErrorKind::Decoder{BadState}` with code `mp2.not_configured`, all at `PipelineStage::Decode` — exact match.
— Baseline `test-workspace.log.txt:390-391` — `error::tests::{bad_state_is_a_decoder_kind_not_corrupt, taxonomy_maps_each_variant_to_its_public_kind}` green; the wider mp2 suite (decoder 6, fuzz 2+, header 1) green at :383-394.
— Display discipline: `Detail` is private, rides only in `source` — the taxonomy test asserts `"[decode] unsupported in mp2 (mp2.codec_descriptor)"` as the full Display output with the reason only in `source("…")` — the no-secrets pattern is enforced by test, not just claimed.
— Rendered-string consistency with the engine's event vocabulary: the asserted `[decode] corrupt structure at byte 1 (mp2.header.not_layer_ii)` matches the family the baseline smoke shows for h264 (`error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)`) — the taxonomy feeds the same user-facing pipeline.
— Variant completeness: `rg` shows decoder.rs uses all of `corrupt` (82,98,137,150,197,436), `unsupported` (174,283,292,307,373), `Mp2Error::BadState` (332,336,404); header.rs uses `corrupt` (157,192,271) — no dead variants, no unconsumed arms.

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/audio/mp2/error.rs:1-2 (self-description), vs the 7 sibling files crates/codecs/src/audio/{aac,alac,flac,mp3,opus,pcm,vorbis}/error.rs
Code:
```
//! Decode-stage error taxonomy for MP2 (card 5.J3, the 5.A3 MP3
//! pattern verbatim): typed, stage-tagged internal errors that map
```
What is wrong: eight per-codec copies of the same ~130-line skeleton (enum + into_engine_error + From + private Detail carrier + two constructors + the same two tests), 1,534 lines total. Measured: after renaming `Mp2→X`/`mp2→x`, mp2-vs-mp3 diff is 67 lines and mp2-vs-aac is 84 lines — the remainder is byte-identical structure including the private `Detail` struct and both test shapes. The pattern is deliberate and self-documented, but every future codec clones it again, and each copy's doc mapping table and Display-discipline tests must be kept equivalent by hand — the exact drift class this wave's other audits keep finding (e.g., mix.rs F-002: a doc taxonomy that drifted from its code).
Why it matters: maintenance cost and drift risk, not a runtime defect; a fix to the mapping pattern (e.g., a `Detail` leak, a kind change) must be applied eight times, and nothing fails when copy #9 diverges.
What correct looks like: a generic taxonomy in `codec-api` (`CodecError<CODE>` or a macro generating the per-codec wrapper) with per-codec code constants — the per-codec *vocabulary* (codes, detail texts) stays local, the skeleton collapses.
Confidence: HIGH — Verification: diff measurements above (commands in header); read of this file.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-20 | REAL | — | taxonomy description matches code; "no panics/total constructors/no unwrap" verified by read; the per-frame-isolation claim (14-17) is decoder.rs's behavior — REAL-UNPROVEN from this file (mp2 fuzz survival tests green in baseline support it; belongs to the decoder.rs auditor) |
| `Mp2Error` enum | 34-57 | REAL | — | 3 variants, all consumed; `&'static str` codes |
| `into_engine_error` | 62-94 | REAL | — | mapping verified against doc table; `FormatInfo` name "mp2" allocates per error — cold path, fine |
| `From<Mp2Error>` | 97-101 | REAL | — | delegating impl ✓ |
| `Detail` + Display + Error | 106-115 | REAL | F-001 | private carrier; duplicated ×8 |
| `unsupported` | 118-123 | REAL | — | total constructor |
| `corrupt` | 126-136 | REAL | — | total constructor |
| `taxonomy_maps_each_variant_to_its_public_kind` | 142-174 | REAL | — | exact Display + stage + source asserts ×3 variants — discriminating |
| `bad_state_is_a_decoder_kind_not_corrupt` | 176-186 | REAL | — | kind pin (Decoder ≠ Corrupt) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | exact rendered strings ("[decode] unsupported in mp2 (…)" etc.), stage, and source text per variant; mutation: swap `ErrorKind::Unsupported`→`Corrupt` changes the rendered string and fails; mutation: move detail into Display fails the source-text assert |
| bad_state_is_a_decoder_kind_not_corrupt | YES | `matches!` kind assert — a Corrupt misclassification fails |
| Both executed green in baseline (:390-391) | — | |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "typed, stage-tagged internal errors that map one-to-one onto EngineError kinds" | module docs 1-3 | REAL | table-vs-code cross-check + tests |
| "Nothing panics; every constructor is total; no `unwrap` on any input path" | module docs 19-20 | REAL | read: no unwrap/expect/panic in production lines (single test-side `unwrap_or_default`) |
| "Per-frame isolation … a damaged packet never poisons the next — the walker resyncs" | module docs 14-17 | REAL-UNPROVEN (this file) | implemented in decoder.rs/header.rs (other agents); baseline mp2 fuzz tests (`a_layer_iii_frame_is_refused_not_decoded`, `a_header_only_buffer_is_survived`) green |
| "an unexpected `extra_data` facts blob (none is defined for Layer II — there is no Xing/LAME construction)" | module docs 8-10 | REAL | decoder.rs configure refuses extra_data (test `configure_rejects_foreign_descriptors_and_unexpected_facts` green, :387); Xing/LAME is genuinely MP3-only |

## 7. Dead / duplicated / unreachable code
- No dead variants, no unreachable functions in this file (consumer sites enumerated in §2).
- Duplication: the 8-way skeleton (F-001) — `Detail` alone exists in 8 private copies.

## 8. Dependencies & flags
- `media-types` only (EngineError/ErrorKind/CorruptDetail/ErrorContext/FormatInfo/PipelineStage); no external deps; no feature gate (in-crate `pub(crate)` behind the mp2 module); no `unsafe`.
- `codec-mp2` feature gates the mp2 module's engine deps — error.rs compiles with the module; default features include it (codecs/Cargo.toml).

## 9. Open questions for the next agent
- The other seven `error.rs` owners (a01: opus/error; a03: flac/error; a06: vorbis/error; a08: aac/error, mp3/error; a09: pcm/error; a10: alac/error): F-001's duplication count includes your files — a wave-level consolidation proposal needs one owner, not eight local rewrites.
- decoder.rs (unassigned in wave-7 lists I can see — mp2/decoder.rs 678 lines, mp2/seek.rs 223 lines, mp2/fuzz.rs 190 lines are absent from all w07-a0X tsvs): confirm someone owns them; the isolation/resync claims of this file's docs live there.
