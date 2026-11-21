# AUDIT: crates/codecs/src/video/hevc/parser/sps.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-21 |
| Lines / functions | 604 / 13 (2 enums/structs impls, 2 free fns, 9 methods) |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO (see chain below) |
| Findings | S0: 0 S1: 0 S2: 3 S3: 3 S4: 1 |
| Commands run | rg "SeqParameterSet|hevc" over apps/ crates/ (callers); sed -n quotes; rg in baseline test-workspace.log.txt (hevc corpus ran, 3 passed) |

## 1. One-paragraph verdict
This is a real, mostly careful clean-room H.265 SPS syntax walk with typed errors
and checked arithmetic — but it is dead code in every shipped binary, and its
three "skipped correctly (bit-accurate walk)" claims (module doc line 6-8) are
each FALSE for a class of VALID streams: the scaling-list walker reads 112
matrix entries where the spec has 20; the VUI bitstream-restriction walker
reads a VVC-only `tiles_fixed_structure_flag` bit that H.265 does not define;
and the inter-RPS walker returns a wrong `NumDeltaPocs` (admitted in its own
comment). None of these shapes is in the corpus the tests run, so the corpus
assertions pass while the walk is wrong. The parser is reached only by
`HevcDecoder` (never registered in the CLI's decoder table) and by tests/fuzz.

## 2. What actually works (verified)
— `ChromaFormat::from_idc/idc/sub_size` match H.265 Table 6-1 and §7.4.3.2.1
SubWidthC/SubHeightC (read, spec cross-check; Monochrome/4:4:4 → (1,1), 4:2:0 →
(2,2), 4:2:2 → (2,1)).
— `cropped_size` (383-398) uses `checked_mul`/`checked_sub` and rejects
non-positive results — no overflow/panic path (read).
— Baseline `cargo test -p codecs` executed the corpus assertions:
`audit/runs/baseline/test-workspace.log.txt:1109-1117` — hevc_parser_corpus
3 passed (0044/0100/0091 lanes).

## 3. Findings

### F-001 [S2] scaling_list_data walks a phantom second dimension — 112 entries read where H.265 §7.3.3 defines 20
Where: sps.rs:497-519
Code:
```rust
pub(crate) fn skip_scaling_list_data(r: &mut BitReader<'_>) -> Result<(), HevcError> {
    for i in 0..4u32 {
        let step = if i == 3 { 3 } else { 1 };
        let mut j = 0;
        while j < 6 {
            let mut k = 0;
            while k < 6 {
                if r.bit()? == 0 {
```
What is wrong: H.265 §7.3.3 scaling_list_data() is a SINGLE inner loop:
`for(matrixId = 0; matrixId < 6; matrixId += (sizeId==3)?3:1)` — 6 matrices for
sizeId 0..2 and 2 for sizeId 3 (20 total). This code nests TWO loops (j and k)
both stepping: sizeId 0..2 read 6×6 = 36 entries each (108), sizeId 3 reads
2×2 = 4 → 112 entries. Every matrix entry consumes ≥1 bit (pred flag) plus
ue/se payloads, so the reader is desynced by 92+ entries' worth of bits.
Why it matters: any conformant SPS/PPS that carries scaling-list data (x265
`--scaling-list`, broadcast encoders) is mis-parsed: every field read after the
scaling list (amp/sao flags, RPS section, VUI, trailing-bit check) is garbage
→ spurious typed errors or silently wrong retained fields. The same function is
shared by the PPS (pps.rs:192), so PPS scaling lists corrupt too. The module
doc's "skipped correctly (bit-accurate walk)" (sps.rs:6-8) is FALSE.
What correct looks like: one loop `let mut matrix_id = 0; while matrix_id < 6 { …; matrix_id += step; }`
Confidence: HIGH — Verification: read + spec cross-check (H.265 7.3.3); no
corpus lane exercises it (baseline corpus test only covers 0044/0100/0091).

### F-002 [S2] VUI bitstream_restriction reads a VVC-only flag — one-bit desync on valid H.265 streams
Where: sps.rs:585-595
Code:
```rust
    if r.bit()? == 1 {
        // bitstream_restriction: one flag triplet + five ue fields.
        let _ = r.bit()?; // tiles_fixed_structure_flag
        let _ = r.bit()?; // motion_vectors_over_pic_boundaries_flag
        let _ = r.bit()?; // restricted_ref_pic_lists_flag
        let _ = bounded_ue(r, "min_spatial_segmentation_idc", 4_095)?;
```
What is wrong: H.265 Annex D bitstream_restriction() contains exactly TWO flags
(`motion_vectors_over_pic_boundaries_flag`, `restricted_ref_pic_lists_flag`)
followed by five ue(v) fields. `tiles_fixed_structure_flag` belongs to
H.266/VVC's VUI, not H.265's. The comment even says "one flag triplet" — the
spec has a pair.
Why it matters: any H.265 stream with `bitstream_restriction_flag=1` (x265
writes it under conformance presets; VLC plays these daily) makes the walk
consume one extra bit before five ue fields — everything after VUI
(`sps_extension_present_flag`, `rbsp_stop_bit` check via `r.finish()`) is read
from the wrong position → spurious typed parse errors on valid streams.
What correct looks like: drop the `tiles_fixed_structure_flag` read; two flags
+ five ue(v).
Confidence: HIGH — Verification: read + spec cross-check (H.265 D.2.3 vs
H.266 D.2.3); the corpus lanes passed, so none of them sets the flag.

### F-003 [S2] skip_st_ref_pic_set returns the reference's NumDeltaPocs — wrong count for predicted RPS sets that drop entries (admitted in-comment)
Where: sps.rs:466-478
Code:
```rust
        for _ in 0..=ref_deltas {
            let used = r.bit()? == 1;
            if !used {
                let _use_delta = r.bit()?;
            }
        }
        // NumDeltaPocs for a predicted set keeps the reference's
        // count minus the unset deltas — the exact value needs the
        // use_delta bookkeeping we deliberately do not retain; the
        // walk itself consumed the right bits either way. Returning
```
What is wrong: for an inter-RPS-predicted set, NumDeltaPocs[stRpsIdx] =
NumDeltaPocs[RefRpsIdx] minus the entries with `used_by_curr_pic_flag==0 &&
use_delta_flag==0`. Returning `ref_deltas` unmodified (line 478) over-counts
whenever the predicted set drops entries; every LATER set that references this
one then iterates its `used`/`use_delta` loop the wrong number of times and
desyncs the bit walk. The comment's claim "the walk itself consumed the right
bits either way" is false — the walk for later sets depends on this count.
Why it matters: valid streams using inter-RPS prediction with dropped entries
(a normal encoder output shape) mis-parse or error after the RPS section; the
module doc's "bit-accurate walk" claim is FALSE for them.
What correct looks like: count `used||use_delta` entries during the loop and
return that.
Confidence: MEDIUM-HIGH — Verification: read + spec §7.3.8 logic; no corpus
lane pinned (would need a crafted stream to execute).
Mutation test reasoning: with a corpus lane whose predicted sets shrink, the
retained post-RPS fields (VUI/extension flag) or `finish()` would fail —
today's tests cannot fail because no lane has the shape.

### F-004 [S3] pix_fmt_name emits non-ffprobe spellings for >8-bit and full-range monochrome
Where: sps.rs:63-80
Code:
```rust
    pub fn pix_fmt_name(self, full_range: bool, bit_depth: u8) -> String {
        let depth = if bit_depth > 8 {
            format!("{bit_depth}le")
        } else if full_range {
            "j".to_string()
```
What is wrong: for 10-bit 4:2:0 this produces `yuv10le420p`; ffprobe (the doc's
own "comparison surface") spells `yuv420p10le`. Full-range monochrome produces
`grayj`, which does not exist in ffmpeg's pixel-format vocabulary (monochrome
is `gray`/`gray16le`).
Why it matters: the doc contract ("ffprobe's `pix_fmt` spelling") is violated on
every 10-bit HEVC stream; the corpus manifest carries an `x265_10bit` lane but
hevc_parser_corpus.rs's three lanes (0044/0100/0091) are all 8-bit, so the bug
is invisible to the tests. Today the only caller is the corpus test itself;
when HEVC probing lands this poisons user-visible output.
What correct looks like: `format!("yuv420p{depth}le")` family; `gray` for
monochrome (full range is a separate color-range attribute, not a pix_fmt).
Confidence: HIGH — Verification: read; callers = rg (only
hevc_parser_corpus.rs:183).
Severity note: S3 not S2 only because no shipped code path consumes it today.

### F-005 [S3] ASPECT_TABLE carries idc 1..4 only; idc 5..254 (incl. 16:9's idc 8) silently dropped, and the doc comment misstates the fallback
Where: sps.rs:107-110 and 532-537
Code:
```rust
const ASPECT_TABLE: [(u8, u16, u16); 4] = [(1, 1, 1), (2, 12, 11), (3, 10, 11), (4, 16, 11)];
```
```rust
        } else {
            sar = ASPECT_TABLE
                .iter()
                .find(|&&(t, _, _)| t == idc)
                .map(|&(_, w, h)| (w, h));
        }
```
What is wrong: Table D-1 defines idc 1..=16 + 255. Unlisted idcs here (5..254)
resolve to `sar = None` — dropped, not "resolved through the explicit
Extended_SAR" as the comment (107-109) claims: Extended_SAR applies ONLY to
idc==255 (handled above). A 16:9-signalling stream (aspect_ratio_idc 8) loses
its SAR.
Why it matters: silent data loss on common streams, plus a doc comment that
misdescribes the behavior. No shipped consumer of `VuiParameters.sar` exists
today (rg over crates/apps: only tests read it), so the blast radius is
contained — S3.
What correct looks like: the full 17-entry Table D-1, or at minimum an honest
comment that unlisted idcs are dropped.
Confidence: HIGH — Verification: read + Table D-1 cross-check + rg callers.

### F-006 [S3] Dead slice-level branch in skip_st_ref_pic_set
Where: sps.rs:447-457
Code:
```rust
        if st_rps_idx == num_sets {
            // Slice-level extra set: delta_idx_minus1 rebases the
            // reference (§7.3.8: RefRpsIdx = stRpsIdx − delta_idx_minus1 − 1).
```
What is wrong: the only caller is the SPS loop (sps.rs:309-315) which passes
`i in 0..num_short_term_ref_pic_sets`, so `st_rps_idx == num_sets` is never
true — the `delta_idx_minus1` path (with its 63 magic bound) is dead code. The
doc at 433-434 says "slice-level RPS semantics are out of the honest subset"
while the function carries half of them.
Why it matters: dead weight with an untested bound that will be trusted when
someone wires slice RPS later; misleading API shape.
What correct looks like: remove the branch or wire the slice caller and test it.
Confidence: HIGH — Verification: rg "skip_st_ref_pic_set" → single caller at
sps.rs:310.

### F-007 [S4] ctb_log2_size_y doc comment is incoherent
Where: sps.rs:400-403
Code:
```rust
    /// `CtbLog2SizeY` — the log2 CTB size (6..=7 for the 64/32 CTBs
    /// the corpus's x265 uses; the range over all legal fields is
    /// 3+3+0 .. 3+3+3 = 6..=9? no: min 8 → log2 3, max 64 → log2 6;
    /// the fields cap it inside 6..=9... computed, not asserted).
```
What is wrong: the comment argues with itself and lands nowhere ("6..=9? no:
…"). With the field bounds (minus3 ≤ 3, diff ≤ 3) the computed range is
3..=9, i.e. CTB 8..512; H.265 level limits restrict real streams to 16..64.
Why it matters: a spec-derived invariant documented as a question mark invites
future misuse of the function's output.
What correct looks like: "CtbLog2SizeY = Log2MinCbSizeY + log2_diff…; 3..=9
from the parsed fields; level-constrained streams sit in 4..=6."
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ChromaFormat::from_idc | 29-37 | REAL | — | exact 0..=3 mapping, None otherwise |
| ChromaFormat::idc | 41-48 | REAL | — | correct inverse |
| ChromaFormat::sub_size | 54-61 | REAL | — | Table 6-1 exact |
| ChromaFormat::pix_fmt_name | 66-80 | SHALLOW | S3 | wrong spelling >8-bit / grayj (F-004); only test caller |
| SeqParameterSet::parse | 194-197 | REAL | — | allocating wrapper, delegates |
| SeqParameterSet::parse_with_buffer | 204-216 | REAL | — | NAL-type gate + unescape + finish; no panic path |
| SeqParameterSet::read | 219-373 | REAL | S2×2 | delegates to the buggy walkers F-001/F-002/F-003; rest of walk matches §7.3.2.2 (ordering-info first-index rule exact at 251-258) |
| SeqParameterSet::cropped_size | 383-398 | REAL | — | checked mul/sub, rejects ≤0 |
| SeqParameterSet::ctb_log2_size_y | 405-409 | REAL | S4 | computation correct (Log2MinCb+diff); doc incoherent (F-007) |
| SeqParameterSet::pic_size_in_ctbs_y | 418-428 | REAL | — | u64 math, typed overflow, div_ceil |
| skip_st_ref_pic_set | 435-492 | SHALLOW | S2 | wrong NumDeltaPocs (F-003); dead slice branch (F-006); non-inter arm exact (bounds 16/16 match spec-level practice) |
| skip_scaling_list_data | 497-521 | FALSE | S2 | 112 vs 20 entries (F-001); shared with PPS at pps.rs:192; dc-coef (`i > 1`) and coef_num (16/64) sub-rules are correct |
| read_vui | 524-604 | SHALLOW | S2 | tiles flag desync (F-002); SAR drop (F-005); timing/HRD/colour walks otherwise match Annex D |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| hevc_parser_corpus 0044/0100/0091 (baseline: 3 passed) | Only for the shapes those lanes carry | assertions compare profile/level/cropped size/pix_fmt vs ffprobe oracle; a scaling-list/VVC-flag/inter-RPS-delta mutation would NOT fail — no lane has those shapes; `parse(nal).ok()` at :128/:140/:163 swallows errors into `None` and only then fails via `assert_facts`'s "no SPS found" row |
| video::hevc::fuzz fuzz-lite (baseline: ok) | No | survival-only: `let _ = …parse_with_buffer` (hevc/fuzz.rs:51) asserts non-panic, not correctness |
| hevc_parser_corpus.rs:425 `cropped_size()==(1024,768)` on synthetic SPS | Yes for cropped_size | direct unit pin on a hand-built SPS |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Scaling lists … skipped correctly (bit-accurate walk)" | sps.rs:5-7 module doc | FALSE | F-001: 112 vs 20 entries |
| "short-term RPS structures … skipped correctly" | sps.rs:6-7 | FALSE (for shrinking predicted sets) | F-003 |
| "HRD structures are skipped correctly" | sps.rs:7-8 | REAL-UNPROVEN | skip_hrd lives in vps.rs (other agent); no independent verification here |
| "unlisted idcs fall back to the explicit Extended_SAR … never a guess" | sps.rs:89-91, 107-109 | FALSE | F-005: unlisted idcs → None, not Extended_SAR |
| "ffprobe's pix_fmt spelling (the corpus assertion's comparison surface)" | sps.rs:63-64 | FALSE for >8-bit | F-004 |
| HEVC decode = honest typed unsupported | hevc/mod.rs docs (cross-ref) | REAL | baseline mp4-smoke-open-session-hevc.txt: Opening → Error; CLI refuses with cli.codec_unavailable (apps/cli/src/containers/es.rs:463-484) |

## 7. Dead / duplicated / unreachable code
- Whole file UNREACHABLE from shipped entry points. Chain of proof:
  `lumen` CLI decoder table = `apps/cli/src/pipeline_host.rs:241-346` — arms
  mjpeg/flac/mp3/mp2/vorbis/opus/aac/alac/adpcm_*/av1/vp9/dnxhd/h264/pcm_* only;
  NO hevc arm. Raw `.265` open lane: `apps/cli/src/containers/es.rs:225`
  (`AnnexbCodec::Hevc => "hevc"`) → decoder_for miss → typed refusal, pinned by
  test `hevc_corpus_lane_opens_demuxes_and_refuses_decode` (es.rs:463) and
  baseline `mp4-smoke-open-session-hevc.txt`. The only consumers of the
  SPS-parsing `HevcDecoder` are tests (`rg -ln HevcDecoder` →
  crates/codecs/tests/hevc_hw_path.rs, hevc_parser_corpus.rs). demux
  (es/hevc.rs) and media-probe have NO dependency on `codecs`
  (crates/media-probe/Cargo.toml deps: media-types, plugin-api only).
- F-006 dead branch (slice-level RPS path).

## 8. Dependencies & flags
- No feature gate on this file: `crates/codecs/src/video/mod.rs:30-32` — `pub
  mod hevc;` unconditional. Compiled into every shipped `codecs` consumer; the
  CLI depends on `codecs` with default features (`apps/cli/Cargo.toml:51`).
- No unsafe, no deps beyond `super::nal` / `super::vps` primitives.

## 9. Open questions for the next agent
- vps.rs (other agent): does `skip_hrd` match Annex D? `bounded_ue` semantics
  (inclusive max?) — the bounds in this file assume inclusive.
- If a future wave registers `HevcDecoder` in the CLI, F-001/F-002/F-003 become
  user-facing SPS rejections; fix before that wave.
