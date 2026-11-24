# CLAIM AUDIT: tools/vta/ruopus/README.md
| Field | Value |
|---|---|
| Auditor | W17-A10 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL 7 / REAL-UNPROVEN 0 / SHALLOW 1 / STUB 0 / PAPER-ONLY 0 / FALSE 1 / UNREACHABLE 0 |
| Internal contradictions found | 1 (README = record = code all agree on the WRONG gain offset vs RFC 7845) |

## 1. Blunt verdict
An honest README for a real spike harness: every pipeline claim traces to code in tools/vta/ruopus/src/main.rs and to the admission record, and the two honesty notes (no end-trim, gain not applied) are true. Two defects: the Run block is a dead path today (corpus file gone), and the "output gain field is reported" claim is false in mechanism — the harness (and the README, and the record) all point at OpusHead[14..16], which is the tail of the input-sample-rate field, not the gain field ([16..18] per RFC 7845 §3.1.1).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Standalone (non-workspace) cargo bin" | README:3 | REAL | tools/vta/ruopus/Cargo.toml:2-4 `[workspace]` (empty) + "NOT a workspace member"; confirmed by W17-A04 main.rs audit |
| 2 | "decodes an Ogg/Opus file through OUR ogg demuxer (crates/demux/src/ogg) into ruopus" | README:3-4 | REAL | main.rs:3-4 `use plugin_api::Demuxer; use ruopus::{MultistreamDecoder, OpusDecoder};` :33 `demux::ogg::OggDemuxer::new(stream)`; grep test_support -> crates/demux/src/avi/mod.rs:121 |
| 3 | "writes raw f32le 48 kHz PCM" | README:4-5 | REAL | main.rs:103 `f.to_le_bytes()`; :78 `MultistreamDecoder::with_rate(48_000, ...)`; record §D "at 48 kHz" (ruopus-0.1.2.md:97) |
| 4 | "Evidence lives in docs/dependencies/records/ruopus-0.1.2.md" | README:6-7 | REAL | file exists, 227 lines; audited by W17-A06: REAL 5 / REAL-UNPROVEN 2, 0 contradictions |
| 5 | Run block: "cargo run --release -- /tmp/my-project/corpus-v4/media/0047-audio-only.ogg --out /tmp/ru.pcm" | README:12-15 | SHALLOW | `ls /tmp/my-project/corpus-v4/media/0047-audio-only.ogg` -> GONE; no cargo in sandbox; historical run recorded in record §D (0047 ff e918a5f757bd3ab2dbfc49ce3fff86fa, record:131) |
| 6 | "--trim-per-ch N cuts the output to N samples/channel ... harness does NOT apply end-trimming; see record §D" | README:20-22 | REAL | main.rs:96 `trim_per_ch.map(|n| (n * channels).min(after_pre.len()))`; record §D "the harness does NOT trim the tail to the last granulepos" (:101-102) |
| 7 | "hostile-input contract check (no-panic requirement)" | README:16-17 | REAL | main.rs:116-135 hostile_suite: 6 cases, catch_unwind, `exit(4)` on panic (logic read; not executed — no cargo) |
| 8 | Pipeline: "mem_stream → OggDemuxer::new → start() → pump next_event → header_packets → OpusDecoder/MultistreamDecoder → pre-skip" | README:24-30 | REAL | main.rs:32,38,42,51-56,71-79,92-94 — every named step present in order; record §D describes the identical pipeline (:92-97) |
| 9 | "output gain field is reported but NOT applied (both corpus and generated lanes measured gain = 0)" | README:31-32 | FALSE (mechanism) / moot | main.rs:64 `i16::from_le_bytes([head[14], head[15]])` — RFC 7845 §3.1.1 puts output gain at bytes 16-17 (after magic); [14..16] is the tail of the input-sample-rate field. W17-A04 F-001 (main.rs:63-65, "OpusHead output-gain read at the wrong offset"). NOT applied: TRUE (no gain application in main.rs). The "gain = 0" measurements were taken from the wrong bytes (UNVERIFIED whether true gain was 0 — corpus gone) |
| 10 | "ruopus emits RFC 7845 Vorbis order (5.1: FL FC FR BL BR LFE) ... record's comparison applies that permutation on the reference side" | README:34-36 | REAL | record:116-118 "RFC 7845 Vorbis order (5.1: FL FC FR BL BR LFE); ffmpeg's native decoder ... apply that permutation to the reference before comparing" — README faithfully mirrors the record; permutation is comparison-side, harness untouched |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| gain is "OpusHead[14..16]" (README:31-32 implicitly; record:98 explicitly) | RFC 7845 §3.1.1: output gain is bytes 16-17 of OpusHead | The RFC. main.rs:64 reads [14],[15] — W17-A04 proved the offset wrong; README and record inherited the same wrong offset, so they agree with each other and disagree with the spec |

## 4. Fabrication / inflation indicators
None. The README advertises its own limitations (no end-trim, gain not applied, SPIKE in the title). The one wrong claim (#9) is a propagated byte-offset mistake shared by code+record+README, not inflation.

## 5. Matrix rows
Not a matrix chunk — no rows.

## 6. Open questions
- Was true output gain ever 0 on the corpus lanes? The number measured came from the sample-rate field's tail bytes; with input rate 480000 the misread yields 7 (0.027 dB), with input rate 0 it yields 0. Corpus file gone — UNVERIFIED.
