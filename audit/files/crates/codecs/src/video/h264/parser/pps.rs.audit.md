# AUDIT: crates/codecs/src/video/h264/parser/pps.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2026-11-21 |
| Lines / functions | 374 / 6 (parse, parse_with_buffer, read, references, bounded_ue, bounded_se) + 3 tests |
| File verdict | UNREACHABLE (real, unit-tested parser; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO. Shipped h264 lane: `lumen open` → apps/cli/src/pipeline_host.rs:346 `H264SwDecoder::new` → crates/codecs/src/video/h264/sw.rs:92-94 imports **only** `parser::nal` (`AnnexBNals, AvcConfig, AvcNals, H264Error, NalHeader, NalUnitType, unescape_rbsp_into`) — rust_h264 does its own PPS. Every consumer of THIS file is dead or non-production: decode.rs:71/382 (`H264Decoder` — zero production callers, only crates/codecs/tests/h264_hw_path.rs + a comment at pipeline_host.rs:342; W06-A01 concurs), hw/vaapi/session.rs:95 + hw/videotoolbox/session.rs:94 (cfg-gated dead lanes, prior W06-A08 session), fuzz.rs:57 (fuzz harness), tests. Compiled unconditionally (`pub mod parser;` h264/mod.rs:54 — no feature gate, codecs/Cargo.toml keeps in-house modules unconditional). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 4 |
| Commands run | Read tool full file (verbatim authority); `rg -n "PicParameterSet"` (all consumers); `rg -n "H264Decoder\|from_setting"` (dead dispatcher); `rg -n "\.references\("` (test-only callers); `sed -n` on pps.rs:42-43,88-100,106,113-118,134-137,142-170,176-190,211-216,191/273/341 (quote/line verification); Read nal.rs:326-357 (unescape clears buffer), 596-739 (BitReader ue/se/finish semantics); Read sps.rs:604-615,664-687 (REAL_PPS bytes + real-corpus-pps test); Read tests/h264_parser_corpus.rs full (PPS tolerance); baseline greps: test-workspace.log.txt:604-606 (3/3 pps tests ok), :1058-1059 (corpus 2/2 ok) |

## 1. One-paragraph verdict
A genuinely spec-faithful parse of the PPS field list it claims (field order and every declared range re-derived against §7.3.2.2: ids 0..=255/0..=31, l0/l1 0..=31, qp/qs −26..=25, chroma −12..=12 — all correct), with a typed-refusal doctrine for FMO and three mutation-sensitive unit tests that ran green in the baseline. But it structurally rejects every PPS carrying the `more_rbsp_data()` High-profile extension (transform_8x8_mode_flag / scaling-matrix / second_chroma_qp_index_offset — the x264/ffmpeg High-profile default), misreporting the valid tail as corruption `Invalid{"h264.rbsp.tail"}`, and the real-bytes corpus differential cannot see this because it swallows PPS failures to `None` without asserting. Today that defect is caged: the shipped h264 lane uses rust_h264 and never calls this file, and the only callers are the doubly-dead hardware dispatcher/lanes. The parser is honest about FMO and dishonest about the far more common High tail.

## 2. What actually works (verified)
— Field order + ranges of `read` (pps.rs:88-130) re-derived against ITU-T H.264 §7.3.2.2 by hand: exact sequence, all `bounded_ue`/`bounded_se` limits correct — verified by Read; bit-level decode of the in-tree real fixture REAL_PPS `68 ce 3c 80` (sps.rs:607) reproduces every asserted field value.
— 3/3 unit tests executed and passed in baseline: test-workspace.log.txt:604-606 (`crafted_pps_covers_the_full_field_list`, `error_paths_are_typed`, `fmo_slice_groups_are_unsupported_not_silent`), re-confirmed in orchrun-test-workspace-nofailfast.log.txt:567-569 and allfeatures:567-569.
— Exp-Golomb safety inherited from BitReader: `ue` rejects >31 leading zeros (`h264.ue.overflow`, nal.rs:659-663), `se` maps k≤2^32 without overflow (nal.rs:676-685) — read-verified; the corpus property sweep (2000 mutation rounds, corpus.rs:289-310) exercises this without panic when the corpus is present.
— Buffer-reuse contract honored: `unescape_rbsp_into` clears `out` first (nal.rs:327), so `parse_with_buffer` on a shared buffer (corpus.rs:252-262) is sound — read-verified.
— FMO refusal is real and typed, pinned by `fmo_slice_groups_are_unsupported_not_silent` (pps.rs:338-373).

## 3. Findings

### F-001 [S2]
Where: crates/codecs/src/video/h264/parser/pps.rs:82-85 (and the finish() semantics it relies on, nal.rs:708-737)
Code:
```
        let mut r = BitReader::new(rbsp);
        let pps = Self::read(&mut r)?;
        r.finish()?;
```
What is wrong: `read` consumes only the base §7.3.2.2 field list and `finish()` then interprets the FIRST remaining `1` bit as the rbsp stop bit and demands all following bits be zero (nal.rs:717-737). Any PPS with the `more_rbsp_data()` extension — `transform_8x8_mode_flag`, `pic_scaling_matrix_present_flag`, `second_chroma_qp_index_offset` — contains a `1` bit before the real stop bit, so every such PPS is rejected with `Invalid { "h264.rbsp.tail" }`. The extension block always terminates in a `1` bit (every se(v)/u(1)=1 payload), and the real stop bit after it is also `1`, so the rejection is structural, not data-dependent: the parser can never return Ok for a PPS with extensions. The tool to detect this exists and is unused: `BitReader::more_rbsp_data` (nal.rs:690-699).
Why it matters: this is the single most common PPS shape in the wild — x264/ffmpeg High profile writes transform_8x8 by default. The module's own doctrine (lines 10-16: FMO is "parsed … then refused with a typed `Unsupported` … never a silent skip") is inverted here: the common case gets a **corruption-class lie** about valid data instead of a typed `Unsupported`/extension parse. Blast radius today is zero (consumers are the dead hw dispatcher/lanes — see header), but the file exists to feed exactly those lanes (its own doc, line 132-133: "the wiring check the hw-feed adapter will run per slice"), and the SPS twin already walks High-profile extensions (per prior W06-A08 vaapi session finding F-004), so the PPS side alone would brick High-profile hardware decode on Phase-7 activation with a misdiagnosed error.
What correct looks like: after `redundant_pic_cnt_present`, branch on `r.more_rbsp_data()`; either parse the extension block (transform_8x8 flag, optional scaling lists, second_chroma_qp_index_offset — fields the struct would carry) or return a typed `Unsupported { "h264.pps.high_extension" }`. Never report valid tail syntax as `h264.rbsp.tail` corruption.
Confidence: HIGH (code path traced bit-for-bit; finish() first-1-bit scan read-verified at nal.rs:708-737) — the §7.3.2.2 extension grammar is from the cited standard (domain knowledge; no spec copy in tree). Not executed (no harness in sandbox; consumers unreachable) — request-level: a crafted PPS with `…redundant=0, transform_8x8=1, scaling=0, second_chroma se(0), stop` yields `Invalid{"h264.rbsp.tail"}`.

### F-002 [S3]
Where: crates/codecs/src/video/h264/parser/pps.rs:42 (doc) vs :106 (read)
Code:
```
    /// `weighted_bipred_idc` (0..=2).
```
```
        let weighted_bipred_idc = r.bits(2)? as u8;
```
What is wrong: the doc declares the §7.4.2.2 range 0..=2, but the read is the only unbounded field in the list: value 3 (reserved/non-conforming) is accepted silently.
Why it matters: every other field goes through `bounded_ue`/`bounded_se` with a typed `OutOfRange`; a non-conforming PPS passes validation, so the "ids/QP offsets … out-of-range" error contract (line 62-64) has an undocumented hole, and any downstream consumer branching on bipred idc semantics (the dead hw feed would) receives a value with no spec meaning.
What correct looks like: `let idc = r.bits(2)?; if idc > 2 { return Err(H264Error::OutOfRange { field: "weighted_bipred_idc", value: idc as u64, max: 2 }) }`.
Confidence: HIGH — Verification: read (both lines).

### F-003 [S3]
Where: crates/codecs/tests/h264_parser_corpus.rs:143 (also 166, 261, 410) and :211-215, :223
Code:
```
    if let Some(nal) = cfg.pps.first() {
        parsed.pps = PicParameterSet::parse(nal).ok();
    }
```
```
    if let Some(pps) = parsed.pps.as_ref()
        && !pps.references(&[sps.seq_parameter_set_id])
    {
        why.push_str("pps->sps id mismatch ");
    }
```
What is wrong: the "real-bytes differential" never asserts a PPS parsed. Failures become `None` via `.ok()`; `assert_facts` only uses `pps` for a conditional mismatch check (skipped when `None`) and prints `pps={}` (`parsed.pps.is_some()`, :223) with no assert; `pass >= 10` counts SPS-only rows.
Why it matters: the differential suite is this parser's only real-bytes evidence, and it is structurally blind to the entire F-001 rejection class — if the parser rejected every real PPS in the corpus, the suite still prints `pps=false` on every row and passes 2/2. A "field-level AC" harness that cannot fail for the code under test is decoration. (Note also: "2/2 ok" in baseline is consistent with the loud-skip path — corpus presence is not provable from the captured `ok` lines.)
What correct looks like: assert `parsed.pps.is_some()` on at least the lanes whose profile implies a parseable PPS, and pin 2-3 PPS field values against ffprobe/`trace_headers` truth the way SPS fields are pinned.
Confidence: HIGH — Verification: read (full corpus file; no `pps.is_some()` assert anywhere).

### F-004 [S4]
Where: crates/codecs/src/video/h264/parser/pps.rs:176-180 and 186-188 (vs the corrected 211-216)
Code:
```
    /// A CABAC High-profile PPS from a real corpus encode
    /// (0014-muxed-av.mkv's avcC; Main-sized, weighted-pred off,
    /// QP offset −2): `68 fb e3 71 a0 9d 80 80 c0 c0`-class bytes
    /// are built below instead from the bit grammar, so the fixture
    /// stays self-describing. See `sps.rs` tests for a real-dump PPS.
```
```
        // qp se(-5)=codeNum9 "000101010", qs se(1)=codeNum2 "011",
        // chroma_qp se(0)=codeNum0... (se 0 is codeNum 1 → "010";
        // se −1 is codeNum 2 → "011"), deblock 1, constrained 1,
```
What is wrong: the comment block contradicts itself and the code. se(−5) is codeNum 10 (stated correctly 25 lines later at :211 — "se(−5) = codeNum 10"); "se(1)=codeNum2" is wrong (codeNum 2 = se(−1); se(+1) = codeNum 1, correctly stated at :213); "se 0 is codeNum 1" is wrong (se(0) = codeNum 0 → bit "1"). The doc block describes the fixture as "Main-sized, weighted-pred off, QP offset −2" while the built fixture is CABAC, weighted-pred ON, qp −5 — and "High-profile" (:176) vs "Main-sized" (:177) disagree.
Why it matters: the fixture is explicitly advertised as "self-describing" — a reader learning se(v) math from these comments learns it wrong.
What correct looks like: delete or fix the stale bit-walk lines to match the corrected comments at :211-216 and the actual field asserts.
Confidence: HIGH — Verification: read; se(v) mapping re-derived by hand (k odd → +(k+1)/2, k even → −k/2).

### F-005 [S4]
Where: crates/codecs/src/video/h264/parser/pps.rs:161-167
Code:
```
    let v = r.se()?;
    if v < min || v > max {
        return Err(H264Error::OutOfRange {
            field,
            value: v.unsigned_abs(),
            max: max.unsigned_abs(),
        });
```
What is wrong: the error collapses both the sign and the violated side. For the asymmetric `pic_init_qp_minus26` range (−26..=25), se(−40) and se(+40) both report `value: 40, max: 25`; the min bound (−26) is never stated and below-min vs above-max is indistinguishable.
Why it matters: `OutOfRange` is this parser's corruption diagnostic; a caller logging it cannot tell which bound tripped, and for min-violations the reported `max` is simply the wrong number (25 for a −26 floor).
What correct looks like: report the actual value and the violated bound (e.g. extend the variant or encode side in `field`), or at minimum use `min.unsigned_abs()` for below-min cases.
Confidence: HIGH — Verification: read.

### F-006 [S4]
Where: crates/codecs/src/video/h264/parser/pps.rs:35 (doc), 93-100 (refusal), 118 (construction)
Code:
```
    /// `num_slice_groups_minus1` (0 everywhere in practice; > 0 is
    /// refused as FMO-Unsupported).
```
```
            num_slice_groups_minus1: 0,
```
What is wrong: the struct field is vestigial — any value > 0 returns `Err` at :94-100, so the only constructible `Ok` value hard-codes `0` at :118. A field that can hold exactly one value in every successful parse is dead weight that implies variability that cannot exist.
Why it matters: API surface lies slightly (suggests the count is observable); every consumer must handle a field that is provably constant.
What correct looks like: drop the field and let the typed `Unsupported` be the only FMO observable (or keep it only if a future extension parser makes it meaningful — none exists).
Confidence: HIGH — Verification: read (all construct sites).

### F-007 [S3]
Where: crates/codecs/src/video/h264/parser/pps.rs:132-137
Code:
```
    /// Does this PPS reference an SPS we actually hold? (The wiring
    /// check the hw-feed adapter will run per slice.)
    #[must_use]
    pub fn references(&self, sps_ids: &[u8]) -> bool {
        sps_ids.contains(&self.seq_parameter_set_id)
    }
```
What is wrong: zero production callers. `rg -n "\.references\("` over the repo: pps.rs's own test (:248-249) and tests/h264_parser_corpus.rs:212 only. The doc admits it in future tense — "the hw-feed adapter **will** run" — and that adapter (`H264Decoder`, decode.rs) is itself production-dead (zero callers; W06-A01, re-verified by my grep: tests + one comment at pipeline_host.rs:342).
Why it matters: dead public API whose only justification is a lane that no shipped build constructs; it inflates the "wiring check" story without wiring.
What correct looks like: either the hw feed actually calls it per slice (Phase-7) or the method moves to the test module that exercises it.
Confidence: HIGH — Verification: rg over the whole tree.

### F-008 [S4]
Where: crates/codecs/src/video/h264/parser/pps.rs:191-200, 273-282, 341-350
Code:
```
        let push_ue = |bits: &mut Vec<bool>, v: u64| {
            let payload = v + 1;
            let len = 64 - payload.leading_zeros() as usize;
```
What is wrong: the identical 10-line `push_ue` bit-builder closure is pasted three times in this file's tests (once per test), the same triplication family as the `MustErr` helper flagged in obu.rs/h264_hw_path.rs/hevc_hw_path.rs by the prior W06-A08 session.
Why it matters: three copies of bit-grammar code in a parser test file is where an off-by-one fix lands in two of three copies.
What correct looks like: one `fn push_ue(bits: &mut Vec<bool>, v: u64)` (and a `bytes_from_bits`) helper in the test module.
Confidence: HIGH — Verification: read (all three copies byte-identical except binding).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PicParameterSet::parse` | 65-68 | REAL (test-executed) | — | thin wrapper; allocates a fresh RBSP Vec per call (PPS NALs are rare — acceptable) |
| `PicParameterSet::parse_with_buffer` | 74-86 | REAL | — | NAL-type gate (:76-80), unescape into caller buffer (cleared first — nal.rs:327, reuse-safe), finish() trailing-bits gate; F-001 lives in the finish contract |
| `PicParameterSet::read` | 88-130 | REAL | S2/S3 | field order + all declared ranges re-derived correct vs §7.3.2.2; stops before the more_rbsp_data() extension block (F-001); weighted_bipred_idc unbounded (F-002) |
| `PicParameterSet::references` | 134-137 | REAL (test-only) | S3 | trivially correct; production-dead (F-007) |
| `bounded_ue` | 142-152 | REAL | — | correct bound check, typed OutOfRange |
| `bounded_se` | 155-170 | REAL | S4 | correct accept/reject; error collapses sign/side (F-005) |
| test `crafted_pps_covers_the_full_field_list` | 182-250 | REAL | S4 | asserts all 15 fields; mutation-sensitive; stale comments (F-004), triplicated helper (F-008) |
| test `error_paths_are_typed` | 253-335 | REAL | — | pins wrong-NAL/forbidden/truncated/sps-id/qp OutOfRange; would fail if bounds checks were removed |
| test `fmo_slice_groups_are_unsupported_not_silent` | 338-373 | REAL | — | pins the FMO typed refusal; would fail if the >0 check were dropped |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| crafted_pps_covers_the_full_field_list | YES | every field asserted (e.g. swap l0/l1 reads → `l0=2` assert fails; wrong se mapping → qp −5 assert fails) |
| error_paths_are_typed | YES | exact `Err` equality on 4 paths + matches! on qp range — removing a bound check flips an assert |
| fmo_slice_groups_are_unsupported_not_silent | YES | exact `Err(Unsupported{…})` equality |
| h264_parser_corpus (real bytes) | **NO for PPS breakage** | `.ok()` + no `pps.is_some()` assert (F-003): a parser that rejects every real PPS still passes; also "ok" is consistent with the corpus-absent skip path, so even SPS coverage is not provable from the baseline lines alone |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "clean-room parse of the card's field list: ids, entropy-coding mode, bottom flag, slice-group count, reference-index defaults, weighted prediction, QP/QS offsets, deblocking/redundant flags" | pps.rs:1-6 | REAL | every listed field parsed with correct ranges (§4 row `read`); verified vs §7.3.2.2 by hand |
| "ITU-T H.264 §7.3.2.2" (as the syntax authority) | pps.rs:1 | SHALLOW | base syntax exact, but the §7.3.2.2 `more_rbsp_data()` extension block is neither parsed nor typed-refused — valid High tails are misreported as `h264.rbsp.tail` corruption (F-001) |
| "parsed, never claimed as a decode capability" (CABAC/CAVLC) | pps.rs:2-3 | REAL | `entropy_coding_mode` is a stored bool, no decode branch |
| FMO "refused with a typed `Unsupported` … honest and observable, never a silent skip" | pps.rs:8-16 | REAL (for FMO) | :94-100 + test pin; the same honesty is NOT extended to the High tail (F-001) |
| "h264 hw lane stays partial pending Phase-7 runtime activation" | docs/feature-matrix.toml row `codec-video-h264` (~:406, quoted by W06-A01) | REAL-UNPROVEN | consistent with the dead-dispatcher trace; this file is part of that partial lane |

## 7. Dead / duplicated / unreachable code
- Whole file UNREACHABLE from shipped entry points (header chain; sw.rs consumes only `parser::nal`).
- `references()` production-dead (F-007); `num_slice_groups_minus1` vestigial constant (F-006).
- `push_ue` triplicated in-file (F-008); mirrors the MustErr triplication family already on record.
- Near-mirror of hevc/parser/pps.rs (same parse/parse_with_buffer/read/references/bounded_* shape) — structural twin of a sibling codec, documented as deliberate; cross-checked for non-duplication with the hevc audit (see hevc pps.rs audit, same agent).

## 8. Dependencies & flags
No Cargo dependencies; `use super::nal::{BitReader, H264Error, NalHeader, NalUnitType, unescape_rbsp_into}` only. Module compiled unconditionally (`pub mod parser;` h264/mod.rs:54; codecs/Cargo.toml keeps in-house parser/hw modules ungated — prior session, Cargo.toml:71-72), so this dead lane ships compiled in every binary.

## 9. Open questions for the next agent
- F-001 execution proof: REQUEST-ORCH-RUN (logged in W06-A08 log): a one-off test parsing a crafted High-extension PPS would pin the rejection; no build allowed in sandbox.
- Does the hevc PPS twin have the same extension-grammar hole? (Answered in my hevc/parser/pps.rs audit — see that file.)
- h264_parser_corpus.rs itself is unaudited as a file; its SPS-only pass criterion (F-003) deserves a row in whichever wave owns tests/.
