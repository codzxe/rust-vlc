# AUDIT: crates/demux/src/mkv/header.rs#part1 (lines 1-516)
| Field | Value |
|---|---|
| Auditor | W05-A06 |
| Date | 2026-11-21 |
| Lines / functions | 516 (chunk 1-516 of 1033; part2 = 517-1033 → W05-A07 — boundary cuts through `parse_track_entry` (495-704): my range owns its signature + declaration block 500-518; the element loop, video/audio masters, `codec_descriptor_for` and the tests module are part2's) / 17 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.{mkv,mka,webm}` → `crates/media-probe/src/signature/phase5.rs:110-111` Matroska signature "0x1A45DFA3 mirrors demux::mkv::probe" → `apps/cli/src/containers/mod.rs:57` `"matroska" => mkv::open(stream, opts)` → `crates/demux/src/mkv/mod.rs:342` `header::parse_ebml_header` / `:408` `header::parse_info` / `:412` `header::parse_tracks` → this file. EXECUTED in baseline: `mkv::header::tests` 4/4 ok (orchrun-test-workspace-nofailfast.log.txt:1754-1757), `tests/mkv_corpus.rs` 2/2 differential vs ffprobe ok (:2098-2101), `tests/mkv_fixtures.rs` 9/9 ok incl. `valid_fixtures_match_ffprobe_packet_counts`, `audio_payload_is_bit_exact_vs_ffmpeg` (:2142-2153), `mkv_editions_tags` 14 ok, CLI `containers::mkv::` 10 ok (:2803-2811, :2900)) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | full Read of 1-519 + boundary read 519-598 (parse_track_entry loop, part2 coherence) + tests module read (855-1033, part2's range, read for test-honesty context); `sed -n` on every quoted range; `rg "parse_ebml_header|parse_info|parse_tracks"` → mkv/mod.rs:342/408/412; mod.rs:400-470 read (caller: `.track(base)` collect + `by_number` HashMap); ebml.rs uint/float/utf8 conversion semantics read (:492-541 — zero-length uint→Some(0), float→Some(0.0), non-{0,4,8} float→None: grammar enforced); media-types ids.rs CodecDescriptor::new (:582-591: empty/control-char rejection); Matroska spec TrackType values checked (matroska.io elements table: 1 video, 2 audio, 3 complex, 0x10 logo, 0x11 subtitle, 0x12 buttons, 0x20 control, 0x21 metadata); baseline greps above |

## 1. One-paragraph verdict
Part 1 is the MKV identity/Info/track-table parse and it is the most heavily evidence-backed demuxer code audited so far in this wave: ffprobe-differential corpus, bit-exact payload fixtures, and 14 editions/tags tests all pass in the baseline, the EBML grammar guards (len ≤ 8, ceiling constants, unknown-size clamp-to-bound) are real and enforced, and the advisory-fields doctrine (Duration/DefaultDuration never trusted for control flow) is implemented as documented. Two genuine defects: the `TrackType` value map both misses two legal spec values (0x10 logo, 0x12 buttons — those tracks are skipped as "degenerate") and invents two non-spec values (0x22, 0x3F), with a comment that asserts the wrong mapping as fact and a test that enshrines it; and a per-track codec-descriptor rejection is mislabeled `no_tracks` and fails the entire file, breaching the module's own containment doctrine. Both are edge-input defects; the mainline lanes (video/audio/subtitle tracks, webm) are verified correct.

## 2. What actually works (verified)
- `parse_ebml_header` (273-378): 4-byte magic check → typed `not_matroska`; unknown-size EBML header → `bad_structure` (the unknown-size form is illegal for the header — correct); header_end > bound → typed truncation; DocType absent → `bad_structure` (identity required); MaxIDLength/MaxSizeLength grammar caps (≤4 / ≤8, ≠0) → `unsupported_ebml_limits`. Pinned by `probe_matches_the_ebml_magic`, `zero_head_is_the_typed_bad_vint`, `unknown_doctype_is_the_typed_refusal` (mkv_fixtures, baseline :2145-2147).
- `parse_info` (389-442): unknown-size Info clamps to bound (doc'd, 383); TimestampScale ≤ u32::MAX typed, zero stored-and-surfaced via `time_base()` (documented, 386-388); Title over-ceiling/invalid-UTF-8 → absent, never fatal — pinned by `info_title_and_track_uid_parse_into_the_models` (:1755) and `segment_info_defaults_and_duration_ticks` (:1756).
- `SegmentInfo::time_base`/`duration_ticks` (117-135): tick = scale ns exactly; duration float → ticks with finite/non-negative/i64-overflow guards — pinned by `segment_info_defaults_and_duration_ticks` (2.023 s → 2023 ticks; −1.0/NaN → None, baseline :1756).
- `parse_tracks` (456-491): per-entry containment (degenerate → damage note, others survive), MAX_TRACKS=64 ceiling damage-noted, ordinal = table position. Pinned by `info_title_and_track_uid_parse_into_the_models` (2 entries, ordinals 0/1, absent-TrackUID → spec default 0, :1755).
- `TrackEntry::track` (227-249): honest span model (start 0, end Missing — "Matroska declares no per-track length", 239-241); TrackId/StreamId from ordinal; language/title/flags passed through.
- Grammar constants (45-60): MAX_TRACKS 64, MAX_CODEC_PRIVATE 4 MiB, MAX_TEXT_FIELD 4 KiB — all used at the parse sites they document; `child_len` (257-259) typed on usize overflow.
- ebml layer cooperation verified: `uint_from_be(empty) = Some(0)`, `float_from_be` enforces {0,4,8} widths (ebml.rs:492-534) — zero-length legal EBML elements parse as spec defaults, malformed float widths are typed refusals.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/mkv/header.rs:138-149
Code:
```rust
/// What a Matroska `TrackType` value carries (the L0 vocabulary's
/// kinds; 0x20 logo, 0x21 buttons, 0x22 control and 0x3F metadata all
/// land in `Data` — the honest catch-all). Unknown values are `None`.
pub fn media_kind_from_track_type(track_type: u64) -> Option<MediaKind> {
    match track_type {
        1 => Some(MediaKind::Video),
        2 => Some(MediaKind::Audio),
        0x11 => Some(MediaKind::Subtitle),
        0x20 | 0x21 | 0x22 | 0x3F => Some(MediaKind::Data),
        _ => None,
    }
}
```
What is wrong: The Matroska spec's TrackType (element 0x83) values are: 1 video, 2 audio, 3 complex, **0x10 logo, 0x11 subtitle, 0x12 buttons, 0x20 control, 0x21 metadata**. This map misses 0x10 (logo) and 0x12 (buttons) — both fall to `None`, i.e. "unknown value" → the track is skipped as `degenerate-track-entry` with damage — while accepting 0x22 and 0x3F, which the spec does not define as TrackType values. The doc comment asserts the wrong mapping as fact: 0x20 is control (not logo), 0x21 is metadata (not buttons), 0x22/0x3F are nothing.
Why it matters: A legal MKV carrying a logo track or a button track (menu-ripped files) loses that track entirely at open — misreported as a degenerate entry rather than exposed as Data (the honest catch-all the comment promises). The test `track_type_maps_to_the_l0_vocabulary` (tests.rs, part2) pins the wrong behavior: it asserts 0x20/0x3F → Data and never tests 0x10/0x12 — the wrong map is enshrined, not challenged.
What correct looks like: `3 | 0x10 | 0x12 | 0x20 | 0x21 => Some(MediaKind::Data)` (complex/logo/buttons/control/metadata), delete 0x22/0x3F, fix the comment, extend the test with 0x10/0x12/3.
Confidence: HIGH — Verification: read + Matroska elements table (spec values); test body read (part2 range).

### F-002 [S3]
Where: crates/demux/src/mkv/header.rs:227-228
Code:
```rust
    pub fn track(&self, base: TimeBase) -> Result<Track, EngineError> {
        let codec = CodecDescriptor::new(&self.codec_descriptor).map_err(|_| error::no_tracks())?;
```
What is wrong: A per-track failure (the registry rejects the descriptor: empty or containing ASCII control characters — media-types/src/ids.rs:582-591) is mapped to `error::no_tracks()` — the whole-table refusal — and the caller aggregates it fatally: `crates/demux/src/mkv/mod.rs:446-450` `.map(|e| e.track(self.base)).collect::<Result<Vec<_>, _>>()?` fails the entire open.
Why it matters: The module's own containment doctrine (header.rs:13-16) promises "A degenerate `TrackEntry` … skips *that track* with damage — the other tracks and the file survive". `codec_descriptor_for` (part2) passes unknown CodecIDs through raw ("Unknown ids pass through raw — honest exposure", pinned by its test); a malformed-but-UTF-8 CodecID containing a control byte (e.g. `A_OPUS\x01`) therefore passes `parse_track_entry`, survives into the table, then kills the whole file at `track()` with a mislabeled error code claiming there were no tracks. Containment breach + misleading error vocabulary.
What correct looks like: Filter at table-build time (a descriptor-rejecting entry is a degenerate skip with damage), or map the error to a per-track typed refusal — `no_tracks` must mean the table is empty.
Confidence: HIGH — Verification: read + caller chain mod.rs:446-450 + CodecDescriptor::new validation read.

### F-003 [S4]
Where: crates/demux/src/mkv/header.rs:314-320 (vs 56-60, 421-428)
Code:
```rust
            ID_DOCTYPE => {
                if len <= MAX_TEXT_FIELD as usize {
                    let bytes = reader.read_prefix_at(len, child.data_pos, child_end)?;
                    facts.doctype = ebml::utf8_from(&bytes)
                        .ok_or_else(|| error::bad_structure(child.data_pos))?;
                    saw_doctype = true;
                }
```
What is wrong: The declared doctrine (56-60) says over-ceiling text "is corruption and the field is treated as absent". For DocType: invalid UTF-8 → immediate fatal `bad_structure`; over-ceiling → silently absent, which then dies at the end-of-parse `!saw_doctype` check (373-376) with a generic `bad_structure` that names the header, not the over-ceiling field. Title (421-428) is fully tolerant (absent on either condition). Three text fields, three policies.
Why it matters: Error specificity and doctrine consistency: a user with a 5 KiB DocType gets "bad structure at header" with no hint which field or why; the module's own tolerance rules are stated once but implemented three ways.
What correct looks like: One helper for ceiling+UTF-8 text fields with a declared per-field policy (identity fields fatal-with-reason, advisory fields absent), and a damage note naming the field for the over-ceiling case.
Confidence: HIGH — Verification: read of all three sites.

### F-004 [S4]
Where: crates/demux/src/mkv/header.rs:472-485 (aggregation at crates/demux/src/mkv/mod.rs:441-445)
Code:
```rust
        if child.id == ID_TRACK_ENTRY {
            if entries.len() >= MAX_TRACKS {
                damage.push((child_end - pos, "track-over-ceiling"));
            } else {
                match parse_track_entry(reader, &child, child_end)? {
                    Some(mut entry) => {
```
What is wrong: `parse_tracks` accepts duplicate `TrackNumber`s without check or damage note (the spec requires uniqueness). Downstream, `mkv/mod.rs:441-445` folds the entries into `by_number: HashMap<u64, u32>` — duplicates silently collapse, last entry wins, and blocks carrying that number route to one track while the other exists in the track list but can never receive data.
Why it matters: Malformed-file blast radius is misrouting plus a dead track, with no damage note anywhere — the module's damage-noting posture (its core selling point) has a hole exactly where two modules meet (header builds the table, mod routes by it).
What correct looks like: In `parse_tracks`, if a `TrackNumber` is already in the table, skip the later entry with a `duplicate-track-number` damage note.
Confidence: HIGH — Verification: read of both sites; block routing by `by_number` confirmed in mod.rs (cluster walk, another agent's chunk).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-23 | REAL | — | Doctrine statements verified against implementation (except F-002's breach) |
| `DOCTYPE_*`/`MAX_*` constants | 40-60 | REAL | — | All consumed at documented sites; ceilings damage-noted |
| `EbmlHeader` | 62-76 | REAL | — | Plain facts, advisory fields marked |
| `SegmentInfo` + `Default` | 78-107 | REAL | — | Default scale 1e6 per spec |
| `SegmentInfo::time_base` | 109-120 | REAL | — | tick=scale ns; zero scale → typed bad_structure via TimeBase |
| `SegmentInfo::duration_ticks` | 122-135 | REAL | — | finite/non-negative/overflow guards; round-to-nearest — pinned by test |
| `media_kind_from_track_type` | 138-149 | REAL | S3 | F-001: spec value map wrong (misses 0x10/0x12, invents 0x22/0x3F) |
| `TrackPrivate` | 151-184 | REAL | — | Honest codec-config seam; lacing doc matches block-level honoring |
| `TrackEntry` | 186-216 | REAL | — | ordinal vs Matroska number distinction documented |
| `TrackEntry::track` | 218-249 | REAL | S3 | F-002: descriptor rejection → no_tracks containment breach; span model honest |
| `child_len` | 252-259 | REAL | — | usize-overflow arm dead on 64-bit but harmless/portable |
| `parse_ebml_header` | 261-378 | REAL | S4 | Grammar guards verified; F-003 text-doctrine inconsistency |
| `parse_info` | 380-442 | REAL | — | Scale/Title/Duration handling verified against spec + tests |
| `TrackDamage` type | 444-445 | REAL | — | The avi DamageLog shape |
| `parse_tracks` | 447-491 | REAL | S4 | Containment + ceiling verified; F-004 duplicate numbers unnoted |
| `parse_track_entry` (signature + declarations) | 493-518 | REAL | — | Boundary: loop/master-parses/assembly are part2 (W05-A07) |

## 5. Test honesty (mkv::header::tests, baseline :1754-1757; fixtures/corpus suites)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `track_type_maps_to_the_l0_vocabulary` (:1757) | YES — but it pins the WRONG map | Asserts 0x20/0x3F → Data (F-001) and never tests 0x10/0x12/3; the test enshrines the defect rather than challenging it |
| `segment_info_defaults_and_duration_ticks` (:1756) | YES | Real arithmetic pins: 2.023 s → 2023 ticks; −1.0/NaN → None; default scale 1e6 |
| `info_title_and_track_uid_parse_into_the_models` (:1755) | YES | Hand-built Info+Tracks through the real parser; title, scale, UID default 0, ordinals pinned |
| `codec_descriptors_follow_the_registry_vocabulary` (:1754) | YES | 30+ vocabulary pins incl. VFW fourcc-in-biCompression and raw pass-through for unknowns (the pass-through is what makes F-002 reachable) |
| `valid_fixtures_match_ffprobe_packet_counts` / `audio_payload_is_bit_exact_vs_ffmpeg` (mkv_fixtures :2151-2153) | YES | ffmpeg-produced fixtures, count + bit-exact payload differential |
| `corpus_matroska_differential_vs_ffprobe` (:2100) | YES | Whole-corpus differential against ffprobe truth |
| `malformed_derivatives_are_typed_or_recover_never_panics` (:2148) | YES | Malformed-input contract: typed refusal or recovery, never panic |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Demux Matroska and WebM: tracks, chapters, attachments, cues-based seeking, block lacing — implemented (win/linux/macos)" | docs/feature-matrix.toml:129-138 (`container-matroska-webm`) | REAL | Baseline: mkv_corpus differential 2/2 (:2098-2101), mkv_fixtures 9/9 (:2142-2153), editions/tags 14, containers::mkv 10 (:2803-2811), header tests 4/4 (:1754-1757) |
| "WebM payload set minus VP9 decode recorded (codec-side)" | feature-matrix.toml:140 | REAL | `webm_vp9_and_opus_decode_from_the_private_head` ok (:2900) — demux delivers; decode limits recorded codec-side |
| "A degenerate TrackEntry skips that track with damage — the other tracks and the file survive" | header.rs module docs 13-16 | FALSE (one breach) | F-002: a descriptor-rejecting entry fails the whole open with `no_tracks` (mod.rs:447-450) |
| "0x20 logo, 0x21 buttons, 0x22 control and 0x3F metadata all land in Data" | header.rs:138-140 comment | FALSE | Spec: 0x10 logo, 0x12 buttons, 0x20 control, 0x21 metadata (F-001) |

## 7. Dead / duplicated / unreachable code
- `child_len`'s usize-overflow arm cannot fire on 64-bit targets (child.size: u64 → usize::try_from always Ok) — kept for portability, dead in practice.
- No other dead code; every constant and type in my range has a consumer (constants at parse sites, structs at mod.rs seams).

## 8. Dependencies & flags
- No feature gates; compiled in the default build (build-release-workspace exit 0) and exercised by the baseline suites.
- Depends on `crate::mkv::ebml` (W05-A03's audit) for parse_header_at/vint semantics — the zero-length-uint/float defaults verified at that seam (ebml.rs:492-534).
- media-types seam: `CodecDescriptor::new` (ids.rs:582-591), `LanguageTag`, `TrackFlags` — all real constructors, no bypass.

## 9. Open questions for the next agent
- W05-A07 (part2, owns the element loop + `codec_descriptor_for`): confirm `parse_track_entry` marks an entry with missing `TrackType` degenerate (the `media_kind_from_track_type` → None path) — and whether a **logo/button track being skipped** (F-001) surfaces as `degenerate-track-entry` damage today.
- mkv/mod.rs + cluster.rs owners: `by_number` duplicate collapse (F-004) — does any block-routing path damage-note a number that maps to a dropped ordinal?
- F-002's fix wants a decision from the mod.rs owner: filter degenerate entries before `track()` aggregation, or change the error mapping here.
