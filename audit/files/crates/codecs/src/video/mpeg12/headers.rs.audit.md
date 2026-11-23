# AUDIT: crates/codecs/src/video/mpeg12/headers.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2025-06-27 |
| Lines / functions | 1105 / ~20 fns+methods, 13 tests, 1 test Writer helper |
| File verdict | REAL-UNPROVEN (parser works per baseline tests; no shipped caller) |
| Reachable from shipped entry point? | NO — only consumer outside the module tree is crates/codecs/tests/mpeg12_corpus.rs (see F-004) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `sed -n` (quotes), `rg` for consumers, baseline citation |

## 1. One-paragraph verdict
This is a genuine, careful MPEG-1/2 sequence/picture/slice header parser: typed
errors, marker-bit enforcement, forbidden-code gates, spec-cited bit widths, and
a correct GCD reduction for frame-rate extension. It parses structure only — by
its own admission there is no decode here, and that admission is honest (no
Decoder impl exists in the tree). The defects are peripheral: a dead VBV-size
helper whose doc claims a byte scale the spec does not use, an unenforced
documented bound on `quantiser_scale_code`, reserved profile ids given
names, and the fact that nothing shipped ever calls this code — the module doc's
"probe/track-info consume" claim is contradicted by the dependency graph.

## 2. What actually works (verified)
— verified by audit/runs/baseline/cargo-test-workspace.txt:855-867 — all 13
`video::mpeg12::headers::tests::*` pass (`... ok` per line). I did not re-run
(cargo-build-budget); baseline is the orchestrator's own run.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/video/mpeg12/headers.rs:241-245
Code:
    /// VBV buffer size in 16·1024-byte units (the spec's scale).
    #[must_use]
    pub fn vbv_buffer_bytes(&self) -> u32 {
        (self.vbv_buffer_size_value + 1) * 16 * 1024
    }
What is wrong: 11172-2 §6.2.2.1 / 13818-2 §6.2.2.1 define the VBV size in
units of 16384 **bits** (2048 bytes), not 16384 bytes; the "+1" and the
bits-vs-bytes scale are both unsupported by the doc's "the spec's scale" claim.
Why it matters: anyone consuming this helper gets a value ~8x the real buffer.
Blast radius is small: `rg` shows the only caller is this file's own test
(line 748), which re-states the implementation (`assert_eq!(seq.vbv_buffer_bytes(), 20 * 16 * 1024)`) — a tautology that pins the formula, not the spec.
What correct looks like: either delete the dead helper or compute per spec and
verify against one ffprobe `buffer_size` value from the corpus lane.
Confidence: MEDIUM — Verification: read (spec text not in repo; UNVERIFIED against 11172-2 paper copy / ffmpeg source — no network).

F-002 [S3]
Where: crates/codecs/src/video/mpeg12/headers.rs:583-584, 651-652
Code:
    /// `quantiser_scale_code` (5 bits; 1..=31, 0 forbidden).
    pub quantiser_scale_code: u32,
...
        let mut r = BitReader::new(payload);
        let quantiser_scale_code = r.read_bits(5)?;
What is wrong: the field doc declares 0 forbidden but `SliceHeader::parse`
never checks it — the same function *does* gate `slice_number == 0`, so the
omission is inconsistent, not deliberate. Why it matters: a damaged slice with
quantiser 0 flows into `SliceHeader` as Ok, contradicting the documented
contract. What correct looks like: an `OutOfRange` gate like the aspect/frame-
rate/coding-type ones 20 lines above it do.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/codecs/src/video/mpeg12/headers.rs:73-82
Code:
    match (profile_and_level >> 4) & 0x7 {
        1 => Some("High"),
        2 => Some("Spatial"),
        3 => Some("SNR"),
        4 => Some("Main"),
        5 => Some("Simple"),
        6 => Some("Multiview"),
        7 => Some("4:2:2"),
        _ => None,
    }
What is wrong: per 13818-2 Table 8-2, non-escape ids 110/111 are reserved;
Multiview and 4:2:2 are escape-coded profiles (the function's own doc cites
0x85 = escape for 4:2:2). Naming reserved ids invents classifications for
streams no encoder should produce. Why it matters: mislabeled probe facts on
reserved values. What correct looks like: `_ => None` for 6/7 unless an
ffprobe differential lane demonstrates ffmpeg names them (the escape case at
lines 67-71 is documented with a measurement; this one is not).
Confidence: LOW — Verification: read; UNVERIFIED against ffmpeg's actual
mapping (no ffmpeg source in sandbox).

F-004 [S3]
Where: whole file; dependency proof via `rg`
What is wrong: `rg "video::mpeg12|mpeg12::facts"` over crates/ (non-test)
returns exactly one hit — crates/codecs/tests/mpeg12_corpus.rs:29. No
production crate (media-probe, video-engine, playback-engine, commands)
references mpeg12; media-probe/Cargo.toml does not even depend on the codecs
crate. The entire 1105-line parser (plus startcode/bitreader/facts/error) is
compiled into the shipped codecs crate and reached by nothing but tests.
Why it matters: mod.rs L26-27 claims "the one-pass total stream scan
probe/track-info consume" — the dependency graph says nothing consumes it.
What correct looks like: either wire `Mpeg12Facts::scan` into probe or mark
the module experimental/blocked in the matrix.
Confidence: HIGH — Verification: ran rg (hits listed above).

F-005 [S4]
Where: crates/codecs/src/video/mpeg12/headers.rs:160-161
Code:
        let horizontal_size_value = r.read_bits(12)?;
        let vertical_size_value = r.read_bits(12)?;
What is wrong: 0x0 and odd sizes for 4:2:0 streams are accepted without
comment while every other forbidden value in this file gets a typed gate.
Why it matters: hostile-focus "sequence bounds" — a zero-size sequence header
becomes a 0x0 "fact" downstream. Blast radius: facts-only (no decoder).
What correct looks like: a documented decision either way.
Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| frame_rate_table | 17-30 | REAL | — | matches Table 6-4/6-5 incl. 1001 denominators |
| ChromaFormat::name | 45-53 | REAL | — | ffprobe spellings |
| profile_name | 65-83 | REAL | S4 | reserved ids 6/7 named (F-003); escape 0x85 modeling documented |
| level_value | 87-90 | REAL | — | low nibble |
| PictureCodingType::{letter,has_motion_vectors} | 107-127 | REAL | — | |
| SequenceHeader::parse | 158-213 | REAL | — | gates aspect 1..=4, frame_rate, marker, matrices |
| SequenceHeader::aspect_ratio | 217-225 | REAL | — | 221:100 for code 4 |
| SequenceHeader::frame_rate | 229-232 | REAL | — | |
| SequenceHeader::variable_bitrate | 236-239 | REAL | — | 0x3FFFF marker |
| SequenceHeader::vbv_buffer_bytes | 241-245 | SHALLOW | S3 | dead + spec-unit claim (F-001) |
| SequenceExtension::parse | 282-325 | REAL | — | id-nibble + chroma-00 gates, marker |
| SequenceExtension::frame_rate_with_ext | 333-351 | REAL | — | GCD reduction correct; guard on a>0 |
| GopHeader::parse / time_code | 378-409 | REAL | — | marker after minutes |
| PictureHeader::has_motion_vectors | 434-438 | REAL | — | |
| PictureHeader::parse | 445-487 | REAL | — | coding-type gate; P/B geometry per 11172-2 |
| PictureCodingExtension::parse | 531-572 | REAL | — | 4x f_code, flags — matches §6.2.3.1 layout |
| decode_macroblock_address_increment | 598-636 | REAL | — | 1..=33 matches Table B-1 as I recall it; UNVERIFIED against paper spec |
| SliceHeader::parse | 645-685 | REAL | S3 | quantiser-0 gate missing (F-002); `.ok()` on truncation mid-VLC is deliberate None, documented |
| tests::Writer | 693-726 | REAL | — | MSB-first mirror |
| 13 tests | 728-1104 | REAL | — | roundtrips; would fail on regression, cannot catch spec mis-modeling (self-built bits) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| sequence_header_roundtrip_ffmpeg_shaped | Yes (regression) / No (spec) | built by in-file Writer; vbv assertion re-states the formula |
| sequence_header_with_quant_matrices | Yes | 128 matrix bytes roundtripped |
| sequence_header_rejects_forbidden_codes | Yes | gates aspect 7 / frame_rate 9 |
| sequence_header_marker_violation_is_typed | Yes | marker=0 → Marker err |
| sequence_extension_roundtrip_and_chroma_gate | Yes | id/chroma gates |
| frame_rate_extension_refines | Yes | 25·2/1 = 50 |
| gop_header_roundtrip | Yes | |
| picture_header_i_p_b_shapes | Yes | incl. forbidden types 0/5/7 |
| picture_coding_extension_roundtrip | Yes | |
| slice_header_with_first_mb_increment | Yes | VLC 1, 011→2, escape→None |
| mb_increment_table_covers_one_through_thirty_three | Partially | checks bijection order, not prefix-freeness (comment admits "spot-checked") |
| frame_rate_table_matches_spec | Yes | literal table echo |
| coding_type_letters_and_motion_vectors | Yes | |
All tests pass per baseline (§2). None is ffprobe-anchored; that anchoring
lives in tests/mpeg12_corpus.rs (audited separately).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every parser takes a complete start-code unit payload ... clean-room from the two specs" | headers.rs:6-10 | REAL-UNPROVEN | structure matches spec layout; clean-room assertion unfalsifiable |
| "VBV buffer size in 16·1024-byte units (the spec's scale)" | headers.rs:241 | FALSE (per my spec reading) | F-001; UNVERIFIED against paper spec |
| "1..=31, 0 forbidden" for quantiser_scale_code | headers.rs:583 | FALSE (not enforced) | F-002 |
| "probe/track-info facts: ... Mpeg12Facts::scan" consumers exist | mpeg12/mod.rs:26-27 | FALSE | F-004 rg proof |

## 7. Dead / duplicated / unreachable code
`vbv_buffer_bytes` — dead in production (test-only caller). The whole file is
currently unreachable from shipped binaries (F-004).

## 8. Dependencies & flags
`super::bitreader`, `super::error`, `testkit::Must` (test). Module is
unconditional (codecs default features) — no feature gate, so it *compiles*
into the release lane; it is simply never called.

## 9. Open questions for the next agent
- mpeg12_corpus.rs (file 6): does any ffprobe lane pin `buffer_size`,
  which would settle F-001 empirically?
- Whoever audits facts.rs / media-probe wiring: confirm whether the
  "probe consumes facts" claim was ever true or is aspirational.
