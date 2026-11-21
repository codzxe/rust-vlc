# AUDIT: crates/codecs/src/audio/mp2/header.rs
| Field | Value |
|---|---|
| Auditor | W07-A09 |
| Date | 2026-11-21 |
| Lines / functions | 577 / 22 (13 prod items, 10 tests) |
| File verdict | UNREACHABLE (real, spec-correct, well-tested code; no shipped caller) |
| Reachable from shipped entry point? | NO. Caller census (rg over `crates/`+`apps/`): `walk_mp2_frames`/`parse_mp2_header`/`geometry` are called only by `mp2/seek.rs` tests, `mp2/fuzz.rs:44-99` (registered fuzz target `fuzz_codec_mp2`), `mp2/decode_stream` path and `crates/codecs/tests/mp2_corpus_differential.rs`; `is_mp2_stream`, `layer_ii_bitrate`, `layer_ii_bitrate_table`, `intensity_bound`, `Mp2Geometry`, `Mp2FrameAt` have ZERO callers outside this file's tests and re-exports. The shipped `lumen` MP2 lane (`apps/cli/src/pipeline_host.rs:266-270` "mp2" → `Mp2Decoder`) feeds packets straight to symphonia and never enters this file (mp2/decoder.rs contains no reference to header.rs). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "walk_mp2_frames\|parse_mp2_header\|is_mp2_stream\|layer_ii_bitrate\|intensity_bound\|Mp2Geometry" crates apps` → caller census above; `sed -n` on all quoted ranges; `od -c` on line 302 (`#[must_use]` — my Bash display mangles `#[m`; raw bytes verified fine); tables checked against ISO/IEC 11172-3 §B.2 / 13818-3 §B.1 from spec knowledge |

## 1. One-paragraph verdict
A tight, honest, spec-correct Layer II header module: the MPEG-1 and LSF bitrate tables match ISO/IEC 11172-3 §B.2 / 13818-3 §B.1 exactly, the mode-extension bound decode (4/8/12/16) matches §2.4.2.3, the CBR gate refuses mid-stream geometry changes with typed errors, and the tests genuinely cross-pin the tables against the family parser (a drift there flips a test). Nothing here is fake. The problems are structural: the entire module is dead weight in the shipped binary — every production-relevant consumer is a test, a differential harness, or a fuzz target; the `is_mp2_stream` probe it advertises ("the probe the future MP2 demuxer / registry row will call") has no caller because the demux crate re-implements its own MP2 framing tables (demux/src/es/mp3.rs), which is a duplication/drift hazard; and `geometry()` does one unchecked subtraction propped up only by an invariant in another file.

## 2. What actually works (verified)
— Bitrate tables: `LAYER_II_BITRATES_MPEG1` (46-48) and `LAYER_II_BITRATES_MPEG2_LSF` (52-53) match the ISO tables (Layer II MPEG-1: 32,48,56,64,80,96,112,128,160,192,224,256,320,384; LSF: 8,16,24,32,40,48,56,64,80,96,112,128,144,160) — verified by reading lines 46-53 and the cross-pin test `bitrate_tables_match_the_family_parser` (395-429), which iterates all 14 indices through the real family parser.
— Layer gate + typed refusals: verified by reading `parse_mp2_header` (149-168) and tests at 355-392 (family codes `mp3.header.sync`/`mp3.free_format`/`mp3.header.short` pass through; non-L2 → `mp2.header.not_layer_ii`).
— Baseline run #3 (audit/runs/baseline/INDEX.md: `cargo test --workspace --release` 1681 passed) includes this module's 10 tests.

## 3. Findings

### F-001 [S3] Dead public API: the advertised MP2 probe and table accessors have no callers; the demux layer re-implements the tables
Where: crates/codecs/src/audio/mp2/header.rs:57-74, 119-128, 299-316
Code:
```rust
/// Whether `data` begins like a raw MP2 stream: an ID3v2 tag or a
/// parseable Layer II frame header at offset 0 (the probe the future
/// MP2 demuxer / registry row will call).
#[must_use]
pub fn is_mp2_stream(data: &[u8]) -> bool {
```
What is wrong: `is_mp2_stream`, `layer_ii_bitrate_table`, `layer_ii_bitrate`, `intensity_bound` (and `bound_subband` outside tests) are public API with zero production callers — the "future MP2 demuxer" does not exist. The demux crate deliberately cannot depend on codecs and carries its own copies of the Layer/bitrate/samplerate tables (crates/demux/src/es/mp3.rs:12-14, "the codecs-side mp3 module's read-only consume: this crate cannot depend on codecs, layer ordering"). So the spec tables now live in (at least) two crates, and this module's cross-pin test only pins codecs-internal consistency, not demux-vs-codecs consistency.
Why it matters: dead exported surface plus duplicated spec tables is the textbook drift hazard the checklist calls out; nothing detects a demux-side table typo.
What correct looks like: either wire the probe into a demux/registry path or drop the exports to `pub(crate)`/tests; move the shared tables to a leaf crate both layers can depend on (media-types precedent).
Confidence: HIGH — Verification: read + caller census (rg across crates/ and apps/).

### F-002 [S4] Unchecked subtraction in `geometry()` propped up by a cross-file invariant
Where: crates/codecs/src/audio/mp2/header.rs:285
Code:
```rust
    let nominal = h0.frame_size - u32::from(h0.padding);
```
What is wrong: `frame_size` comes from the family parser (mp3/header.rs, not this file). The subtraction is safe only because that parser guarantees `frame_size ≥ padding` (minimum Layer II frame is 24 bytes at 8 kbps/48 kHz LSF). Nothing local enforces it; if the family arithmetic ever changes, this is a debug-build panic (`attempt to subtract with overflow`) in `geometry`, which the seek path treats as infallible.
Why it matters: fragile cross-file invariant without a guard or a comment naming it.
What correct looks like: `h0.frame_size.saturating_sub(u32::from(h0.padding))` or a debug_assert documenting the dependency.
Confidence: HIGH — Verification: read; minimum frame size derived from the tables (144×8000/48000 = 24).
### F-003 [S4] Probe fragility + dead match arm
Where: crates/codecs/src/audio/mp2/header.rs:152-156 and 307-313
Code:
```rust
        let which = match family.layer {
            Layer::L1 => "Layer I",
            Layer::L3 => "Layer III",
            Layer::L2 | Layer::Reserved => "reserved",
        };
```
What is wrong: (a) the `Layer::L2` arm is unreachable — the guard at 151 already returned for L2, so it exists only to satisfy exhaustiveness; labeled "reserved" it would mislabel a hypothetical L2 as "reserved" if the guard ever moved. (b) `is_mp2_stream` (307-313): when `id3v2_len` returns `Err` (e.g., a legit tag larger than the probe buffer — id3.rs:228-242 refuses tag-past-buffer), the probe answers `false`: a perfectly valid MP2 stream behind a large ID3 tag fails the probe. Harmless today only because nothing calls it (F-001).
Why it matters: hygiene; the probe's honest-window contract is undocumented.
What correct looks like: `Layer::Reserved => "reserved"` plus a distinct arm or comment for L2; document that the probe needs the whole tag in-buffer.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `LAYER_II_BITRATES_MPEG1` | 46-48 | REAL | — | Matches ISO 11172-3 §B.2; cross-pinned by test at 395-411. |
| `LAYER_II_BITRATES_MPEG2_LSF` | 52-53 | REAL | — | Matches 13818-3 §B.1; cross-pinned at 412-424. |
| `layer_ii_bitrate_table` | 57-64 | REAL | S3 (F-001) | Dead pub API; Reserved→None correct. |
| `layer_ii_bitrate` | 68-74 | REAL | S3 (F-001) | Index edges 0/15 refused; get-based, no panic. |
| `LayerIiModeExtension` + `from_bits` + `bound_subband` | 82-117 | REAL | S3 (F-001, from_bits/bound used via Mp2FrameHeader) | Bits→bound 4/8/12/16 matches §2.4.2.3. |
| `intensity_bound` | 122-128 | REAL | S3 (F-001) | JointStereo-gated, honest N/A → None. |
| `Mp2FrameHeader` | 132-139 | REAL | — | Data type. |
| `parse_mp2_header` | 149-168 | REAL | S4 (F-003a) | Family parse + L2 gate; typed codes verified by tests 355-392. |
| `Mp2FrameAt` | 171-177 | REAL | — | Data type. |
| `walk_mp2_frames` | 187-214 | REAL | — | Delegates framing/resync to family walker; first non-L2 frame typed at its offset. |
| `Mp2Geometry` | 232-252 | REAL | — | Data type; doc honest about padding being the only legal variation. |
| `geometry` | 261-297 | REAL | S4 (F-002) | CBR gate on (version,bitrate,rate,mode); `total_samples = frames×1152`; empty→None. |
| `is_mp2_stream` | 303-316 | REAL | S3/S4 (F-001, F-003b) | Zero callers; ID3 path clamps safely. |
| 10 tests | 318-577 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `parses_the_canonical_layer_ii_headers` (328) | YES | Asserts version/layer/bitrate/rate/mode/spf and both frame sizes (1253, 720) — real arithmetic pins. |
| `non_layer_ii_is_a_typed_refusal` (356) | YES | L3 and L1 headers → `mp2.header.not_layer_ii`; mutating the gate flips it. |
| `family_header_failures_pass_through_typed` (373) | YES | Sync/free-format/short → exact family codes. |
| `bitrate_tables_match_the_family_parser` (395) | YES | 28 parser round-trips + refusal edges; a single wrong table cell fails with the index in the message. |
| `mode_extension_decodes_the_layer_ii_bound` (432) | YES | All four bounds + the N/A case. |
| `walk_gates_layer_and_reports_geometry` (463) | YES | Resync over inter-frame garbage, offsets, geometry tuple, empty-walk None. |
| `walk_refuses_a_midstream_layer_switch` (501) | YES | L3 frame inside stream → `mp2.stream.layer_mismatch`. |
| `geometry_refuses_a_bitrate_switch` (516) | YES | 80→112 kbps mid-stream → `mp2.stream.cbr_mismatch`. |
| `padding_bit_varies_frame_size_but_not_the_nominal_geometry` (539) | YES | Padded 1254 vs nominal 1253; mixed padding legal. |
| `probe_accepts_layer_ii_only` (566) | YES | Negative probes (L3, bad sync, RIFF, empty, 3-byte tail). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Layer II bitrate tables … pinned against the family parser by unit test" | module doc 15-17 | REAL | Test at 395-429 does exactly this. |
| "there is no Xing/Info/VBRI construction in Layer II" | module doc 219-221 | REAL | Correct per MPEG family specs (Xing/VBRI are Layer III encoder side-channels). |
| "the walk … plus the CBR geometry the seek arithmetic consumes" | module doc 2-4 | REAL-UNPROVEN | Code exists and is tested (walk/geometry/seek tests), but no shipped consumer executes it — the shipped `lumen` MP2 lane bypasses it entirely (see reachability). |

## 7. Dead / duplicated / unreachable code
- Unreachable in shipped builds: the whole module (see header table). Compiled (feature `codec-mp2` → default on) but never executed by `lumen`/`lumen-tauri`.
- Duplicated: MPEG audio Layer/bitrate/rate tables exist in both crates/codecs/src/audio/mp3/header.rs (family) and crates/demux/src/es/mp3.rs (demux-side re-implementation, by layering rule). This module adds a third, Layer II-only copy.

## 8. Dependencies & flags
- Feature `codec-mp2` (implies `codec-mp3`; codecs/Cargo.toml:77). Default-on → compiled into shipped CLI/web builds.
- Imports from `crate::audio::mp3` (family parser) — the documented "thin sibling" shape; no external deps, no unsafe.

## 9. Open questions for the next agent
- mp2/decoder.rs + mp2/seek.rs owner: confirm `Mp2Decoder`'s packet lane never needs the geometry gate (it currently relies on symphonia's own framing — if a stream has a mid-stream bitrate switch, symphonia handles or fails it internally, untested here).
- Demux wave: demux/src/es/mp3.rs mints "mp2" tracks for Layer I too (es/mp3.rs:250: `Layer::L1 | Layer::L2 => "mp2"` — Layer **I** mapped to the MP2 descriptor; check the decoder's acceptance gate covers L1 honestly).
