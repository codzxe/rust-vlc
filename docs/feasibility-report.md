# Feasibility Report — Rust-only VLC reimplementation vs reality

Phase 0 deliverable (card 0.D3, Lead-assembled 2026-10-19). Sources: the four
feasibility sections (0.C1-I video, 0.C2-I audio+subtitles, 0.C3-I
subsystems), the platform records + synthesis (0.C5), the dependency
pre-audits (0.C4), the inventories (0.B1–0.B4), the baseline pin (0.A), the
feature matrix (0.D2). This report states conclusions; evidence lives in the
linked sections. Honesty rules apply: every claim is either evidenced or
marked. Nothing here is "verified" in the §17 sense — no CI exists yet.

## 1. Executive summary

A Rust-only reimplementation of the VLC 3.0.23 baseline is **feasible for the
large majority of the inventory, with bounded, honestly-reported exceptions**.
Of 252 matrix rows: 61 are P0; 33 are blocked-with-reason (13%); the blocked
set is dominated by four clusters — software H.264/HEVC (mitigated by
ADR-001 hw-first + a conformance-gated software track), Opus/HE-AAC audio
(self-build tracks, no viable crate today), legacy/protected surfaces (discs,
DVB, RTSP/UDP/RTP family, capture), and styling-heavy rendering (full ASS).
The single planning-blind-spot discovered in Phase 0 is **HTTPS/TLS purity**
(ADR-011), which affects Phase 6 but nothing earlier. The project's parity
ceiling on day-one shipping is therefore high but not total, and the matrix
already encodes it.

## 2. What the baseline actually is (measured, not assumed)

- **Modules**: 502 registrations / 356 unique / 56 capabilities (Linux census;
  Win/mac hardware runs pending). Seven-category split: demux 57, decode 35,
  access 57, filters 110, subtitle 32, playlist 21, services-discovery 26,
  other 164.
- **Options**: 1,476 (100% dump coverage; 58.3% advanced-folded; 442 core +
  1,034 module-scoped; 252/252 default cross-checks agreed).
- **Formats**: 82 families inventoried; 59 probe-verified against the pinned
  oracle with synthetic fixtures; 27 P0.
- **UI surface**: 8 menus / 58 items / 23 dialogs; 111 hotkey actions; the
  design package supplies all 13 settings areas + player states.
- **Baseline identity**: VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd),
  release 2026-01-08, pinned per-platform with hash-verified artifacts.

## 3. Codec verdicts (detail: feasibility/sections/*)

- **Video**: MJPEG pure-Rust-clean (Phase 2). AV1 via rav1d asm-off
  (statically proven cfg-gating; proof plan P5.1–P5.7). VP9/MPEG-1/2/4 have
  candidate paths (upgraded from doc 04's "missing" by 0.C1-I re-verification).
  H.264/HEVC: young pure-Rust wave exists (rusty_h264-decoder 0.16.1,
  rust_h264 0.4.0, rusty_h265, rusty_vp9 — tarball-scanned pure, all
  self-validated); ADR-001 adopts **hw-first + conformance-gated software
  track**; parsers self-built regardless. Theora/VC-1 missing (Theora
  candidate via 0.C1-I; VC-1 blocked). Indeo/RealVideo: blocked, accepted.
- **Audio**: Symphonia v1 covers PCM/MP3/MP2/FLAC/ALAC/Vorbis/AAC-LC/ADPCM;
  HE-AAC has **zero upstream code** (README aspirational) — blocked v1;
  Opus placeholder crate is 1-byte empty — **blocked v1 with a Phase 5/6
  self-build track (L–XL) + go/no-go gate**; WMA/DTS absent from compliant
  paths (DTS proved only as spdif passthrough in probes); AC-3 self-build M;
  TrueHD via young `truehdd` (hardening = testing project); WavPack crate is
  GPL-3.0 (license flag for our MIT repo). MIDI: rustysynth closes the S
  estimate; v1 posture per 0.C2-I.
- **Subtitles**: text formats self-build S across the board; **ASS subset
  matrix** (render \b\i\fs\fn\colors/\pos/\an/\k/\r/\fad; degrade \move/\t/
  \bord/\frz; drop vector-clip/\org/blur) measured by unsupported-tag
  counters (R-09 trigger); PGS/DVB bitmap via CPU-rasterized RGBA atlas
  compositing; CC 608/708 extraction viable (public specs).

## 4. Subsystems (detail: feasibility/sections/subsystems.md)

- **Demuxers**: viable across the P0 set; frag-MP4 corpus plan written (moof/
  mdat/sidx cases); mpeg-ps scope defined (PS subset v1, VOB/nav → DVD ADR);
  first vertical AVI+MJPEG+PCM (D-001) needs no change.
- **Network**: **ADR-011** (no pure-Rust HTTPS today — the Phase 0
  blind-spot); HLS/DASH self-build estimates stand; RTSP blocked v1 (retina
  is pure Rust but has codec/async/quirk blockers); reconnect/auth scoped.
- **Outputs**: cpal 0.18.2 conditional-E2 acceptable (all native deps are
  OS-audio bindings); direct escape hatches per OS documented (ADR-005
  inputs); wgpu rendering path per platform records.
- **Filters**: the design package's audio panels (EQ/compressor/spatializer/
  stereowiden) + video panels (colors/crop/essential) are self-build S/M;
  **doc 13 gap found: no filter-implementation cards** → amendment D-012.
- **Playlists/SD/transcode/devices**: parsers self-build S; services-
  discovery minimal v1; sout 3-of-46 honest scope; capture hardware-gated;
  Kate drop-v1, FFV1 later, DVD nav blocked (no E2 path).

## 5. Platform verdicts (detail: inventory/platform-synthesis.md)

- **ADR-003 landing**: A-on-macOS + A-or-B-on-Windows + B-on-Linux, gated on
  Phase 2 prototypes (2.D3/2.E2-V); engine is strategy-agnostic.
- **ADR-001**: hw-decode Rust paths ready (windows-rs MF, objc2 VT);
  VA-API classified E2-conditional (ADR-001 §5).
- **ADR-005**: cpal-first with per-OS direct escape hatches.

## 6. Dependency posture

Foundation pre-audit (18 crates): pass × 8, conditional-E2 × 7,
exception-ADR-004 × 2 (tauri/wry), license-recheck × 1 (toml). WASM runtime
decided (ADR-002: wasmi). Allowlist format + audit method + worked records
exist; the workspace gets them at Phase 1.

## 7. Honest gap register (top, ranked)

1. TLS/HTTPS purity (ADR-011) — new, affects Phase 6+.
2. Opus software decode — blocked v1 (P0 audio pillar), self-build track.
3. H.264/HEVC software decode — hw-first mitigates; software track undated.
4. Full ASS styling — subset matrix bounded, anime-content gap visible.
5. DTS/WMA family — blocked/late; surround movie audio partial.
6. RTSP/UDP/RTP + discs + capture + DVB — blocked/deferred with reasons.
7. HEVC store-install friction on Windows (preinstall rate [VERIFY SV-10]).

## 8. Parity ceiling statement

With ADR-001/002/003 accepted: day-one shipping parity ≈ the 252-row matrix
minus the 33 blocked rows and minus hardware-dependent variance, **as
measured by future CI evidence only**. This report claims nothing as
verified; it defines what Phase 1+ must prove. The 61 P0 rows are the first
three phases' bread: 27 format P0s map onto Phases 2–5; playback/UI P0s onto
Phases 2–4; hw-decode P0s onto Phase 7.

## 9. Phase 1 readiness

All Phase 0 exit artifacts exist (see gates/phase0-exit-report.md). Open
[VERIFY] items (≈70 across ledgers) are tracked per-owning-card; none block
Phase 1 foundation work.
