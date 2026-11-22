# AUDIT: crates/demux/src/mkv/header.rs

| Field | Value |
|---|---|
| Auditor | W05-A04 |
| Date | 2026-11-17 |
| Lines / functions | 1033 / 19 functions + 4 test fns + 4 test helpers + 3 consts + 5 types |
| File verdict | REAL-UNPROVEN (genuine, reachable parser; execution blocked by no-cargo adaptation — W00 baseline: demux tests inside a passing 1751/2/25 workspace run) |
| Reachable from shipped entry point? | YES — `lumen` CLI: `apps/cli/src/main.rs` → `run.rs`/`dispatch.rs` → `demux_host.rs:458 crate::containers::open` → `containers/mod.rs:57 "matroska" => mkv::open` → `apps/cli/src/containers/mkv.rs:147 MkvDemuxer::new` → `start()` → `mkv/mod.rs:342/408/412` calls `parse_ebml_header`/`parse_info`/`parse_tracks`, `mod.rs:449 e.track(base)` |
| Findings | S0:0 S1:0 S2:1 S3:3 S4:2 |
| Commands run | `wc -l` (1033, exit 0); Read/Grep over `ebml.rs`, `error.rs`, `mod.rs`, `media-types/src/ids.rs`, `apps/cli/src/pipeline_host.rs`, `apps/cli/src/containers/{mod,mkv}.rs`; `sed -n` verification of every quote below (exit 0). No cargo/npm/node (adaptation §ADAPTATIONS); runtime behavior = static trace + W00 baseline |

## 1. One-paragraph verdict

This file is the real Matroska `EBML`/`Info`/`Tracks`/`TrackEntry` parser: genuine bounded EBML walking, typed errors, per-track damage containment, and an honest codec-id→descriptor table (unknown ids pass through raw). It is reachable from the shipped CLI. It is not clean: one degenerate-track case (missing/control-char `CodecID`) escapes the containment doctrine and kills the whole file with a lying `no-tracks` error (F-001); the documented fixed-width grammar model is enforced inconsistently across call sites (F-002); an over-ceiling `CodecPrivate` is dropped with zero damage accounting (F-003); and two doc comments state falsehoods (F-004, F-005). No panic path, no unbounded allocation, no infinite loop was found — allocations are capped at 12 B / 8 B / 4 KiB / 4 MiB and every read is clamped to the stream length (`ebml.rs:306-309 prefix_len`).

## 2. What actually works (verified)

- All file-size-controlled allocations are capped: header windows ≤ 12 B (`header.rs:277`, `ebml.rs:450-451`), fixed-width fields ≤ 8 B, text ≤ `MAX_TEXT_FIELD` 4 KiB, `CodecPrivate` ≤ `MAX_CODEC_PRIVATE` 4 MiB; `read_prefix_at`/`read_exact_at` clamp to `until.min(self.length)` (`ebml.rs:306-309`, `385`). No `Vec::with_capacity(file_size)` anywhere in this file — verified by reading every allocation site.
- Loop progress is structurally guaranteed: every walk advances `pos = child_end` where `child_end = child.declared_end()` = `checked_add(data_pos, size)` (`ebml.rs:418-424`) and `data_pos > pos` (id ≥ 1 + size ≥ 1 vint bytes, `ebml.rs:485`). No infinite loop. — verified by reading `ebml.rs:439-488` + all four walk loops.
- Unknown elements are skipped by declared size at every level (header `:366-369`, Info `:437`, Tracks `:487`, TrackEntry `:654-657`); unknown-size children are refused typed `bad_structure` (`:307-309`, `:404-406`, `:469-471`, `:525-527`).
- `duration_ticks` guards NaN, negative, and > `i64::MAX` (`:124-135`); `timestamp_scale = 0` surfaces as typed `bad_structure` through `time_base` (`:117-120`, consumed `mod.rs:440`).
- `MAX_TRACKS` (64) ceiling enforced with a damage row (`:473-474`); over-ceiling `CodecID` degrades the track, not the file (`:554-557`).
- The VFW `biCompression` extraction at `codec_private[16..20]` is the correct BITMAPINFOHEADER layout (biSize@0, biWidth@4, biHeight@8, biPlanes@12, biBitCount@14, biCompression@16).
- The four in-file tests assert concrete values and would fail if the code broke (see §5); W00 baseline run (audit/runs/baseline, `tests 1751 pass / 2 fail / 25 ignored`) executed the demux test suite containing the mkv fixture tests — indirect execution evidence (UNVERIFIED for this file's own tests in isolation).

## 3. Findings

### F-001 [S2] One bad `CodecID` fails the entire file with a false "no tracks" error — containment doctrine broken
Where: crates/demux/src/mkv/header.rs:668 and 227-228; blast radius at crates/demux/src/mkv/mod.rs:446-450
Code (header.rs:666-671):
```rust
    let (Some(number), Some(kind)) = (number, track_type.and_then(media_kind_from_track_type))
    else {
        return Ok(None);
    };
    let codec_id = codec_id.unwrap_or_default();
```
Code (header.rs:227-228):
```rust
    pub fn track(&self, base: TimeBase) -> Result<Track, EngineError> {
        let codec = CodecDescriptor::new(&self.codec_descriptor).map_err(|_| error::no_tracks())?;
```
Code (mod.rs:446-450):
```rust
        self.tracks = self
            .entries
            .iter()
            .map(|e| e.track(self.base))
            .collect::<Result<Vec<_>, _>>()?;
```
What is wrong: `CodecID` is mandatory in Matroska, but a `TrackEntry` without one (or with a UTF-8 `CodecID` containing an ASCII control byte, e.g. `"A_OPUS\x01"`) is *not* treated as degenerate: `unwrap_or_default()` yields `""`, `codec_descriptor_for("", …)` returns `""` (`:848`), and `CodecDescriptor::new` rejects empty/control-char descriptors (crates/media-types/src/ids.rs:582-583: `if descriptor.is_empty() || descriptor.chars().any(|c| c.is_ascii_control())`). `track()` maps that rejection to `error::no_tracks()`, and `mod.rs:450` propagates it — the **whole file open fails** even when other tracks are perfectly valid.
Why it matters: the module's own doctrine (header.rs:13-16) promises "A degenerate `TrackEntry` … skips *that track* with damage — the other tracks and the file survive; only an empty surviving table is the caller's typed 'no tracks' refusal." This path violates both halves: the file does not survive, and the `demux.mkv.no-tracks` error is false (tracks exist). The registry's real diagnostic (`track.codec.descriptor_invalid`) is also swallowed by `map_err(|_| …)` — the operator cannot tell which track or why.
What correct looks like: treat an absent/unusable `CodecID` as a degenerate entry (`Ok(None)` + damage row, like over-ceiling `CodecID` at `:554-557`), or map the descriptor rejection to a per-track skip with the true error code preserved.
Confidence: HIGH — Verification: read (full static trace through ids.rs:582-590 and mod.rs:446-450; runtime UNVERIFIED, no cargo).

### F-002 [S3] The documented fixed-width grammar ("uint/float wider than 8 = bad_structure") is enforced inconsistently — three call sites silently tolerate instead
Where: crates/demux/src/mkv/header.rs:431, 757, 723, 760 (vs. the model at :17-19 and the typed sites, e.g. :531-532)
Code (module model, header.rs:17-19):
```rust
//! - Fixed-width fields obey the EBML grammar: a uint wider than 8
//!   bytes or a float wider than 8 is corruption, not a big number
//!   (`bad_structure`, never an oversized allocation).
```
Code (header.rs:430-435, `Duration`):
```rust
            ID_DURATION => {
                let bytes = reader.read_prefix_at(len.min(8), child.data_pos, child_end)?;
                match ebml::float_from_be(&bytes) {
```
Code (header.rs:723-727, `PixelWidth`/`PixelHeight`):
```rust
        let len = child_len(&child)?;
        if len <= 8 {
            let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
            match child.id {
```
What is wrong: a 9+-byte `Duration` (`:431`) or `SamplingFrequency` (`:757`) is silently **truncated to its first 8 bytes** and interpreted as a value (no error); a 9+-byte `PixelWidth`/`PixelHeight` (`:723`) or `Channels`/`BitDepth` (`:760`, `if len <= 8` guard) is silently treated as absent. Meanwhile `TrackNumber`/`TrackUID`/`TrackType`/flags/etc. (`:531-532`, `:537-539`, `:546-548`, …) do return `bad_structure` for `len > 8`, exactly as the doc claims.
Why it matters: no memory-safety impact (all reads bounded), but the module's stated robustness contract is false at four call sites, and the tolerant paths can fabricate a duration/frequency from the first 8 bytes of a corrupt field — the exact "big number from corruption" outcome the doc promises never happens.
What correct looks like: one shared `read_fixed(len ≤ 8)` helper returning `bad_structure` on overflow, used by every fixed-width site.
Confidence: HIGH — Verification: read (`sed -n '429,436p;721,730p;755,768p'`).

### F-003 [S3] Over-ceiling `CodecPrivate` is dropped with no damage row — the honesty counter hides the loss
Where: crates/demux/src/mkv/header.rs:564-571
Code:
```rust
            ID_CODEC_PRIVATE => {
                if len <= MAX_CODEC_PRIVATE as usize {
                    codec_private = reader.read_prefix_at(len, child.data_pos, child_end)?;
                }
                // Over-ceiling: skipped (the bytes are never read); the
                // entry survives without its private payload — the
                // decoder seam reports the honest empty.
            }
```
What is wrong: a `CodecPrivate` larger than 4 MiB is never read and **no damage note is recorded** — contrast `track-over-ceiling` (`:474`) and `degenerate-track-entry` (`:483`) for the other ceiling hits, and the over-ceiling `CodecID` degenerate path (`:554-557`).
Why it matters: `damage_bytes()` (mod.rs:317-319) is the demuxer's honesty counter; here a multi-megabyte skip reads as a clean file. Downstream, codecs that require the blob (flac STREAMINFO, vorbis chain, h264 avcC — seeded via `apps/cli/src/containers/mkv.rs:244-256 config_blob`) refuse at `configure` with a codec-level "missing header" error, hiding that the container itself dropped the bytes.
What correct looks like: push a `(len, "codec-private-over-ceiling")` damage row when the ceiling is exceeded.
Confidence: HIGH — Verification: read.
 
### F-004 [S4] `TrackDamage` alias doc says "(absolute offset, reason)" — the values are byte lengths
Where: crates/demux/src/mkv/header.rs:444-445 vs. :474, :483; consumer mod.rs:414
Code (header.rs:444-445):
```rust
/// Track-table damage rows: (absolute offset, reason).
pub type TrackDamage = Vec<(u64, &'static str)>;
```
Code (header.rs:473-474):
```rust
            if entries.len() >= MAX_TRACKS {
                damage.push((child_end - pos, "track-over-ceiling"));
```
Code (mod.rs:414):
```rust
                    self.damage_total += damage.iter().map(|(n, _)| *n).sum::<u64>();
```
What is wrong: `child_end - pos` is the skipped element's byte count, and `mod.rs:414` sums it into `damage_total` ("Total damage bytes skipped by recovery", mod.rs:315). The alias doc's "absolute offset" is false.
Why it matters: an API doc that misstates the tuple's meaning invites future callers to render damage offsets that are actually byte counts.
What correct looks like: fix the alias doc to "(byte count, reason)" — the function doc at `:448-449` already says byte count.
Confidence: HIGH — Verification: read.

### F-005 [S4] Stale doc comment documents a non-existent import silencing
Where: crates/demux/src/mkv/header.rs:851-855
Code:
```rust
/// Silence the `MAX_PARSE_DEPTH` import when the fixed-shape parsers
/// above carry the depth bound structurally (the constant documents the
/// bound the walk shape enforces; the cluster walker uses it at
/// runtime).
#[cfg(test)]
mod tests {
```
What is wrong: this file imports no `MAX_PARSE_DEPTH` (imports at `:29-37` are IDs only); nothing is being silenced. The comment is a leftover from a removed `use` and now documents nothing as the tests module's doc.
Why it matters: misleading comment; a reader hunts for an import that does not exist.
What correct looks like: delete the comment or replace it with the tests module's actual purpose.
Confidence: HIGH — Verification: read (`sed -n '29,37p'` shows the full import list).

### F-006 [S3] Codec-id honesty: 12 of the emitted vocabulary names have no decoder in the shipped CLI — "mapped" must not be read as "decodable"
Where: crates/demux/src/mkv/header.rs:779-849 (mapping) vs. apps/cli/src/pipeline_host.rs:372-379 (dispatch)
Code (header.rs:796-798, the overstatement):
```rust
    // Known fourccs map to the vocabulary; unknown ones stay raw for
    // the typed refusal downstream — never a silent guess.
    if codec_id == "V_MS/VFW/FOURCC" {
```
Code (pipeline_host.rs:372-378, the shipped refusal):
```rust
            other => Err(host_error(
                PipelineStage::Decode,
                "cli.codec_unavailable",
                format!(
                    "no decoder for `{other}` in this build (landed: mjpeg, pcm, flac, \
                     mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264)"
```
What is wrong: `codec_descriptor_for` maps `V_MPEGH/ISO/HEVC`→"hevc", `V_MPEG4/ISO/ASP`→"mpeg4", `V_VP8`→"vp8", VFW→"ffv1", `A_AC3`→"ac3", `A_EAC3`→"eac3", `A_DTS`→"dts", `A_TRUEHD`→"truehd", `S_TEXT/UTF8`→"subrip", `S_TEXT/ASS`→"ass", `S_TEXT/SSA`→"ssa", `S_TEXT/WEBVTT`→"webvtt" — **none of these 12 names has a match arm in the shipped CLI's `HostDecoderProvider`** (pipeline_host.rs:241-380 arms: mjpeg, flac, mp3/mp1, mp2, vorbis, opus, aac/aac-lc, alac, adpcm_*, av1/av01, vp9, dnxhd, h264/avc1, pcm_*). All 12 demux, surface in facts, and refuse typed at decode. The design is honest (raw passthrough + typed refusal, and `apps/cli/src/containers/mkv.rs:72` admits "ffv1's raw fourcc descriptors refuse typed (no decoder)"), but the comment "Known fourccs map to the vocabulary" overstates: exactly **one** fourcc (FFV1) is mapped, and it has no decoder either.
Why it matters: the claim ledger (wave 18/19) must not count these CodecID rows as playable formats; HEVC-in-MKV and AC3/EAC3/DTS/TrueHD-in-MKV are accepted-and-refused, not implemented. W00 baseline corroborates the pattern (h264+aac MP4: 0 frames decoded).
What correct looks like: keep the mapping, fix the plural comment, and ensure matrix rows for these ids are not marked implemented on the strength of this table.
Confidence: HIGH — Verification: read (full dispatch table cross-checked).

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DOCTYPE_MATROSKA` / `DOCTYPE_WEBM` consts | 40-43 | REAL | — | Duplicated as literals in mod.rs:344 policy match; consts not used there (hygiene note) |
| `MAX_TRACKS` / `MAX_CODEC_PRIVATE` / `MAX_TEXT_FIELD` | 45-60 | REAL | — | Ceilings enforced at :473, :565, :554/:315/:425/:618/:624/:630; PROPOSAL annotations honest |
| `EbmlHeader` struct | 62-76 | REAL | — | Data only; max_id/size_length advisory once ≤4/≤8, enforced :329/:340 |
| `SegmentInfo` + `Default` | 78-107 | REAL | — | Default scale 1_000_000 correct per spec |
| `SegmentInfo::time_base` | 109-120 | REAL | — | Zero scale → typed bad_structure; consumed mod.rs:440 |
| `SegmentInfo::duration_ticks` | 122-136 | REAL | — | NaN/negative/overflow guarded; scale=0 → None via inf/NaN check, no panic |
| `media_kind_from_track_type` | 138-149 | REAL | — | 0x20/0x21/0x22/0x3F→Data catch-all; unknown→None; tested :894-903 |
| `TrackPrivate` struct | 151-184 | REAL | — | `lacing_allowed` informational only (cluster honors block bits) — documented, cross-file |
| `TrackEntry` struct | 186-216 | REAL | — | ordinal/number/uid separation correct |
| `TrackEntry::track` | 218-250 | REAL | — | **F-001**: descriptor rejection → whole-file `no_tracks()`; span start 0/end Missing honest |
| `child_len` | 252-259 | REAL | — | `usize::try_from` guard; 32-bit >4 GiB elements refused typed |
| `parse_ebml_header` | 261-378 | REAL | — | Magic check short-circuit-safe (:278); doctype over-ceiling → error blames header pos not child (S4 note); MAX_ID_LENGTH reads `len.min(8)` before the `len>8` check (:323-325) — harmless, inconsistent with MAX_SIZE_LENGTH order (:334-336); versions advisory (documented :357-363); no unit test in this file (covered by mkv_fixtures.rs:412 `unknown_doctype_is_the_typed_refusal` and mkv_extras_wiring.rs) |
| `parse_info` | 380-442 | REAL | — | **F-002** (Duration min(8)); title tolerant both ways, documented; scale >u32::MAX refused |
| `TrackDamage` alias | 444-445 | REAL | — | **F-004**: doc says offset, values are byte counts |
| `parse_tracks` | 447-491 | REAL | — | MAX_TRACKS + damage rows; per-track containment real except F-001 |
| `parse_track_entry` | 495-704 | REAL | — | **F-001** (missing CodecID), **F-003** (silent private drop); viability check correct; IETF-over-legacy language with unparseable→None; unknown children (ContentEncodings!) skipped — an encrypted track parses as a normal track with undecryptable payload (typed refusal lands at decode; note for matrix) |
| `parse_video_master` | 706-734 | REAL | — | **F-002** (len>8 → field silently absent) |
| `parse_audio_master` | 736-773 | REAL | — | **F-002** (SamplingFrequency min(8); Channels/BitDepth `if len <= 8` guard skips) |
| `codec_descriptor_for` | 775-849 | REAL | — | **F-006**; prefix families (AVC/ASP/A_AAC/A_MPEG/A_PCM) + exact table + raw fallback honest; VFW biCompression offset correct |
| `mod tests` helpers `id_bytes`/`size_bytes`/`el`/`uint_payload` | 858-892 | REAL | — | Correct minimal-width vint encoders |
| test `track_type_maps_to_the_l0_vocabulary` | 894-903 | REAL | — | Would fail on mapping break |
| test `segment_info_defaults_and_duration_ticks` | 905-930 | REAL | — | Concrete values incl. NaN/negative; would fail on math break |
| test `info_title_and_track_uid_parse_into_the_models` | 932-973 | REAL | — | Real EBML bytes through parse_info/parse_tracks; `.must().must()` is Result<Option<T>>→T (legit); damage only asserted empty, contents never checked |
| test `codec_descriptors_follow_the_registry_vocabulary` | 975-1032 | REAL | — | 27 concrete assertions incl. raw passthrough and VFW lane |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `track_type_maps_to_the_l0_vocabulary` | YES | Direct assertions on the pure mapping; any changed arm fails an `assert_eq!` |
| `segment_info_defaults_and_duration_ticks` | YES | Asserts `Some(2023)`, `None` for -1.0/NaN — arithmetic or guard regressions fail |
| `info_title_and_track_uid_parse_into_the_models` | YES | Builds EBML bytes, asserts parsed title/scale/uid/ordinal values; a parse regression fails. Gap: over-ceiling CodecID (→`Ok(None)`), over-ceiling CodecPrivate, MAX_TRACKS, and the missing-CodecID path of F-001 are all untested — the F-001 defect is invisible to this suite |
| `codec_descriptors_follow_the_registry_vocabulary` | YES | 27 exact-match assertions covering table, prefixes, VFW, PCM depths, raw fallback |

Runtime pass/fail of these tests: UNVERIFIED directly (no cargo); W00 baseline ran the workspace suite (1751 pass / 2 fail / 25 ignored, audit/waves/wave-00/BASELINE-SUMMARY.md) which includes this crate's unit tests.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A degenerate TrackEntry … the other tracks and the file survive" | header.rs:13-16 | FALSE (for missing/control-char CodecID) | F-001 trace: :668 → :227-228 → mod.rs:446-450 → whole-file `no_tracks` |
| "a uint wider than 8 bytes or a float wider than 8 is corruption … bad_structure" | header.rs:17-19 | FALSE (at 4 call sites) | F-002: :431, :757 truncate; :723, :760 skip silently |
| "Unknown children are skipped by declared size" | header.rs:10-12 | REAL | :366-369, :437, :487, :654-657 |
| "Declared element sizes are hints: every child read is clamped to its parent's effective end and the stream's physical end" | header.rs:6-9 | SHALLOW | Reads clamp to the child's own `child_end` and stream length (ebml.rs:306-309) — a child declaring past its parent reads bytes beyond the parent's end (in-file, bounded); the parent bound only ends the walk. No OOB, but the stated clamp is not the mechanism |
| "Title … issue #34 … retained, no longer dropped" | header.rs:89-93, :196-199 | REAL | :421-428 parse; test :945; mkv.rs harvest consumes |
| "Mapped before the move into TrackPrivate (the private bytes feed the VFW fourcc lane)" | header.rs:669-671 | REAL | :798-803 reads `codec_private[16..20]` |
| "the codec registry refuses them as typed Unsupported" (unknown ids) | header.rs:775-778 | REAL | pipeline_host.rs:372-379 `cli.codec_unavailable` typed refusal |

## 7. Dead / duplicated / unreachable code

- None dead. `DOCTYPE_MATROSKA`/`DOCTYPE_WEBM` (pub consts) are not referenced by mod.rs:343-346, which matches the literals `"matroska" | "webm"` directly — the pub consts are documentation-only (dead-ish; S4 note, folded into per-function table).
- The module doc's "card 5.D1 / 5.D2 / issue #34" references are process labels, not code defects.

## 8. Dependencies & flags

- No feature flags gate this file (`crates/demux/Cargo.toml` — mkv module unconditional; verified via `mod.mkv` in src tree; no `#[cfg]` in this file except `#[cfg(test)] mod tests`).
- Dependencies used: `media_types` (ids/time/EngineError/Track), `crate::mkv::ebml`, `crate::mkv::error`. Tests additionally use `crate::avi::test_support::mem_stream` and `testkit::Must` — the mkv zone's tests depend on the AVI vertical's test support (cross-zone coupling; hygiene).

## 9. Open questions for the next agent

- `bench.rs:334` hard-codes "no decoder in this build (first vertical: mjpeg + pcm)" while `pipeline_host.rs:346` dispatches `H264SwDecoder` for "h264" — is the bench message stale, or is the h264 lane not actually reachable in the release build? (apps/cli/src/bench.rs owner; W13-A03 already flagged.)
- ContentEncodings (encryption) is skipped as an unknown child (:654-657) — an encrypted MKV parses as a plain track; verify the decode-time refusal is typed and the matrix doesn't claim encrypted-MKV support (wave 18/19).
- ebml.rs:386-387 `read_exact_at`: the bound check uses `at.saturating_add(want)` but the error mint uses non-saturating `at + want` — overflow only reachable with `at` near `u64::MAX`, which the walk shape prevents here; worth a defensive fix in ebml.rs (its owner).
- Whether `mkv_extras_wiring.rs` (my next-but-one file) or `mkv_fixtures.rs` covers the missing-CodecID path of F-001 — checked in my 4th file; preliminary grep says no (no `unwrap_or_default`-triggering case).
