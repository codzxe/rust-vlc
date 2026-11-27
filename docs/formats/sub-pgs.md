# Subtitle record — PGS (HDMV / Blu-ray) parse + decode

Records-consolidation entry (card 8.G1, issue #58 — the format-records
audit found the implemented `subtitle-pgs` matrix row had no
docs/formats record; engine evidence from card 5.N1, ADR-012
self-build). Front-matter keys per doc 04 §6:

- id = `subtitle-pgs` (matches `docs/feature-matrix.toml`; row
  status `implemented` on win/linux/macos, `not_started` on web, with
  evidence `crates/subtitles/src/pgs/**` — this record consolidates
  it)
- status = **implemented** (segment grammar, epoch state machine, RLE
  + palette decode to straight-alpha RGBA, reduce to the compositing
  contract — the ADR-012 self-build, zero dependencies)
- phase = 5 (card 5.N1; in tree by the 5.R2-V Phase-5 exit `3116aec`)
- fuzz_target = `fuzz_sub_pgs` (in-crate survival harness
  `crates/subtitles/src/pgs/fuzz.rs` + the integration survival
  suite `crates/subtitles/tests/pgs_fuzz_survival.rs`)
- crates = [] — self-built, std-only
- self_built = [`subtitles::pgs` (`segment`/`palette`/`rle` + the
  epoch state machine in `mod.rs`)]
- last_audit = 2026-11-10 (this consolidation; engine evidence from
  card 5.N1, re-run this session — below); **amended 2026-11-27
  (Devil-Fix FX9-A07 — the FX9 consumer lanes, see the amendment at
  the end)**

## Parse / Decode / Render

- **Parse + decode (landed)**: PCS/WDS/PDS/ODS/END segments in BOTH
  framings (`.sup` PTS-prefixed and Matroska-stripped); the epoch
  state machine (acquisition/epoch-start flush, object and palette
  banks, multi-fragment ODS assembly); the PGS RLE codes (literal
  pixels, `0x00` escapes: EOL, 1–63 runs, 16-bit long runs) with
  typed clip/snap damage; PDS 16-bit YCbCrA → straight-alpha RGBA8
  with BT.601-vs-BT.709 selection by video height (the reference
  decoder's rule).
- **Render**: reduce to `compositing::CompositedCue` — the 5.L1
  compositing contract. The renderer overlay contract was the open
  R-27 gap at this record's 2026-11-10 writing — **closed 2026-11-27
  by the FX9 consumer lanes (see the amendment below; ADR-012's
  landing tracker closed with it)**.

## Evidence

- Module: `crates/subtitles/src/pgs/` — the module header carries
  the ownership map and the epoch lifecycle model.
- Matrix row evidence quotes: "32 unit tests + fuzz_sub_pgs survival
  harness, all green in this tree 2026-10-26" (5.R2-V exit).

## Gates (this consolidation, exact commands, run 2026-11-10)

Shared-target worktree `wt/w24e` @ `c339ee8`, 2-core Linux sandbox:

- `cargo test --locked -p subtitles --lib pgs` — **32 passed /
  0 failed**.
- `cargo test --locked -p subtitles --test pgs_fuzz_survival` —
  **2/0**.

## Known gaps / honest limits

- ~~No renderer-side overlay proof yet (R-27; ADR-012) — parse/decode
  to composited cues is landed, on-screen compositing is not.~~
  Closed 2026-11-27 — the FX9 consumer lanes composite onto the
  presented frames (amendment below).
- Linux-only evidence (program-wide limit, `docs/KNOWN-LIMITS.md`
  §1); web column `not_started`.

## FX9 amendment (2026-11-27, Devil-Fix wave FX9, agent FX9-A07)

What the tree supports TODAY (2026-11-27) — both consumers of the
5.N1 grammar are now shipped and wired in the CLI session host
(`apps/cli/src/pipeline_host.rs`):

- **Sidecar `.sup`** — `subtitle.add-file` on a `.sup` path:
  `SidecarFormat::Pgs` → `load_pgs_sidecar` (PgsDecoder over the
  PTS-prefixed framing) → `CompositedCue`s on the consumer's
  subtitle lane. Acceptance:
  `fx9_a03_pgs_sup_sidecar_composites_the_crafted_display_set`
  (crafted display set, ink present inside the cue window and
  byte-identical outside it, bottom-band placement) in
  `apps/cli/tests/fx9_a03_subtitle_composite.rs`.
- **Embedded MPEG-TS track** — the TS demuxer's stream-type `0x90`
  mint (`"pgssub"`, `crates/demux/src/ts/mod.rs:367`),
  `HostDecoderProvider::decoder_for`'s `"pgssub"` arm wiring
  FX9-A04's `codecs::subtitle::pgs::PgsSubtitleDecoder`
  (`crates/codecs/src/subtitle/pgs.rs`, the `Frame::Subtitle`
  overlay payload of `crates/media-types/src/frame.rs`), the
  consumer's `Frame::Subtitle` accept arm reconstructing overlay
  cues onto the lane. Acceptance:
  `fx9_a03_embedded_overlay_reconstructs_and_composites` +
  `crates/codecs/tests/fx9_a04_pgs_subtitle_decoder.rs` (8 tests,
  e.g. `fx9_a04_decode_emits_subtitle_frame_with_overlay_geometry`,
  `fx9_a04_overlay_cues_z_ordered_per_pcs_object_order`,
  `fx9_a04_ts_pgs_descriptor_and_engine_lane_semantics`);
  registry pins `apps/cli/tests/fx2_a01_registry_truth.rs` +
  `apps/cli/tests/command_surface.rs`.
- **R-27 closed**: both lanes reach
  `SubtitleLane::composite_onto` before the scheduler push, so the
  presented-frame tap inherits the composited pixels (determinism
  oracle `fx9_a03_composited_presented_frames_are_deterministic`).

Honest gaps that remain (post-FX9):

- No real-Blu-ray `.sup` corpus differential — the FX9 oracles use
  crafted display sets (same no-differential honesty class as the
  DVB/VobSub records; ffmpeg refuses text→bitmap transcoding).
- The embedded lane is TS-only: no MKV `S_HDMV/PGS` descriptor mint
  exists (MKV subtitle tracks deselect at open —
  `apps/cli/src/containers/mkv.rs`
  `multitrack_fixture_deselects_subtitles_at_open`).
- Linux-only evidence; web column `not_started`.
- Matrix row `subtitle-pgs` re-stated 2026-11-27 with this record
  amended (the row's earlier "record owed" note was stale — the
  8.G1 record existed since 2026-11-10).
