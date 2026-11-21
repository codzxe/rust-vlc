# AUDIT: crates/transcode/src/mp4mux.rs#part1
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-21 |
| Chunk boundaries | **part1 = lines 1–463** (file is 912 lines; the midpoint 456 falls inside `put_tkhd` (439–462), so the split was moved to the function boundary after it — part2 (A09) = 464–912: `put_mdhd`, `put_hdlr`, `put_stbl`, `nanos_to_ticks`, `sample_deltas`, duration/matrix/dimension helpers, `put_box`, sink folds, and the `tests` module (712–912, which exercises both halves). |
| Lines / functions | 463 / 16 (module doc + 4 consts + 2 fourcc/setup types + 4 structs + 8 methods/fns incl. `sample_entry_fourcc`) |
| File verdict | UNREACHABLE (REAL, heavily verified muxer — round-trip e2e green — but no shipped entry point links the transcode crate at all) |
| Reachable from shipped entry point? | **NO.** `Mp4Remuxer::new` has exactly one production caller: `crates/transcode/src/runner.rs:767` (the transcode runner). No app depends on `transcode`: rg "transcode" in apps/{cli,desktop,desktop-tauri,control-server,web}/Cargo.toml and crates/commands/Cargo.toml → zero matches. The crate builds as a workspace member (baseline log line 12) and is shipped nowhere. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "MAX_SAMPLE_ENTRY_BYTES"` → enforced ONLY at graph.rs:361. `sed -n '735,775p' runner.rs` → the direct-API host checks fourcc + native timescale, never the entry size. `rg -l "transcode"` over apps/*/Cargo.toml → nothing. Baseline: `orchrun-test-workspace-nofailfast.log.txt:5841-5847` (mp4mux unit 4/4 ok), `:5880-5889` (tests/mp4_remux.rs 7/7 ok incl. packet-identical round-trip), `:5891-5900` (record 5/5 ok, re-demux+re-decode bit-exact). `sed -n` on all quotes. |

## 1. One-paragraph verdict
This is the best-engineered file in my assignment: a clean-room ISO-BMFF writer with an honesty table that matches the code (verified field-by-field for `mvhd`/`tkhd` against ISO 14496-12 layouts), checked arithmetic on every size/timescale path, typed refusals instead of wraps, and an integration suite that remuxes real fixtures and re-demuxes them packet-identical — all green in the citable baseline, including a cancel-leaves-playable-prefix test. The defects are contract-level, not correctness-level: the advertised 64 KiB sample-entry ceiling is enforced by exactly one of the two host paths and never by the muxer itself, so the runner lane bypasses it; the `tkhd` flags comment mislabels `0x7` (it includes `track_in_preview`); and `mvhd`/`tkhd` durations saturate at u32 while `mdhd` upgrades to v1 — an internally inconsistent (but documented) story for absurd durations. Nothing here is reachable from the shipped binary — the entire transcode crate has no app consumer.

## 2. What actually works (verified)
— Round-trip proof: `orchrun-test-workspace-nofailfast.log.txt:5880-5889` — `progressive_source_remuxes_and_redemuxes_packet_identical ... ok`, `fragmented_source_remuxes_into_a_progressive_output ... ok`, `edit_list_source_re_anchors_the_media_timeline ... ok`, `mid_remux_cancel_leaves_an_honest_playable_prefix ... ok` (7/7). The e2e demuxes the muxer's own output with the real `demux::mp4` and compares packets.
— Unit tests 4/4 ok (`:5841-5847`): byte-level box assertions (`one_track_two_samples_finalizes_a_wellformed_file` pins ftyp at 4..8, mdat fourcc at 36..40, the patched mdat size = 8+64, and that the moov size field is honest), typed refusal codes for bad ordinal / missing DTS / backwards DTS (`:5861-5847` → `transcode.mp4.track-index`, `.stamp-missing`, `.dts-non-monotonic`), and the sample-table ceiling driven directly (`:5886-5909`).
— `mvhd`/`tkhd` v0 layouts hand-checked against ISO/IEC 14496-12 §8.2.2/§8.3.1 field order and sizes (rate 0x00010000, volume 0x0100, 10 reserved bytes, 36-byte matrix, 24 pre_defined; tkhd: flags/id/reserved/duration/8 reserved/layer/group/volume/2 reserved/matrix/width/height 16.16) — all match `put_mvhd` (423-436) and `put_tkhd` (439-462).
— `add_packet`'s ceiling arithmetic traced: `after + 8 > u32::MAX || mdat_payload_at + after > u32::MAX` (285) keeps both the mdat extent and every stco offset inside 32 bits before any byte is written; `finalize`'s patch is a checked re-derivation (317-321), not a trust.

## 3. Findings

### F-001 [S3] — the advertised sample-entry ceiling is enforced by one host path out of two, and never by the muxer itself
Where: mp4mux.rs:66-73 (doc contract) + 210-233 (`new` performs no check); enforcement exists only at graph.rs:361; runner.rs:745-766 (the direct host) never checks it
Code:
```rust
//! # Typed ceilings (MP §12 — bounded, never wrapped)
//!
//! * per-track sample count ≤ [`MAX_REMUX_SAMPLES`] (the demux
//!   layer's own sample ceiling, mirrored — a 1 Mi-sample table stays
```
(and `new`, which accepts any entry:)
```rust
    pub fn new(
        out: Box<dyn WriteSeek>,
        setups: Vec<Mp4TrackSetup>,
    ) -> Result<Mp4Remuxer, EngineError> {
        let tracks = setups
            .into_iter()
            .map(|setup| TrackState {
```
(and the only enforcement, in a different lane — graph.rs:361-365:)
```rust
                    if config.sample_entry.len() > MAX_SAMPLE_ENTRY_BYTES {
                        return Err(mp4_sample_entry_over_budget(
                            config.sample_entry.len(),
                            MAX_SAMPLE_ENTRY_BYTES,
                        ));
                    }
```
What is wrong: the module doc lists "sample-entry bytes ≤ MAX_SAMPLE_ENTRY_BYTES" among the muxer's typed ceilings, but `Mp4Remuxer::new`/`add_packet` never look at the entry size. The graph lane (graph.rs:361) refuses oversized entries; the runner lane (runner.rs:735-767) checks the fourcc and the native timescale and then hands the entry straight to `Mp4Remuxer::new` — a runner-driven remux of a source with a >64 KiB sample entry writes it verbatim into `stsd`.
Why it matters: the doc's "bounded, never wrapped" section is the API contract; one of the two in-tree hosts violates it silently today, and every future host inherits the hole. Practical risk is low (the source demuxer's own 16 MiB moov ceiling bounds the entry), but a contract that is enforced in one lane and not the other is exactly how drift starts — and the sample-table ceiling directly above it IS enforced in the muxer (275-280), so the asymmetry is accidental, not principled.
What correct looks like: check `setup.config.sample_entry.len() > MAX_SAMPLE_ENTRY_BYTES` in `Mp4Remuxer::new` (typed refusal), keeping graph.rs's earlier check as a friendly pre-flight.
Confidence: HIGH — Verification: read + rg (single enforcement site) + runner.rs source read.

### F-002 [S4] — `tkhd` flags comment mislabels `0x7`: it also sets `track_in_preview`
Where: mp4mux.rs:117-118 (and the module doc line 18 repeating "flags 0x7")
Code:
```rust
/// `tkhd` flags: track_enabled | track_in_movie.
const TKHD_FLAGS_ENABLED: u32 = 0x7;
```
What is wrong: per ISO/IEC 14496-12 §8.3.1, bit 0 = `track_enabled`, bit 1 = `track_in_movie`, bit 2 = `track_in_preview`. `0x7` sets all three; the comment names only two.
Why it matters: pure doc accuracy on a spec-facing constant; a reader auditing flag semantics against the comment concludes preview playback is disabled when it is not. Harmless in practice (players ignore in_preview).
What correct looks like: comment "track_enabled | track_in_movie | track_in_preview" or the constant `0x3` if preview was meant to be off.
Confidence: HIGH — Verification: read + ISO 14496-12 §8.3.1 flag-bit semantics.

### F-003 [S4] — duration overflow policy is inconsistent across the header boxes: `mvhd`/`tkhd` saturate u32, `mdhd` upgrades to v1
Where: mp4mux.rs:428 and 445 (`put_u32_saturating` calls in `put_mvhd`/`put_tkhd`) vs `put_mdhd`'s v1 branch (466-483, part2)
Code:
```rust
    body.extend_from_slice(&MOVIE_TIMESCALE.to_be_bytes());
    put_u32_saturating(body, movie_dur);
```
What is wrong: a track whose movie-unit duration exceeds u32::MAX (≈49.7 days in ms) gets a saturated (wrong) `mvhd`/`tkhd` duration while its `mdhd` switches to v1 and reports the true value — the same fact is reported inconsistently within one file. The helper's own comment admits the policy ("durations past 49 days in movie units are absurd, and saturating is the crate's overflow policy").
Why it matters: absurd-input edge, self-documented, zero realistic blast radius — but a demuxer that trusts tkhd duration for timeline math sees a lie while mdhd tells the truth. Worth one v1 upgrade for consistency or a doc note in the module's honesty table.
Confidence: HIGH — Verification: read (both paths).
What correct looks like: v1 tkhd/mvhd when duration > u32::MAX (the demux side reads both, per the mdhd comment), or an explicit note in the module docs.

### F-004 [S4] — `new` accepts an empty setup list; the direct API can finalize a trak-less MP4
Where: mp4mux.rs:210-233
Code:
```rust
        let mut muxer = Mp4Remuxer {
            out,
            tracks,
            mdat_size_at: 0,
```
What is wrong: `Mp4Remuxer::new(Vec::new())` succeeds and `finalize()` writes ftyp + empty mdat + a moov with zero trak boxes — a file our own demuxer refuses. The runner lane refuses empty remuxes at submit (`submit_refuses_an_empty_remux` test, baseline :5886), but the muxer's own API is silent about it.
Why it matters: an easy mis-use footgun for direct-API hosts; one typed refusal in `new` (`setups.is_empty()`) would make the API self-defending. Blast radius: none today (the only host already refuses).
Confidence: HIGH — Verification: read; behavior traced (no empty-check anywhere in 210-233 or finalize).
What correct looks like: `if setups.is_empty() { return Err(mp4_no_tracks()) }` in `new`.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-88 | REAL | — | Honesty table verified against code (verbatim payload copy at 516 (part2); rebuilt tables; re-anchored timeline at 263-264; 32-bit ceilings at 285; one-sample-per-chunk stsc at 584-590 (part2)). |
| consts ×4 | 106-118 | REAL | S4 (F-002) | Values sane; flag comment mislabeled. |
| `Mp4TrackConfig`/`Mp4TrackSetup` | 126-149 | REAL | — | Data; `timescale` doc honestly points at the runner's native-base refusal. |
| `sample_entry_fourcc` | 154-160 | REAL | — | Exact stream-copy set; pinned by test incl. cross-kind lies. |
| `SampleRow`/`TrackState` | 163-186 | REAL | — | Data; `origin: Option<i64>` supports negative first DTS per the doc. |
| `Mp4Remuxer` (struct) | 194-203 | REAL | — | Not Clone (one writer owns one output) — enforced by type. |
| `new` | 210-233 | REAL | S3 (F-001) + S4 (F-004) | No entry-size or empty-setup validation; ftyp+mdat header emitted eagerly. |
| `add_packet` | 247-304 | REAL | — | The core: typed ordinal/stamp/monotonic/ceiling/size checks all verified by hand and by tests (861-909); `checked_sub` re-anchor makes negative origins safe; u64→i64 dts store round-trips (`u64::try_from(dts_signed)` cannot fail since dts_signed ≥ 0); row pushed before `write_all` (a sink error leaves the muxer poisoned — acceptable: fallible API, caller aborts). |
| `finalize` | 314-329 | REAL | — | Checked size patch; seek-to-end then single moov write; flush. Defensive re-check of the per-add invariant. |
| `write_ftyp` | 332-343 | REAL | — | isom/0/isom/iso2/avc1/mp41 — pinned by e2e re-demux. |
| `write_mdat_header` | 347-355 | REAL | — | Records both patch and payload offsets via `stream_position`. |
| `build_moov` | 360-419 | REAL | — | Bounded by table ceilings; mvhd uses max movie duration; per-track ids `ti+1`; single `moov` wrapper (justified by comment 410-413 + round-trip test). |
| `put_mvhd` | 423-436 | REAL | S4 (F-003) | Layout hand-verified vs ISO; duration saturates. |
| `put_tkhd` | 439-462 | REAL | S4 (F-003) | Layout hand-verified; width/height from entry's u16 fields at 24/26 via `video_dimensions` (part2); volume per kind. |

## 5. Test honesty (tests live in part2's range 712-912; they pin my functions)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fourcc_mapping_is_the_stream_copy_set` (796) | YES | Exhaustive incl. cross-kind lies. |
| `one_track_two_samples_finalizes_a_wellformed_file` (816) | YES | Byte offsets, patched mdat size, honest moov size — a layout regression anywhere in ftyp/mdat/moov fails it. |
| `typed_refusals_of_the_direct_api` (861) | YES | Three exact dotted codes for my `add_packet` guards. |
| `the_sample_table_ceiling_is_typed` (886) | YES | Pre-fills 1 Mi rows and drives the next add — pins my 275-280 guard without materializing sample bytes. |
| tests/mp4_remux.rs e2e (other file) | YES | Real fixtures → remux → re-demux packet-identical; elst re-anchor; fragmented→progressive; cancel prefix. Baseline 7/7 (`:5880-5889`). |
| Missing coverage | — | No test feeds an over-`MAX_SAMPLE_ENTRY_BYTES` entry to the RUNNER lane (F-001's hole); graph.rs has one for its own lane (graph::tests `:5831`). No u32-saturation duration test (F-003). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "sample-entry bytes ≤ MAX_SAMPLE_ENTRY_BYTES" (typed ceiling) | mp4mux.rs:72 | PARTIAL/FALSE for the runner lane | F-001: enforced only at graph.rs:361; muxer and runner do not check. |
| "total `mdat` payload and every `stco` offset must stay inside the classic 32-bit fields" | mp4mux.rs:73-75 | REAL | add_packet 281-293 + finalize 317-321; ceiling test green. |
| "an encoder-delay trim surfaces as a negative first DTS; the copy anchors that sample at tick 0" | mp4mux.rs:59-64 | REAL | origin re-anchor at 263-264; e2e `edit_list_source_re_anchors_the_media_timeline` green (baseline :5883). |
| "No dependency, no unsafe (C5)" | mp4mux.rs:5 | REAL | std + workspace crates only; no `unsafe` in file (read). |
| matrix row for remux/convert (transcode lanes) | feature-matrix.toml (category transcode-*) | UNREACHABLE from shipped binary regardless of status | No app depends on transcode (rg over all app manifests). |

## 7. Dead / duplicated / unreachable code
- Unreachable: the whole crate (no app consumer) — the muxer is library+tests only.
- The `debug_assert!(fourcc.is_some())` at 510 (part2) documents the runner precondition; the `unwrap_or(b"mp4a")` fallback keeps release builds honest — good pattern (noted for A09).

## 8. Dependencies & flags
- `media_types` (Packet/Stamp/EngineError), `crate::error` typed-ceiling helpers, `crate::graph::WriteSeek`. No feature gates, no unsafe, no external deps (C5 verified). Tests use `testkit::{Must, assert_err}` and an in-memory Write+Seek twin.

## 9. Open questions for the next agent
- For A09 (part2): `put_stbl`'s `u32::try_from(...).unwrap_or(u32::MAX)` fallbacks (stts delta at 525, run counts at 532/553/576, stsz/stco counts at 595/604, put_box size at 696) all write a *lying* size field instead of refusing when a table exceeds u32 — with MAX_REMUX_SAMPLES = 1 Mi these are unreachable (1 Mi × 4 B ≈ 4 MB ≪ 4 GiB), but confirm no path (e.g. stts run explosion from alternating deltas — runs ≤ samples, so bounded) can reach them.
- For A04/A05 (runner.rs): F-001 — add the entry-size check at the runner's setup loop, or fix the doc.
- For the matrix owners: which row claims the remux/record lane, and does its evidence mention that no shipped binary reaches it?
