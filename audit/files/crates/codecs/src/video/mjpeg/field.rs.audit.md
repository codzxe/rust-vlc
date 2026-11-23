# AUDIT: crates/codecs/src/video/mjpeg/field.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 23 / 1 enum (no fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the shipped MJPEG decoder path: parser.rs:330-347 `parse_avi1` produces it, decoder.rs:675/1040 carries it on decoded metadata, mod.rs:123 re-exports |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
A correct three-variant AVI1 field-polarity vocabulary (0 = first field only, 1 = second field only, 2 = interleaved — matching the OpenDML AVI M-JPEG convention), genuinely consumed by the shipped MJPEG parser/decoder, whose placement rationale (the `stage_bench` example `#[path]-includes sources without mod.rs) is real — `crates/codecs/examples/stage_bench.rs` exists. The consumer contract is honest and test-pinned: the marker is **metadata only** (decoder test `avi1_interleaved_frame_is_metadata_only_bit_exact` asserts pixel bytes equal the marker-free decode, with the order-preserving guarantee documented). No findings.

## 2. What actually works (verified)
- Consumers verified by rg: parser.rs:345-347 maps the polarity byte to exactly these variants (with parser tests at :972+), decoder.rs:1848-1877 asserts all three variants + the None/progressive case, mod.rs:210 exposes it on metadata.
- `crates/codecs/examples/stage_bench.rs` exists → the file's stated reason for existing as a separate module is not phantom.
- mjpeg tests pass live this session (`mjpeg::error` filter run earlier; full decoder suite green per baseline cargo-test-workspace.txt).

## 3. Findings
None. (Checked: variant semantics vs the AVI1/OpenDML convention — correct; naming honesty (FirstField/SecondField/Interleaved vs polarity 0/1/2) — matches parser mapping and doc; dead code — none, three consumers; the absence-of-marker case deliberately excluded (progressive, `None`) — consistent with parser behavior; no unsafe, no panics possible in a data-less enum.)

## 4. Per-function table
File < 100 LOC: single item — `pub enum FieldPolarity { FirstField, SecondField, Interleaved }` (L13-23) — REAL, correct semantics, 3 shipped consumers.

## 5. Test honesty
No tests in-file; the pinning tests live in parser.rs (:972+) and decoder.rs (:1828-1877) — cross-checked, they assert the exact variant per polarity byte and the metadata-only guarantee.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Absence of the marker (the JFIF world) is a progressive frame and is not represented here" | field.rs:11-12 | TRUE | parser: no AVI1 → `field_polarity: None` (decoder test :1848) |
| separate-file rationale (stage_bench #[path] include) | field.rs:3-7 | TRUE | examples/stage_bench.rs exists |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None (std-less enum). Unconditional mjpeg module (shipped).

## 9. Open questions for the next agent
- Downstream consumers: what does the CLI/renderer DO with `field_polarity` on VideoFrame metadata (weave? flag? drop)? The decoder is metadata-only by contract; whether the presenter honors it is pipeline_host/renderer territory (pipeline_host.rs:1554 maps a different `interlaced` flag — worth confirming field-polarity frames are not silently treated as full progressive frames downstream).
