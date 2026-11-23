# AUDIT: crates/codecs/tests/vorbis_bitexact.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Lines / functions | 409 / 4 helpers + 5 tests (support module tests/vorbis/mod.rs, 391 loc, verified for the oracle functions) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test: NO. Decoder under test (`codecs::audio::vorbis::VorbisDecoder`): YES — pipeline_host.rs:271 `"vorbis" =>` arm in the shipped CLI. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/vorbis_bitexact-e8223d5cf8b5f437 --nocapture` → **5 passed / 0 failed** (0.53s), with ffmpeg ON PATH so the live differential ran: DIFF-ROWs show `max_delta=3.7252903e-8 (bound 1e-7)`, `4.4703484e-8`, `2.9802322e-8`, and the silence lane **bit-exact** (`lumen_window_md5=9f3a19b1740ac999088756035046c146 ffmpeg_md5_f32le=9f3a19b1...` identical, `max_delta=0e0`); corruption row: `1 typed refusal at packet 20, 65 blocks emitted`; HEADTRIM rows: `emission 66150 == granule (delay 128 trimmed)`, `9600 == granule (delay 256)`. |

## 1. One-paragraph verdict
A real bit-exact/differential suite doing lossy-codec comparison the honest way: exact granule-timeline geometry against ffmpeg-recorded values, an aligned-window MD5 hard pin, and a LIVE ffmpeg f32le re-diff per fixture with a recorded per-sample bound — including a digital-silence corridor asserted strictly bit-exact, which my run confirms (identical MD5s, delta 0). The oracle chain even re-verifies the oracle itself (`ffmpeg digest drift` asserts ffmpeg's live digest equals the recorded one), pinning both fixture bytes and oracle version. The mid-stream corruption AC asserts exactly one typed refusal with resumed decoding, and the head-trim regression pins emission == granule with the encoder delay removed. Only blemish: the corpus-0085 row skips vacuously when the /tmp corpus is absent (the default), honestly named `_or_skip` and leaving 4 of 5 tests fully live.

## 2. What actually works (verified)
- 5/5 tests pass with the live oracle engaged (output pasted above).
- Live per-sample max |delta| within the recorded bounds on all three sine lanes; silence corridor byte-identical to ffmpeg.
- `mid_stream_corruption_is_typed_and_recovery_scoped`: exactly 1 typed `vorbis.packet.decode` refusal at the poisoned packet, 65 blocks emitted around it, flush drains — the recovery contract executes.
- `stream_head_encoder_delay_is_trimmed_decoder_side`: multi-page lanes emit exactly `granule` samples with first Present pts 0; single-page lanes pinned to the un-trimmed shape (both directions asserted).

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/vorbis_bitexact.rs:199-216
Code:
```rust
    let Some(root) = corpus_dir() else {
        eprintln!("corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP");
        return;
    };
```
What is wrong: the corpus-0085 row reports `ok` without testing anything when the /tmp-resident corpus is absent (the default state here) — same vacuous-pass pattern as the h264 corpus suite, though honestly named (`_or_skip`) and limited to 1 of 5 tests. Confidence: HIGH — Verification: ran (SKIP line in my output).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| usiz / opt_usiz / f32_of | 40-61 | REAL | — | reference-section accessors with loud failures |
| assert_row | 65-164 | REAL | — | geometry + window MD5 pin + live bounded delta + oracle-digest-drift check; executed on 4 fixtures |
| fixtures_are_pinned_against_the_ffmpeg_reference (test) | 166-175 | REAL | — | live run verified |
| tagged_fixture_exposes_its_vorbis_comments (test) | 177-196 | REAL | — | vendor + 5 fields vs ffmpeg-written values; case-insensitive + unknown-field surface |
| corpus_0085_differential_or_skip (test) | 198-216 | REAL | S4 | F-001 vacuous skip by default |
| mid_stream_corruption_is_typed_and_recovery_scoped (test) | 218-329 | REAL | — | 0xFF blast at packet 20; exact counts asserted; honest comment that single flipped bytes can still decode |
| stream_head_encoder_delay_is_trimmed_decoder_side (test) | 338-408 | REAL | — | both multi-page (trimmed) and single-page (un-trimmed pinned shape) directions |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixtures_are_pinned_against_the_ffmpeg_reference | YES | geometry counts, MD5 pins, live bounded delta, oracle-digest drift — any decode regression moves the MD5 or the delta |
| tagged_fixture_exposes_its_vorbis_comments | YES | exact vendor/title/artist/album/date strings |
| mid_stream_corruption... | YES | exact refusal count, code string, emission floor, flush drain |
| stream_head_encoder_delay... | YES | exact emission lengths, pts ladder invariants |
| corpus_0085_differential_or_skip | N/A when corpus absent (skip) / YES when present (same assert_row machinery) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "pinned against tests/data/vorbis_reference.txt (the ffmpeg 7.1.5 oracle) and ... re-diffed **live**" | header L4-6 | REAL | my run: live DIFF-ROWs with ffmpeg |
| "the digital-silence corridor must be **bit-exact**" | header L12-15 | REAL | silence lane md5s identical, delta 0 |
| "canonicalized-f32 MD5" (-0.0 → 0.0) | vorbis/mod.rs:332-341 | REAL | canonicalization read; only affects ±0 representation |
| reference provenance (recorded commands, why the two digests differ) | tests/data/vorbis_reference.txt:1-30 | REAL | header read; live ffmpeg digest equals the recorded one (the drift assert passed) |

## 7. Dead / duplicated / unreachable code
None in this file.

## 8. Dependencies & flags
Workspace crates + external ffmpeg subprocess (probed). Support module tests/vorbis/mod.rs (ffmpeg_decode_f32le is a real `ffmpeg ... -c:a pcm_f32le -f f32le -` subprocess — verified by reading and by the live run). Env knob LUMEN_CORPUS. No feature gates.

## 9. Open questions for the next agent
- tests/vorbis/mod.rs (391 loc) is shared with other vorbis tests and not in my assignment — its CaptureSink/pts semantics back the head-trim assertions; worth its own pass.
- The 1e-7 bound's provenance (why 1e-7 suffices for q4/q2 sine lanes) is recorded only in the reference header; a pathological fixture could need a looser bound — not a defect, an observation.
