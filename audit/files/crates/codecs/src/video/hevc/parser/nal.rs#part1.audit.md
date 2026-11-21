# AUDIT: crates/codecs/src/video/hevc/parser/nal.rs#part1 (lines 1–758)
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-11-21 |
| Lines / functions | 758 / 20 (error type + Display/Error impls, 26-variant NAL-type enum + 4 methods, NAL-header split, Annex-B splitter, EPB removal ×2, length-prefixed splitter, hvcC record parse + heuristic + 2 readers, BitReader head) — chunk of a 1517-line file, equal 2-way split: part1=1–758, part2=759–1517 (BitReader methods + tests) |
| File verdict | UNREACHABLE (real, spec-correct, fuzz-covered code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — the only production consumer is `crates/codecs/src/video/hevc/decode.rs:78-80` (`HevcDecoder`, the hw-only lane), and `HevcDecoder::new/with_bridges` is constructed ONLY in `crates/codecs/tests/hevc_hw_path.rs:331,349,365,376` (tests). The shipped `lumen` binary has no HEVC decoder: baseline `lumen open sample-hevc.mp4` → `Opening → Error → Idle (no HEVC decoder)` (audit/runs/baseline/INDEX.md corrected smoke matrix). Demux/probe do not use this parser (crates/demux/src/mp4/mod.rs:441 maps `hvc1/hev1`→`"hevc"` by fourcc with the comment at :86 "avcC/hvcC … parse belongs to the codec cards"; media-probe has its own ES sniffer, signature/es.rs:221-241). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n` structure map (30 items); `sed -n '1,760p'` full read; `rg "hevc::parser|HvcCConfig|AnnexBNals|LengthPrefixedNals"` across crates+apps (callers: hevc/decode.rs:78-80 only); `rg "HevcDecoder"` across apps+crates (tests only); `rg "hvcC|hevc"` in crates/demux/src/mp4/mod.rs + media-probe/src/signature/es.rs (independent implementations); `rg -A40 "pub fn fuzz_codec_hevc"` (fuzz covers annexb_nals/HvcCConfig/LengthPrefixedNals) |

## 1. One-paragraph verdict
This is the cleanest file I have audited in this wave: every function is spec-checked (I verified the two-byte NAL header bit layout, the Annex-B 3/4-byte start-code + trailing-zero semantics, the overlapping-escape EPB removal, the five hvcC reserved patterns and every field extraction against ISO/IEC 14496-15 §5.3.3.1 / ITU-T H.265 §7.3.1/§7.4.2.1), every failure is typed, every index is bounds-checked, and the surfaces are actually fuzzed (`fuzz_codec_hevc` calls all three context-free entry points — unlike the H.264 harness). The defects are structural, not technical: none of it is reachable from a shipped entry point (no HEVC decoder exists in the shipped binary; the hw-lane consumer is test-only), one constructor doc is a copy-paste glitch, one field doc misstates the spec, and the length-prefixed splitter's padding rule is asymmetric on an edge.

## 2. What actually works (verified)
— `NalHeader::split` (267–295): bit layout `forbidden(1)|type(6)|layer(1+5)|tid+1(3)` extracted exactly per §7.3.1; `forbidden_zero_bit` and `nuh_temporal_id_plus1==0` typed-refused per §7.4.1. — verified by read + spec comparison.
— `AnnexBNals` (346–388): first start code skips leading garbage; 4-byte start codes handled via `end_raw = next-3` + `trim_trailing_zeros` (the extra `zero_byte` lands in the previous body's tail and is trimmed); adjacent start codes yield empty bodies that fail header parsing typed (documented). O(n) total scan.
— `unescape_rbsp_into` (410–446): suffix re-scan handles overlapping escapes (`00 00 03 00 00 03 00`); dangling `00 00 03` at EOS and suffix > 0x03 are typed errors per §7.4.2.1.
— `HvcCConfig::parse` (610–688): all five reserved patterns (`0x0f`,`0x3f`,`0x3f`,`0x1f`,`0x1f`) and all 17 field extractions verified against ISO/IEC 14496-15 §5.3.3.1 byte-for-byte; arrays/NAL counts bounds-checked with typed truncation.
— Fuzz coverage is real: `fuzz_codec_hevc` (hevc/fuzz.rs:45-66) drives `annexb_nals`, `HvcCConfig::parse`, and `LengthPrefixedNals` at length sizes 1/2/4 over arbitrary bytes (registry-checked, baseline #18 "35 targets, diff empty").

## 3. Findings
### F-001 [S3] The entire NAL layer is unreachable from shipped builds — parallel in-house parser the shipped path never loads
Where: crates/codecs/src/video/hevc/parser/nal.rs:1-758 (the surface) vs crates/codecs/tests/hevc_hw_path.rs:331-376 (the only constructions)
Code (the only production import, hevc/decode.rs:78-80):
```rust
use super::parser::nal::{
    AnnexBNals, HevcError, HvcCConfig, LengthPrefixedNals, NalHeader, NalUnitType,
};
```
What is wrong: `HevcDecoder` (the sole consumer) is a hardware-only lane whose constructor appears nowhere outside `hevc_hw_path.rs` tests; apps/cli, apps/desktop, apps/desktop-tauri never reference it. The shipped binary's HEVC story is: demux maps `hvc1/hev1` to a descriptor (demux/mp4/mod.rs:441), the open path finds no decoder, the session errors — `sample-hevc.mp4 … Opening → Error → Idle` (baseline INDEX.md). So 758 lines of fuzz-tested parser (plus the sps/pps/vps/slice siblings other agents hold) are compiled into every shipped binary and execute never.
Why it matters: Dead-but-maintained parsers drift from the demux-side reality (demux/probe each carry their own HEVC byte-walking code); the hvcC knowledge now lives in three places with one wiring.
What correct looks like: Either wire `HevcDecoder` into the pipeline host's descriptor table (giving users the typed hw-required refusal path) or move the parser behind the hw feature gate until Phase 7.
Confidence: HIGH — Verification: read + exhaustive `rg` (commands above); baseline smoke cited.
### F-002 [S4] `BitReader::new` doc comment is a copy-paste glitch describing a different function
Where: crates/codecs/src/video/hevc/parser/nal.rs:745-749
Code:
```rust
impl<'a> BitReader<'a> {
    /// Read `bits` (1..=32) from `data` starting at bit 0.
    #[must_use]
    pub fn new(data: &'a [u8]) -> BitReader<'a> {
```
What is wrong: The doc says "Read `bits` (1..=32) …" — that is the contract of the `bits()` method (part2's range), not of this constructor, which takes no `bits` parameter at all.
Why it matters: API docs that describe a different signature mislead every caller and every doc-generating pipeline.
What correct looks like: `/// A reader over `data` starting at bit 0; all reads bounds-checked and typed.`
Confidence: HIGH — Verification: read.
### F-003 [S4] `avg_frame_rate` doc misstates the spec: the 256-fps unit is unconditional
Where: crates/codecs/src/video/hevc/parser/nal.rs:577-580
Code:
```rust
    /// `avgFrameRate` (16 bits, in units of 256 fps when
    /// `constant_frame_rate` is set — raw value retained).
    pub avg_frame_rate: u16,
```
What is wrong: ISO/IEC 14496-15 defines `avgFrameRate` in units of 256 fps unconditionally (0 = unspecified); `constantFrameRate` is an independent 2-bit flag about frame-rate constancy, not a gate on the unit.
Why it matters: A future consumer dividing by 256 only when the flag is set would compute wrong frame rates for VFR-flagged records.
What correct looks like: `/// avgFrameRate (16 bits, in units of 256 fps; 0 = unspecified — raw value retained).`
Confidence: HIGH — Verification: read + spec text recalled from ISO/IEC 14496-15 §5.3.3.1 (no copy in sandbox to quote — spec cited from parser's own § references).
### F-004 [S4] Length-prefixed splitter: exactly-`length_size` zero tail is "damage" while shorter zero tails are "padding", on a premise MP4 does not guarantee
Where: crates/codecs/src/video/hevc/parser/nal.rs:458-462 and 496-513
Code:
```rust
/// Trailing padding (fewer than `length_size` bytes left, all zero)
/// ends the iteration, as MP4 samples are zero-padded to sample
/// boundaries; anything else that runs past the data is the typed
```
What is wrong: A tail of exactly `length_size` zero bytes parses as a length prefix of 0 → `Invalid { "hevc.av.zero_length" }` (512-516) — an error — while a shorter all-zero tail silently ends iteration (502-507). The behavior split is arbitrary at the boundary, and the justifying premise ("MP4 samples are zero-padded to sample boundaries") is not a general MP4 property — sample sizes are exact; padding is a writer quirk. A zero-padded sample whose padding happens to be exactly 4 bytes (length_size=4) would be reported as damage.
Why it matters: Edge-asymmetric rules on a documented-but-wrong premise produce false "corrupt" verdicts on the exact inputs the rule was written to tolerate.
What correct looks like: Treat any all-zero tail ≤ `length_size` as padding (or drop the padding concept and require exact lengths), and fix the doc to say "some writers zero-pad".
Confidence: MEDIUM — Verification: read; no MP4-padding corpus lane examined to prove a real-world hit (would need a padded-sample fixture — UNVERIFIED impact).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HevcError` enum + `Display` + `Error` | 36–100 | REAL (unreachable) | — | six typed variants, all carrying needed/have or rule codes; no swallowed errors |
| `NalUnitType` enum (26 variants) | 106–158 | REAL (unreachable) | — | Table 7-1 exact incl. reserved/unspecified preserved verbatim (no semantic guessing) |
| `NalUnitType::from_id` | 162–194 | REAL (unreachable) | — | exact id map; 10..=15/22..=31/41..=47 Reserved, 48..=63 Unspecified |
| `NalUnitType::id` | 196–228 | REAL (unreachable) | — | exact inverse |
| `NalUnitType::is_irap` | 231–234 | REAL (unreachable) | — | 16..=23 incl. reserved 22/23 — correct (IRAP range per §7.4.4.2) |
| `NalUnitType::is_vcl` | 238–241 | REAL (unreachable) | — | ≤31 — correct |
| `NalHeader` struct | 245–254 | REAL (unreachable) | — | — |
| `NalHeader::split` | 267–295 | REAL (unreachable) | — | §7.3.1 bit-exact; <2 bytes, forbidden bit, tid==0 all typed |
| `next_start_code` | 300–314 | REAL (unreachable) | — | returns index past `01`; bounds safe (`end = len-2`) |
| `trim_trailing_zeros` | 317–333 | REAL (unreachable) | — | spec-correct (RBSP stop bit ⇒ nonzero tail) |
| `AnnexBNals::new` | 349–357 | REAL (unreachable) | — | trivial |
| `AnnexBNals::next` | 362–388 | REAL (unreachable) | — | verified: 4-byte codes via trim; `end_raw.max(start).min(len)` defensive; O(n) total |
| `annexb_nals` | 390–392 | REAL (unreachable) | — | collect convenience |
| `unescape_rbsp_into` | 410–446 | REAL (unreachable) | — | overlapping-escape re-scan verified; dangling/bad-suffix typed |
| `unescape_rbsp` | 448–455 | REAL (unreachable) | — | one-shot wrapper |
| `LengthPrefixedNals::new` | 475–486 | REAL (unreachable) | — | length_size 1..=4 typed-validated |
| `LengthPrefixedNals::next` | 489–535 | REAL (unreachable) | S4 | F-004 padding asymmetry; zero-length anti-spin guard; `len+length_size` no overflow on 64-bit (len ≤ 2^32) |
| `HvcCArray` struct | 537–548 | REAL (unreachable) | — | — |
| `HvcCConfig` struct (17 fields) | 554–590 | REAL (unreachable) | S4 | F-003 avg_frame_rate doc |
| `HvcCConfig::parse` | 610–688 | REAL (unreachable) | — | all 5 reserved patterns + 17 extractions spec-verified; version!=1 typed; array/NAL bounds typed |
| `HvcCConfig::looks_like_hvcc` | 691–699 | REAL (unreachable) | — | honest heuristic (4 checks); only decode.rs/tests call it |
| `read_u16` | 701–711 | REAL (unreachable) | — | bounds-checked |
| `read_counted_nal` | 714–737 | REAL (unreachable) | — | zero-length typed; bounds typed |
| `BitReader` struct + `new` + `total_bits` | 740–754 | REAL (unreachable) | S4 | F-002 doc glitch; read methods are part2's range |

## 5. Test honesty
No tests in part1's range (the `#[cfg(test)]` module starts at 893, part2's range). Fuzz coverage of this part's surfaces exists and is registered (hevc/fuzz.rs:45–66; baseline #18 "35 targets, diff empty" + #3/#4 test suites green). Part2's auditor should confirm the unit tests pin the overlapping-escape and 4-byte-start-code cases (the corpus test `corpus_0044…` at 1440).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Both splitters are iterators of borrowed slices (zero copy, no allocation)… steady-state parse path allocates nothing" | nal.rs:24–28 module docs | REAL | read: `AnnexBNals`/`LengthPrefixedNals` yield `&[u8]`; `unescape_rbsp_into` clears+reserves one reusable buffer |
| "MP4 samples are zero-padded to sample boundaries" | nal.rs:460–461 | FALSE as a general premise | MP4 sample sizes are exact; padding is writer-specific (F-004) |
| (module docs, hevc/mod.rs — other agent) "no software HEVC decode, hw-only by ADR-001" | hevc/decode.rs:2-3 | REAL | baseline: sample-hevc.mp4 → Error, no decoder; HevcDecoder unconstructed in shipped hosts |

## 7. Dead / duplicated / unreachable code
- The whole part is unreachable (F-001). 
- Triple duplication of HEVC byte-walking across the codebase: this parser (codecs), the demux-side ES/probe sniffers (crates/demux/src/es/hevc.rs, media-probe/src/signature/es.rs:221-241), and the container fourcc mapping — three implementations of "what is an HEVC NAL", one wired (demux/probe), two not.

## 8. Dependencies & flags
- Unconditional module (in-house, zero external deps — codecs/Cargo.toml:26-27 lists hevc among the unconditional in-house decoders). Compiles into every shipped build, runs never (F-001).
- Wave-6 hostile question: HEVC has NO decode lane in shipped builds at all (software: none exists; hardware: `HevcDecoder` test-only) — contrast H.264, whose sw lane IS reachable (see mjpeg/decoder.rs#part2.audit.md §8: apps/cli/Cargo.toml:51 → codecs/Cargo.toml:56,73 `codec-h264-sw` default-on → h264/mod.rs:55,61 → pipeline_host.rs:346; baseline sample-420.mp4 PLAYS). The asymmetry is honest in the matrix (`codec-video-hevc-*` rows blocked; INDEX.md smoke confirms).

## 9. Open questions for the next agent
- part2 (nal.rs 759–1517): the `BitReader` ue/se methods and `finish()` — check ue range clamping (H.265 ue values up to 2^32-1 vs the `u64` return) and whether `more_rbsp_data` handles the trailing-stop-bit edge exactly; also confirm the corpus tests ran in baseline (grep the test log for `hevc` corpus names — the Lead sandbox corpus presence is established for h264 slice tests, hevc corpus presence unverified by me).
- hevc/decode.rs owner: if the hw lane ever ships, F-001 flips to reachable — the fuzz gap for slice-segment headers ("slice-segment unit suites own that pass", fuzz.rs:17-20) should close first.
- demux es/hevc.rs owner: the demux-side ES probe duplicates NAL-type knowledge (`b >> 1` at signature/es.rs:235) — drift risk against this file's Table 7-1.
