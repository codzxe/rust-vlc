# Video record — MPEG-1 (ISO/IEC 11172-2) / MPEG-2 (ITU-T H.262) video

Decision record (card 5.H1). **Code landed this card**: the
sequence-layer parser (`crates/codecs/src/video/mpeg12/` — start
codes, sequence/GOP/picture/slice headers, MPEG-2 extension family,
the one-pass `Mpeg12Facts::scan`, the descriptor gate and the typed
decode refusal). **No decoder is admitted** — the boundary below.
Front-matter keys per doc 04 §6:

- id = `codec-video-mpeg1-2` (matches `docs/feature-matrix.toml`,
  which carries `status = blocked-decode` ambition via
  `not_started` rows and this record as their evidence)
- status = **blocked** (software decode; sequence-layer facts +
  typed refusal landed 5.H1)
- phase = 6 for decode admission (feasibility video-codecs §4 F-5
  schedules the gated evaluation; matrix `phase_target = 5` recorded
  the ambition)
- fuzz_target = `fuzz_codec_mpeg2` (planned name, doc 04 §3.1 row —
  does not exist until a decoder is admitted; fuzz-lite of the
  landed scan lives in `facts.rs` today)
- crates = [] — **none admitted**; two candidates recorded below
- self_built = [`codecs::video::mpeg12` (sequence layer, 5.H1)]
- last_audit = 2026-10-24 (card evidence; prior: feasibility
  video-codecs §1 row 9, 2026-10-19, all `[RV]`)

## Verdict (sequence layer now; decode blocked, two named candidates)

1. **Sequence layer: SELF-BUILT, landed.** The clean-room parser is
   the honest substrate for everything else: probe/track facts
   (resolution, rate, profile/level, chroma, interlace, GOP
   timecode, picture-type census), the hw-feed seam (ADR-001
   Phase-7 adapters consume exactly these structures), and the front
   half of any future decoder. Differential vs ffprobe (ffmpeg
   7.1.5, the build that authored every fixture byte): 4 raw-ES
   fixtures + 1 TS lane + 2 committed PS fixtures + all 3 corpus
   mpeg2video lanes — **10/10 PASS, 0 FAIL** (table in the 5.H1
   worklog entry).
2. **Software decode: BLOCKED for v1 — no validated pure-Rust
   engine.** Two pre-1.0 candidates sit below the admission bar
   (feasibility §1 row 9, tarball-scanned 2026-10-19):
   - `oxideav-mpeg12video` 0.0.13 (2026-04-18, MIT, 2,427 dl;
     tarball: 124 `.rs`, 0 C/asm, no build.rs; claims full MPEG-1 +
     MPEG-2 decode *and* encode, clean-room) — 0.0.x version, no
     conformance evidence, unproven vs our admission bar (doc 11).
   - `na_mpeg2_decoder` 0.1.0 (2026-10-08, 72 dl) — newborn,
     essentially unaudited.
   - `libmpeg2` (the reference decoder VLC/ffmpeg lineage uses) is
     C — forbidden (doc 11 §2).
3. **Typed-refusal semantics, not silence.** `mpeg1video` /
   `mpeg2video` descriptors gate in (`facts::descriptor_gate`) and
   decode refuses with `ErrorKind::Unsupported` at
   `mpeg12.decode.blocked` — probe/track facts still flow (the
   stream-granularity policy: demux + identify + play other
   streams).

## Evidence

- **Differential table (from the 5.H1 worklog entry; live ffprobe
  7.1.5, the build that authored every fixture):** 10/10 PASS, 0
  FAIL — mpeg2-main-ipb.m2v (Main@8, I+P+B census),
  mpeg2-422.m2v (escaped 4:2:2 0x85), mpeg2-interlaced-ntsc.m2v
  (ILACE, 30000/1001), mpeg1-ipb.m1v, mpeg2-in-ts.ts (our TsDemuxer
  + PES seam), ps-mp2-video-audio.mpg + ps-vob-ac3.vob (our
  PsDemuxer, 720×480), corpus 0065-avi-video.avi, corpus
  0128-param-midchange.mp4 (`sequence_changes=true`, 4 seqs), corpus
  0130-avi-video.avi.
- **Tests at card gate:** 380 lib + 43 integration suites, 0 FAILED
  (incl. mpeg12_corpus 3/3 tests, 10/10 rows); fmt + clippy clean.
- **Fuzz-lite:** 8k rounds random+mutated+truncated, scan total,
  counters monotone.

- Feasibility video-codecs.md §1 row 9 + §5 "MPEG-1/2" (2026-10-19,
  `[RV]` tags; crates.io API + static.crates.io tarball scans):
  the two candidates above, the libmpeg2-is-C fact, hw map below.
- **This session (2026-10-24) could NOT re-query crates.io** (no
  egress from the worktree environment; every exact-name query
  returned empty). The candidate list is therefore dated
  2026-10-19, flagged here rather than silently re-asserted — the
  Phase-6 admission spike MUST re-verify versions/status before any
  Cargo.toml edit (its first step regardless).
- Corpus: **3 mpeg2video lanes** in corpus-v4/media (0065 + 0130
  avi 4:2:2, 0128 mp4 param-midchange) — all scanned through our
  parser post `ffmpeg -c copy` extraction and matched against
  container-level ffprobe: PASS on size/profile/level/chroma/rate
  (0128 additionally exercises `sequence_changes = true`, the
  mid-stream header change the corpus lane was built for).
- Fixture inventory (committed, ffmpeg 7.1.5-authored):
  `mpeg2-main-ipb.m2v`, `mpeg2-422.m2v` (escaped 4:2:2 profile
  0x85), `mpeg2-interlaced-ntsc.m2v`, `mpeg1-ipb.m1v`,
  `mpeg2-in-ts.ts`.

## Gaps (honest)

- **No macroblock-layer decode** — no IDCT, no motion compensation,
  no inverse quant; slice parsing stops after macroblock-addressing
  VLC evidence (`slices_with_mb_address`).
- **AVI/MP4 corpus lanes are not demuxed by our own demuxers for
  this test**: the landed AVI demuxer is MJPG-only v0 and MP4 maps
  OTI-less `mp4v` without the MPEG-1/2 refinement — the corpus test
  extracts ES bytes with `ffmpeg -map 0:v:0 -c copy` instead and the
  gap is recorded here (the TS lane DOES run through our TsDemuxer
  PES seam; PS lanes through our PsDemuxer).
- `sequence_display_extension` (colour description) and
  `quant_matrix_extension` are counted (start codes) but not parsed
  into facts — no consumer yet.
- Fuzz is fuzz-lite in-tree (8k rounds, `facts.rs`); the registered
  `fuzz_codec_mpeg2` target arrives only with a decoder.

## Next step (Phase 6, the F-5 gated evaluation)

Re-verify both candidates on crates.io (versions move — see
Evidence), then S-spike `oxideav-mpeg12video` in a scratch harness
(NOT the workspace graph): Xiph/ITU conformance vectors + ffmpeg
7.1.5 differential on our corpus lanes + purity re-scan; admit via
the ruopus validate-then-admit pattern (pin exact version, glue
ours, fuzz under `fuzz_codec_mpeg2`) or fall back to the L-sized
self-build on the sequence layer already landed here. HW parallel
track (ADR-001 Phase 7): Windows "MPEG-2 Video Extension"
(`9n95q1zzpmh4`), VA-API Intel/AMD-class + NVIDIA shim; macOS has
no public VT MPEG-2 decode.
