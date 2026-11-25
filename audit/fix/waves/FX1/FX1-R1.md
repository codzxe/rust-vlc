<!--FIXV1
agent: FX1-R1
title: remedial lint fallout — OpenedMedia Debug format + unused Demuxer import in the two new apps/cli FX1 tests; sibling FX1 test files swept clean
verdict: DONE
changed:
- apps/cli/tests/fx1_a09_ps_guard.rs
- apps/cli/tests/fx1_a05_argv_adts.rs
tests:
- fx1_a05_argv_string_passes_valid_tokens_through
- fx1_a05_argv_string_refuses_non_utf8_typed
- fx1_a05_zero_length_adts_frame_does_not_wedge_the_au_split_walk
- fx1_a05_zero_length_adts_frame_stops_the_walk_after_good_aus
- fx1_a09_ps_mpeg1_header_overruns_declared_len_is_typed_not_a_panic
- fx1_a09_ps_mpeg1_lane_still_opens_after_the_guard
evidence_read:
- audit/fix/RULES.md
- audit/fix/FIX-PLAN.md
- audit/fix/waves/FX1/FX1-A05.md
- audit/fix/waves/FX1/FX1-A09.md
-->

# FX1-R1 — test lint fallout (wave FX1 remedial batch)

Scope: the two cited compile/lint errors in the new apps/cli FX1 test
files, plus a preemptive sweep of the same failure classes across all
eleven new FX1 test files. Tests only — no src file touched, no
Cargo.toml touched, no Debug impl added to any src type.

## Approach

1. Fix the two cited errors with minimal diffs, in the files' own style
   (match + `unreachable!`, `Ok(_)` pattern like the sibling demux
   tests).
2. Sweep every other new FX1 test file for classes (a) unused imports,
   (b) `{:?}` on types without a provable `derive(Debug)`, (c)
   `.unwrap()`/`.expect()` against the crate lint style, (d)
   hallucinated APIs. No cargo is allowed in this box, so every claim
   was grounded by rg into the crates' `src/` (each `use` line has a
   user; each `{:?}` target's Debug derive located; each referenced
   fn/field/constant located with its real signature).

## Changes per file

- `apps/cli/tests/fx1_a09_ps_guard.rs:75` —
  `Ok(opened) => unreachable!("… got {opened:?}")` →
  `Ok(_) => unreachable!("the over-run PES must refuse, got an open
  session")`. `OpenedMedia` (apps/cli/src/demux_host.rs:270) has no
  `derive(Debug)` and the assignment forbids adding one; the `Ok(_)`
  pattern also keeps `unused_variables` (denied workspace-wide) clean
  where the old binding would now be unused. The assertion above the
  arm (the typed `demux.ps.bad-pes-header` refusal) is untouched.
- `apps/cli/tests/fx1_a05_argv_adts.rs:30` —
  `use plugin_api::{DemuxEvent, Demuxer};` → `use plugin_api::DemuxEvent;`.
  `Demuxer` is genuinely unused: `opened.demuxer` is
  `Box<dyn Demuxer>` (demux_host.rs:275) and trait-object method calls
  (`next_event`) resolve without the trait in scope — exactly why rustc
  flagged `unused import: Demuxer`. `DemuxEvent` stays (the four match
  arms use it).

Both files rustfmt'd (RULES exception, only files I edited); line
counts unchanged (111 / 314) — no reflow beyond my two edits.

## Sweep results (classes a–d, all eleven FX1 test files)

Grounding method: rg every `use` line against the file body; rg every
`{:?}`/`{}` target's type for `derive(Debug)`/`Display`; rg every
referenced path/fn/field in the owning crate's `src/`.

- `crates/codecs/tests/fx1_a08_tag_bounds.rs` — clean. Imports all used
  (`EngineError` backs `code_of`). `{e}` needs Display:
  `EngineError` implements `Display` (media-types/src/error.rs:289) and
  `Debug` (:258). `parse_vbri`/`parse_xing` re-exported at
  `codecs::audio::mp3` (mp3/mod.rs:74), `parse_streaminfo` at
  `codecs::audio::flac` (flac/mod.rs:63-65); `StreamInfo` fields match
  (metadata.rs:89-106); `media-types` is a regular dep of `codecs`
  (available to integration tests).
- `crates/demux/tests/fx1_a02_short_ts.rs` — clean. `{err:?}` targets
  `TsDemuxer::new`'s `EngineError` (Debug ✓). `plugin_api::Demuxer` IS
  used here — `demuxer.start()` on the concrete `TsDemuxer` is a trait
  method (plugin-api/src/demux.rs:58). `err.context.code` is `String`
  (error.rs:224) so `assert_eq!(…, "demux.ts.no-pat")` is `String == &str`
  ✓. `mem_stream`/`probe`/`PACKET_BYTES` verified (ts/mod.rs:81/315/596,
  ts/test_support.rs:20).
- `crates/demux/tests/fx1_a03_mp4_box_bounds.rs` — clean. `{kind:?}` is
  `&[u8; 4]` (Debug ✓). `Demuxer` import used (`Mp4Demuxer::next_event`,
  trait method — mp4/mod.rs:2025). `TrackSpec`/`SampleSpec` fields match
  the builders exactly (mp4/test_support.rs:18-39); `progressive`,
  `mem_stream` exist (:313/:566). `Stamp::Present(Timestamp)` with pub
  `ticks` (media-types time.rs:138/723); `Packet.pts: Stamp` ✓.
- `crates/demux/tests/fx1_a04_mkv_lacing_window.rs` — clean. `{other:?}`
  targets the test's own `WalkEnd` (`#[derive(Debug)]` ✓). All five
  `plugin_api` imports used (`SourceStream` impl + `Box<dyn SourceStream>`,
  `SeekOrigin`, `SourcePosition`, `DemuxEvent`/`Demuxer` for the
  concrete `MkvDemuxer`). `parse_block(&data, 0, true)` matches
  cluster.rs:155; `Lacing`/`BlockFrame{start,len}` ✓; all nineteen
  `ebml::ID_*` constants exist (ebml.rs:52-148); `MAX_CLUSTER_WINDOW`
  (mkv/mod.rs:72) and `MkvDemuxer::new(Box<dyn SourceStream>)` (:163) ✓;
  `avi::test_support::{BytesStream, mem_stream}` ✓; `p.data.to_vec()`
  rides `SharedBytes`'s `Deref<Target = [u8]>` (packet.rs:219);
  `.must()` on `recv_timeout` is legal (`RecvTimeoutError: Display`).
- `crates/demux/tests/fx1_a07_asf_child_size.rs` — clean. All imports
  used (`Demuxer` for `AsfDemuxer::start`, asf/mod.rs:636).
  `test_support::{mem_stream(Arc<[u8]>), minimal_asf_audio, waveformatex}`
  verified (asf/test_support.rs:19/105/163); `tracks[0].codec.as_str()`
  ✓ (`CodecDescriptor::as_str`, media-types ids.rs:594).
- `crates/media-probe/tests/fx1_a02_short_ts.rs` — clean. `{report:?}`
  targets `ProbeReport` (`#[derive(Debug)]` — report.rs:269).
  `ProbePlugin` used (trait method `probe`, signature/mod.rs:65);
  `registry.probe(prefix, meta, config)` 3-arg ✓ (:175);
  `ProbeConfig::new(usize, u8) -> Result` ✓ (config.rs:52);
  `SourceMeta::new(Option<u64>, Option<&str>)` ✓ (report.rs:302);
  `hit.prefix_consumed` / `report.bytes_examined` / `FormatId::as_str` ✓;
  `err.stage` is a pub field (error.rs:260).
- `crates/streaming/tests/fx1_a06_numeric_guards.rs` — clean. All
  imports used: `SourceStream` is needed — `open_master` returns
  `HlsAdaptiveStream` which implements `SourceStream` (master.rs:655),
  so `read`/`seek`/`position` are trait calls; `SeekOrigin`/
  `SourcePosition(pub u64)` ✓ (plugin-api access.rs:88-99).
  `plan_representation(&Mpd, &Period, &AdaptationSet, &Representation,
  &str)` ✓ (resolve.rs:534); `DashResolveError` has
  `PartialEq+Debug` (resolve.rs:60) so `assert_eq!(…, Err(…))` compiles;
  `RepresentationPlan{timescale,segments}` / `SegmentPlan{time,duration}`
  ✓ (:147-180). `playlists::parse_mpd_bytes` + the
  `.mpd/.periods/.adaptation_sets/.representations` chain ✓
  (playlists/src/mpd.rs:139-259/685; `playlists` is a regular dep of
  streaming). `MpdError: Display` ✓ (mpd.rs:482). `HlsMasterConfig::
  default()` + pub `max_retained_bytes` ✓ (master.rs:334-354).
  `TimestampMap` has `Debug+PartialEq`, `parse_tag_value`/
  `from_playlist_text`/pub `local_ms` ✓ (timestamp_map.rs:54-103).
  `MockConfig{routes, ..Default::default()}` — all fields pub, Default
  ✓ (mockhttp.rs:342-427); `MockServer::spawn -> io::Result` /
  `url` / `client` ✓ (:453/:476/:481), `MockClient: HttpFetch` ✓
  (:892). `testkit::assert_err` exists (testkit/src/lib.rs:100).
- `crates/subtitles/tests/fx1_a07_ass_overflow.rs` — clean. `ass::parse
  -> AssDocument` (ass/parser.rs:349); `stats.{applied_override_tags,
  position_overrides_applied, position_overrides_unmappable}` ✓
  (parser.rs:206-223); `Positioning{alignment, line, position}` ✓
  (model/position.rs:217-224); `Percent::HALF`/`get()` ✓ (:32/:49);
  `LinePlacement::Percent(Percent)` ✓ (:205); `CueText::Plain(String)`
  with `PartialEq+Debug` ✓ (model/cue.rs:44-50); `CueInterval::
  start_us()/end_us()` ✓ (model/time.rs:56-63); `AssDamage: Debug` ✓
  (parser.rs:113).
- `crates/subtitles/tests/fx1_a09_guards.rs` — clean. `with_odd_parity`
  exists and is pub (cc/eia608.rs:78 — the same import path the neighbor
  `tests/cc_acceptance.rs:18` already uses); `CcDataTriplet::cc(field,
  [u8;2])` ✓ (cc_data.rs:72); `CcDecoder::decode(&[CcDataTriplet],
  Option<i64>) -> CcUpdate{events, damage}` ✓ (cc/mod.rs:237/189-199);
  `CcCaptionEvent{channel, grid}` + `CcGrid::row_text(usize)` ✓
  (cc/mod.rs:177-184, eia608.rs:209); `CcDamage::InvalidPac` ✓
  (cc/mod.rs:106). `pgs::MAX_COMPOSED_RGBA_BYTES` (pgs/mod.rs:91),
  `PgsDamage::{RleOverrun, CompositionRetained{bytes}}` (:123/:139),
  `PgsDecoder::decode_display_set(&[u8], Option<i64>)` (:319),
  `PgsDisplaySet{objects, damage}` (:226-228), `PgsObjectImage.rgba`
  (:209) ✓. `DvbDecoder::decode_pes(&[u8], Option<i64>)` (dvb/mod.rs:339),
  `DvbDamage::{BadClutId{clut_id,region_id}, StaleClutIndex{region_id,
  index}}` (:104/:129), `DvbPageUpdate{regions, damage}` (:231/:236),
  `DvbRegionImage.rgba` (:217) ✓. All damage enums derive `Debug` ✓
  (the `{:?}` assert args).
- Class (a) — unused imports: the cited `Demuxer` in fx1_a05 was the
  ONLY instance across all eleven files; every other `use` line has a
  user (verified one by one above).
- Class (b) — `{:?}` sweep: full rg inventory across the eleven files
  yields exactly the cases `{token:?}` (String ✓), `{err:?}`
  (EngineError: Debug ✓), `{kind:?}` (&[u8;4] ✓), `{other:?}`
  (WalkEnd: Debug ✓), `{report:?}` (ProbeReport: Debug ✓), the
  positional `{:?}` on damage enums (Debug ✓) — and the one offender
  `{opened:?}` (OpenedMedia: no Debug), now fixed.
- Class (c) — `.unwrap()`/`.expect()`: rg finds ZERO occurrences in any
  of the eleven files (the files use `.must()`/match+`unreachable!`,
  the house style; apps/cli neighbors like ttf_seek.rs /
  container_conformance.rs are `.must()`-only too). Nothing to relax.
- Class (d) — hallucinated APIs: none found. Every path/fn/field/
  constant referenced by the nine sibling files was located in the
  crate's src with a matching signature (the groundings listed above).
  No test needed an API correction.

No test fn renamed; all six names in the two changed files are
preserved verbatim (the cfg(unix)/cfg(windows) twins share one name by
design — only one compiles per target, as authored by FX1-A05).

## Risks & followups

- No local compile is allowed in this box (RULES): the sweep is
  rg-grounded, not compiler-grounded. Residual risk is low but not
  zero — the classes checked are exactly the four the gate hit, and the
  one unprovable-Debug case plus the one unused import were the only
  members found.
- The two edited lines change only failure-path message text; the
  runtime oracles (typed refusal codes, recovered-damage tuples,
  `mpegps`/`mpegts` container ids) are untouched and remain the fix
  agents' responsibility (FX1-A05/A09 reports).
- If the next central gate still fails inside one of the nine swept
  files, the failure would be a class outside my sweep scope (e.g. a
  runtime assertion), not one of classes (a)–(d).
