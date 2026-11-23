# AUDIT: crates/codecs/src/video/hevc/parser/pps.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Lines / functions | 245 / 4 (parse, parse_with_buffer, read, struct) |
| File verdict | REAL parser (spec-order verified; executed on committed bytes + fuzz) — thin test coverage, UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO — sole production caller chain `HevcDecoder` (hevc/decode.rs:346) is never constructed by any shipped code (rg over apps/playback-engine/transcode: no caller); the CLI has no "hevc" lane (baseline mp4-smoke: exit 65 "no decoder for `hevc`") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 video::hevc::parser` -> 19 passed (incl. `seam_derivation_matches_the_0100_sps` which parses this module on committed PPS_0100 bytes); `... video::hevc::fuzz` -> 2 passed; `./target/debug/deps/hevc_parser_corpus-04a983e2bb21396a --nocapture` -> 3 passed but ALL printed "SKIP hevc … corpus-v4 absent" |

## 1. One-paragraph verdict
The §7.3.2.3 syntax walk is correct: I checked field order, gates, and bounds one-by-one against the spec (ids ≤63/≤15, ref-idx defaults ≤15, diff_cu_qp_delta_depth gated on cu_qp_delta_enabled, tile block with non-uniform width/height payloads walked-not-retained, deblock control with offset pair only when not disabled, scaling-list walk, extension flag refused typed) — it matches. It executed green on the one committed real PPS (PPS_0100, 6 bytes) and survives the fuzz-lite survival tests. But coverage is thin for a parser this gate-rich: there are NO in-file unit tests (contrast the h264 PPS parser's `crafted_pps_covers_the_full_field_list` / `error_paths_are_typed`), the three corpus differential lanes that would exercise real-world PPS diversity all SKIP in this environment, and every `se(v)` field is taken with a truncating `as i32` cast from the reader's i64 — an adversarial `init_qp_minus26` of +2^31 silently wraps to i32::MIN instead of the doc-promised typed out-of-range error. And like the h264 slice parser, nothing shipped ever calls it.

## 2. What actually works (verified)
— `./target/debug/deps/codecs-bae0f1756cc0a809 video::hevc::parser` -> **19/19 pass**: `seam_derivation_matches_the_0100_sps` parses this module on the committed `PPS_0100 = [0x44, 0x01, 0xc0, 0x73, 0xc1, 0x89]` (nal.rs:987) and derives the slice seam from it; the slice-header tests construct PPS-derived seams end-to-end.
— `... video::hevc::fuzz` -> 2/2: `fuzz_codec_hevc` runs PicParameterSet::parse_with_buffer over every fuzzed NAL (fuzz.rs:52) — hostile bytes produce Ok or typed Err, never panic (empty/marker + hostile-wide lanes).
— Spec-order review (§7.3.2.3): the read() walk L127-205 matches the spec syntax element-for-element, including the three conditional blocks (tiles, deblock control, scaling list) and the trailing `pps_extension_present_flag` typed refusal.
— Zero-allocation steady state: `parse_with_buffer` reuses a caller-owned RBSP buffer (L111-123), with the allocating `parse` documented as the convenience form.

## 3. Findings
F-001 [S3]
Where: pps.rs:96-100 (doc) vs 138, 147-148, 185
Code:
    /// # Errors
    /// Typed [`HevcError`]s: wrong NAL type, header violations,
    /// emulation violations, truncation mid-syntax, out-of-range
    /// fields, `pps_extension_present_flag` set (`Unsupported {
What is wrong: "out-of-range fields" are typed errors only for the `ue(v)` fields (bounded_ue). Every `se(v)` field is unbounded and then TRUNCATED: `let init_qp_minus26 = r.se()? as i32;` (L138) — the reader's `se()` returns i64 (nal.rs:819) and can produce ±~2^31; `as i32` wraps +2^31 to i32::MIN silently. Same for `cb_qp_offset`/`cr_qp_offset` (L147-148) and the deblocking offsets (L185). Spec ranges (init_qp ±(26+6·B), cb/cr ±12, offsets ±12) are never enforced, so an adversarial PPS yields silently-wrong signed values, contradicting the doc. What correct looks like: bound the se fields (spec ranges are small constants) and return `HevcError::Invalid` on violation. Confidence: HIGH — Verification: read + read BitReader::se (i64, k ≤ 2^32 mapping).
F-002 [S3]
Where: pps.rs:126-244 (whole read fn) — no in-file #[cfg(test)]
What is wrong: zero unit tests in this file. The gate-rich paths — non-uniform tile width/height walk (L164-171), deblocking offsets (L184-186), scaling-list walk (L189-193), extension refusal (L200-205), every typed error — have NO executed coverage: the corpus lanes that would exercise them (0044/0100/0091) all SKIP in this environment (verified: all three printed "SKIP hevc … corpus-v4 absent"). The only executed PPS bytes are the single 6-byte PPS_0100 (which exercises the plain path only: no tiles, no scaling list, no offsets). Mutation reasoning: a mis-walked `skip_scaling_list_data` or a wrong tile-payload loop would not fail any test that runs here. Why it matters: the h264 twin parser (parser/pps.rs) carries crafted-stream tests for exactly these paths; this one shipped without them. What correct looks like: port the h264 PPS test pattern (BitWriter-crafted PPS per gate). Confidence: HIGH — Verification: rg (no tests in file), ran corpus binary (3 SKIPs), ran parser tests (19 pass, only PPS_0100-based).
F-003 [S4]
Where: pps.rs:142-146 vs spec constraint
Code:
        let diff_cu_qp_delta_depth = if cu_qp_delta_enabled {
            bounded_ue(r, "diff_cu_qp_delta_depth", 4)?
What is wrong: bound 4 is looser than the spec's SPS-coupled constraint (0..=CtbLog2SizeY−MinCbLog2SizeY, max 3 with legal SPS values); likewise tile counts bounded at 30 vs level-dependent maxima (20 columns/22 rows). Parse-level-only defense — a value 4 parses where the spec would reject — but without SPS context exact validation is impossible and the struct keeps the value verbatim for downstream checks. Acceptable seam; documenting it would be better. Confidence: MEDIUM — Verification: read (spec constraint from H.265 7.4.3.1.1 recollection; no cross-SPS check possible in this card by design).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PicParameterSet (struct, 34 fields) | 10-89 | REAL | — | field docs carry spec ranges |
| parse | 101-104 | REAL | — | allocating convenience form, documented |
| parse_with_buffer | 111-123 | REAL | — | NAL type check, EP removal, finish() trailing-bit check |
| read | 126-244 | REAL | S3/F-001, S3/F-002, S4/F-003 | spec-exact order; se truncation; no unit tests; loose bounds |

## 5. Test honesty
| Test (elsewhere) | Would it fail if code were broken? | Reason |
|---|---|---|
| hevc parser slice::seam_derivation_matches_the_0100_sps (ran, ok) | PARTIAL | exercises the plain path on one real PPS; none of the conditional gates |
| hevc fuzz empty_and_marker/hostile_wide (ran, ok) | YES for panics/hangs only | survival, not correctness |
| corpus 0044/0100/0091 lanes | NO (in this env) | all SKIP (corpus absent) — would be YES with corpus |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "out-of-range fields" are typed errors | pps.rs:98-99 | PARTIAL/FALSE for se fields | F-001 |
| "Reference-list defaults and the deblocking/entropy fine print are retained where the slice header's minimal subset needs them" | pps.rs:3-5 | REAL | SliceSegmentPpsInfo::from_pps consumes exactly these (slice.rs:460-471, ran green) |
| `pps_extension_present_flag` → typed Unsupported | pps.rs:99-100 | REAL | code path exists (L200-205) — but UNTESTED (F-002) |

## 7. Dead / duplicated / unreachable code
- Production-unreachable: `HevcDecoder` (the caller chain) is never constructed by shipped code; the CLI provider's landed list has no "hevc" (baseline: exit 65 "no decoder for `hevc`"). This parser is library + test/fuzz only.
- The `parse` allocating form vs `parse_with_buffer` — both used (tests use parse, fuzz uses parse_with_buffer); not dead.

## 8. Dependencies & flags
In-crate (super::nal BitReader/unescape, super::vps bounded_ue, super::sps skip_scaling_list_data). No unsafe, no features.

## 9. Open questions for the next agent
- hevc/parser/sps.rs `skip_scaling_list_data`: if the walk is wrong, this PPS parser mis-syncs on any PPS with `pps_scaling_list_data_present_flag=1` — audit that walk and demand a crafted-PPS test before trusting this parser on real-world streams (F-002).
- If corpus-v4 ever lands in CI, the three SKIP lanes become the real differential; until then, treat this parser's conformance as verified-on-one-sample.
