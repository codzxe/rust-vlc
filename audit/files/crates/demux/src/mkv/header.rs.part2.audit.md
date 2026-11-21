# AUDIT: crates/demux/src/mkv/header.rs#part2 (lines 495–1033; pulled back from nominal 517 for `parse_track_entry` integrity)
| Field | Value |
|---|---|
| Auditor | W05-A07 |
| Date | 2026-11-21 |
| Lines / functions | 539 audited (assigned ≈517–1033; start pulled back to 495 = start of `parse_track_entry`, a 22-line overlap into A06's part1 — part1 effectively owns 1–494; chunk includes the whole `mod tests` 856–1033) / 10 items in table |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen open <mkv/webm>` → media-probe `matroska` signature (phase5.rs:119) → apps/cli/src/containers/mod.rs:61 `"matroska" => mkv::open` → containers/mkv.rs:147 `MkvDemuxer::new` → mkv/mod.rs:342/408/412 `parse_ebml_header`/`parse_info`/`parse_tracks` → `parse_track_entry` (495) → `parse_video_master`/`parse_audio_master`/`codec_descriptor_for` (this chunk). EXECUTED in baseline: real-binary CLI test `matroska_container_opens_and_decodes_h264 ... ok` on `generated/valid/mkv_h264_aac.mkv` (nofailfast :3043); `containers::mkv::tests::webm_vp9_and_opus_decode_from_the_private_head ... ok` (:2900); `corpus_mkv_h264_tracks_field_assertions ... ok` (:1018); `mkv::header::tests` ×4 (:1754–1757) — all `ok`. |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:4 |
| Commands run | rg "MkvDemuxer/header::/parse_ebml_header/parse_tracks" over mkv/mod.rs + containers/mkv.rs (call chain); sed of ebml.rs:379-425 (read_exact_at clamps to child end ∧ stream length; declared_end = data_pos+size, no parent clamp), :439-488 (parse_header_at cuts the HEADER window at the parent bound only), :492-555 (uint_from_be empty→Some(0), float 0/4/8, utf8_from); sed of header.rs part1 context (parse_tracks 456-491 damage-notes `Ok(None)` as "degenerate-track-entry"; track() 227-228); sed of mkv/mod.rs:446-450 (the `collect::<Result>()?`); rg "matroska" in media-probe + CLI tests; rg pixel_width/channels consumers (containers/mkv.rs:341-352); CodecDescriptor::new semantics (media-types/src/ids.rs:582-591: rejects only empty/control-char strings); python-repl byte verification of every quote; baseline log greps for mkv/header/webm results. |

## 1. One-paragraph verdict
Real, reachable, and executed end-to-end: `parse_track_entry` walks TrackEntry children with per-field width caps, honest typed errors, and a viability check whose degenerate results the caller damage-notes; the codec table maps 25+ Matroska CodecIDs to registry vocabulary with raw passthrough for the rest (verified against `CodecDescriptor::new`'s contract), and the whole lane ran in the baseline CLI binary on an h264+aac MKV and a VP9+Opus WebM. Two doctrine violations mar it: a TrackEntry with a missing/empty CodecID survives the viability check with an empty descriptor and then detonates later in `TrackEntry::track` — failing the ENTIRE file open with a mislabeled `no_tracks` error, contradicting the module's per-track containment promise — and an over-ceiling CodecPrivate is dropped with no damage note despite the code comment claiming honest handling. The rest is small-bore: a module-doc clamp claim the payload reads don't honor, codec-vocabulary gaps (big-endian PCM, MPEG-1/2 video, Theora, non-FFV1 VFW fourccs), the empty-uint-payload-as-zero quirk creating a reference-dead track number 0, and a stale orphaned comment.

## 2. What actually works (verified)
- The track-entry walk — verified by baseline `mkv::header::tests::info_title_and_track_uid_parse_into_the_models ... ok` (:1755), which builds real EBML bytes and drives `parse_info` + `parse_tracks` + `parse_track_entry` (number, type, codec id, present/absent TrackUID → 0xDEADBEEF / 0, ordinals 0/1).
- The codec vocabulary — `mkv::header::tests::codec_descriptors_follow_the_registry_vocabulary ... ok` (:1754) pins 25+ mappings incl. the VFW biCompression lane (offset 16, byte-order correct) and the raw-passthrough refusals; executed for real by `containers::mkv::tests::webm_vp9_and_opus_decode_from_the_private_head ... ok` (:2900) and the CLI `matroska_container_opens_and_decodes_h264 ... ok` (:3043) — the latter runs the shipped binary on an MKV whose h264 track my chunk's `codec_descriptor_for("V_MPEG4/ISO/AVC", …) → "h264"` names.
- Width discipline: every uint field is `len > 8 → bad_structure` (531-533, 538-540, 547-549, 573-575, 587-589, 594-596, 603-605, 610-612); floats read via `len.min(8)` (757); text fields under `MAX_TEXT_FIELD` (554, 618, 624, 630); `read_exact_at` returns typed truncation (ebml.rs:379-396) — no panic path exists in this chunk.
- Damage containment for degenerate entries — `parse_tracks` (part1, 476-484) notes every `Ok(None)` as "degenerate-track-entry" and enforces `MAX_TRACKS` with a damage row; verified by read + the empty-damage assertion in the model test (:966).
- Language handling: IETF preferred over legacy, empty filtered, unparseable → None (672-675) — the honest shape the MP4 lane lacks.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/mkv/header.rs:668 (this chunk — the empty descriptor is minted here); detonation at header.rs:227-228 (part1) via mkv/mod.rs:446-450
Code (668, this chunk):
```
    let codec_id = codec_id.unwrap_or_default();
```
(header.rs:227-228, part1):
```
    pub fn track(&self, base: TimeBase) -> Result<Track, EngineError> {
        let codec = CodecDescriptor::new(&self.codec_descriptor).map_err(|_| error::no_tracks())?;
```
(mkv/mod.rs:446-450):
```
        self.tracks = self
            .entries
            .iter()
            .map(|e| e.track(self.base))
            .collect::<Result<Vec<_>, _>>()?;
```
What is wrong: A TrackEntry whose CodecID element is absent (`codec_id = None → unwrap_or_default() = ""`) or zero-length (`utf8_from(&[]) = Some("")`) passes the viability check (664-667 needs only number + track type), gets `codec_descriptor_for("") = ""`, and survives into the demuxer's start. There, `CodecDescriptor::new("")` fails (media-types/src/ids.rs:583 rejects empty strings), `track()` maps that failure to `error::no_tracks()`, and the `collect::<Result<Vec<_>, _>>()?` at mod.rs:450 aborts the ENTIRE open.
Why it matters: Three defects in one chain. (1) Containment doctrine violated: the module doc (header.rs:11-14) promises a degenerate TrackEntry "skips *that* track with damage — the other tracks and the file survive"; a missing-CodecID entry instead kills every track. (2) The error is mislabeled: the file HAS tracks; the real condition is "entry N has no CodecID" — a user debugging "no tracks" will not find it. (3) Doc drift: `track()`'s `# Errors` doc says "Propagates the registry's descriptor rejection" while `map_err(|_| error::no_tracks())` replaces the registry's `Unsupported/descriptor_invalid` error with a different kind. Blast radius: malformed/nonconformant files only (CodecID is spec-mandatory), hence S3 not S2.
What correct looks like: treat an empty codec id as degenerate at the viability check (`if codec_id.as_deref().map_or(true, str::is_empty) { return Ok(None); }` → the caller's existing "degenerate-track-entry" damage note), and in `track()` propagate the registry error instead of remapping it to `no_tracks`.
Confidence: HIGH — Verification: read of all three sites + `CodecDescriptor::new` contract at ids.rs:582-591; no runtime repro (cargo forbidden).

### F-002 [S3]
Where: crates/demux/src/mkv/header.rs:564-571
Code:
```
            ID_CODEC_PRIVATE => {
                if len <= MAX_CODEC_PRIVATE as usize {
                    codec_private = reader.read_prefix_at(len, child.data_pos, child_end)?;
                }
                // Over-ceiling: skipped (the bytes are never read); the
                // entry survives without its private payload — the
                // decoder seam reports the honest empty.
```
What is wrong: An over-ceiling (>4 MiB) CodecPrivate is skipped with NO damage note. `parse_track_entry` has no damage channel (it returns `Ok(None)` for degenerates; the caller `parse_tracks` only notes whole-entry skips); this in-place field drop produces neither a `TrackDamage` row nor any later `Recovered` event. The comment's claim "the decoder seam reports the honest empty" describes the downstream consequence, not an emitted signal — nothing in the pipeline tells the user the avcC/vorbis-cookie/private config was discarded.
Why it matters: The file's own doctrine is "skips *that track* with damage" for over-ceiling fields (the CodecID twin at 554-557 does return Ok(None) → damage-noted); this field drop is silent — a swallowed conditional. A track whose private bytes were dropped will misbehave at decode (no SPS/PPS for h264) with no diagnostic trail. Realistic frequency is low (>4 MiB private is degenerate), keeping it at S3.
What correct looks like: thread a damage row out of `parse_track_entry` (or return a flag) and let `parse_tracks` push `(len, "codec-private-over-ceiling")`.
Confidence: HIGH — Verification: read; grep confirms no note is emitted on this path.

### F-003 [S4]
Where: crates/demux/src/mkv/header.rs:8-10 (module doc, part1) vs 535 (this chunk, the payload-read shape)
Code (module doc, part1):
```
//! - Declared element sizes are hints: every child read is clamped to
//!   its parent's effective end and the stream's physical end; a
```
(payload read, this chunk — 534-535):
```
                number =
                    ebml::uint_from_be(&reader.read_exact_at(len, child.data_pos, child_end)?);
```
What is wrong: The doc promises every child read is clamped to the PARENT's effective end. The payload reads in `parse_track_entry`/`parse_video_master`/`parse_audio_master` are bounded by the CHILD's own declared end (`child_end`, clamped only to the stream length — ebml.rs:379-396 `read_exact_at` uses `until.min(self.length)`); only the element-HEADER read is cut at the parent bound (ebml.rs:450). A child declaring a size that overruns its master reads payload bytes belonging to the next element (mis-parsed values), then `pos = child_end` ends the master walk.
Why it matters: Doc overstates the robustness model; behavior stays bounded and typed (no panic, no unbounded read — `read_exact_at` refuses past the stream), so this is diagnosis-quality drift, not a safety hole.
What correct looks like: pass the master's `end` as the read bound (`read_exact_at(len, child.data_pos, child_end.min(end))`) so an over-declared child is the typed truncation at the parent boundary — matching the documented model.
Confidence: HIGH — Verification: read of ebml.rs:379-396 and 439-488.

### F-004 [S4]
Where: crates/demux/src/mkv/header.rs:832-848
Code:
```
    if codec_id.starts_with("A_PCM/INT/LIT") {
        return match bit_depth {
            Some(8) => "pcm_u8".to_string(),
            Some(16) => "pcm_s16le".to_string(),
            Some(24) => "pcm_s24le".to_string(),
            Some(32) => "pcm_s32le".to_string(),
            _ => codec_id.to_string(),
        };
    }
```
What is wrong: The vocabulary has asymmetric gaps. Little-endian PCM is mapped by bit depth, but the spec's big-endian twin `A_PCM/INT/BIG` (→ pcm_s16be/…) is absent; likewise `V_MPEG1`, `V_MPEG2`, `V_THEORA`, `A_SPEEX`, `A_TTA1`, and every VFW fourcc except FFV1 (XVID/DX50/HFYU… stay raw). All of these fall to the raw passthrough → typed Unsupported at the codec registry — the documented honest refusal (775-778), so nothing is silent — but a VLC-class player plays several of these families and this build cannot even name them.
Why it matters: Honest but incomplete; the descriptor table is the gate for future decoder cards, and the missing symmetric PCM mapping is the clearest miss (the code already knows the bit-depth shape).
What correct looks like: add the `A_PCM/INT/BIG` arm (pcm_s16be/s24be/s32be) and the remaining spec video/audio families; keep raw passthrough for vendor fourccs.
Confidence: HIGH (for the gaps; verified by reading the full table 779-849) — the playability comparison to VLC is context, not a repo claim.

### F-005 [S4]
Where: crates/demux/src/mkv/header.rs:530-536 (with ebml.rs:492-501)
Code:
```
            ID_TRACK_NUMBER => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                number =
                    ebml::uint_from_be(&reader.read_exact_at(len, child.data_pos, child_end)?);
```
What is wrong: `uint_from_be` returns `Some(0)` for a ZERO-LENGTH payload (ebml.rs:498-501, "empty = 0 — the spec's default-value quirk"). Applied to TrackNumber, a zero-length element yields `number = Some(0)`: the viability check passes and a track with Matroska track number 0 is registered — a number no block can reference (block track numbers start at 1), so the track is permanently silent with no damage note. (The same quirk on TrackType is harmless: 0 maps to None → skip.)
Why it matters: Malformed-input edge; a dead track occupies an ordinal and shows in the UI but never receives a packet, with no diagnostic. Bounded, no panic.
What correct looks like: for TrackNumber specifically, require a non-empty payload (`if len == 0 { return Ok(None); }` → damage-noted degenerate).
Confidence: HIGH — Verification: read of both sites.

### F-006 [S4]
Where: crates/demux/src/mkv/header.rs:851-855
Code:
```
/// Silence the `MAX_PARSE_DEPTH` import when the fixed-shape parsers
/// above carry the depth bound structurally (the constant documents the
/// bound the walk shape enforces; the cluster walker uses it at
/// runtime).
#[cfg(test)]
mod tests {
```
What is wrong: The comment justifies silencing an unused `MAX_PARSE_DEPTH` import — but no such import exists in this file (the use-list at 21-34 has none; `chapters.rs` reaches it as `ebml::MAX_PARSE_DEPTH`, qualified). The comment is an orphan describing a state that no longer exists, parked confusingly directly above `mod tests` as if it gated it.
Why it matters: Stale doc noise at a structural boundary; a reader hunting the depth bound in header.rs is misled.
What correct looks like: delete the comment (or move the depth-bound explanation into the module doc where the parsers' structural bound is described).
Confidence: HIGH — Verification: read + rg "MAX_PARSE_DEPTH" over mkv/*.rs (no import in header.rs).

## 4. Per-function table (chunk lines 495–1033)
| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_track_entry` | 495-704 | REAL | S3×2 | F-001 (empty codec id default at 668), F-002 (silent private drop at 564-571), F-005 (number 0 quirk); otherwise: per-field `len > 8` caps ✓, text ceilings ✓, viability → caller damage note ✓, language IETF-over-legacy ✓, lacing/default/forced flags parsed ✓ |
| `parse_video_master` | 707-734 | REAL | — | PixelWidth/Height only (documented renderer seam); `len <= 8` gate; other Video children skipped by size; no direct unit test (covered indirectly via corpus/CLI lanes) |
| `parse_audio_master` | 738-773 | REAL | — | SamplingFrequency via `len.min(8)` + float_from_be (0/4/8 only — other widths → None → kept absent ✓); Channels/BitDepth behind `len <= 8` guard arm ✓; outputs consumed at containers/mkv.rs:341-346 (channels 1..=64, rate ≤ 4.096 MHz validated downstream) |
| `codec_descriptor_for` | 779-849 | REAL | S4 | F-004 vocabulary gaps; VFW biCompression offset 16 ✓ byte-order correct; prefix matches (`V_MPEG4/ISO/AVC*`, `A_AAC*`) ✓; raw passthrough verified against CodecDescriptor::new's contract; pinned by 25+ test assertions |
| test helper `id_bytes` | 860-863 | REAL | — | minimal-width id vint builder ✓ |
| test helper `size_bytes` | 866-877 | REAL | — | 1/2/8-byte vint shapes (4-byte region unused — fine for the payloads built) |
| test helper `el` / `uint_payload` | 880-892 | REAL | — | element builder; uint_payload v=0 → single zero byte ✓ |
| test `track_type_maps_to_the_l0_vocabulary` | 894-903 | REAL | — | genuine mapping pins incl. 0/3 → None |
| test `segment_info_defaults_and_duration_ticks` | 905-930 | REAL | — | pins 1_000_000 default, 2.023s → 2023 ticks, −1.0/NaN → None — would fail on regression |
| test `info_title_and_track_uid_parse_into_the_models` | 932-973 | REAL | — | drives MY `parse_track_entry` with real EBML bytes (number/type/CodecID/absent-UID); empty-damage assertion ✓ |
| test `codec_descriptors_follow_the_registry_vocabulary` | 975-1032 | REAL | — | 25+ mapping assertions incl. FFV1, raw refusals, PCM depths — would fail on any mapping regression |

## 5. Test honesty (tests in this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| track_type_maps_to_the_l0_vocabulary | YES | exact enum pins |
| segment_info_defaults_and_duration_ticks | YES | exact tick arithmetic pins (2023) + negative/NaN branches |
| info_title_and_track_uid_parse_into_the_models | YES | real byte-level EBML through parse_info/parse_tracks/parse_track_entry; uid/number/ordinal asserts |
| codec_descriptors_follow_the_registry_vocabulary | YES | 25+ exact-string asserts |
| (coverage gap) parse_video_master / parse_audio_master | NO direct test in this file | no assertion anywhere in header.rs on pixel_width/height or sampling/channels/depth values; indirect coverage exists via `containers::mkv::tests` (geometry lanes, e.g. :2900) and corpus suites (:1018) — a width/height swap would surface only there |
| (context, other file) `matroska_container_opens_and_decodes_h264` (apps/cli/tests/unsupported_path.rs:142) | name overpromises | body asserts exit 0 + session accepted + absence of the old "no decoder for `h264`" refusal — it does NOT assert any decoded frame; "decodes" is unproven by this test (flag for that file's auditor) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A degenerate `TrackEntry` … skips *that* track with damage — the other tracks and the file survive" | header.rs:11-14 (module doc) | SHALLOW | holds for missing-number/unknown-type/over-ceiling-CodecID (parse_tracks:483 notes), NOT for missing CodecID (F-001: whole-file failure) |
| "Unknown ids pass through **raw** — the honest exposure: the codec registry refuses them as typed Unsupported" | header.rs:775-778 | REAL | CodecDescriptor::new accepts any non-empty control-free string (ids.rs:582-591); raw strings flow to Track.codec |
| `track()` "# Errors — Propagates the registry's descriptor rejection" | header.rs:~224-226 | FALSE | line 228 `map_err(|_| error::no_tracks())` REPLACES the registry error kind instead of propagating it |
| "every child read is clamped to its parent's effective end" | header.rs:8-10 | FALSE (for payload reads) | F-003: payload reads bounded by the child's own end + stream length only |
| "Over-ceiling: skipped … the decoder seam reports the honest empty" | header.rs:568-570 | SHALLOW | nothing reports the skip (F-002); the "honest empty" is an absence, not a signal |

## 7. Dead / duplicated / unreachable code
- No dead public API in this chunk: `TrackPrivate` fields consumed at containers/mkv.rs:341-352; `uid` is the tags/chapters join key (issue #34 note); `flags`/`language`/`title` flow through `TrackEntry::track`.
- Stale orphan comment F-006 (851-854).
- Test helpers are test-only and used (no dead tests).

## 8. Dependencies & flags
- No [features] in crates/demux/Cargo.toml — mkv module compiled unconditionally; shipped CLI reaches it via the `matroska` registry arm.
- Depends on `crate::mkv::ebml` (A03's file — helpers verified this audit: `read_exact_at` typed-truncation, `uint_from_be` empty→0 quirk, `float_from_be` 0/4/8, `utf8_from` None-on-invalid) and `crate::mkv::error`.
- std-only; no unsafe; no unwrap/expect in lines 495-1033 outside `#[cfg(test)]` (test helpers' `unwrap_or(7)`/`.must()` are testkit patterns).
- Ceilings: MAX_TRACKS 64 (enforced in part1 caller), MAX_CODEC_PRIVATE 4 MiB (this chunk), MAX_TEXT_FIELD 4 KiB (this chunk) — all enforced, two of the three with damage notes (F-002 is the exception).

## 9. Open questions for the next agents
- A06 (part1, header.rs 1–494): F-001's detonation site `track()` (227-228) is in your chunk — the `map_err` remap and the `# Errors` doc fix belong there; consider whether `parse_tracks` should pre-filter empty descriptors so `mod.rs:450` can never see one.
- A08 (mkv/mod.rs part1): the `collect::<Result<Vec<_>, _>>()?` at 446-450 turns one bad entry into a whole-file failure — a per-entry containment shape (skip + note) matches the doctrine.
- Whoever audits apps/cli/tests/unsupported_path.rs: `matroska_container_opens_and_decodes_h264` (142-160) asserts no decoded frames — retitle or add a frame-count assertion.
- The audio-geometry validation (channels 1..=64, rate ≤ 4.096 MHz) lives in containers/mkv.rs:341-346 — verify BitDepth==0 handling on the decoder seam when the PCM card lands.
