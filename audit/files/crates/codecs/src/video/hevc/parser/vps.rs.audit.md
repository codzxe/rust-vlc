# AUDIT: crates/codecs/src/video/hevc/parser/vps.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2026-11-21 |
| Lines / functions | 427 / 9 (no tests in-file; corpus tests exercise the parser from tests/hevc_parser_corpus.rs) |
| File verdict | UNREACHABLE (real, mostly correct parser; no shipped consumer of any HEVC decode lane) |
| Reachable from shipped entry point? | NO — callers: `hevc/hw.rs:67`, `hevc/decode.rs:84` (import VideoParameterSet), fuzz + `tests/hevc_parser_corpus.rs:124-125,424-431`. No host dispatches `hevc` (grep `\"hevc\"` in apps/cli/src/pipeline_host.rs → none; the refusal list at :373 names no hevc decoder). Baseline: sample-hevc.mp4 → `Error` (INDEX.md corrected matrix: "no HEVC decoder"). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | Read full file; grep `hrd\|timing` in crates/codecs (found the AVC twin h264/parser/sps.rs:530-538 skip_hrd for comparison); grep vps in tests/hevc_parser_corpus.rs (:32,124,424-431 — no HRD-path coverage). |

## 1. One-paragraph verdict
A disciplined §7.3.2.1 walk with real spec discipline everywhere I could check it: the 16-bit reserved field enforced, the PTL bit layout (48-bit constraint word, presence-bit fillers) exactly per §7.3.3, per-field ue bounds, typed refusal of `vps_extension_flag`, and a zero-alloc steady-state entry. One genuine spec defect: the HEVC `hrd_parameters` walk reads the per-sub-layer CPB entries exactly once unconditionally, where the syntax (H.265 F.2.2, confirmed against the repo's own correct AVC twin) reads one `sub_layer_hrd_parameters` structure *per present HRD type* (NAL then VCL) and none when neither is present — any VPS with timing info and an HRD structure that signals 0 or 2 HRD types desynchronizes the walk and gets rejected typed. No test or corpus data covers an HRD-bearing VPS, so nothing caught it.

## 2. What actually works (verified)
— `ProfileTierLevel::read` bit-exactness: compat flags packed MSB-first (:83-86), the four source/constraint flags at bits 47..44 + 44 walked bits at 43..0 forming the exact 48-bit `general_constraint_indicator_flags` an hvcC record carries (:93-101) — cross-checkable against the corpus assertion `sps.profile_tier_level == vps.profile_tier_level` (hevc_parser_corpus.rs:426, baseline ok).
— Sub-layer presence bits `for i in 0..maxNumSubLayersMinus1` + reserved fillers `maxNumSubLayersMinus1..8` × 2 bits (:106-120) — the total is the spec's 16 presence/filler bits; hand-counted correct.
— Bounded walks everywhere: `bounded_ue` with per-field maxima (2047 / 31 / 1023 / 15), layer-set bit matrix ≤ 1023×64 bits, num_hrd ≤ 1023 — no unbounded work on crafted input.
— `vps_extension_flag` → typed `Unsupported { "hevc.vps.extension" }` (:403-410) — honest refusal instead of a mis-parse, exactly as the comment claims.
— Baseline: `tests/hevc_parser_corpus.rs` ran (test log: hevc suites ok), so the non-HRD walk is corpus-verified.

## 3. Findings
### F-001 [S2]
Where: crates/codecs/src/video/hevc/parser/vps.rs:217-226 (flags read at :184-186)
Code:
```rust
        let cpb_cnt_minus1 = bounded_ue(r, "cpb_cnt_minus1", 31)?;
        for _ in 0..=cpb_cnt_minus1 {
            let _ = r.ue()?; // bit_rate_value_minus1
            let _ = r.ue()?; // cpb_size_value_minus1
```
What is wrong: H.265's `hrd_parameters` (F.2.2) reads the CPB-entry structure once **per present HRD type** — `if( nal_hrd_parameters_present_flag ) sub_layer_hrd_parameters(1, …)` then `if( vcl_hrd_parameters_present_flag ) sub_layer_hrd_parameters(0, …)` — and reads **none** when both flags are 0 (only `cpb_cnt_minus1` itself is unconditional). This code reads exactly one entry-set per sub-layer regardless: (a) both types present → half the entries consumed → walk desyncs; (b) neither present (any_hrd=false) → phantom entries consumed → walk desyncs. Only the exactly-one-type case walks correctly. The repo's own AVC twin implements the per-type semantics correctly (`h264/parser/sps.rs:530-538`: one `skip_hrd` per present flag) — the HEVC side is the divergent one.
Why it matters: a valid VPS that signals timing + an HRD structure with 0 or 2 HRD types (broadcast HEVC commonly signals both NAL and VCL HRD) desynchronizes; the subsequent misread bits land on `vps_extension_flag`/trailing-bits checks → the valid stream is refused typed (`hevc.vps.reserved` / truncation). Blast radius today: zero (no shipped HEVC consumer); the day a lane exists, broadcast streams fail at configure.
What correct looks like: mirror the AVC twin — `if nal_hrd { sub_layer_hrd_entries(cpb_cnt_minus1)? } if vcl_hrd { sub_layer_hrd_entries(cpb_cnt_minus1)? }`, nothing when both clear.
Confidence: MEDIUM-HIGH — Verification: read + spec structure cross-checked against the repo's correct AVC implementation and ffmpeg's cbs_h265 hrd_parameters template (from knowledge; no network). No in-repo test pins an HRD-bearing VPS (corpus files carry none — grep of hevc_parser_corpus.rs shows no hrd/timing asserts), so UNVERIFIED by execution.
### F-002 [S3]
Where: crates/codecs/src/video/hevc/parser/vps.rs:8-9, 165-166, 293-295
Code:
```rust
//! HRD structures are *walked correctly* (bit-accurate) but
//! not retained — a summary flag is recorded (the card's "HRD note").
```
What is wrong: the module's headline claim "walked correctly (bit-accurate)" is false for the HRD corner in F-001. The claim is the only description of the walk's quality; a reader has no signal that HRD walking is untested against real HRD data.
Why it matters: doc-vs-code drift on exactly the subtle-Annex-C territory where the code is wrong.
What correct looks like: after fixing F-001, add a hand-built HRD-bearing VPS to the corpus suite (both-flags and neither-flags forms) and keep the claim.
Confidence: HIGH (as a doc-accuracy finding) — Verification: read.
### F-003 [S4]
Where: crates/codecs/src/video/hevc/parser/vps.rs:271-272, 340
Code:
```rust
    /// `vps_max_layers_minus1` (0..=62).
    pub max_layers_minus1: u8,
```
What is wrong: the doc declares the spec range 0..=62, but the 6-bit read accepts 63 with no typed rejection (contrast the enforced 0xffff reserved field and the per-field ue bounds elsewhere).
Why it matters: advisory field, no functional consequence — recorded for the doc-range/enforcement inconsistency pattern.
What correct looks like: one `if max_layers_minus1 > 62 { Err(OutOfRange …) }` or drop the range from the doc.
Confidence: HIGH — Verification: read (u(6) yields 0..=63; no check present).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SubLayerProfileTierLevel` (struct) | 15-23 | REAL | — | data; presence + level only, payload walked (documented) |
| `ProfileTierLevel::profile_name` | 62-72 | REAL | — | corpus spellings + verbatim fallback, never a guess |
| `ProfileTierLevel::read` | 74-162 | REAL | — | §7.3.3 verified bit-by-bit (see §2); sub-layer profile payload walked not retained |
| `skip_hrd` | 176-229 | REAL | S2 | F-001 per-HRD-type CPB duplication missing; v1 conditional within_cvs form documented (:168-175) |
| `bounded_ue` | 231-246 | REAL | — | per-field maxima, typed OutOfRange |
| `SubLayerOrdering` (struct) | 250-260 | REAL | — | data |
| `VideoParameterSet::parse` / `parse_with_buffer` | 308-333 | REAL | — | alloc/reuse pair; wrong-NAL-type + finish() (trailing-bits) checks |
| `VideoParameterSet::read` | 336-426 | REAL | S4 | reserved-0xffff enforced; ordering-info first-sub-layer logic correct (:351-356); layer-set matrix walked bounded; timing triple retained; num_hrd loop correct incl. cprms inference (:396); extension refused typed; F-003 range nit |

## 5. Test honesty (no in-file tests; external coverage assessed)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| hevc_parser_corpus VPS parse + PTL-equality (:124-125, :424-431) | PARTIAL | real x265 bytes, SPS-vs-VPS PTL equality — but no corpus VPS carries timing/HRD, so F-001's path has never executed against real data |
| (nothing covers skip_hrd with 0 or 2 HRD types) | NO | the desync classes are untested — a hand-built fixture would fail today |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "HRD structures are walked correctly (bit-accurate)" | :8-9 | FALSE (HRD corner) | F-001 — 0-or-2-HRD-type streams desync |
| "the chosen [fixed_pic_rate_within] form is pinned by the corpus" | :168-175 | REAL-UNPROVEN | corpus VPS bytes exist and parse, but the corpus carries no HRD-bearing VPS (grep), so the pin covers only the absent-HRD path |
| "unlisted idcs report verbatim — never a guess" | :60-62 | REAL | `format!("idc {other}")` |
| "`vps_extension_flag` set → Unsupported" | :306-307 | REAL | :403-410 |

## 7. Dead / duplicated / unreachable code
Unreachable as a whole (no HEVC lane). Not duplicated: the AVC `skip_hrd` (h264/parser/sps.rs:571) is a different syntax (E.1.2) — but its correct per-type CALLER (:530-538) is the model F-001 says HEVC should follow.

## 8. Dependencies & flags
Only `super::nal` (BitReader, HevcError, NalHeader, unescape_rbsp_into) — in-crate. No unsafe, no feature gates; compiled unconditionally (including wasm tree-shakes, where it is decode-refuser weight per apps/web/Cargo.toml:94-96 comment).

## 9. Open questions for the next agent
- W06-A02 owns hevc/decode.rs + hw.rs: if any lane ever consumes `timing`/`hrd_present`, F-001's walk accuracy becomes load-bearing — fix before wiring.
- W06-A09 owns h264/parser/nal.rs parts: confirm `BitReader::finish()` (trailing-bits check) is what converts F-001's desync into a typed error rather than a silent misparse (assumed from :331).
