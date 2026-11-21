# AUDIT: crates/codecs/src/video/mjpeg/field.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2026-11-21 |
| Lines / functions | 23 / 0 functions — one `pub enum` (3 variants, 4 derives) + module/variant docs |
| File verdict | REAL (reachable, values executed in baseline tests; end-to-end AVI1 lane absent from shipped smoke inputs) |
| Reachable from shipped entry point? | YES. `lumen open` AVI → apps/cli/src/pipeline_host.rs:242-243 `"mjpeg" => … MjpegDecoder::new()` → decoder.rs:675/1040 (`field_polarity: info.field_polarity` / `meta.field_polarity`) → parser.rs:263 `info.field_polarity = Some(polarity?)` ← parse_avi1 (parser.rs:330-349) constructs exactly this enum's three variants. `parse_avi1`'s marker check runs on every frame's APP0 walk (JFIF markers take the None path). The shipped AVI first-vertical lane (baseline: avi-firstvertical-open-session.txt plays; bench 60/60 frames) drives the carrier path; the `Some(_)` values were executed by baseline unit tests (test-workspace.log.txt:677-678, 721-724, 1025, 1123). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read tool full file; `rg -n "FieldPolarity"` repo-wide (consumer census incl. stage_bench example + worklog); `rg "field_polarity\|interlaced"` repo-wide (consumption census — only constructions, 2 internal reads); `rg -n "mjpeg" apps/cli/src/pipeline_host.rs:242-243`; `sed -n` verification field.rs:1-10,13-23, mod.rs:385-392; Read parser.rs:320-350 (parse_avi1 mapping), decoder.rs:136-165, mod.rs:41-63 (AVI1 handling docs); baseline greps: test-workspace.log.txt:677-678, 721-724, 1025, 1123 |

## 1. One-paragraph verdict
A 23-line vocabulary type whose existence rationale (keeping `super::FieldPolarity` resolvable for the `stage_bench` example's `#[path]`-include tree) is verifiably true, whose three variants map 1:1 to the OpenDML AVI1 polarity bytes 0/1/2, and whose parse/decode/differential behavior is pinned by eight baseline-executed tests covering all three values, the out-of-vocabulary refusal, between-walks reset, field-height decode, and mutation survival. The vocabulary itself is honest. The defect is where the information goes: the only production code that reads the variants collapses them to a single boolean (`interlaced: meta.field_polarity.is_some()`, mod.rs:389), that boolean is copied once into the engine frame (pipeline_host.rs:1554) — and no code in the entire repository ever reads it. Field-based AVI1 MJPEG therefore plays as sequential half-height frames with the polarity vocabulary exerting zero influence on any shipped behavior; the module docs' deferral of weaving to "the layer that owns it" points at a layer that does not exist.

## 2. What actually works (verified)
— Mapping verified against OpenDML AVI1: polarity 0 → FirstField, 1 → SecondField, 2 → Interleaved (parser.rs:344-348), out-of-vocabulary bytes typed-refused (`mjpeg.app0.avi1.polarity`, parser.rs:348) — read-verified.
— Baseline execution, all ok: `video::mjpeg::parser::tests::avi1_polarity_parses_for_all_three_values` (test-workspace.log.txt:723), `…resets_between_walks` (:721), `…out_of_vocabulary_is_structural` (:722), `…segment_without_the_polarity_byte_is_structural` (:724); `video::mjpeg::decoder::tests::avi1_interleaved_frame_is_metadata_only_bit_exact` (:677), `avi1_single_field_decodes_at_field_height` (:678); fuzz `avi1_field_marker_mutations_survive_the_harness` (:1025); differential `interlaced_content_fixtures_stay_polarity_free` (:1123).
— The stage_bench rationale: crates/codecs/examples/stage_bench.rs:85-93 `#[path]`-includes decoder/parser and re-imports `use field::FieldPolarity` — the example exists and resolves; the E0432 fix history is recorded (docs/worklog/archive-2026-10.md:9054); prior audit of the example confirms the include tree.
— Carrier path executed end-to-end: AVI first-vertical open plays (avi-firstvertical-open-session.txt) with 60/60 frames decoded in bench — every frame walked the APP0 marker check that gates this enum's construction.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/mjpeg/field.rs:13-23 (the vocabulary), consumption census: mjpeg/mod.rs:389, apps/cli/src/pipeline_host.rs:1554
Code:
```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldPolarity {
```
```
            interlaced: meta.field_polarity.is_some(),
```
What is wrong: the enum's discriminative information (which field, vs both) is read by exactly one production expression — `is_some()` at mod.rs:389, collapsing `FirstField | SecondField | Interleaved` + `None` to one boolean. That boolean is copied at pipeline_host.rs:1554 (`interlaced: frame.interlaced`) and then **read by nothing**: repo-wide grep for reads of `.interlaced` returns only dnx's internal header flag (dnx/decoder.rs:312) and a mpeg12 corpus test assertion. The `FirstField` vs `SecondField` distinction has zero production consumers (tests only).
Why it matters: mod.rs:54-57 assigns weaving to "the layer that owns it … the polarity is surfaced on `DecodedFrame` for the layer that owns it" — no such layer exists anywhere in the tree (CLI, desktop, web, renderer, compositor all either construct `interlaced: false` or copy it). Field-based AVI1 MJPEG plays as sequential half-height frames whose playback is bit-identical whether this file exists or not: the entire card-5.H3 feature is inert decoration beyond parsing. A vocabulary that changes no shipped behavior is "config declared but never read" — the docs' honesty about the decoder boundary conceals that the deferral target is a ghost.
What correct looks like: either a presentation-layer consumer (weave/deinterlace or at minimum a frame-flag read that sizes/orders output), or the docs and matrix rows stating plainly that v0 surfaces polarity without any consumer and interlaced MJPEG displays as half-height sequential frames.
Confidence: HIGH — Verification: rg consumption census over the whole repo + read of both consumption sites.

### F-002 [S4]
Where: crates/codecs/src/video/mjpeg/field.rs:20-22
Code:
```
    /// Polarity 2: both fields, lines interleaved in one frame (a
    /// complete interlaced picture; field 1 = the even/top lines).
    Interleaved,
```
What is wrong: "field 1 = the even/top lines" collides two naming systems: in ITU-R BT.601 terminology field 1 is the **odd** field (lines 1,3,5…) and field 2 the **even** field; "even" here means 0-based line indices (0,2,4 = top). A reader fluent in ITU field naming concludes the opposite (bottom-first) from the same words.
Why it matters: this comment is the definitional documentation of the vocabulary; the operational claim it supports ("top-field-first content stays top-field-first", mod.rs:49-50) is order-preservation, which is true regardless — but the definition itself is ambiguous in exactly the domain it exists to define.
What correct looks like: "field 1 = the top lines (0-based even indices — the ITU 'odd field')".
Confidence: HIGH — Verification: read; terminology per BT.601 domain knowledge.

## 4. Per-function table
No functions in this file. Per rule 9, the units are:
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (stage_bench `#[path]` rationale) | 1-7 | REAL | — | verified true: example exists (stage_bench.rs:85-93), worklog :9054 records the E0432 fix |
| `pub enum FieldPolarity` (type + derives) | 13-23 | REAL (executed) | S3 | Debug/Clone/Copy/PartialEq/Eq — Copy correct for a 1-byte vocabulary; production fate is `is_some()` (F-001) |
| variant `FirstField` + doc | 15-17 | REAL | — | maps polarity 0; field-height semantics pinned by avi1_single_field_decodes_at_field_height (:678) |
| variant `SecondField` + doc | 18-19 | REAL | — | maps polarity 1; same test; no production reader distinguishes it from FirstField (F-001) |
| variant `Interleaved` + doc | 20-22 | REAL | S4 | maps polarity 2; bit-exact metadata-only decode pinned (:677); "even/top" terminology F-002 |

## 5. Test honesty
No tests in this file; the vocabulary's behavior is pinned externally (all executed ok in baseline):
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parser avi1_polarity_parses_for_all_three_values (:723) | YES | byte→variant mapping asserted for 0/1/2 (parser.rs:972-989) — swapping two variants flips it |
| parser avi1_polarity_out_of_vocabulary_is_structural (:722) | YES | polarity 3+ must be typed Err |
| parser avi1_field_polarity_resets_between_walks (:721) | YES | second walk without marker must yield None (no leak) |
| parser avi1_segment_without_the_polarity_byte_is_structural (:724) | YES | truncated AVI1 must be typed Err, not read past the segment |
| decoder avi1_single_field_decodes_at_field_height (:678) | YES | asserts First/SecondField decode at half SOF height |
| decoder avi1_interleaved_frame_is_metadata_only_bit_exact (:677) | YES | interleaved = full frame, rows order-preserved, pixel-exact |
| fuzz avi1_field_marker_mutations_survive_the_harness (:1025) | NO (by design) | panic-freedom only |
| differential interlaced_content_fixtures_stay_polarity_free (:1123) | YES (negative pin) | ffmpeg-generated interlaced content must NOT carry polarity (pins the none-path against false AVI1 detection) |
| Gap: no test pins any DOWNSTREAM consumer of the variants | — | because none exists (F-001) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the `stage_bench` example `#[path]`-includes the decoder and parser sources … a dedicated module keeps that resolution working" | field.rs:3-7 | REAL | stage_bench.rs:85-93 read-verified; worklog archive-2026-10.md:9054 |
| "Polarity 0: first field only — SOF height is the field height" / "Polarity 1: second field only" | field.rs:15-19 | REAL | parser.rs:344-347 mapping + decoder test :678 |
| "Polarity 2: both fields interleaved … field 1 = even/top lines" | field.rs:20-22 | REAL (semantics) / AMBIGUOUS (wording) | decoder test :677 (order-preserving); F-002 |
| "polarity is surfaced on `DecodedFrame` for the layer that owns it" (weaving deferred to presentation layer) | mjpeg/mod.rs:54-57 (the handling docs for this type) | SHALLOW | surfaced: yes (mod.rs:210, :389); owning layer: does not exist anywhere in the repo (F-001 census) |
| "ffmpeg 7.1's mjpeg codec neither writes nor parses AVI1" | mjpeg/mod.rs:58-63 (worklog-measured) | UNVERIFIED | measurement claim about external code; no ffmpeg source in sandbox; consistent with the differential's polarity-free fixtures |

## 7. Dead / duplicated / unreachable code
- The enum itself is reachable and constructed; dead at the consumption end (F-001): variants' distinctions have no production reader; the derived boolean has no reader at all.
- No duplication: the vocabulary exists exactly once; parser/decoder/mod reference it, stage_bench re-imports it.

## 8. Dependencies & flags
None. `pub(crate)`-adjacent leaf under `pub mod field` (mjpeg/mod.rs); re-exported at mod.rs:123 (`pub use field::FieldPolarity`) — the re-export is the surface `stage_bench`'s include tree and downstream crates see. No feature gates; mjpeg is a default, zero-dependency in-house codec (codecs/Cargo.toml:26,47-59).

## 9. Open questions for the next agent
- Whichever wave owns the renderer/compositor or desktop presentation: does ANY planned layer read `VideoFrame::interlaced`? If not, F-001 upgrades from "inert vocabulary" to "the AVI1 feature claim in the matrix is decoration" — check the `codec-video-mjpeg` matrix row's wording against this.
- The h264 sw lane sets `interlaced` for PAFF field pictures (sw.rs:316) — same no-reader fate; whoever audits sw.rs should cross-reference F-001.
