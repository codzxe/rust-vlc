# AUDIT: crates/demux/src/mkv/ebml.rs
| Field | Value |
|---|---|
| Auditor | W05-A03 |
| Date | 2026-11-21 |
| Lines / functions | 670 / 70 items (2 budget consts, 51 ID consts, 2 vint parsers + VintError, EbmlReader×9, ElementHeader::declared_end, parse_header_at, 4 value decoders, 4 tests) |
| File verdict | REAL (with two wrong element-ID constants — one proven against on-disk bytes) |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → apps/cli/src/containers/mod.rs:57 `"matroska" => mkv::open` → MkvDemuxer::new → demux/src/mkv/mod.rs:164 `EbmlReader::new` → this file's reader + `parse_header_at` used by all mkv parsers; probe: media-probe phase5.rs MkvSignature 0x1A45DFA3. No feature gate — demux crate has no `[features]`, `pub mod mkv` unconditional) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 4 |
| Commands run | `rg` callgraph (EbmlReader::new/parse_header_at/read_* callers across mkv/*.rs — all 11 mkv files call this layer); `rg` ID constants repo-wide (dead-constant sweep); python3 independent EBML walker over 4 committed fixtures + 3 freshly ffmpeg-7.1.5-generated files (element-ID ground truth); `ffmpeg`/`ffprobe` 7.1.5 (pcm_s24le/pcm_s16le/flac/opus mkv generation + duration); `git show 52595bc8` (vint-length fix history); `sed -n` quote verification; baseline cites below |

## 1. One-paragraph verdict
The vint grammar, bounded reader, and typed value decoders are real, careful, and executed (baseline test-workspace.log.txt:1784-1787 runs this file's 4 unit tests; the fixture differentials that sit on top of it pass), and my independent python EBML walker re-parsed all four committed mkv/webm fixtures to EOF without disagreement. But the constant table — the file's second job — carries two wrong Matroska element IDs: `ID_BIT_DEPTH = 0x55EE` is MaxBlockAdditionID's ID, not BitDepth (0x6264 on disk, proven from the committed fixtures' own bytes and from fresh ffmpeg output), so `bit_depth` is `None` for every real file ever parsed and the A_PCM descriptor mapping is dead in effect; `ID_LANGUAGE_IETF = 0x24B7D` breaks the legacy+1 sibling convention the repo itself follows three times elsewhere. Five of the 51 ID constants are referenced nowhere. The module doc also overstates the depth guard and the bound-checking discipline (both are caller-side and partially absent).

## 2. What actually works (verified)
- Vint parsing vs on-disk truth: python3 independent EBML walker (written for this audit) parsed `tests/fixtures/generated/valid/mkv_h264_aac.mkv` (201 elements), `mkv_h264_flac.mkv` (132), `mkv_multitrack.mkv` (281), `webm_vp9_opus.webm` (206) to EOF — every ID/size vint agrees with this file's grammar, including the unknown-size and 4/8-octet forms.
- Executed: `audit/runs/baseline/test-workspace.log.txt:1784-1787` — `mkv::ebml::tests::{id_vints_reject_invalid_grammar, id_vints_roundtrip_all_four_lengths, size_vints_roundtrip_and_flag_unknown, typed_values_follow_the_grammar}` all `ok`; lines 1755-1783+ show the mkv suites that drive this reader (fixtures' packet counts == ffprobe, payload MD5 bit-exact — mkv_fixtures.rs:116,148).
- `git show 52595bc8` — the earlier 5-octet-ID-cap and 0x7F-vs-0x3F unknown-size bugs were genuinely fixed here; current code (ebml.rs:188-198, 230-238) is the corrected form.
- `read_prefix_into_at` (ebml.rs:361-370) is the real zero-alloc packet path: called at mod.rs:812 via `self.pool.fill(...)`; `crates/demux/tests/zero_alloc_read_chain.rs:981` documents the READ_STEP_MAX chop and is in the baseline test run.
- Overflow audit: all positions derive from vint widths (size ≤ 2^56-1, id ≤ 4 octets) + stream-bounded offsets, so `pos + 12` (ebml.rs:450) and `at + want` (ebml.rs:387) cannot reach u64 overflow; `declared_end` uses `checked_add` (ebml.rs:422). No panic path found in this file (no unwrap/expect/index/div — verified by reading every function).

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/mkv/ebml.rs:122-126 (used at header.rs:760-766)
Code:
```
/// `Audio` master + `SamplingFrequency`/`Channels`/`BitDepth`.
pub const ID_AUDIO: u32 = 0xE1;
pub const ID_SAMPLING_FREQUENCY: u32 = 0xB5;
pub const ID_CHANNELS: u32 = 0x9F;
pub const ID_BIT_DEPTH: u32 = 0x55EE;
```
What is wrong: 0x55EE is MaxBlockAdditionID's element ID (a TrackEntry child), not BitDepth. BitDepth's on-disk ID is 0x6264. Proof from this repo's own bytes: my python EBML scan of `tests/fixtures/generated/valid/mkv_h264_aac.mkv` shows Audio master children `0x9F` (Channels) `0xB5` (SamplingFrequency) `0x62 0x64` (BitDepth=32) — and a `0x55EE` (value 0) **outside** Audio as a TrackEntry child; same shape in mkv_h264_flac (0x6264=16), mkv_multitrack (0x6264=16/32), webm_vp9_opus (0x6264=16). Fresh ffmpeg 7.1.5 confirms: `ffmpeg -c:a pcm_s24le` → Audio `62 64 81 18` (BitDepth=24); `pcm_s16le` → `62 64 81 10`; the CLI-relevant claim "ffmpeg's matroska muxer omits it" (apps/cli/src/containers/mkv.rs:74-78) is false.
Why it matters: header.rs matches `ID_CHANNELS | ID_BIT_DEPTH` inside the Audio walk (header.rs:760-766); since 0x55EE never occurs inside Audio and 0x6264 is skipped as unknown, `bit_depth` is `None` for **every real file**. Consequences: (a) `codec_descriptor_for("A_PCM/INT/LIT", None, ..)` returns the raw string (header.rs:832-839) → the codec registry refuses typed → **the entire PCM-in-MKV lane is dead for every muxer** (ffmpeg and mkvmerge alike — both write 0x6264); (b) `TrackPrivate.bit_depth` (header.rs:173) is always None — the fact surface lies. The suite never caught it because no fixture carries a PCM track (fixtures: h264/aac, h264/flac, multitrack, vp9/opus) and header.rs's own tests pass `Some(16)` directly into `codec_descriptor_for` (header.rs:1014-1017) — the ID→value extraction is untested against on-disk bytes.
What correct looks like: `pub const ID_BIT_DEPTH: u32 = 0x6264;` — then re-run a PCM fixture lane (ffmpeg `-c:a pcm_s16le`) end-to-end; also fix the masking doc claim in apps/cli/src/containers/mkv.rs:74-78 (owned by that file's auditor).
Confidence: HIGH — Verification: ran (python EBML scan of committed fixtures + fresh ffmpeg 7.1.5 generation; both cited above).

### F-002 [S3]
Where: crates/demux/src/mkv/ebml.rs:113-115
Code:
```
/// `LanguageIETF` — the BCP 47 language element (preferred over the
/// legacy form when both exist).
pub const ID_LANGUAGE_IETF: u32 = 0x0002_4B7D;
```
What is wrong: The Matroska spec value is 0x22B59D; this repo itself follows the legacy+1 sibling convention in the two other IETF pairs — chapters.rs:88-90 (`ID_CHAP_LANGUAGE 0x437C` / `ID_CHAP_LANGUAGE_IETF 0x437D`) and tags.rs:84-86 (`ID_TAG_LANGUAGE 0x447A` / `ID_TAG_LANGUAGE_IETF 0x447B`) — while here `ID_LANGUAGE` (ebml.rs:112) is the correct 0x0022_B59C but the IETF sibling is 0x24B7D, matching no known Matroska element (not in the ffmpeg-written TrackEntry set observed on disk: d7/73c5/9c/22b59c/88/86/56aa/56bb/83/e1/55ee/63a2).
Why it matters: files from muxers that write LanguageIETF (mkvmerge, libmatroska-based writers) have the BCP 47 tag skipped as an unknown element; header.rs:672-674 falls back to the legacy `Language` element, so the degradation is metadata-only when both forms are present (the common case), total when only the IETF form is (spec-legal). Impact bounded → S3, but it is a wrong constant, not a tolerance choice.
What correct looks like: `pub const ID_LANGUAGE_IETF: u32 = 0x0022_B59D;` plus one fixture with a LanguageIETF-only track pinning the fallback.
Confidence: MEDIUM on the exact correct value (sandbox has no mkvmerge and ffmpeg 7.1.5 refuses to emit the element — measured: `-metadata:s:a:0 language=yue-Hant` writes only 0x22B59C); HIGH that 0x24B7D is wrong (sibling-convention evidence above).

### F-003 [S3]
Where: crates/demux/src/mkv/ebml.rs:133-140 (constants), mod.rs:718-752 + cluster.rs:58-60,217 (the missing consumer)
Code:
```
pub const ID_BLOCK_DURATION: u32 = 0x9B;
/// `ReferenceBlock` — its presence marks a non-keyframe in a
/// `BlockGroup`.
pub const ID_REFERENCE_BLOCK: u32 = 0xFB;
```
What is wrong: `ID_REFERENCE_BLOCK` (and `ID_BLOCK_DURATION`) are referenced by no parser (repo-wide `rg` sweep: zero uses outside ebml.rs). The BlockGroup walk in mod.rs:718-748 parses only `ID_BLOCK`; its comments claim "flags from ReferenceBlock presence" (mod.rs:720-721) and "the keyframe derivation is ReferenceBlock-absence" (mod.rs:751-752), and cluster.rs:58-60 documents "BlockGroup: derived" — but cluster.rs:217 computes `keyframe: simple && flags & FLAG_KEYFRAME != 0,` — hard-coding `keyframe=false` for every BlockGroup block. The derivation the three files describe does not exist.
Why it matters: per spec a BlockGroup without ReferenceBlock IS a keyframe; marking all BlockGroup packets non-keyframe feeds false keyframe metadata to the engine for muxers that use the BlockGroup form (mkvmerge-authored files; the committed corpus is ffmpeg-authored SimpleBlock-only, so no test sees it). Dead constants + comments asserting absent behavior = the breadcrumb trail of an unimplemented rule. (The packet-flag consumer impact is bounded: seek landings come from Cues, not packet flags — mod.rs:837.)
What correct looks like: in the BlockGroup walk, set the parsed block's keyframe = no-ReferenceBlock-seen; delete or use the two constants.
Confidence: HIGH — Verification: read (mod.rs:700-830, cluster.rs:217) + rg dead-constant sweep.

### F-004 [S4]
Where: crates/demux/src/mkv/ebml.rs:40-46
Code:
```
/// The parse-depth guard (MP §12, card 5.D1): the known parse shape
/// nests at most Segment → {Info,Tracks,Cues,Cluster} →
/// {TrackEntry,BlockGroup} → {Video,Audio,children} — 5 levels of
/// interest. Every data-driven master descent counts against this
/// bound; deeper declared nesting is corruption
```
What is wrong: "Every data-driven master descent counts against this bound" is false. `MAX_PARSE_DEPTH` (16) is consumed in exactly one place — chapters.rs:427 (nested ChapterAtom). tags.rs uses its own `MAX_TAG_NESTING = 8` (tags.rs:111) for nested SimpleTag; all other walkers are fixed-shape loops with no depth state (header.rs:851-854 admits the import is test-only "silenced").
Why it matters: a reader of ebml.rs believes one uniform guard exists; in reality two independent constants (16 vs 8) govern, and neither is enforced by this file. Doc overstatement on a security-adjacent bound.
What correct looks like: state the truth: chapters nests under MAX_PARSE_DEPTH=16, tags under MAX_TAG_NESTING=8, everything else is fixed-shape; or route both through one constant.
Confidence: HIGH — Verification: read + rg (`MAX_PARSE_DEPTH` users: chapters.rs:427 only; tags.rs:111).

### F-005 [S4]
Where: crates/demux/src/mkv/ebml.rs:15-24
Code:
```
//!   with an unknown size is a typed [`crate::mkv::error::bad_vint`].
//!   An unknown-size `Segment`/`Cluster` therefore clamps to the file /
//!   segment end (the streaming-EBML case, doc 04 §2.2's live gap).
//! - **Declared sizes are hints**: every size is checked against the
//!   enclosing bound before it is trusted into a read or a skip; the
//!   reader clamps at the stream's physical end and reports the real
//!   available prefix (the avi vertical's contained-lie discipline,
//!   applied to EBML).
```
What is wrong: two overstatements. (1) A non-master unknown size is minted by callers as `bad_structure`, not `bad_vint` (header.rs:410-411, 751-753: `return Err(error::bad_structure(child.data_pos))`). (2) "every size is checked against the enclosing bound" — masters clamp their own end (`parse_info`/`parse_tracks`: `declared_end().unwrap_or(bound).min(bound)`), but a **child's** declared end is passed unclamped as the read bound (e.g. header.rs:316 `read_prefix_at(len, child.data_pos, child_end)`), so a lying last-child size reads past the parent's end into following bytes; the only hard clamp is the physical stream end (`prefix_len`, ebml.rs:306-309, does `.min(self.length)`).
Why it matters: the doc promises contained-lie discipline the callers do not fully implement; the drift misleads the next maintainer auditing bomb-resistance. (Practical risk is low: reads stay stream-bounded and ceilings exist at header.rs:54/60 — MAX_CODEC_PRIVATE 4 MiB, MAX_TEXT_FIELD 4 KiB.)
What correct looks like: either clamp `child.declared_end()` to the parent's `end` in the walkers, or rewrite the claim to "masters clamp; children trust their declared end up to the stream end".
Confidence: HIGH — Verification: read (header.rs:396-470, 746-773; ebml.rs:306-309).

### F-006 [S4]
Where: crates/demux/src/mkv/ebml.rs:63-71 (representative of five dead constants: ID_VOID 64, ID_CRC32 66, ID_SEEKHEAD 71, ID_BLOCK_DURATION 137, ID_REFERENCE_BLOCK 140)
Code:
```
/// `Void` — padding, skipped wherever it appears.
pub const ID_VOID: u32 = 0xEC;
/// `CRC-32` — skipped (no verification; the card's CRC-less tolerance).
pub const ID_CRC32: u32 = 0xBF;
```
What is wrong: repo-wide `rg` shows zero references outside ebml.rs for these five. The skip behavior they document is real but implemented implicitly by every walker's default arm (`_ => {}`), so the names are pure documentation dressed as code.
Why it matters: dead constants imply a switch that does not exist; a reader auditing "is CRC-32 really skipped?" finds no handling site to check. Same class as F-003 (which has real behavioral consequence and is graded there).
What correct looks like: delete them or handle them explicitly in the walkers (one match arm each naming the tolerance).
Confidence: HIGH — Verification: rg sweep `for id in $(rg -o "pub const (ID_...)")` → 5 constants with zero external uses.

### F-007 [S4]
Where: crates/demux/src/mkv/ebml.rs:74-78
Code:
```
/// `Duration` — segment duration in scale units... in *seconds* (a
/// float; the spec's own unit quirk — see `header::SegmentInfo`).
pub const ID_DURATION: u32 = 0x4489;
```
What is wrong: the comment is self-contradictory ("in scale units... in *seconds*") and its pointer to `header::SegmentInfo` lands on a real bug in the other file: header.rs:430-436 stores the raw float into `duration_secs` with **no TimestampScale conversion**, while the on-disk unit is scale ticks — committed fixture `mkv_h264_aac.mkv` carries Duration=2023.0 with TimestampScale=1_000_000 and ffprobe reports 2.023 s (duration = value × scale ns). So `duration_secs` = 2023.0 for a 2-second file (1000× at the default scale), and the hand-built test helper bakes the wrong unit in (mkv_extras_wiring.rs:169-174 writes `secs.to_f64_bytes()` raw). Consumer impact today is bounded to the chapters beyond-duration window (mod.rs:455-461: ×1000 too large → the tolerant note never fires) because `duration_ticks()` has no production caller and the CLI mkv host reports span-end `Missing` (apps/cli/src/containers/mkv.rs:98-100, test at 473).
Why it matters: a wrong-unit duration field named `_secs` is a landmine — the first future consumer (UI duration, seek clamp) inherits a 1000× error. The garbled comment here is the only signpost, and it points at the wrong file as if the quirk were handled.
What correct looks like: `duration_secs = Some(value * f64::from(scale) / 1e9)` in parse_info, tests built with tick-unit payloads, and a one-line comment here stating the on-disk unit is TimestampScale ticks.
Confidence: HIGH — Verification: ran (ffprobe 2.023000 on the committed fixture; python decode of the Duration float = 2023.0) + read (header.rs:430-436, 124-135; mod.rs:455-461). The substantive fix belongs to header.rs's owner — see §9.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| READ_STEP_MAX | 35-38 | REAL | — | 64 KiB per-source-call budget; used at ebml.rs:316; mirrors avi/riff.rs:177. |
| MAX_PARSE_DEPTH | 40-46 | REAL (doc drift) | S4 | Value 16; only chapters.rs:427 enforces it (F-004). |
| 51 ID constants | 48-148 | REAL except 2 | S2/S3 | ID_BIT_DEPTH wrong (F-001), ID_LANGUAGE_IETF wrong (F-002); 5 dead (F-003/F-006); rest byte-verified against fixtures (§2). |
| VintId / VintSize | 152-173 | REAL | — | Plain data records. |
| parse_vint_id | 179-207 | REAL | — | Marker scan, 4-octet cap, NeedMore/Invalid split; correct per fixtures + tests; accepts non-minimal encodings (tolerant; harmless — unmatched ids skip). |
| parse_vint_size | 210-244 | REAL | — | 1..=8 octets, all-ones unknown detection incl. 0x7F FF 2-octet form (fix 52595bc8); value ≤ 2^56-1 keeps caller adds overflow-free. |
| VintError | 248-255 | REAL | — | Two-variant typed verdict. |
| EbmlReader::new | 274-292 | REAL | — | Refuses unknown-length/unseekable/empty sources typed Unsupported (streaming MKV out of scope — recorded, not hidden). |
| EbmlReader::length | 294-297 | REAL | — | Trivial getter. |
| EbmlReader::seek_to | 299-302 | REAL | — | Errors propagate; no clamp (callers bound — verified prefix_len does the real clamping). |
| prefix_len | 306-309 | REAL | — | The bound clamp: `.min(until).min(length).min(at+max)` + saturating_sub; correct. |
| read_bounded | 313-324 | REAL | — | Step-capped loop, clean-EOF break; no panic path. |
| read_prefix_at | 336-348 | REAL | — | Allocates `want` (≤ parent-declared, ≤ stream length); no own ceiling — callers hold MAX_CODEC_PRIVATE/MAX_TEXT_FIELD (verified header.rs:54,60). Short read = data, not error (documented + honored by callers). |
| read_prefix_into_at | 361-370 | REAL | — | Zero-alloc twin; the actual packet path (mod.rs:812 + zero_alloc_read_chain.rs). |
| read_exact_at | 379-394 | REAL | — | Bound precheck + typed truncation on short fill; `at+want` overflow impossible (§2). |
| ElementHeader::declared_end | 415-425 | REAL | — | checked_add; None for unknown size. |
| parse_header_at | 439-488 | REAL | — | 12-byte window cut by parent bound; NeedMore split into parent-clean-end vs stream-truncation is correct (verified by reasoning over `window < pos+12`); size not validated vs parent bound here (by design; see F-005 for the doc overstatement). |
| uint_from_be | 492-501 | REAL | — | 0..=8, empty=Some(0) spec quirk; >8 → None. |
| int_from_be | 505-518 | REAL | — | Two's-complement sign extension verified by hand for -1/-128/i64::MIN cases; shift-left on i64 never value-panics. |
| float_from_be | 523-534 | REAL | — | 0/4/8 grammar, else None. |
| utf8_from | 539-541 | REAL | — | None on invalid UTF-8 (callers skip with damage — verified header.rs title handling). |
| tests (4) | 547-670 | REAL | — | See §5. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| id_vints_roundtrip_all_four_lengths (548-575) | YES | Byte-exact expected VintId values for all four widths; mutating marker handling or the shift loop breaks at least one assert. |
| id_vints_reject_invalid_grammar (577-595) | YES | Zero head, 5-octet patterns (0x08/0x04), truncation each pinned to a variant; this is the test that caught the original cap bug (52595bc8 diff). |
| size_vints_roundtrip_and_flag_unknown (597-653) | YES | Unknown-size all-ones at 1/2/3 octets + 8-octet value; would fail on the pre-52595bc8 0x7F confusion. |
| typed_values_follow_the_grammar (655-669) | YES | uint/int/float/utf8 positives and negatives (9-byte uint, 3-byte float, invalid UTF-8). |
| Coverage gap (not a test) | — | EbmlReader (7 methods), parse_header_at, prefix_len, read_bounded: zero unit tests in this file; exercised only via the mkv integration suites (fixtures + wiring) — real files, but no direct unit pin on e.g. the NeedMore parent-bound-vs-stream split. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "vints, unknown-size — full parse + typed refusal" | docs/formats/container-matroska.md:21 | REAL | Fixture scan §2; baseline tests 1784-1787; ebml.rs:179-244. |
| "`Info` (0x1549A966): `TimestampScale`, `Duration` — full parse" | container-matroska.md:23 | SHALLOW (Duration unit wrong) | header.rs:430-436 stores ticks as secs; fixture 2023.0 vs ffprobe 2.023 s (F-007). |
| Unknown sizes "legal only for master elements… typed bad_vint"; "every size is checked against the enclosing bound" | ebml.rs:15-24 (module doc) | PARTIALLY FALSE | bad_structure in callers; child ends unclamped (F-005). |
| "Every data-driven master descent counts against this bound" | ebml.rs:40-46 | FALSE (as stated) | Only chapters uses it; tags uses 8 (F-004). |
| ReferenceBlock doc "its presence marks a non-keyframe" | ebml.rs:138-139 | PAPER-ONLY (in-tree) | No consumer; derivation absent (F-003). |
| "Unknown elements skipped by declared size, never fatal, no recursion" | ebml.rs:9-14 | REAL | All walkers are flat loops; verified in header/cluster/mod walk code. |
| feature-matrix row container-matroska-webm "implemented" (tracks/chapters/attachments/cues/lacing) | docs/feature-matrix.toml:129-139 | REAL-UNPROVEN at binary level (no baseline `lumen open` on mkv), REAL via test run | Baseline test-workspace.log.txt:1755-1787+ (demux mkv suites ok); rc-smoke was AVI-only by design (INDEX.md #20). |

## 7. Dead / duplicated / unreachable code
- Dead constants: ID_VOID (64), ID_CRC32 (66), ID_SEEKHEAD (71), ID_BLOCK_DURATION (137), ID_REFERENCE_BLOCK (140) — zero external references (F-003/F-006).
- `duration_ticks()` (header.rs:124) has no production caller — noted here because its only exercise is its own unit test with the wrong-unit input (F-007).
- No unreachable code: the whole file is on the reachable mkv path (§ Reachability).

## 8. Dependencies & flags
- Depends only on `media_types` (EngineError) + `plugin_api` (SourceStream/SeekOrigin) + `crate::mkv::error` — matches the C5 std-only, zero-dep claim (demux/Cargo.toml header, verified: no external deps). No feature gates anywhere on this path (demux has no `[features]`). No unsafe in this file (verified by reading; hygiene-counts baseline concurs for demux).

## 9. Open questions for the next agent
1. **header.rs owner**: Duration unit bug (F-007) — parse_info stores scale-ticks float into `duration_secs`; fix + regenerate the hand-built test vectors (mkv_extras_wiring.rs:169-174 writes seconds raw — the tautology that hid it).
2. **mod.rs/cluster.rs owner**: BlockGroup keyframe derivation absent (F-003) — cluster.rs:217 hard-codes `simple && …`; comments at mod.rs:720-721/751-752 describe code that does not exist. No BlockGroup fixture exists (all corpus files are ffmpeg SimpleBlock) — add one.
3. **apps/cli/src/containers/mkv.rs owner**: the "ffmpeg's matroska muxer omits BitDepth / mkvmerge-authored lanes map and decode" claim (mkv.rs:74-78) is FALSE by measurement — the real cause is F-001 here. Doc must be rewritten when the constant is fixed.
4. **tags.rs/chapters.rs owners**: two independent nesting guards (8 vs 16) — unify or document (F-004).
5. Would a shipped `lumen open` on an mkv file play? Untested at binary level in baseline (rc-smoke is AVI-only). REQUEST-ORCH-RUN: `./target/release/lumen open tests/fixtures/generated/valid/mkv_h264_flac.mkv` (expect tracks → Error or Playing per the decode registry — the flac/h264 decoders are not in the shipped set per baseline engine-info, so likely typed decode.unsupported with EXIT=0 — worth pinning for the parity table).
