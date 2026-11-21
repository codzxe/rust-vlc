# AUDIT: crates/codecs/src/video/mpeg12/facts.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-11-21 |
| Lines / functions | 546 / 24 (16 pub fns, 1 const fn, 2 test helpers + BitWriter put/take, 6 tests) |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `Mpeg12Facts::scan`/`descriptor_gate`/`decode_refusal` have ZERO callers outside `crates/codecs/tests/mpeg12_corpus.rs` (integration test). The PS demuxer classifies MPEG-1/2 video with its own inline 64-byte window peek instead (crates/demux/src/ps/mod.rs:761–784). No probe/track-info/CLI path calls it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `sed -n` full read (1–546); `rg -n "Mpeg12Facts|descriptor_gate|mpeg12::facts"` over crates+apps (only self + mpeg12_corpus.rs test); `rg -ln "mpeg12" crates apps` (module map); `rg -n "0xB5|probe_video_codec"` in crates/demux/src/ps/mod.rs (the competing inline peek); `rg -n "horizontal_size_extension"` in headers.rs (shift-overflow check — fields are u32, safe); `rg -n "mpeg12|mpeg2video"` docs/feature-matrix.toml (row `codec-video-mpeg1-2`); `od -c` on attr lines (display artifact `#ust_use]` is actually `#[must_use]` — terminal ate the `[m`) |

## 1. One-paragraph verdict
The code is genuinely written and genuinely tested — the scan walk is total, damage-counted, first-header-wins, and the six tests assert real bit-level semantics against a hand-built BitWriter stream, not tautologies. But the entire surface is production-dead: nothing in the shipped `lumen` CLI (or anywhere outside the codecs crate's own test binary) calls `Mpeg12Facts::scan`, `descriptor_gate`, or the refusal helpers; the PS demuxer re-implements a weaker 64-byte classification peek instead of consuming this module. So "card 5.H1's deliverable" exists as a tested island. The feature-matrix row is honest about decode being blocked; nothing in the matrix claims this scan is wired into probe output — only the module's own doc comment gestures at "the facts probe/track-info report", which today is nobody's input.

## 2. What actually works (verified)
— `Mpeg12Facts::scan` totality and counters: verified by reading the walk (78–173) plus the file's own fuzz-lite test (508–542, 8000 xorshift rounds asserting `picture_count <= start_codes`, `mask <= 0b1111`); the tests ran in baseline `cargo test --workspace` (1681 passed, audit/runs/baseline/INDEX.md #3) — I cite, not re-run.
— First-header-wins + change detection: `sequence_count`/`sequence_changes` logic at 90–102 traced; test `sequence_change_is_flagged` (480–504) splices a real second header and asserts the first wins.
— MPEG-1 vs MPEG-2 classification: extension-id dispatch at 106–124 (`b >> 4`, ids 0b0001/0b1000) traced; `mpeg1_stream_classifies_as_mpeg1` (454–476) pins NTSC 30000/1001 from frame_rate_code 4.

## 3. Findings
### F-001 [S3] The whole facts surface is dead code: no production consumer; PS demuxer duplicates the classification with a weaker inline peek
Where: crates/codecs/src/video/mpeg12/facts.rs:1-5 (module doc) and crates/demux/src/ps/mod.rs:761-784 (the competing implementation)
Code (facts.rs doc):
```rust
//! The stream-level fact scan (card 5.H1's deliverable): walk an
//! MPEG-1/2 elementary-stream buffer start code by start code and
//! collect the sequence-layer facts probe/track-info report —
```
Code (the peek that actually ships, ps/mod.rs:775-779):
```rust
                if window[i..].starts_with(&[0x00, 0x00, 0x01, 0xB5]) {
                    return "mpeg2video";
                }
```
What is wrong: `rg` across `crates/` and `apps/` finds `Mpeg12Facts` referenced only by `crates/codecs/tests/mpeg12_corpus.rs:29`. No demuxer, no probe, no CLI host, no tauri path calls `scan`/`descriptor_gate`/`decode_refusal`/`accepts` (mpeg12's). The real MPEG-1/2 classification used by the shipped PS lane is `PsDemuxer::probe_video_codec`, a 64-byte prefix peek that cannot see a sequence extension beyond byte 64 and defaults to `"mpeg2video"` on any read error — strictly weaker than the full start-code walk this module implements (resolution, rate, profile, GOP, census all discarded).
Why it matters: 546 lines of parser + its fixture corpus are maintained and fuzz-tested but contribute nothing to any shipped behavior; the risk is drift — the shipped peek and the dead facts scan can disagree (e.g. extension past byte 64: peek says mpeg1video, scan says mpeg2video) and nobody notices because no test compares them.
What correct looks like: either wire `Mpeg12Facts::scan` into the PS/TS/ES hosts' track-fact harvest (replacing the inline peek), or delete the module and keep only the peek — one classifier, tested against both.
Confidence: HIGH — Verification: read + exhaustive `rg` (commands above); no cfg-gate hides the callers (module is unconditionally `pub mod mpeg12`, video/mod.rs:39).

### F-002 [S4] `slices_with_mb_address` census silently capped at 64 slices; asymmetric retry for all-broken streams
Where: crates/codecs/src/video/mpeg12/facts.rs:139-153
Code:
```rust
                StartCode::Slice(n) => {
                    facts.slice_count += 1;
                    if facts.slice.is_none() || facts.slice_count <= 64 {
                        match SliceHeader::parse(n, unit.payload) {
```
What is wrong: The `slices_with_mb_address` field doc (63–64: "Slice headers whose first macroblock VLC matched a modeled code") does not mention that only the first 64 slices are examined. Additionally, if every slice fails to parse (`facts.slice` stays `None`), the `|| facts.slice.is_none()` arm keeps parsing every subsequent slice to the end of the stream — unbounded re-attempt — while a stream with one valid early slice stops detailed parsing at 64. Work stays bounded by input size (one pass, no allocation), so no perf hazard; it is an undocumented, asymmetric census.
Why it matters: A consumer comparing `slices_with_mb_address` against `slice_count` on a long stream gets a silently wrong ratio after slice 64.
What correct looks like: Document the cap in the field doc, or count matches for all slices (the parse is cheap) — and drop the redundant `slice.is_none()` retry arm or document it.
Confidence: HIGH — Verification: read; branch logic traced for both cases.
### F-003 [S4] `descriptor_gate` doc claims "the single spelling the registry consumes" — no registry consumes it
Where: crates/codecs/src/video/mpeg12/facts.rs:283-289
Code:
```rust
/// The codec-api descriptor gate for demuxer wiring: true when the
/// descriptor is one of ours (re-exported from [`super::error`] as
/// the single spelling the registry consumes).
#[must_use]
pub fn descriptor_gate(descriptor: &str) -> bool {
    accepts(descriptor)
}
```
What is wrong: "for demuxer wiring" and "the registry consumes" assert a consumer that does not exist (F-001 evidence). The function's only callers are its own tests. Same for `Mpeg12Facts::decode_refusal` (260–263: "the codec-api gate") — the real `mpeg2video` refusal the CLI produces comes from the pipeline host's generic unsupported-decoder path, not from here (es.rs:655's local `decode_refusal` is an unrelated h264/hevc test helper).
Why it matters: Docs that name phantom consumers are how later waves "verify" wiring that isn't there.
What correct looks like: "Not yet wired; the intended consumer is the demuxer/registry once the 5.H1 egress lands."
Confidence: HIGH — Verification: read + `rg` (no callers).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Mpeg12Facts::scan` | 78–173 | REAL (unreachable) | — | total walk; first-wins; damage counted not fatal; extension-id dispatch `b>>4` ids 1/8; empty payload → id 0 → ignored, no panic |
| `dimensions` | 165–179 | REAL (unreachable) | — | u32 fields, `ext << 12` cannot overflow (headers.rs:133,260); 12+2-bit composition correct per ISO 13818-2 |
| `frame_rate` | 182–191 | REAL (unreachable) | — | delegates to headers' ext-refined table |
| `frame_rate_string` | 195–197 | REAL (unreachable) | — | `format!("{num}/{den}")` — matches ffprobe r_frame_rate shape |
| `is_mpeg2` | 202–204 | REAL (unreachable) | — | trivial |
| `profile` | 208–212 | REAL (unreachable) | — | None for escape bit — honest |
| `level` | 216–219 | REAL (unreachable) | — | delegates to `level_value` |
| `chroma` | 224–229 | REAL (unreachable) | — | MPEG-1 → 4:2:0 default per definition |
| `interlaced` | 234–237 | REAL (unreachable) | — | `!progressive_sequence` only when ext present |
| `has_motion_vectors` | 242–245 | REAL (unreachable) | — | mask bits 1–2 (P/B); bit 0 (I) correctly excluded |
| `descriptor` | 249–256 | REAL (unreachable) | — | mpeg2 iff ext seen |
| `decode_refusal` | 260–263 | REAL (unreachable) | S4 | phantom "codec-api gate" consumer (F-003) |
| `is_video_stream` | 267–269 | REAL (unreachable) | — | trivial |
| `kind_as_index` | 273–283 | REAL | — | const fn; shift by 0..=3 on u8 mask — safe |
| `descriptor_gate` | 287–289 | REAL (unreachable) | S4 | F-003 |
| `tiny_mpeg2_stream` (test helper) | 296–339 | REAL | — | hand-built honest ES: seq header + ext + GOP + I + slice + end |
| `BitWriter::put`/`take` (test helper) | 361–386 | REAL | — | MSB-first; `take` pads the tail byte — correct |
| tests: `tiny_stream_facts`, `damage_is_counted_not_fatal`, `empty_and_garbage_inputs_are_total`, `mpeg1_stream_classifies_as_mpeg1`, `sequence_change_is_flagged`, `fuzz_lite_scan_is_total` | 388–542 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `tiny_stream_facts` (389) | YES | 20+ concrete assertions (352×288, 25/1, "Main", level 8, mask 0b0001, GOP "00:00:00:00", refusal kind Unsupported) — a mutation in any walk arm flips one |
| `damage_is_counted_not_fatal` (424) | YES | flips real payload bytes; asserts walk continues (`picture_count==1`, `slice_count==1`, `bad_headers>=1`) — fails if a parse error aborted the scan |
| `empty_and_garbage_inputs_are_total` (440) | YES for the == 0 asserts | pins that garbage/zeros/truncated-prefix yield `start_codes == 0` — fails if the scanner hallucinates codes |
| `mpeg1_stream_classifies_as_mpeg1` (454) | YES | pins extension-absence → mpeg1 + NTSC 30000/1001 |
| `sequence_change_is_flagged` (480) | YES | second differing header: count 2, change flag, first wins dimensions |
| `fuzz_lite_scan_is_total` (508) | PARTIAL | asserts invariants (monotone counters, mask ≤ 0b1111) — would catch panics/hangs, but NOT wrong values (no oracle); acceptable for a totality test |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "sequence-layer facts land, decode typed-refused … 10/10 differential, 380/43 gates, 8k fuzz rounds" | feature-matrix.toml row `codec-video-mpeg1-2` (line 523–524, status all-platforms `blocked`) | REAL for the code's existence/tests (baseline suite green, INDEX.md #3); the decode-blocked status is honest (mod.rs:9 "There is **no software MPEG-1/2 decode here**") | file read + baseline cite |
| "collect the sequence-layer facts probe/track-info report" | facts.rs:1–3 module doc | FALSE as wiring | no probe/track-info consumer exists (F-001 `rg` evidence) |
| "the single spelling the registry consumes" | facts.rs:284–285 | FALSE | no registry caller (F-003) |

## 7. Dead / duplicated / unreachable code
- The entire public surface is unreachable from shipped entry points (F-001) — verdict UNREACHABLE, code kept honest by its own tests.
- Duplication: `probe_video_codec` (ps/mod.rs:761–784) vs this module's scan — two classifiers for the same question, one shipping, one dead (F-001).

## 8. Dependencies & flags
- No feature gates: mpeg12 is an unconditional in-house module (crates/codecs/Cargo.toml:26–27 "The IN-HOUSE decoders (mjpeg, pcm, adpcm, h264, hevc, mpeg12 …) stay unconditional"; video/mod.rs:39 `pub mod mpeg12`). Compiles in every shipped build — dead weight in the binary unless LTO strips it (unverified).
- Wave-6 hostile question: this file cannot affect H.264 reachability; for the answer see mjpeg/decoder.rs#part2.audit.md §8 (H.264 decode IS reachable: apps/cli/Cargo.toml:51 default features → codecs/Cargo.toml:56,73 → h264/mod.rs:55,61 → pipeline_host.rs:346; baseline sample-420.mp4 plays, INDEX.md CORRECTION v2).

## 9. Open questions for the next agent
- mpeg12_corpus.rs owner: do the fixture lanes exercise `descriptor_gate`/`decode_refusal` against the real pipeline, or only the facts API in isolation?
- PS/TS demuxer owners: `probe_video_codec` defaults to `"mpeg2video"` on read error (ps/mod.rs:771) — is that default surfaced to users as a fact they can trust? (Classification-by-default, not detection.)
- headers.rs owner (W06 other agent): confirm `frame_rate_with_ext`'s frame-rate-extension table (the 30000/1001 path) — my file trusts it.
