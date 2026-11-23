# AUDIT: crates/codecs/src/video/hevc/parser/vps.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 427 / 8 (no in-file tests — verification lives in hevc corpus suites) |
| File verdict | REAL with 2 spec-conformance defects in rare syntax paths — UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO — `VideoParameterSet::parse` ← `hevc/decode.rs:308 HevcDecoder` + `hevc/hw.rs:93` (neither registered: apps/cli pipeline_host.rs has no hevc arm; baseline smoke: exit 65 "no decoder for `hevc`"). Also consumed by sps/slice/pps (shared walkers) — same unreachable hevc surface. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 hevc` → 21 passed (exit 0); `./target/debug/deps/hevc_parser_corpus-04a983e2bb21396a` → 3 passed (exit 0) |

## 1. One-paragraph verdict
The common-case walk is correct: I verified the §7.3.2.1 field order bit-by-bit (id u(4), base flags, max_layers u(6), max_sub_layers u(3), temporal nesting, 0xffff reserved enforced, PTL, ordering-info present-flag semantics with the first-index rule, layer-set membership bits, timing info, extension refusal), the general profile_tier_level (8+32+4+44 bits, 48-bit constraint word assembled at the right offsets for hvcC cross-checks), and the sub-layer reserved 2-bit fillers (only when maxSubLayersMinus1>0). 21 lib hevc tests + 3 corpus tests pass. But the module doc's claim that HRD structures are "walked correctly (bit-accurate)" is false in two corners my spec check found: the sub-layer profile walk is FOUR BITS SHORT of the H.265 syntax (it skips the four sub_layer_*_source/constraint flags — 84 bits walked where the spec's payload is 88), and the VPS hrd_parameters call passes `vps_max_sub_layers_minus1` where the spec passes literal `1`. Both live on paths absent from the 1-layer corpus (which is why 24 passing tests cannot see them), and the whole file is dead code in the shipped build (no HEVC decoder registered). The sub-layer PTL defect is shared with sps.rs (same `ProfileTierLevel::read`).

## 2. What actually works (verified)
— verified by executing `codecs-bae0f1756cc0a809 hevc` → "21 passed" and `hevc_parser_corpus-04a983e2bb21396a` → "3 passed" (corpus field assertions vs ffprobe); general-PTL and 1-layer VPS walks read-verified against H.265 §7.3.2.1/§7.3.3 syntax; ordering-info first-index rule (L352-358) and reserved-fillers guard (L111-120) traced correct.

## 3. Findings
F-001 [S3]
Where: vps.rs:124-135
Code:
```
            if profile_present {
                // Sub-layer profile payload: 2+1+5 + 32 + 44 bits,
                // walked (the general layer above is the retained
```
What is wrong: the sub-layer profile payload per H.265 §7.3.3 is 2+1+5 (space/tier/idc) + 32 compat + **4 flags** (progressive/interlaced/non_packed/frame_only) + 43 reserved + 1 inbld = 88 bits; this walk reads 8+32+44 = **84 bits** — the four sub-layer flags are never read. Any stream with `sub_layer_profile_present_flag[i]=1` desynchronizes the reader (subsequent level_idc and everything after is garbage → typed parse failure or silently wrong fields). The comment documents the wrong arithmetic as if it were the spec. Why it matters: temporal-sub-layer HEVC is realistic (broadcast scalability), and the same `ProfileTierLevel::read` is shared by sps.rs:223, so the SPS walk inherits the defect. The corpus (1-layer streams, per L54-55's own admission) cannot catch it. What correct looks like: read 8+32+4+44 (mirror the general PTL tail). Confidence: HIGH on the arithmetic (spec structure), MEDIUM on spec wording — Verification: read; spec cross-checked against the general-PTL shape the same author got right (48-bit tail), UNVERIFIED against an in-repo copy of H.265 (none available in sandbox).
F-002 [S3]
Where: vps.rs:393-399
Code:
```
            let num_hrd = bounded_ue(r, "vps_num_hrd_parameters", 1023)?;
            for i in 0..num_hrd {
                bounded_ue(r, "hrd_layer_set_idx", 1023)?;
                let cprms = if i > 0 { r.bit()? == 1 } else { true };
                if skip_hrd(r, cprms, max_sub_layers_minus1)? {
```
What is wrong: H.265 §7.3.2.1 calls `hrd_parameters( cprms_present_flag, 1 )` inside the VPS — the literal 1, NOT `vps_max_sub_layers_minus1` (ffmpeg's CBS parser passes the literal too). This code forwards the VPS-wide value, so any stream with VPS timing+HRD and `max_sub_layers_minus1 != 1` walks the wrong number of sub-layer HRD entries (e.g. the dominant `max_sub_layers_minus1 == 0` streams walk 1 entry where the spec walks 2), desynchronizing the tail. The module header's "HRD structures are *walked correctly* (bit-accurate)" is false for this case. Same corpus-blindness as F-001. What correct looks like: `skip_hrd(r, cprms, 1)`. Confidence: MEDIUM-HIGH — Verification: read; spec-literal from memory + ffmpeg-CBS recollection, no in-repo spec copy to cite (UNVERIFIED locally). Note the SPS call site (sps.rs:582, `skip_hrd(r, true, max_sub_layers_minus1)`) is the CORRECT form for the SPS VUI — the defect is only this VPS call.
F-003 [S4]
Where: vps.rs (whole file — no `#[cfg(test)]` module)
What is wrong: zero in-file tests; all verification is deferred to the corpus suites, which (by the file's own documentation) carry 1-layer streams only — exactly the inputs that cannot reach F-001/F-002's code. A hand-built multi-sub-layer VPS test would have caught the 4-bit shortfall. Confidence: HIGH — Verification: read + test inventory.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SubLayerProfileTierLevel / SubLayerOrdering / VideoParameterSet (data) | 15-296 | REAL | — | documented field semantics; honest bounds notes |
| ProfileTierLevel::profile_name | 62-72 | REAL | — | 5 named idcs + verbatim fallback (never guesses) |
| ProfileTierLevel::read | 75-163 | REAL | F-001 | general PTL verified correct (8+32+4+48-tail, 48-bit word at hvcC offsets); sub-layer walk 4 bits short; reserved fillers guard correct |
| skip_hrd | 176-229 | REAL (as a walker) | F-002 (call-site) | v1 conditional fixed_pic_rate form documented and pinned to corpus; sub-pic fields, scales, per-entry ue walks traced against F.2.2 — internally consistent |
| bounded_ue | 232-246 | REAL | — | typed OutOfRange helper, shared by sps/pps/slice |
| VideoParameterSet::parse / parse_with_buffer | 298-333 | REAL | — | NAL type gate, EP removal, finish() tail check; buffer-reuse form for steady state |
| VideoParameterSet::read | 336-426 | REAL | — | field order verified §7.3.2.1; 0xffff reserved enforced; ordering first-index rule correct; extension refused typed; layer-set bits walked; timing+HRD block (F-002) |

## 5. Test honesty
No in-file tests. External: 21 lib hevc tests + 3 hevc_parser_corpus integration tests pass (executed by me) — but they assert the 1-layer corpus shape, so F-001/F-002 paths are untested by construction. Would a corpus test fail if the sub-layer walk were broken? NO — no corpus stream has sub-layer profiles present. That is the coverage hole.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "HRD structures are *walked correctly* (bit-accurate)" | vps.rs:8-9 | FALSE for VPS-HRD streams with max_sub_layers_minus1 ≠ 1 | F-002 |
| "the sub-layer profile payload (84 bits) is walked, not retained" | vps.rs:13-14 | FALSE arithmetic (spec: 88 bits) | F-001 |
| "`fixed_pic_rate_within_cvs_flag` ... the chosen form is pinned by the corpus" | vps.rs:168-175 | REAL (honest spec-divergence record) | the v1-conditional form is documented with its rationale |
| bounds (buffering 15, layer sets 1023, etc.) | various | REAL | declared-range enforcement, typed OutOfRange |

## 7. Dead / duplicated / unreachable code
Whole module unreachable from shipped entry points (no HEVC decoder registered in apps/cli; hevc hw lane unwired — same class as h264/hw/capability.rs). No in-file duplication; the PTL/HRD walkers are the SHARED implementation for vps+sps (deliberate, documented).

## 8. Dependencies & flags
No unsafe; depends on super::nal (HEVC BitReader/NalHeader/unescape — the hevc twin of the audited h264 nal.rs). No feature gate (in-house parser rides along unconditionally but tree-shakes in wasm per codecs Cargo.toml comments).

## 9. Open questions for the next agent
- sps.rs owner: F-001 (shared `ProfileTierLevel::read`) affects SPS sub-layer streams too — one fix, two call sites.
- Whoever wires HEVC decode later: these two conformance bugs will bite the first real temporal-sub-layer or VPS-HRD stream; fix before registration.
