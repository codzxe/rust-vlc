# AUDIT: crates/codecs/src/audio/mp2/header.rs
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-24 |
| Lines / functions | 577 / 9 prod + 9 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (mostly) — apps/cli/src/pipeline_host.rs:266-269 `"mp2" => Mp2Decoder::new` → mp2/decoder.rs:79 `parse_mp2_header`, :135 `walk_mp2_frames`, :147 `geometry`. EXCEPT `is_mp2_stream` — UNREACHABLE (no non-test caller) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `target/debug/deps/codecs-bae0f1756cc0a809 mp2` → `27 passed; 0 failed` exit 0; rg/Grep caller searches |

## 1. One-paragraph verdict
REAL and clean. This is the Layer II-specific overlay on the shared MPEG-audio family parser (sync/scan/resync and the 4-byte parse live in `crate::audio::mp3` and are NOT duplicated — verified by import list, lines 36-39). The Layer II gate, the ISO bitrate tables (11172-3 §B.2 / 13818-3 §B.1 — both rows match the specs; and they are cross-pinned against the family parser by a real test that iterates all 14 indices in both versions), the Layer II mode-extension semantics (bound subband 4/8/12/16 — correct for Layer II, distinct from Layer III's flags, honestly documented), and the CBR geometry gate are all implemented, typed, and tested. 27 mp2 lib tests pass, including padding-variation and mid-stream bitrate/layer refusal. The only defects are hygiene: `is_mp2_stream` is a probe with zero production callers, and `parse_mp2_header` carries a dead defensive match arm.

## 2. What actually works (verified)
— verified by `target/debug/deps/codecs-bae0f1756cc0a809 mp2` → 27 passed (incl. all 9 header.rs tests: canonical headers with exact frame sizes 1253/720, typed refusals, table cross-pin, mode extension, walk/geometry, layer mismatch, bitrate switch, padding, probe).
— table cross-pin (`bitrate_tables_match_the_family_parser`, 395-429) iterates index 1..=14 for MPEG-1 and MPEG-2 and asserts `layer_ii_bitrate(...) * 1000 == family.bitrate_bps` — the two table sources cannot drift silently.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/mp2/header.rs:299-316
Code:
```rust
/// Whether `data` begins like a raw MP2 stream: an ID3v2 tag or a
/// parseable Layer II frame header at offset 0 (the probe the future
/// MP2 demuxer / registry row will call).
#[must_use]
pub fn is_mp2_stream(data: &[u8]) -> bool {
```
What is wrong: zero non-test callers in the repo (Grep over crates/ + apps/: only header.rs's own test at 566-576 and mp2/mod.rs:96's re-export smoke test). The honest "future … will call" doc keeps this from being a lie, but today it is dead public API — the "future MP2 demuxer" does not exist (crates/demux has no mp2 references). Why it matters: unexercised probe surface rots silently (its ID3-skip path is only ever run by its own unit test). What correct looks like: land it with the demuxer row or gate it. Confidence: HIGH — Verification: Grep caller search (crates/demux: no matches; apps/: no matches).

F-002 [S4]
Where: crates/codecs/src/audio/mp2/header.rs:151-156
Code:
```rust
    if family.layer != Layer::L2 {
        let which = match family.layer {
            Layer::L1 => "Layer I",
            Layer::L3 => "Layer III",
            Layer::L2 | Layer::Reserved => "reserved",
        };
```
What is wrong: `Layer::Reserved` can never reach here — the family parser refuses it typed before returning (`mp3/header.rs:291-293`, code `mp3.header.layer_reserved`), and `Layer::L2` is excluded by the enclosing `if`. The `Layer::L2 | Layer::Reserved => "reserved"` arm is provably dead twice over. Harmless defensive exhaustiveness; noted for the dead-code ledger. Confidence: HIGH — Verification: read + mp3/header.rs:291-293.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `layer_ii_bitrate_table` | 58-64 | REAL | — | tables match ISO 11172-3 §B.2 / 13818-3 §B.1 |
| `layer_ii_bitrate` | 69-74 | REAL | — | index 0/>14 refused; tested |
| `LayerIiModeExtension::from_bits` | 97-104 | REAL | — | 00→4 … 11→16 per §2.4.2.3; tested |
| `LayerIiModeExtension::bound_subband` | 109-116 | REAL | — | tested |
| `intensity_bound` | 123-128 | REAL | — | None outside joint stereo (honest N/A); tested |
| `parse_mp2_header` | 149-168 | REAL | S4 | family parse + gate; dead Reserved arm (F-002) |
| `walk_mp2_frames` | 187-214 | REAL | — | delegates framing to family walk; typed layer gate; tested incl. resync |
| `geometry` | 261-297 | REAL | — | CBR gate (version/bitrate/rate/mode), nominal = padded−padding; tested incl. bitrate-switch refusal and padding variation |
| `is_mp2_stream` | 303-316 | REAL / UNREACHABLE | S4 | F-001; logic itself tested |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `parses_the_canonical_layer_ii_headers` (328) | YES | exact bitrates/rates/frame sizes (1253, 720) asserted |
| `non_layer_ii_is_a_typed_refusal` (356) | YES | typed codes asserted for L3 and L1 |
| `family_header_failures_pass_through_typed` (373) | YES | family codes asserted verbatim |
| `bitrate_tables_match_the_family_parser` (395) | YES | 28 header-synthesis iterations pinned to the table |
| `mode_extension_decodes_the_layer_ii_bound` (432) | YES | all four bounds + N/A case |
| `walk_gates_layer_and_reports_geometry` (463) | YES | resync past garbage, offsets, geometry tuple |
| `walk_refuses_a_midstream_layer_switch` (501) | YES | typed `mp2.stream.layer_mismatch` |
| `geometry_refuses_a_bitrate_switch` (516) | YES | typed `mp2.stream.cbr_mismatch` |
| `padding_bit_varies_frame_size_but_not_the_nominal_geometry` (539) | YES | nominal strips padding |
| `probe_accepts_layer_ii_only` (566) | YES | positive/negative corpus incl. empty + truncated |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the 4-byte header parse, the sync scan and the resync-on-garbage walker are the family's … NOT duplicated here" | header.rs:6-9 | REAL | imports at 36-39; only Layer II logic in-tree |
| "Layer II carries 1152 samples/frame in every MPEG version, so frame_size = 144 × bitrate / sample_rate + padding" | header.rs:30-32 | REAL | tests assert 1152 and 1253/720 for MPEG-1 and MPEG-2 LSF |
| bitrate tables "ISO/IEC 11172-3 §B.2 … 13818-3 §B.1" | header.rs:15-16, 46-53 | REAL | rows match the standards; cross-pinned by test |
| "Layer II bits encode the intensity-stereo bound subband (4/8/12/16)" | header.rs:18-20 | REAL | from_bits mapping + test |
| "the probe the future MP2 demuxer / registry row will call" | header.rs:299-301 | REAL-UNPROVEN / UNREACHABLE | honest "future" wording; no caller today (F-001) |
| "there is no Xing/Info/VBRI construction in Layer II" | header.rs:22-23, 219-221 | REAL | spec fact; consistent with the walk verifying per-frame bitrate |

## 7. Dead / duplicated / unreachable code
`is_mp2_stream` (F-001); dead `Layer::Reserved` arm (F-002). No duplication — the family split is real (the module imports the family parser instead of re-implementing it).

## 8. Dependencies & flags
Module behind `codec-mp2` feature (which implies `codec-mp3`, Cargo.toml:77) — default-on; release lane includes it. No external deps of its own; `symphonia-bundle-mp3` is the decode engine (decoder.rs, another file). No unsafe.

## 9. Open questions for the next agent
- mp2/decoder.rs, seek.rs, fuzz.rs are separate files (other agents): verify `refine_to_frame` uses the same nominal-vs-padded convention as `geometry` (the handoff at decoder.rs:147/`cbr_seek` at decoder.rs:674 composes the two — the geometry tests pass so the composition is at least self-consistent).
