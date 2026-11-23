# AUDIT: crates/codecs/src/video/hevc/parser/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-R3 (credit W06-A08) |
| Date | 2026-11-23 |
| Lines / functions | 37 / 0 (module manifest: 5 `pub mod` decls + docs) |
| File verdict | REAL (every module-map and conventions claim cross-verifies against code) |
| Reachable from shipped entry point? | NO — the hevc tree's only consumers outside itself are tests (rg: crates/codecs/tests/hevc_hw_path.rs, crates/codecs/tests/hevc_parser_corpus.rs); the CLI refuses hevc tracks pre-decode (`no decoder for \`{other}\`` pipeline_host.rs:376; baseline mp4-smoke: HEVC variant exit 65) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg cross-checks of every map entry (nal/vps/sps/pps/slice) + external-consumer scan (crates/, apps/) |

## 1. One-paragraph verdict
A 37-line module manifest, no logic, and — unlike its h264 twin — every claim is true without caveat: nal owns the two-byte header (`NalHeader` nal.rs:245), Annex-B + length-prefixed framing (:390/:463), EPB removal §7.4.2.1 (`unescape_rbsp_into` :410), NAL classification (`NalUnitType` :106), the hvcC record (:554) and the genuinely shared Exp-Golomb reader (slice.rs:18 `use super::nal::{BitReader, ...}` — hevc avoided the duplicate-reader mistake the h264 tree made); vps owns profile_tier_level and the HRD walker, and "shared" is true (sps.rs:223 reuses `ProfileTierLevel::read`); sps owns conformance-window arithmetic (:136), sub-layer ordering (:147), CTB/transform-size fields (:148-154) and VUI basics (:88); pps owns ids/tile-grid/loop-filter bits/init-QP (:13/:55-61/:66-72/:34); slice owns the minimal-honest §7.3.6.1 subset with the RPS machinery explicitly declared unparsed (slice.rs:10). The tree is real, tested (19 in-crate parser tests + corpus + fuzz-lite), and production-unreachable in this build (no HEVC decoder dispatched).

## 2. What actually works (verified)
- Module map vs reality: all five entries verified symbol-by-symbol (see §1 cites; all rg/sed-verified).
- "the shared Exp-Golomb [`nal::BitReader`]" — TRUE here: slice.rs:18 imports it from nal (contrast: h264's slice/sei use a private duplicate reader — my h264/parser/mod.rs.audit.md F-001).
- Conventions paragraph (:26-31): two-byte-header convention matches HEVC spec; reusable-buffer unescape exists (nal.rs:410) and is consumed by vps/sps/pps parse_with_buffer entries (vps.rs:318, sps.rs:204, pps.rs:111); typed-error totality pinned by the fuzz-lite suites + my executed hevc fuzz tests (2/2, exit 0).

## 3. Findings
None. What was checked and how: (a) all 5 module-map entries against rg-verified symbols in each submodule; (b) both "shared" claims (BitReader, profile_tier_level) against actual import/use sites — both true; (c) the conventions paragraph against function signatures (parse_with_buffer, unescape_rbsp_into, complete-NAL convention); (d) the slice "minimal honest subset" admission against slice.rs's field set (first_slice_segment_in_pic :133, dependent :141, slice_type :145, POC lsb :143, RPS unparsed :10); (e) external reachability via rg across crates/ and apps/ (tests only). Zero defects in the manifest itself.

## 4. Per-function table
Not required (<100 LOC, no functions — 5 `pub mod` declarations: nal, pps, slice, sps, vps, all unconditional; each submodule has an existing audit from earlier W06 agents).

## 5. Test honesty
No tests in this file. The totality claim is backed by the tree's own suites: 19 hevc::parser unit tests (verified via `--list` on the prebuilt binary), hevc::fuzz smoke tests executed by me (2/2, exit 0), plus tests/hevc_parser_corpus.rs and tests/hevc_hw_path.rs as external consumers.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| nal owns 2-byte header, both framings, EPB, classification, hvcC, shared BitReader | mod.rs:6-10 | REAL | NalHeader nal.rs:245, annexb_nals :390, LengthPrefixedNals :463, unescape_rbsp_into :410, NalUnitType :106, HvcCConfig :554, slice.rs:18 imports nal::BitReader |
| vps owns §7.3.2.1 incl. shared profile_tier_level() + HRD walker | mod.rs:11-12 | REAL | ProfileTierLevel vps.rs:12+, reused sps.rs:223; skip_hrd vps.rs:176 |
| sps owns §7.3.2.2 geometry/conformance-window/sub-layer/CTB/transform/VUI | mod.rs:13-15 | REAL | sps.rs:136 (conf_win), :147 (sub_layer_ordering), :148-154 (log2 CTB/transform fields), :88 (VuiParameters) |
| pps owns §7.3.2.3 ids/tile grid/loop-filter bits/init QP | mod.rs:16-17 | REAL | pps.rs:13, :55-61, :66-72, :34 |
| slice owns §7.3.6.1 minimal subset; RPS deliberately unparsed | mod.rs:18-22 | REAL | slice.rs:133/:141/:145/:143; RPS admission slice.rs:10 |
| "All parsers return typed errors: no panic, no hang" | mod.rs:30-31 | REAL (tested surface) | hevc fuzz tests executed 2/2 this run; fuzz-lite suites in nal.rs tests |

## 7. Dead / duplicated / unreachable code
Nothing dead or duplicated inside this tree's manifest. The whole hevc tree is production-unreachable in this build (no decoder dispatched; tests are the only consumers) — an honesty posture the parent hevc/mod.rs documents at length ("There is **no software HEVC decode here**").

## 8. Dependencies & flags
None. No feature gates on any `pub mod` here; the tree compiles in every configuration including the release lane.

## 9. Open questions for the next agent
- hevc got the shared-reader architecture right; the h264 tree's duplicate bitreader (my h264/parser/mod.rs.audit.md F-001) should be hoisted to match this pattern.
- If ADR-001's conformance-gated HEVC software track ever lands, re-run reachability for this tree.
