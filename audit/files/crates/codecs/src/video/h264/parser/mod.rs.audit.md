# AUDIT: crates/codecs/src/video/h264/parser/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-R3 (credit W06-A08) |
| Date | 2026-11-23 |
| Lines / functions | 33 / 0 (module manifest: 5 `pub mod` decls + docs) |
| File verdict | REAL (manifest whose module map cross-verifies, with one overstated "shared" claim) |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:346 `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),` → h264/sw.rs:92 `use super::parser::nal::{AnnexBNals, AvcConfig, AvcNals, H264Error, NalHeader, NalUnitType, unescape_rbsp_into,}` → this tree's modules |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg/sed cross-checks of every module-map entry (nal/sps/pps/slice/sei) + consumer wiring; rg BitReader/RbspBitReader uses across the parser tree |

## 1. One-paragraph verdict
A 33-line module manifest, no logic. Every module-map entry cross-verifies against the code: nal owns Annex-B + AVC-length-prefixed splitting, avcC, EPB removal, NAL classification (`NalUnitType` nal.rs:98, `NalHeader` nal.rs:184) and a bit reader; sps owns §7.3.2.1.1 with cropping (`crop: Option<(u32,u32,u32,u32)>` sps.rs:151) and VUI (`VuiParameters` sps.rs:75); pps owns §7.3.2.2; slice owns §7.3.3 with the `SliceSpsInfo`/`SlicePpsInfo` seams (slice.rs:302+); sei owns §7.3.2.3/Annex D with all five advertised payloads (buffering_period/pic_timing/user_data_unregistered/recovery_point/filler, sei.rs:9-22). The tree is CLI-reachable via the software decoder. One defect: the map presents "the shared Exp-Golomb `nal::BitReader`" as the parser tree's single reader, but slice AND sei actually parse through a second, semantically-identical reader (`slice::RbspBitReader`, slice.rs:41, imported at sei.rs:47) — a documented-but-never-executed hoist (slice.rs:37-40).

## 2. What actually works (verified)
- Module map vs reality: all 5 declared modules exist and own exactly what the map says — verified by rg of each claimed symbol (annexb_nals nal.rs:310, AvcNals nal.rs:378, AvcConfig nal.rs:453, unescape_rbsp_into nal.rs:326, VuiParameters sps.rs:75, crop sps.rs:151/:290, SliceSpsInfo slice.rs:302, SEI payload set sei.rs:9-22).
- Conventions paragraph (mod.rs:21-27) matches the code: complete-NAL convention with `rbsp`-named params for unescaped input (parse_sei(rbsp) sei.rs:221, parse_slice_header(rbsp,...) slice.rs:738-743); caller-owned reusable unescape buffer exists (unescape_rbsp_into, used by sps.rs:183/pps.rs:82 and sw.rs:92-93); typed-error discipline backed by the fuzz module's totality tests (h264/fuzz.rs.audit.md, tests executed 2/2).
- Consumer wiring: sw.rs:92 imports five parser symbols; pipeline_host.rs:346 wires H264SwDecoder into the CLI.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/video/h264/parser/mod.rs:8-9 (contradicted by slice.rs:41 + sei.rs:47)
Code:
```
//!   emulation-prevention removal, NAL classification, and the shared
//!   Exp-Golomb [`nal::BitReader`] (card 5.F1a).
```
What is wrong: "the shared Exp-Golomb `nal::BitReader`" is misleading — the parser tree carries TWO Exp-Golomb readers with identical semantics (MSB-first, u(n)/ue(v)/se(v), bounds-checked typed errors): `nal::BitReader` (nal.rs:598, used by SPS/PPS) and `slice::RbspBitReader` (slice.rs:41, used by slice headers AND sei — `use super::slice::{BitReadError, RbspBitReader};` sei.rs:47). slice.rs:37-40 admits the duplication and says it "should hoist to `h264/parser/bitreader.rs`" at Lead merge — the hoist never happened. Why it matters: duplicated syntax-reader semantics = double maintenance surface for the most security-sensitive primitive in the parser; a bounds fix applied to one reader does not protect the other's callers (sei + slice). What correct looks like: one reader in the tree (or a shared bitreader module), or the map documenting the split honestly. Confidence: HIGH — Verification: read (all cites sed/rg-verified).

## 4. Per-function table
Not required (<100 LOC, no functions — 5 `pub mod` declarations: nal, pps, sei, slice, sps, all unconditional, all with existing per-file audits by earlier W06 agents).

## 5. Test honesty
No tests in this file. The suites behind its totality claim (fuzz smoke + parser unit suites) are real and green — h264 fuzz tests executed by me this run (2/2, exit 0); parser submodule suites green in baseline cargo-test-workspace.txt:645-740 range.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| nal owns Annex-B + length-prefixed + avcC + EPB + classification + Exp-Golomb reader | mod.rs:6-9 | REAL (contents) / FALSE ("shared") | annexb_nals nal.rs:310, AvcNals nal.rs:378, AvcConfig nal.rs:453, unescape nal.rs:326; but slice/sei use their own reader (F-001) |
| sps owns §7.3.2.1.1 incl. cropping + VUI basics | mod.rs:10-11 | REAL | crop sps.rs:151/:290, VuiParameters sps.rs:75 |
| slice owns §7.3.3 with minimal SPS/PPS context seams | mod.rs:13-14 | REAL | SliceSpsInfo slice.rs:302, parse_slice_header(rbsp, nal, sps, pps) slice.rs:738 |
| sei owns §7.3.2.3 + Annex D payloads (5 named) | mod.rs:15-17 | REAL | sei.rs:9-22 lists all five payload types |
| "steady-state parsing of a stream performs no allocation per NAL" (via offered reusable buffer) | mod.rs:24-26 | REAL (as offered) | unescape_rbsp_into + parse_with_buffer consumed by sw.rs; the FALSE variant of this claim lives in fuzz.rs's own docs (my h264/fuzz.rs.audit.md F-002) |
| "All parsers return typed errors: no panic, no hang" | mod.rs:26-27 | REAL (tested surface) | fuzz totality tests executed 2/2; baseline parser suites ok |

## 7. Dead / duplicated / unreachable code
None dead. Duplication: the two-reader split (F-001). All five submodules are reachable (sw path) or test-reachable.

## 8. Dependencies & flags
None. No feature gates on any `pub mod` in this manifest; the tree compiles in every shipped configuration.

## 9. Open questions for the next agent
- Who executes the bitreader hoist (slice.rs:37-40 TODO)? Until then, any Exp-Golomb bounds fix must be applied twice — flag for security review.
- nal.rs:598-600's own doc ("the shared primitive of every h264::parser syntax reader (SPS/PPS here; slice headers/SEI in sibling card 5.F1b)") repeats the same overstatement — belongs in a nal.rs follow-up (nal.rs already audited; pass this upstream).
