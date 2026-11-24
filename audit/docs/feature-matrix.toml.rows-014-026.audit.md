# CLAIM AUDIT: docs/feature-matrix.toml — rows 014–026 (chunk 1/2)

| Field | Value |
|---|---|
| Auditor | W18-A08 |
| Date | 2026-11-23 |
| Rows in chunk | 13 (matrix rows 014–026 of 252; ids container-mpeg-es-video … codec-video-av1) |
| Claim verdicts | REAL: 5 · honest-absence TRUE (not_started/blocked honest): 6 · SHALLOW (honest-partial): 2 |
| Internal contradictions found | 2 (row 022 hw-lane reachability vs status cells; row 026 description vs code) |
| Reachability method | call-chain traced + orchestrator baseline (audit/runs/baseline/cargo-test-workspace.txt) — no cargo builds run (2-core/3GB sandbox; target/ absent) |

## 1. Blunt verdict

This chunk is the honest end of the matrix: 6 of 13 rows claim nothing and have
nothing (verified absence), and 5 rows (h264-annexb, hevc-annexb, ivf demux,
h264-software, av1 decoders) are backed by real, tested, shipped-reachable code
whose baseline test runs I cite line-exact. The two "partial" rows (h264 hw,
hevc) are truthful about their gaps — with one material catch the matrix does
NOT say out loud: the H.264 hardware lane has **no consumer in any shipped
binary** (`H264Decoder::from_setting` has zero production call sites; the
`playback.hw-decode-policy` setting is declared but never read), so "partial"
hw decode is real library code that no shipped entry point can engage. The
release binary itself decodes h264/av1 in software (default features include
`codec-h264-sw` and `codec-av1`); the two stale lying registries are
`apps/cli/src/host.rs engine_info()` (hardcodes "mjpeg, pcm_s16le") and
`apps/cli/src/bench.rs` (hardcodes "first vertical: mjpeg + pcm" and refuses
to bench anything else) — both understate, they do not fabricate.

## 2. Claim ledger (matrix rows)

Row numbering = Nth `[[feature]]` in file order (252 total; verified by enumeration).

| Row id | Status claimed (line) | Evidence claimed | Code found | Executed? | Verdict |
|---|---|---|---|---|---|
| 014 container-mpeg-es-video | not_started x4 (L306) | "" (L307) | NONE — `crates/demux/src/es/` has exactly 4 lanes: aac, annexb, h264, hevc, mp3 (`es/mod.rs:41-47`); no mpegv/m1v/m2v wrapper; CLI registry has no such arm | n/a | HONEST (claim true) |
| 015 container-h264-annexb | implemented w/l/m (L319) | es/** + container-es.md (L320) | `demux/src/es/h264.rs`, `annexb.rs`; shipped arm `apps/cli/src/containers/mod.rs:64` `"h264" => es::open_annexb(...H264...)`; probe signature `media-probe/src/signature/es.rs` mints `h264` | baseline: `es_corpus.rs` `corpus_h264_lane_matches_ffprobe_packet_geometry ... ok`; `h264_sw_differential lane_a_baseline_annexb ... ok` (baseline L2240, L1167ff) | REAL |
| 016 container-hevc-annexb | implemented w/l/m (L332) | es/** + container-es.md (L333) | `demux/src/es/hevc.rs`; shipped arm mod.rs:63 `"hevc"` | baseline: `corpus_hevc_lane_matches_ffprobe_packet_geometry ... ok` | REAL |
| 017 container-ivf | implemented w/l/m (L344) | ivf/** record container-ivf.md; "fixture-only evidence — ZERO ivf corpus lanes; VP8 typed v0 refusal" (L345) | `demux/src/ivf/mod.rs` (DKIF walk, fourcc gate AV01/VP90 only — VP80 typed refusal); shipped arm mod.rs:55 `"ivf" => ivf::open` | baseline: `ivf_fixtures.rs`, `ivf_av1_family_lane ... ok`, `ivf_vp9_family_lane ... ok` (L2291ff, L3158-3159) | REAL |
| 018 container-realmedia | not_started x4 (L357) | "" | NONE — `rg -i realmedia|rmvb|RMF` over crates+apps: 0 hits | n/a | HONEST |
| 019 container-midi-smf | not_started x4 (L370) | "" | NONE (only an AVI stream-type comment mentions MIDI) | n/a | HONEST |
| 020 container-archive-zip | not_started x4 (L383) | "" | NONE — "zip" hits in demux are iterator `.zip()` calls (asf/tests.rs:1364 etc.) | n/a | HONEST |
| 021 container-census-long-tail | not_started x4 (L396) | "" | NONE — no caf/voc/xa/tta/mod/sid/nsv demux modules in `demux/src/lib.rs` | n/a | HONEST |
| 022 codec-video-h264 (hw) | partial w/l/m (L414) | h264/** parser corpus-verified 19/19 + 5.F2 hw seam; record video-h264.md: 0019 26/26 + 0014 185/185 (L415) | parser (5 files) + `hw/` (capability/feed/frame_pool + vaapi + videotoolbox lanes); MF lane absent per `hw/mod.rs:51-56` (quote below). Baseline: `h264_parser_corpus` 2 ok, `h264_hw_path` 20 ok; record `docs/formats/video-h264.md:92,97,102` carries the 19/19 + 185/185 numbers. BUT `H264Decoder::from_setting` has NO production caller (only `codecs/tests/h264_hw_path.rs`); `playback.hw-decode-policy` read by no shipped app | tests: yes (baseline L1127-1160); hardware decode: NEVER exercised anywhere [VERIFY — sandbox no GPU] | SHALLOW (honest partial; hw lane UNREACHABLE from shipped entry) |
| 023 codec-video-h264-software | implemented w/l/m (L427) | sw/** rust_h264 =0.6.1 + differential + "engine e2e apps/cli mp4_h264_family_lane (mp4_h264_aac.mp4 plays to Ended, exit 0)" (L428) | `codecs/src/video/h264/sw.rs` (640 LOC rust_h264 wrapper); `Cargo.toml:174 rust_h264 = { version = "=0.6.1", optional = true }`; shipped dispatch `apps/cli/src/pipeline_host.rs:346` `"h264" \| "avc1" => self.seeded(track, config, H264SwDecoder::new)`; `codec-h264-sw` in codecs `default` features → in release lane | baseline: `h264_sw_differential` 17/17 ok (L1161-1185); `mp4_h264_family_lane ... ok` (L3164) — pins `state-changed Playing → Ended`, 100.0%, no `cli.codec_unavailable` | REAL |
| 024 codec-video-hevc | partial w/l/m (L440) | hevc/** parser 3/3 + seam 571/0 (L441) | parser (5 files, 4638 LOC total) + `hw.rs` seam; NO per-OS hw lane, NO software decode (`decode.rs:1-14` "no software decode, by design") | baseline: `hevc_parser_corpus` 3 ok, `hevc_hw_path` 17 ok; shipped CLI refuses hevc (baseline mp4-smoke: exit 65 `cli.codec_unavailable`) | SHALLOW (honest partial; no HEVC decode exists in any shipped binary) |
| 025 codec-video-hevc-software | blocked x4 (L453) | "" + blocked_reason (L455) | NONE — no h265 crate in Cargo.lock; `decode.rs` doc says typed `hevc.hw.required` refusal instead | n/a | HONEST |
| 026 codec-video-av1 | implemented w/l/m (L467) | av1/** + rav1d =1.1.0 asm-off; conformance 15 rows pixel-MD5 PASS (L468) | `codecs/src/video/av1/decoder.rs` (1043 LOC rav1d FFI) + `obu.rs`; `Cargo.lock:674-675` `name = "rav1d" / version = "1.1.0"`; shipped dispatch `pipeline_host.rs:304` `"av1" \| "av01" => Av1Decoder::new()`; `codec-av1` in default features | baseline: `av1_conformance` 1 ok (71.72s), `av1_differential` 6 ok (incl. `differential_pixel_md5_against_ffmpeg_dump`), `ivf_av1_family_lane ... ok` | REAL (description clause "UI states active backend" — no such UI surface exists: PAPER-ONLY clause, see §3) |

## 3. Contradictions

| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Row 022 notes: hw path is "settings-driven, default OFF", implying it can be turned on | `rg 'from_setting\('` → call sites only in `decode.rs` itself + tests; `playback.hw-decode-policy` appears in `apps/cli/src/pipeline_host.rs:342` only inside a comment | The setting is declared (`settings-schema/src/runtime.rs`) and read by NOBODY. Turning it on in any shipped app changes nothing. Doc is honest about OFF but silent about "no consumer exists at all" |
| Row 026 description: "UI states active backend" | No code: `rg -i 'active backend'` in ui/ apps → 0 hits (only transport backends) | No UI surface states the AV1 backend. FALSE description clause (S3) |
| Baseline litmus: "engine-info: decoders: mjpeg, pcm_s16le" | `pipeline_host.rs:241-346` dispatches mjpeg/pcm/flac/mp3/mp2/vorbis/opus/aac-lc/alac/adpcm/av1/h264/vp9/dnxhd | The shipped decoder registry is the 14-decoder one; `host.rs engine_info()` hardcodes 2. engine_info LIES (understates). `bench.rs:334` also hardcodes the first-vertical refusal and refuses to bench real decoders — stale lane, not fabrication |

## 4. Fabrication / inflation indicators

1. Row 022's own notes confess a prior fabrication ("this note previously claimed
   VT/MF/VA-API all landed ... the Media Foundation claim was false (no MF code
   ever existed...)"). The current text matches the code (MF absent — verified
   `hw/mod.rs:51-56`). Retraction recorded, not hidden: good.
2. Evidence strings rely on "record" docs (docs/formats/*.md) whose one-shot numbers
   (19/19, 185/185, 26/26, 3/3, 571/0) I could NOT re-execute (no corpus, no
   ffmpeg in sandbox, no target/ build). They are consistent with the baseline
   test suites that DID run; the raw one-shot differentials are UNVERIFIED-recorded.
3. Row 016 note flags itself `[VERIFY]` (HVCC); row 017 self-flags "fixture-only
   evidence — ZERO ivf corpus lanes" while the baseline actually runs ivf family
   lanes — evidence string is STALE-conservative (understates), the opposite of
   inflation.

## 5. Key verbatim evidence (sed-verified)

Matrix (docs/feature-matrix.toml):
- L306-307: `status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }` / `evidence = ""`
- L427-428 (row 023): `status = { windows = "implemented", ... }` / `evidence = "... engine e2e apps/cli mp4_h264_family_lane (mp4_h264_aac.mp4 plays to Ended, exit 0); Linux-evidenced"`
Code:
- apps/cli/src/containers/mod.rs:63-64:
  `"hevc" => es::open_annexb(demux::es::annexb::AnnexbCodec::Hevc, stream, opts),`
  `"h264" => es::open_annexb(demux::es::annexb::AnnexbCodec::H264, stream, opts),`
- apps/cli/src/pipeline_host.rs:346: `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),`
- crates/codecs/src/video/h264/hw/mod.rs:51-56: "**Media Foundation** (Windows, 7.B2): **NOT IMPLEMENTED** — no MF lane exists in this crate ... retracted."
- crates/codecs/Cargo.toml:174: `rust_h264 = { version = "=0.6.1", optional = true }`
- Cargo.lock:674-675: `name = "rav1d"` / `version = "1.1.0"`
- apps/cli/src/bench.rs:333-336: `"no decoder in this build (first vertical: mjpeg + pcm)"` (stale bench-only lane)
Baseline (audit/runs/baseline/cargo-test-workspace.txt):
- L1161-1185: h264_sw_differential 17 tests ... `test result: ok. 17 passed; 0 failed`
- L3158/3164: `test ivf_av1_family_lane ... ok` / `test mp4_h264_family_lane ... ok`
- L1127-1152: h264_hw_path 20 passed; L1207-1213: hevc_parser_corpus 3 passed.

## 6. Row-level findings

- F-001 [S3] Row 022: hardware lane unreachable from shipped build. Where:
  crates/codecs/src/video/h264/decode.rs:310 (`pub fn from_setting`) — zero
  production callers; setting declared (`settings-schema/src/runtime.rs`) but
  never read by apps/cli, apps/desktop, apps/desktop-tauri. "partial" is
  documented honestly, but no shipped binary can ever negotiate hw decode today.
- F-002 [S3] Row 026 description clause "UI states active backend" — no UI code
  exists for it. PAPER-ONLY clause inside an otherwise REAL row.
- F-003 [S4] Row 017 evidence string says "ZERO ivf corpus lanes" — stale vs
  baseline `ivf_av1_family_lane`/`ivf_vp9_family_lane` (running and green).
- F-004 [S2] Off-row but chunk-adjacent (affects 022/023 credibility):
  `apps/cli/src/host.rs engine_info()` hardcodes `decoders: [mjpeg, pcm_s16le]`
  — a lying registry understating 12 shipped decoders; `bench.rs` (L280-336)
  benches only mjpeg/pcm and labels every other track "no decoder in this build".
