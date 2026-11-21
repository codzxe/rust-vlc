# AUDIT: crates/codecs/src/audio/mp3/xing.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-21 |
| Lines / functions | 578 / 10 prod items + 2 test helpers + 5 tests |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO. Chain proven dead: shipped `.mp3` lane = `RawMp3Demuxer` (`crates/demux/src/es/mp3.rs:543-551` framing-only-detects and packet-SKIPS the Xing/Info/VBRI first frame; sets no `extra_data`) → `Mp3Decoder::configure` (`decoder.rs:461-466`: `extra_data` None → delay=padding=total_frames=0) → per-packet `decode` never calls `parse_xing`/`parse_vbri`. The only callers of `parse_xing`/`parse_vbri` are `decode_stream` (`decoder.rs:213/215`), which is called ONLY by tests (`crates/codecs/tests/mp3*.rs`), the fuzz harness (`mp3/fuzz.rs:131`), and a doc example (`mp3/mod.rs:110`). `facts::encode` (the blob `configure` needs) has zero non-test callers. |
| Findings | S0: 0 S1: 1 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n 'parse_xing\|parse_vbri\|decode_stream\|facts::encode' crates apps` (caller map); `sed -n` quote checks; `rg -n 'extra_data' crates/demux/src/es/mp3.rs` (empty — demuxer never seeds facts); `sed -n 535,650p` of es/mp3.rs (metadata-frame skip); `rg -n 'xing\|gapless' docs/feature-matrix.toml docs/formats/audio-mp3-mp2.md` (claims) |

## 1. One-paragraph verdict
The Xing/Info/LAME/VBRI parser is real code with genuine hand-crafted-fixture tests and correct spec arithmetic (verified by hand: side-info offsets 36/21/21/13 match `header.rs:458-477`; LAME 12+12-bit delay/packing at 21..24 is the published layout). But none of it is reachable from the shipped binary: the demuxer packet-skips the metadata frame and nobody produces the facts blob the shipped decoder consumes, so gapless/VBR metadata handling lives entirely in a test/fuzz-only free function. Worse, `parse_vbri` has an out-of-bounds panic: it validates only 14 bytes but indexes through `rest[21]`, violating `decode_stream`'s own doc promise "Damage is never a panic" — concretely constructible from the committed fuzz-harness input surface.

## 2. What actually works (verified)
— verified by hand-decode of `parse_lame` (xing.rs:305-315) against the test packer (tests :437-449): d=1105,p=340 → bytes 0x45,0x11,0x54 → `(0x45<<4)|(0x11>>4)=1105`, `((0x11&0xF)<<8)|0x54=340` ✓; `trim_window` clamp math ✓ (:478-480).
— verified by reading tests + baseline `test-workspace.log` (1681 passed): all 5 tests exercise the real parser on constructed frames, asserting exact field values and typed error codes.
— `ancillary_offset` arithmetic cross-checked against `side_info_size` (header.rs:458-477): 4+crc+{32,17,17,9} = {36,21,21,13} as documented (xing.rs:120-122).

## 3. Findings
### F-001 [S1]
Where: crates/codecs/src/audio/mp3/xing.rs:339-367
Code:
```
    let rest = frame.get(pos + 4..).unwrap_or(&[]);
    if rest.len() < 14 {
        return Err(corrupt(
```
…
```
    let toc_entries = u16::from_be_bytes([rest[14], rest[15]]);
    let toc_scale = u16::from_be_bytes([rest[16], rest[17]]);
    let toc_entry_size = u16::from_be_bytes([rest[18], rest[19]]);
    let toc_frames_per_entry = u16::from_be_bytes([rest[20], rest[21]]);
```
What is wrong: The truncation guard demands only 14 bytes but the fixed VBRI field block is 22 bytes (version/delay/quality/bytes/frames = 14, then toc_entries/toc_scale/toc_entry_size/toc_frames_per_entry = 8 more). For `rest.len()` in [14, 21] the indexes `rest[14]`..`rest[21]` are out of bounds → index panic.
Why it matters: `parse_vbri` is a public parser (`mp3/mod.rs:74`) whose whole job is hostile bytes; `decode_stream`'s doc (decoder.rs:184-186) promises "Damage is never a panic". Concrete construction (no fuzzing needed): MPEG-2.5 Layer-III mono 8 kbps 8 kHz header `FF F3 18 C0` → frame_size 72 (72·8000/8000), side-info base 13, place `VBRI` at frame offset 49 → `rest.len() = 72−53 = 19` ≥ 14 passes the guard, `rest[19]` (frame byte 72) panics. Reachable via the committed fuzz harness (`mp3/fuzz.rs:131` calls `decode_stream` on arbitrary bytes) and any caller of the pub API; not reachable from the shipped CLI playback lane (see F-002), which is the only reason this is not a shipped-input DoS.
What correct looks like: `if rest.len() < 22 { return Err(corrupt("mp3.vbri.truncated", …)) }` (or a `take`-style checked read mirroring parse_xing's field-by-field truncation errors), plus a test with rest.len() ∈ [14, 21].
Confidence: HIGH — Verification: read (index arithmetic hand-checked against a concrete header construction); panic NOT executed (no rustc in sandbox).

### F-002 [S2]
Where: crates/codecs/src/audio/mp3/xing.rs:1-46 (whole-file purpose) vs crates/demux/src/es/mp3.rs:543-551, crates/codecs/src/audio/mp3/decoder.rs:461-466, apps/cli/src/pipeline_host.rs:261-268
Code (es/mp3.rs:543-551):
```
        // The demux-level Xing/Info/VBRI rule (measured on the corpus:
        // ffmpeg's raw mp3 demuxer parses the first frame's metadata
        // block and does NOT emit it as a packet — `0139`'s VBR header
        // frame at 44 is packet-skipped, packet 1 starts at 1088).
        // Framing-only detection: the side-info arithmetic + the
        // 4-byte tag; the TOC/gapless payloads stay decode-domain.
        if self.frames_out == 0 && self.first_frame_is_metadata(h) {
```
What is wrong: The gapless/Xing surface this file implements is not wired into the shipped playback lane. In the shipped build the demuxer *detects* the metadata frame only to skip it (never extracting delay/padding/frames), sets no `extra_data` (`rg extra_data crates/demux/src/es/mp3.rs` → no hits), and `pipeline_host.rs:261` configures `Mp3Decoder` unseeded, so `configure` (decoder.rs:461-466) always sees delay=padding=total_frames=0 → no start trim beyond engine latency, `is_last` never true → no end trim. LAME gapless facts parsed by this file never affect any shipped output; they only run in `decode_stream` (tests/fuzz/bench/doc-test).
Why it matters: `docs/formats/audio-mp3-mp2.md:9` claims "status = **implemented** (probe/decode/seek/gapless both layers)" and feature-matrix row 649 notes "ID3v2/VBRI/Xing + gapless 5.A3". For the shipped binary the claim is false: a real LAME-encoded VBR MP3 played through `lumen open` keeps its encoder-delay/padding silence and gets no VBR-aware duration. The differential evidence (same gapless window vs ffmpeg) was gathered on the `decode_stream` lane, not the shipped lane.
What correct looks like: Either the demuxer (or the CLI host's facts pre-pass) harvests the first frame's Xing/LAME facts into the track's `extra_data` via `facts::encode` and `pipeline_host.rs:261` uses the `seeded()` lane — or the docs state gapless is test-lane-only.
Confidence: HIGH — Verification: read (caller chain traced end to end; `facts::encode` callers = tests only).

### F-003 [S3]
Where: crates/codecs/src/audio/mp3/xing.rs:97-118, 323-401 (VbriInfo fields)
Code:
```
    /// Total stream byte length.
    pub bytes: u32,
    /// Total frame count.
    pub frames: u32,
    /// TOC entries (flat bytes; entry size/scale per header fields).
    pub toc: Vec<u8>,
```
What is wrong: Everything `parse_vbri` extracts is parse-and-discard. `rg` finds no consumer of `VbriInfo`'s `bytes`/`frames`/`toc`/`toc_entry_size`/`toc_frames_per_entry`/`delay`: `decode_stream` stores it in `DecodedMp3.vbri` (decoder.rs:337) and nothing reads it — VBR seeking uses only `XingInfo`'s TOC (`seek.rs:102`), and VBRI's `delay` never feeds any gapless window. Additionally `toc_len` computes with `toc_entry_size.max(1)` (line 368) but the struct stores the raw `toc_entry_size` (0 stays 0), so a consumer doing `entries * size` arithmetic on the published field would compute a different TOC length than was parsed — an inconsistency trap even for a future consumer.
Why it matters: Dead data surface presented as a feature ("the rare Fraunhofer VBRI header", module doc :1-2, :38-46); the VBRI path costs an S1 panic (F-001) while contributing nothing any code uses.
What correct looks like: Either consume VBRI facts (seek planning, gapless) or cut the struct down to the detection bit the demux lane needs; make `toc_entry_size` the same value the length computation used.
Confidence: HIGH — Verification: read (repo-wide consumer grep).

### F-004 [S4]
Where: crates/codecs/src/audio/mp3/xing.rs:154-167
Code:
```
    let rest = match frame.get(pos + 4..) {
        Some(r) => r,
        None => {
            return Err(corrupt(
                "mp3.xing.truncated",
```
What is wrong: Dead error branch. `pos` comes from `find_magic`, which only returns positions where a full 4-byte window matched inside `frame`, so `pos + 4 <= frame.len()` always and `frame.get(pos + 4..)` is always `Some`. The `mp3.xing.truncated` error code is unreachable; truncation is actually caught one step later by `rest.len() < 4` (`mp3.xing.flags_truncated`, :168-178).
Why it matters: An unreachable typed-error path that suggests a coverage that does not exist; error-code consumers/testers can chase a code that never fires.
What correct looks like: Delete the match (use `frame[pos + 4..].get(..)`/plain slice) or assert the invariant; keep the single honest truncation error.
Confidence: HIGH — Verification: read (find_magic window arithmetic, :293-296).

### F-005 [S4]
Where: crates/codecs/src/audio/mp3/xing.rs:138-139, 150, 305-315
Code:
```
/// The scan accepts the magic at the spec offset or (tolerantly)
/// within the following 64 bytes — some encoders insert filler.
```
What is wrong: Two tolerance choices without bounds: (a) the 64-byte magic scan means a plain CBR first frame whose audio payload happens to contain `Xing`/`Info` at 0..64 bytes past the ancillary offset is misdetected as a metadata header — with nonzero flags (probability ~15/16 for random payload) the stream then either typed-errors out or drops frame 1 (`has_header_frame`, decoder.rs:216-223). ffmpeg/VLC match at the spec offset only. (b) `parse_lame` returns `None` when the tag is present but shorter than 24 bytes (`tail.get(21..24)?`, :310) — a truncated LAME tag is silently swallowed, no typed damage signal, contradicting the file's own "impossible claims are typed errors" stance (parse_xing doc :132-136).
Why it matters: False-positive Xing detection turns a playable CBR file into an error (realistic: crafted files, exotic encoders); silent LAME truncation hides damage the module doc says is surfaced.
What correct looks like: Spec-offset match first, tolerant scan only as an explicit fallback with a damage note; `parse_lame` should distinguish "no tag" from "tag truncated" (typed error or logged recovery).
Confidence: MEDIUM — Verification: read; false-positive probability argued, not measured (no corpus of CBR files with accidental `Xing` strings).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| XingInfo (struct) | 54-74 | REAL (unreachable) | — | Plain data; `frame_end` = offset+len ✓. |
| LameTag::trim_window | 86-95 | REAL | — | Saturating clamp + max(start) — correct; tested (:478-480). |
| VbriInfo (struct) | 97-118 | SHALLOW | S3 | Parse-and-discard fields (F-003). |
| ancillary_offset | 120-126 | REAL | — | 4+crc+side_info; matches header.rs tables; correct. |
| parse_xing | 128-287 | REAL (unreachable) | S4 | Field-by-field typed truncation errors ✓; zero-frames typed ✓; dead `mp3.xing.truncated` branch (F-004); tolerant scan (F-005a); magic re-read safe (pos+4≤len). |
| parse_head | 289-291 | REAL | — | Thin wrapper; `.ok()` swallows the header error detail into a generic message — acceptable for a first-frame guard. |
| find_magic | 293-296 | REAL | — | 64-byte window; `get` on short frames → None → Ok(None) ✓. |
| parse_lame | 298-315 | REAL | S4 | Bit extraction matches LAME spec (hand-verified); silent truncation swallow (F-005b); `Lavc` acceptance is ffmpeg-reality ✓. |
| parse_vbri | 317-401 | SHALLOW | S1 | OOB panic window [14,21] (F-001); toc_len bounded by real frame bytes (no unbounded alloc ✓); scale≠1 typed refusal ✓; zero-totals typed ✓; usize-overflow wrap on 32-bit targets with ~4.3 GB toc_len claims degrades to typed error (release wrap) — noted, not separately rated. |
| find_magic_vbri | 403-409 | REAL | — | 40-byte window from ancillary base; covers the fixed offset 36 for every layout (base ≤ 36). |
| tests: xing_frame | 415-435 | helper | — | Builds real MPEG-1 stereo frames only — mono/MPEG-2 offsets never exercised (see §5). |
| tests: lame_tag | 437-449 | helper | — | Correct packer (hand-checked against parse_lame). |
| parses_frames_bytes_toc_quality_and_lame | 451-481 | REAL test | — | Exact values; would fail on any field-extraction mutation. |
| info_magic_is_cbr_flagged | 483-494 | REAL test | — | Pins is_vbr=false for Info. |
| no_xing_is_none_and_zero_frames_is_typed | 496-514 | REAL test | — | None + two typed codes; truncation branch actually covered (flags_truncated). |
| truncated_field_claims_are_typed | 516-526 | REAL test | — | TOC truncation → typed. |
| hand_crafted_vbri_header_parses | 528-577 | REAL test | — | Full field assertions + zero-totals + cut-TOC; does NOT cover rest.len() ∈ [14,21] (F-001's window untested). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parses_frames_bytes_toc_quality_and_lame | YES | Exact scalar assertions on every field incl. LAME bits. |
| info_magic_is_cbr_flagged | YES | is_vbr + frames + offset asserted. |
| no_xing_is_none_and_zero_frames_is_typed | YES | Error codes asserted by string. |
| truncated_field_claims_are_typed | YES | Error code asserted. |
| hand_crafted_vbri_header_parses | PARTLY | Field mutations yes; but the guard bug F-001 is exactly in the untested length gap — the tests would NOT fail for the `<14`-vs-`<22` defect because every fixture has ≥ 22 bytes. Also all fixtures are MPEG-1 stereo, so a broken `ancillary_offset` for mono/MPEG-2 would still pass (the 64/40-byte magic scans mask the offset). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "probe/decode/seek/gapless both layers … implemented" | docs/formats/audio-mp3-mp2.md:9 | FALSE (for the shipped lane) | Shipped lane never parses Xing/LAME (F-002 chain); implemented+tested only on `decode_stream`. |
| "ID3v2/VBRI/Xing + gapless 5.A3" (matrix notes) | docs/feature-matrix.toml:649 | SHALLOW | Parser exists + tested; VBRI results unconsumed (F-003); gapless unwired in shipped lane (F-002). |
| "verified empirically on ffmpeg 7.1-authored fixtures: delay/padding at offsets 21..24" | xing.rs:299-304 | REAL-UNPROVEN | Bit layout hand-verified + test-pinned; ffmpeg-fixture differential exists (crates/codecs/tests/mp3_corpus_differential.rs) and passed in baseline run #3, but I did not re-run it. |
| "Damage is never a panic" (decode_stream doc, the consumer of this file) | decoder.rs:184-186 | FALSE | F-001 panic window on malformed VBRI. |
| "No known offline encoder … produces VBRI, so the VBRI path is unit-tested against hand-crafted headers" | xing.rs:43-46 | REAL | Honest; and the hand-crafted tests are the only VBRI evidence (consistent with F-003's dead status). |

## 7. Dead / duplicated / unreachable code
- Whole file: unreachable from shipped entry points (F-002 chain) — verdict basis.
- `VbriInfo` parsed fields: unconsumed anywhere (F-003).
- `mp3.xing.truncated` error branch: unreachable (F-004).
- Duplication: `first_frame_is_metadata` (es/mp3.rs:595-631) re-implements this file's `ancillary_offset` arithmetic (side-info sizes duplicated as literals 17/32/9/17) — drift risk between crates (codecs vs demux cannot share the module by layering, but the constants could live in media-types).

## 8. Dependencies & flags
- `codec-mp3` feature (default ON per crates/codecs/Cargo.toml) — file compiles in every shipped build; only its *callers* are missing.
- Uses `super::error`, `super::header` (FrameHeader, side_info_size), media-types error taxonomy. No unsafe, no allows.

## 9. Open questions for the next agent
- decoder.rs / seek.rs auditor: does any shipped seek path use `XingInfo.toc` (VbrToc), or is VBR seek also test-lane-only? (My grep found consumers only in seek.rs internals + fuzz.)
- es/mp3.rs auditor: the demuxer skips the metadata frame *before* checking whether the stream is VBR — confirm no shipped surface reports VBR-ness from Xing (would extend F-002).
- Who should own the side-info-size constants to kill the cross-crate duplication (§7)?
