# Feasibility §3 — Video codecs (decode)

Task ID: 0.C1-I (Wave 0.C, investigation). Date of verification: **2026-10-19**.
Feeds: `docs/feasibility-report.md` §3 (doc 02 §5 item 3) and **ADR-001**
(decision event 0.D3; this section provides verdict + recommendation framing
only — it does not decide). Source tables: doc 04 §4 (video decoders), §7 (gap
register items 1–2, 4, 8, 10), §8.6 audit-list items **2 (rav1d asm-off)** and
**8 (WebCodecs ADR scope)**. Risk linkage: R-01 (H.264 software decode), R-02
(HEVC). Baseline scope: VLC 3.0.23 Linux census — `video decoder` capability =
14 registrations, `hw decoder` = 3 (`vaapi` 100, `vaapi_drm` 0,
`vdpau_avcodec` 100) in docs/inventory/modules-linux.json. Priority oracle:
**`avcodec` ffmpeghw scores 10001 (the highest decoder score in the entire
census) and `dav1d` 10000** — the reference player itself ranks
hardware-accelerated decode above every software decoder. That is independent
corroboration for the hw-first direction ADR-001 is weighing.

**Method.** Every claim marked **[RV]** was re-verified today (2026-10-19) via
the crates.io HTTP API (`https://crates.io/api/v1/crates/<name>`, User-Agent
`lumen-phase0-audit`), crates.io search API (`?q=…`), and/or published-tarball
download from `static.crates.io` with a scan for `*.c`, `*.cpp`, `*.h`, `*.S`,
`*.asm`, `build.rs` (doc 04 §1.2 method). Web facts re-fetched today: W3C
WebCodecs codec registry, MDN WebCodecs API page, MDN BCD `api.VideoDecoder`,
WebKit Safari 16.4 blog (via the platform records' cached fetch, re-read),
Wikipedia x264 article, Cisco openh264 README. Anything **not** marked **[RV]**
keeps doc 04's 2026-10-19 verification and its original `[VERIFY]` status.
Purity designations remain "verified promising, not approved" (doc 04 §1.1):
the automated dependency audit (doc 04 §8.6 item 10, 0.C4) still must run
`cargo tree`/tarball scan/cargo-deny on every crate named here before any is
*approved*. **Conformance claims quoted from crate authors are self-reported
and remain unverified until our Phase 5 corpus runs.**

**Headline finding (must reach 0.D3):** doc 04's §4.2 sentence "no
production-grade pure-Rust H.264 decoder [exists]" is **no longer literally
true as of this re-verification** — a wave of young pure-Rust video decoders
published on crates.io between 2026-02 and 2026-10 (H.264, HEVC, VP9, MPEG-1/2,
MPEG-4 ASP, FFV1, ProRes, DNxHD; §2, §5) now exists. Every one of them is
0.x-versioned, months old, low-download, and self-validated. The H.264/HEVC
risk therefore **reframes**: from *"no implementation exists (build it
ourselves, XL)"* to *"implementations exist but are unproven (verify before
trusting, then maybe adopt)"*. This section's recommendation framing accounts
for that shift; nothing here upgrades any crate past `partial (young,
claims unverified)`.

---

## 1. Per-codec verdict table

Vocabulary per doc 04 §1.1 (`pure-rust-exists` / `partial` / `missing` /
`hw-only` / `blocked` / `later`). "v1 path" uses the task's values:
`software` (compliant pure-Rust decode ships in v1), `hw-first` (OS decoders
via ADR-001 are the primary path; software absent), `partial-subset` (ships
with a named, matrix-visible subset/gap), `blocked` (no compliant v1 path;
typed error, never silence), `later` (doc 04 P3/post-v1 rows). "hw per OS"
columns summarize the platform records: Media Foundation (Windows,
docs/inventory/platform-windows.md §4), VideoToolbox (macOS,
platform-macos.md §4), VA-API (Linux, platform-linux.md §3 — note its E2
classification is still an open Lead question), WebCodecs (web, §4 below).

| # | Codec | Prio | Pure-Rust availability (2026-10-19) | Win / MF | macOS / VT | Linux / VA-API | Web / WebCodecs | v1 path | Parity consequence (matrix row says) |
|---|---|---|---|---|---|---|---|---|---|
| 1 | AV1 | P0 | `partial`: `rav1d` 1.1.0 (2025-05-07, BSD-2, 84,579 dl) — asm-off compliant, see §3 **[RV]**; `rav1d-safe` 0.6.0 (imazen) is AGPL-3.0-only OR commercial → license-hostile, rejected **[RV]**; `dav1d` 0.11.1 crate = C bindings, forbidden **[RV]** | via free Store "AV1 Video Extension" (`9mvzqvxjbq9v`) — runtime MFTEnumEx probe + install prompt (record §4.1) | hw decode on M3/A17-Pro-gen SoCs+; software VT decoder on macOS 14+ commonly reported **[VERIFY]** (record §4.1) | all 3 vendor classes decode AV1 (Intel TGL-class+; AMD RDNA2+/VCN; NVIDIA shim) (record §3.2) | `av01.*` registered; Chrome/Edge yes; Safari hw-only gen-dependent; WebKitGTK flag `WebCodecsAV1Enabled` exists in-binary (record L) | **software (rav1d asm-off) + hw-first where present** | `av1.software = partial (no-asm speed cost §3)`; `av1.hw = partial`; UI states active backend |
| 2 | Motion-JPEG | P1 | `pure-rust-exists`: `jpeg-decoder` 0.3.2 (2025-06-21, MIT/Apache-2.0, 93.0 M dl) **[RV]**; `zune-jpeg` 0.5.16-rc2 (2026-09-08, 135.2 M dl) **[RV]** | in-box MJPEG decoder MFT (record §4.1) | not exposed via VT for our purposes [VERIFY: MJPEG in VT] | Intel driver-class JPEG decode (record §3.2) | MJPEG not in WebCodecs registry (§4) | **software** | `mjpeg = implemented` with `interlaced-field` + AVI1-glue caveats visible (doc 04 row) |
| 3 | Uncompressed / RGB(YUV) fourcc zoo | P1 | `missing (trivial)`: own format defs, S (doc 04, unchanged) | pass-through (NV12 surfaces) | pass-through | pass-through | n/a (RGBA path is ours) | **software** | `rawvideo = implemented`; fourcc coverage enumerated per record |
| 4 | Theora | P1 | `missing`: `theora` crate = 0.0.0 reserved placeholder (2017) **[RV]**; `oxideav-theora` 0.0.12 self-describes "orphan-rebuild scaffold pending clean-room re-implementation" **[RV]** — not a decoder | none (not in MF table, record §4.1) | none | none (not in any VA-API driver class, record §3.2) | `vp8`-class legacy: no `theora` string in registry (§4) | **blocked (v1) → later (L self-build)** | `theora = missing`; common Ogg/Theora files degrade to audio-only playback, honestly reported |
| 5 | VP8 | P1 | `missing→candidates`: `vp8` NOT_FOUND **[RV]**; `oxideav-vp8` 0.2.7 = scaffold **[RV]**; `zvidlib-vp8` 0.5.0 (2026-10-07, 79 dl) too young to assess **[RV]**; `shiguredo_libvpx` = C bindings, forbidden **[RV]** | in-box status unlisted in record → [VERIFY: MFTEnumEx probe for VP8] | Safari plays VP8 in software; VT hw exposure unclear [VERIFY] | Intel class VP8 decode; NVIDIA shim VP8 (record §3.2) | `vp8` registered (§4); Chrome/Edge/Safari yes | **hw-first (partial) → later** | `vp8.software = missing`; `vp8.hw = partial (Linux/Win only)` |
| 6 | VP9 | P0 | **changed since doc 04**: `vp9` NOT_FOUND **[RV]** but new pure-Rust decoders exist — `rusty_vp9` 0.1.3 (2026-07-29, Apache-2.0, claims bit-exact on all 315 libvpx conformance vectors; ~0.16–0.21× FFmpeg speed) **[RV]**; `vp9dec` 0.1.1 (clean-room, zero-dep) **[RV]**; `rvp9-decoder` 0.2.0-alpha.4; `vp9-mt` (tile-threaded fork) — all young/unproven | via free "VP9 Video Extensions" (`9n4d0msmp0pt`) + "Web Media Extensions" (`9n5tdp8vcmhs`), preinstalled Win10 1709+ per record §4.1 (probe anyway) | VP9 decode available in VT on recent macOS; hw-vs-sw split per SoC [VERIFY] (record §4.1) | all 3 vendor classes (record §3.2) | `vp09.*` registered; broad browser support (§4) | **hw-first; software = conformance-gated candidate [PROPOSAL]** | `vp9.hw = partial`; `vp9.software = missing→candidate` (no adoption before our corpus, §2.4 method) |
| 7 | H.264/AVC | P0 | **changed**: young pure-Rust decoders exist (§2) — `rusty_h264-decoder` 0.16.1 (2026-06-27, BSD-2; claims Baseline+B-slices+most-of-High CAVLC + CABAC Main, bit-exact vs openh264 over 35/35 streams; `forbid(unsafe)`; portable-Rust SIMD, no assembler) **[RV]**; `rust_h264` 0.4.0 (roticv; Baseline/Main/High 8-bit 4:2:0 incl. MBAFF, CAVLC+CABAC, 174 byte-exact tests vs FFmpeg) **[RV]**; `h264-decoder` 0.2.5 (xiu; NAL/SPS/PPS parse only, no slice decode) **[RV]**; `hibernia` 0.1.0; `sillstone-h264` 0.2.1 (published today). All unproven at scale. `openh264` 0.9.9 crate = C source build (default feature `source`→`openh264-sys2/source`) — forbidden **[RV]**; `h264` name = 0.0.0 reserved **[RV]** | **in-box H.264 decoder MFT (Win 7+, DXVA 1.0/2.0, profiles ≤ High)** (record §4.1/§4.2) — strongest in-box story of any codec | ubiquitous hw decode (Intel + Apple Silicon); `VTIsHardwareDecodeSupported` probe (record §4.1) | all 3 vendor classes (record §3.2) | `avc1.*`/`avc3.*` registered; "most widely supported" video codec in browsers (MDN, §4) | **hw-first + conformance-gated software track [PROPOSAL]** (§2) | `h264.hw = partial/hw-only`; `h264.software = blocked→candidate` — no overclaim until corpus (§2.5) |
| 8 | HEVC/H.265 | P1 | **changed**: `rusty_h265` 0.6.2 (2026-09-06, Apache-2.0, zero-dep core; claims Main/Main-10/Main-Still, 8+10-bit 4:2:0, full v1 toolset incl. tiles/WPP/SAO, gated on JCT-VC HEVC_v1 suite 147 streams, built-in SEI hash verification; SIMD kernels in separate `rusty_h265-accel` = "the one crate allowed unsafe") **[RV]**; `rust_h265` 0.1.0 (roticv; Main/Main-10, byte-exact vs FFmpeg on fixtures+BBB-1080p, ~5× slower than single-thread FFmpeg 8-bit, HVCC unsupported) **[RV]**; `scuffle-h265` 0.2.2 = **header parser only** (357.9 k dl, likely HEIC-driven) **[RV]**; `hevc`/`de265`/`libhevc-rs` NOT_FOUND **[RV]**; libde265 = C, forbidden (doc 04, unchanged) | **not in-box**: Store "HEVC Video Extensions" **USD 0.99, id changed — old deep-link id `9N4WGH0Z6VHM` is dead (HTTP 410), current id `9nmzlz57r3t7`**; free OEM "Device Manufacturer" variant unlisted [VERIFY] (record §4.1) | hw decode on all Apple-Silicon + HEVC-capable Intel Macs; software decode on newer OSes [VERIFY floor] (record §4.1) | all 3 vendor classes incl. 8/10/12-bit + 4:2:2/4:4:4 on Intel (record §3.2) | `hev1.*`/`hvc1.*` registered but "significant gaps in browser support outside of Apple platforms" (MDN, §4); Edge needs platform HEVC; WebKitGTK flag `WebCodecsHEVCEnabled` | **hw-first [PROPOSAL]; patents = ADR-010 gate** | `hevc.hw = partial`; `hevc.software = blocked→candidate`; distribution legality unresolved until ADR-010 |
| 9 | MPEG-1/2 video | P1 | **changed**: `oxideav-mpeg12video` 0.0.13 (2026-04-18, MIT, 2,427 dl; tarball scan: 124 `.rs`, 0 C/asm, no build.rs **[RV]**; claims full MPEG-1+MPEG-2 decode *and* encode pipeline, clean-room) **[RV]**; `na_mpeg2_decoder` 0.1.0 (2026-10-08, 72 dl) **[RV]**; libmpeg2 = C (doc 04, unchanged) | via free "MPEG-2 Video Extension" (`9n95q1zzpmh4`) (record §4.1) | no public VT MPEG-2 decode [VERIFY: Apple DVD frameworks are private] | Intel class MPEG-2; AMD UVD-class; NVIDIA shim (record §3.2) | no `mp2v`/`mpeg2` string in registry (§4) | **partial-subset (evaluate oxideav) [PROPOSAL]** | `mpeg2 = missing→candidate`; DVD/DVB workhorse gap shrinks from L-self-build to verify-then-adopt |
| 10 | MPEG-4 Part 2 / ASP (DivX/Xvid) | P1 | **changed**: `oxideav-mpeg4video` 0.1.8 (2026-04-17, MIT, 2,083 dl; decoder + encoder claims) **[RV]** | **in-box** MPEG-4 Part 2 decoder MFT (record §4.1) | none | **none** — no VA-API driver class exposes MPEG-4 ASP (Intel matrix lacks it; NVIDIA shim explicitly excludes MPEG-4, record §3.2) | not in registry (§4) | **hw-first on Windows only + partial-subset elsewhere [PROPOSAL]** | `mpeg4asp = missing→candidate`; classic DivX files: Windows hw path, else candidate-software or blocked |
| 11 | VC-1 / WMV9 | P2 | `missing` (no new entrant found in today's searches **[RV]**; reference decoder unavailable — doc 04, unchanged) | in-box "WMV family" decoders (record §4.1) | none | Intel class VC-1; NVIDIA shim VC-1 (record §3.2) | not in registry (§4) | **hw-first (Windows + partial Linux)** | `vc1.software = missing` forever-visible; `vc1.hw = partial` |
| 12 | Windows Screen 1/2, TSCC-class | P3 | none (doc 04, unchanged) | n/a (these are the codecs, not hw paths) | none | none | — | **later** (S–M self-builds, doc 04) | P3 rows; not in v1 matrix beyond `later` |
| 13 | Apple ProRes | P3 | **changed**: `oxideav-prores` 0.1.1 (2026-07-20, claims 422 Proxy/LT/Standard/… decode+encode) **[RV]**; `veloster-prores` 0.1.10 / `veloce-prores` 0.1.7 (RDD 36 claims) **[RV]** | none | **VT decode built into macOS** (Apple's own codec) [VERIFY floor] (record §4.1) | none | not in registry (§4) | **later** (hw on macOS whenever prioritized) | P3; `prores = later`, macOS hw path noted |
| 14 | DNxHD/HR | P3 | **changed**: `soundkit-dnx` 0.13.3 (2026-09-13; "VC-3, DNxHD, DNxHR decoder") **[RV]**; `veloster-dnx` 0.1.10 = header parser **[RV]** | none | none | none | not in registry | **later** | P3; candidate named for the future record |
| 15 | FFV1 (lossless) | P3 | **closes doc 04's [VERIFY-P0]**: `ffv1` crate name = 0.0.0 reserved (2017) **[RV]**, but `oxideav-ffv1` 0.0.9 (2026-04-17, RFC 9043 claims) **[RV]** and `codec_ffv1` 0.1.0 (2026-08-10, 161 dl) **[RV]** now exist — both young | none | none | none | not in registry | **later** (M effort either way, doc 04) | P3; `ffv1 = later`; candidates recorded for the archival-video record |
| 16 | Indeo 3/4/5 | P3 | none; legacy + patent haze (doc 04, unchanged) | none (removed from Windows long ago; not in MF table) | none | none | — | **blocked** | permanent `blocked` row with reason (doc 14 A5) |
| 17 | RealVideo RV10–RV40 | P3 | none (doc 04, unchanged) | none | none | none | — | **blocked** | permanent `blocked` row |
| 18 | Hap, Dirac, Cinepak, MS Video 1… | P3 | none verified (doc 04, unchanged; no credible entrant in today's searches **[RV]** for hap/dirac/cinepak) | none | none | none | — | **later/blocked per codec** | long tail, per-record when demanded |

Reading notes for 0.D3:
- Rows 7/8 are the R-01/R-02 core; §2 is their deep-dive. The "changed"
  annotations are the material delta vs doc 04 §4.1 and should be folded back
  into doc 04 by its owner after 0.D3 (worklog cross-reference, no edit from
  this agent — scope discipline, AGENTS §3.7).
- No row above claims `pure-rust-exists` for any of the five P0/P1
  *modern-codec* rows (H.264/HEVC/VP9/AV1/MJPEG) except AV1-partial (rav1d)
  and MJPEG. Everything else is `partial (young, unproven)` at best.
- The Windows MF column is the strongest in-box story (H.264, MPEG-4 ASP,
  WMV/VC-1, MJPEG, H.263, DV), with the HEVC paywall + dead-link wrinkle
  (record §4.1) as the one distribution hazard.

---

## 2. H.264/HEVC deep-dive (R-01/R-02 — the project's biggest codec risk)

### 2.1 The re-verification: what exists on 2026-10-19

Doc 04 §4.2's facts (verified 2026-10-19 by its author) still hold for the
*names it checked*: `h264` is an empty 0.0.0 reservation (re-verified **[RV]**,
2,227 downloads, 2017), `openh264` 0.9.9 and `x264` 0.5.0 are C-binding crates
(re-verified **[RV]**; `openh264`'s default `source` feature builds the Cisco C
code via `openh264-sys2`), Symphonia is audio-only, rav1d is AV1-only. But the
conclusion "no production pure-Rust decoder exists today" is **stale against
today's crates.io searches** — it missed a cluster of new entrants. All facts
below fetched 2026-10-19 via the crates.io API; the five leading candidates
were additionally tarball-scanned (0 C/C++/asm files, no build.rs in any):

| Candidate | Created | Version | License | Claimed scope (self-reported) | Provenance signals |
|---|---|---|---|---|---|
| `rusty_h264-decoder` (+`rusty_h264` facade) | 2026-06-27 | 0.16.1 (18 releases in ~4 months **[RV]**) | BSD-2 | Constrained Baseline + B-slices + most of High (CAVLC) + CABAC Main I/P/B; bit-exact vs Cisco `h264dec` 35/35 openh264 conformance streams; pixel-exact vs ffmpeg on CABAC paths; `forbid(unsafe_code)` core; portable Rust SIMD (no assembler), scalar via `--no-default-features`; 8-bit 4:2:0 only; Annex-B input only; explicitly *not yet*: CABAC I_PCM, High 8×8 CABAC residual, full JVT suite | Remade-With-Rust / "Mata Network"; 27.8 k dl; umbrella project `remade_ffmpeg_rs` self-labels "pre-1.0, and not yet independently audited" |
| `rust_h264` | 2026-04-05 | 0.4.0 | MIT/Apache-2.0 | Baseline + Main + High (8-bit 4:2:0), CAVLC+CABAC, all partitions, B-slices (direct modes), weighted prediction, deblocking, **MBAFF interlaced**; 174 byte-exact tests vs FFmpeg up to 1080p; Annex-B **and** AVCC input; `OrderedDecoder` display-order API | roticv (solo dev, rust_media lineage); 36.3 k dl |
| `h264-decoder` (xiu) | 2023-03-12 | 0.2.5 | MIT | "a h264 decoder" — tarball holds only 6 modules (`define/errors/lib/pps/sps/utils`): **NAL/SPS/PPS parsing, no slice decoder** **[RV]** | 30.8 k dl; probe/parse-grade, not a playback decoder |
| `hibernia` | 2026-02-13 | 0.1.0 | MIT | "clean-room H.264 decoder" | 54 dl; early experiment |
| `sillstone-h264` / `pith-h264` | 2026-10-19 / 2026-10-14 | 0.2.1 / 0.0.0 | Apache-2.0 | subset: NAL/SPS/PPS + CAVLC/CABAC slices + deblocking | published during/just before this audit; unclassifiable |

HEVC equivalents (same re-verification): `rusty_h265` 0.6.2 (created
2026-09-06, Apache-2.0, zero-dependency core, claims Main/Main-10/Main-Still
with the full v1 toolset — CABAC, merge/AMVP/TMVP, weighted pred, PCM,
lossless, transform-skip, scaling lists, SDH, deblocking, SAO, tiles, WPP,
dependent slices — gated on the 147-stream JCT-VC HEVC_v1 suite with per-frame
SEI decoded-picture-hash verification; SIMD kernels isolated in
`rusty_h265-accel`, described by its own README as "the one crate allowed
`unsafe`"); `rust_h265` 0.1.0 (roticv; Main/Main-10, byte-exact vs FFmpeg on
in-tree fixtures + real 1080p x265 ultrafast/medium/slow 8- and 10-bit; **no
threading or SIMD; ~5× slower than single-threaded FFmpeg at 8-bit, ~2.5× at
10-bit**; Annex-B only, HVCC must be converted by the caller; 401.5 k downloads
— recent download volume is dominated by the HEIC still-image ecosystem
(`heif-oxide` et al.), not video playback); `scuffle-h265` 0.2.2 (header/NAL
parser only — 357.9 k dl — useful for probe/parse, **not** a decoder).

Honest maturity read: **none of these is "production-grade" in the sense VLC's
H.264 path is** — months old, 0.x, single-maintainer or single-org provenance,
no third-party audit, conformance numbers self-reported, and the leading
H.264 candidate explicitly scopes itself below full High profile. Doc 04's
risk is not deleted; it is **transformed** (see §2.5).

### 2.2 Path (a) — ADR-001 hardware-first (unchanged primary)

Content (from doc 04 §4.2 + the three platform records, all cited in §1's
table): Media Foundation `MFTEnumEx`/`IMFTransform` on Windows (in-box H.264 ≤
High via DXVA), `VTDecompressionSession` on macOS (ubiquitous H.264; HEVC on
capable hw, `VTIsHardwareDecodeSupported` probe), VA-API on Linux (all three
vendor classes for H.264/HEVC; note the open E2-classification question the
Lead owns, platform-linux.md §6.2), WebCodecs on web per ADR-008 (§4). Rust
access is real on all three: `windows` 0.62.2 MF feature-flags verified,
`objc2-video-toolbox` 0.3.2 symbols verified, VA-API binding lineage
(cros-libva family) field-proven (platform records §4.3/§4.3/§3.3).

- Coverage: good on desktop for H.264-in-MP4/MKV/TS; HEVC good except the
  Windows 0.99-USD Store gate (id `9nmzlz57r3t7`; dead old id documented in
  platform-windows.md §4.1) and patent overhang (ADR-010).
- Gaps that remain no matter what: hardware-less desktops (VMs, old office
  boxes, software-rendered Linux), exotic profiles (10-bit Hi10P anime
  famously), Firefox-Android-class browsers on web, and any machine where the
  Store extension isn't installed and the user won't pay/install.
- Parity-share impact: H.264 is P0 with doc 04's planning estimate "~half of
  real-world video `[VERIFY]`" (usage-share replacement is audit item 7 /
  0.B4's job — this section keeps the mark). HEVC P1 "growing share
  `[VERIFY]`".
- Dependencies: ADR-001 (per-API escalation framing is 0.D3's), unsafe-FFI
  platform adapters with documented invariants (AGENTS §3.3), ADR-010 for
  HEVC distribution legality, ADR-008 for the web leg.

### 2.3 Path (b) — long-term self-build (XL; now the fallback, not the plan)

Honest cost, citable: x264 — the *encoder* alone — has been in continuous
development since Laurent Aimar's original work, taken over by Loren Merritt
(2004), Fiona Glaser (2008), and today Anton Mitrofanov and Henrik Gramner
(Wikipedia, fetched 2026-10-19) — a two-decade relay of specialist maintainers.
Cisco's openh264 — funded specifically to ship a binary-safe BSD H.264 —
scopes its decoder to **Constrained Baseline up to Level 5.2** for real-time
use (README fetched 2026-10-19) and is still maintained 13 years on. A
clean-room full-profile H.264 decoder from spec remains XL (26–104+ pw,
multi-quarter; doc 04 §1.4) even for a strong team. **The 2026 entrants
change the calculus: before committing person-years, spend person-weeks
verifying theirs (§2.4).** Self-build remains the honest fallback if every
candidate fails our corpus, and the parser self-build happens regardless
(§2.6).

### 2.4 Path (c) — subset software decode (feasibility: materially improved)

Doc 04's (c) was "Baseline-only software decoder (M-sized) … covers
low-complexity streams". Today that box is *already built by others*, twice,
with overlapping-but-different subsets: `rusty_h264-decoder` (Baseline + B +
most-High-CAVLC + CABAC-Main, `forbid(unsafe)`, no assembler) and `rust_h264`
(Baseline+Main+High incl. MBAFF, AVCC input, ffmpeg-differential-tested). A
third frame is HEVC-side: `rusty_h265` (Main/Main-10, conformance-suite-gated)
and `rust_h265` (same profiles, slower). Feasibility verdict: **plausible but
unproven** — the gating work is exactly what Phase 5 already does for rav1d:
our own conformance corpus (JVT/AVC + JCT-VC vectors where redistributable),
differential decode vs the VLC 3.0.23 oracle (ADR-007 boundary: run the
binary, never read VLC source), `fuzz_codec_h264`/`fuzz_codec_hevc` panic-free
gates, and perf measurement. Adoption conditions to write into ADR-001's
"conditions" section [PROPOSAL]: (i) purity re-audit per 0.C4 method incl.
transitive graph; (ii) license OK (BSD-2/MIT/Apache-2.0 pass today); (iii)
`forbid(unsafe)` or an unsafe-intrinsics ADR if `-accel`/SIMD crates are
pulled in (rusty_h265-accel uses `unsafe` intrinsics; rusty_h264's SIMD is
portable-Rust `core::arch` inside a forbid(unsafe) crate — verify how);
(iv) profile/bit-depth gaps stay matrix-visible (`partial (subset)`);
(v) toolchain compatibility (rusty_h265 is zero-dep; rust_h264 targets
FFmpeg-parity perf and is ~5× off — fine as fallback tier, not default).

### 2.5 Path (d) — accept blocked

Still the truthful *status quo* default if 0.D3 defers everything: matrix says
`h264.software_decode = blocked; h264.hw_decode = partial/hw-only` (doc 04 §7
row 1's reporting rule), UI states when hw is active, wasm32 column `blocked`
pending ADR-008. Not recommended as the *plan* (P0 codec, ~half of real-world
share `[VERIFY]`) — but it is the zero-overclaim floor every path above is
measured against.

### 2.6 What we self-build regardless [PROPOSAL, carries doc 04 §4.2]

The **pure-Rust H.264/HEVC parser** (NAL/SPS/PPS/slice-header, MP4 avcC/hvcC
↔ Annex-B unwrapping) — M-sized, needed to feed hardware decoders and to
probe/profile streams for capability routing (`scuffle-h265` shows there is
prior art for header-parse crates, but ours must match our `codec-api`
contract). It gets `fuzz_codec_h264`/`fuzz_codec_hevc` at parser level even
where decode is hw (doc 04 §4.1 rows).

### 2.7 Recommendation framing for ADR-001 [PROPOSAL — 0.D3 decides]

**H.264**: adopt (a) hw-first as the primary v1 path on all desktops +
WebCodecs on web per ADR-008; in parallel, open a *conformance-gated software
adoption track* for `rusty_h264-decoder` and `rust_h264` (§2.4 conditions),
targeting a "fallback tier" ship in Phase 5/6 if the corpus passes — turning
doc 04's (b)/(c) into "verify candidates first, self-build only if they fail".
**HEVC**: hw-first only for v1 (patents make a software ship a separate
ADR-010 question anyway); keep `rusty_h265`/`rust_h265` on the same gated
track for post-v1. **Always**: parser self-build (§2.6), truthful matrix rows
(§2.5 vocabulary), `system.get-engine-info → decoders[].backend` reporting
per doc 09 §10. Conditions that should flip the recommendation: if the Lead
rules VA-API out of the E2 envelope (Linux hw leg dies → software track
upgrades from parallel to urgent), or if ADR-010 counsel flags HEVC
distribution as not-clearable (HEVC rows go `blocked`, software included).

---

## 3. rav1d asm-off closure (doc 04 §8.6 item 2)

### 3.1 What today's re-verification established [RV, 2026-10-19]

Fetched: crates.io API `crates/rav1d` + `crates/rav1d/1.1.0/dependencies`;
tarball `static.crates.io/crates/rav1d/rav1d-1.1.0.crate` (extracted, scanned).
Findings, each superseding doc 04's `[VERIFY-P0: confirm zero non-Rust build
activity with asm off]` with direct evidence:

1. **Version state**: rav1d 1.1.0 remains `max/newest` (published 2025-05-07;
  84,579 total / 64,542 recent downloads; 3 versions, none yanked; BSD-2-Clause;
  repo memorysafety/rav1d). No newer release since doc 04's check.
2. **Feature graph (from the API's `features` map)**:
   `default = ["asm","asm_arm64_dotprod","asm_arm64_i8mm","asm_arm64_sve2","bitdepth_8","bitdepth_16"]`;
   every `asm_arm64_*` feature **implies `asm`**. Therefore
   `default-features = false` disables *all* assembly paths in one switch.
3. **`default-features = false` alone does not build** — `lib.rs:12-13` has
   `#[cfg(not(any(feature = "bitdepth_8", feature = "bitdepth_16")))]
   compile_error!("No bitdepths enabled…")`. The compliant configuration is:
   `rav1d = { version = "1.1.0", default-features = false, features =
   ["bitdepth_8", "bitdepth_16"] }`. (Concrete integration fact for doc 03's
   crate registry.)
4. **build.rs is a no-op without `asm`**: the entire `main()` body is
   `#[cfg(feature = "asm")] { asm::main(); }` — the `cc`/`nasm-rs` invocations,
   the `config.asm`/`config.h` emission, and the
   `cargo:rustc-link-lib=static=rav1dasm` link line all live inside the
   cfg-gated `mod asm`. With asm off, build.rs compiles to nothing, writes
   nothing to `OUT_DIR`, and links nothing. Cross-checked: no `include!`/`OUT_DIR`
   reference exists anywhere in `src/` or `lib.rs`.
5. **Tarball contents**: 91 assembly files (`src/arm/**/*.S`, `src/x86/*.asm`)
   ship in the package — doc 04's "contains ARM `.S` and x86 asm" holds —
   **but zero `.c`/`.cpp`/`.h` files** (the package `exclude` list strips the
   C originals; the port is fully translated). The asm files are reachable
   only through the build.rs `asm` path.
6. **Rust fallbacks exist and are the non-asm default**: the `wrap_fn_ptr!`
   dispatch tables default to `*_rust` implementations; the `init_x86`/
   `init_arm` asm installers are `#[cfg(all(feature = "asm", …))]` (12 source
   files carry the gate).
7. **Dependency status, asm-off**: 11 normal dependencies — all pure-Rust
   crates (`assert_matches`, `atomig`, `bitflags`, `cfg-if`, `libc`,
   `parking_lot`, `paste`, `raw-cpuid`, `strum`, `to_method`, `zerocopy`).
   `cc` 1.0.79 and `nasm-rs` 0.3 (+`parallel`) are **unconditional
   build-dependencies**: with asm off they still *compile* (they are
   themselves Rust crates) but are never *invoked* — no C toolchain or nasm
   binary execution occurs. **Policy nuance for cargo-deny/allowlist: `cc`
   stays in the resolved graph as declared-but-inert; the audit record must
   say so explicitly, and the Phase 5 build proof (below) must demonstrate a
   successful build in a sandbox with no C compiler on `PATH`.**
8. **Toolchain caveat**: the repo pins `nightly-2025-05-01`
   (`rust-toolchain.toml` ships in the tarball) while `Cargo.toml` declares
   `rust-version = "1.79"`. The unstable features (`stdarch_arm_feature_detection`)
   are `cfg_attr`-gated on `target_arch = "arm"` only — x86_64/aarch64 stable
   builds look plausible but are **[VERIFY: Phase 5 build matrix]**; arm32
   needs nightly.
9. **Alternatives surveyed**: `rav1d-safe` 0.6.0 (imazen, 2026-09-08) — a
   "safe SIMD fork of rav1d with archmage intrinsics" — is **AGPL-3.0-only OR
   LicenseRef-Imazen-Commercial**: license-hostile to our MIT repo (doc 06 /
   ADR-006 context), rejected. The `dav1d` 0.11.1 crate (1.54 M dl) is
   libdav1d C bindings — forbidden per §9. rav1d 1.1.0 stays the only
   compliant AV1 software path.
10. **Speed cost remains unquantified**: no asm-off benchmark numbers exist in
    the crate's README/NEWS (checked) — doc 04's "speed without asm" gap
    stands until the Phase 5 measurement. The remade_ffmpeg_rs project's
    honest-perf table (fetched 2026-10-19) shows what a young Rust decoder's
    expectations look like (their VP9: ~0.16–0.21× FFmpeg single-thread) —
    use that class of number as the prior, not as rav1d's.

### 3.2 The Phase 5 proof plan (what must be demonstrated before `av1 = implemented`)

Phases per doc 04 §8.3 item 3 ("AV1 via rav1d (asm-off) — the flagship
software video codec; conformance corpus + differential vs VLC"):

- **P5.1 Build matrix**: `cargo build --release` with
  `--no-default-features --features bitdepth_8,bitdepth_16` on
  x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu,
  x86_64-pc-windows-msvc, aarch64-apple-darwin, wasm32-unknown-unknown
  (wasm leg per doc 04 §8.5 "targets also build on wasm32 where feasible").
  Gate: build succeeds **in a sandbox with no C toolchain and no nasm on
  PATH** (empirical proof of finding 4/7), and links with no `rav1dasm`
  static lib in the artifact.
- **P5.2 cargo-deny clean**: `deny.toml` (licenses/advisories/sources/bans)
  passes on the asm-off dependency graph; `cc`/`nasm-rs` recorded as
  declared-but-inert with the justification note; `cargo tree -e build`
  output attached to the audit record (0.C4's allowlist format).
- **P5.3 Conformance**: decode a redistributable AV1 vector corpus
  (libaom/Argon-class test vectors; license per file recorded in
  `tests/fixtures/CORPUS.md` per doc 04 §8.5) and diff output-frame hashes
  against a **reference dav1d CLI run** (external differential oracle — same
  boundary as VLC-as-oracle: running the BSD-2 binary is clean-room-safe,
  reading its source is not; ADR-007 framing). Gate: 100 % matching streams
  on the corpus, mismatches triaged to zero.
- **P5.4 Differential vs VLC 3.0.23 oracle**: end-to-end AV1-in-MKV/MP4
  playback comparison (doc 04 §8.3's own requirement).
- **P5.5 Perf, asm-on vs asm-off**: benchmark both configurations on fixed
  720p/1080p/2160p AV1 clips on reference hardware; record the slowdown
  factor in doc 12's budget and in the feature-matrix `notes` for
  `av1.software` (this quantifies doc 04's "speed without asm" gap — the
  number that decides whether an asm ADR is ever worth opening).
- **P5.6 Fuzz**: `fuzz_codec_av1` runs 10 min clean in CI (release), seed
  corpus committed (doc 04 §8.5 registry rules).
- **P5.7 Anti-regression pin**: a CI check (or cargo-deny ban) that fails if
  any dependency graph change re-enables rav1d default features — asm must
  not sneak back in via a transitive `features = ["default"]`.

This closes §8.6 item 2 to the extent Phase 0 can: the *plan* is now concrete
and evidence-backed (build.rs gate verified by inspection today); the
*execution* is Phase 5 work by design.

---

## 4. WebCodecs scope (doc 04 §8.6 item 8 + doc 09)

### 4.1 What WebCodecs is (and is not)

WebCodecs is a **browser API** — `VideoDecoder`/`AudioDecoder`/
`EncodedVideoChunk`/`VideoFrame`/`AudioData` — that gives per-frame access to
the browser's (hardware-accelerated where available) encode/decode pipeline:
"enables web developers to encode and decode video and audio in the browser
efficiently (using hardware acceleration) and with very low-level control
(processing on a per-frame basis)" (MDN WebCodecs API page, fetched
2026-10-19). It is **not a bundled codec**: nothing is compiled or linked into
our engine; we call a JS API across the wasm boundary the same way we call
wgpu for rendering. That is the policy-clean reasoning doc 09 §10 records
("a browser API, not a bundled native codec — policy-clean in principle
pending the Phase 0 audit") — the same E2/OS-service logic as ADR-001's
OS decoders, applied to the web's "OS". What it is *not*: a way to satisfy
"same engine everywhere" — decode quality/availability becomes
browser-dependent, which is exactly why it needs its own ADR rather than
riding in silently (and why the `<video>`-tag fallback was rejected as a
hidden native engine, doc 09 §10 last row).

### 4.2 Which codecs browsers actually decode (cited)

- **Registry (the spec's list of codec strings)**: W3C WebCodecs Codec
  Registry (Editor's Draft 2026-09-24, fetched 2026-10-19) registers exactly
  five video codec strings: `av01.*` (AV1), `avc1.*`/`avc3.*` (AVC/H.264),
  `hev1.*`/`hvc1.*` (HEVC/H.265), `vp8`, `vp09.*` (VP9). The registry
  explicitly states: "Implementers of WebCodecs are not required to support
  any particular codec nor registry entry" — support is per-browser,
  per-platform.
- **In practice (MDN, fetched 2026-10-19)**: H.264 (AVC) — "the most widely
  supported video codec" in WebCodecs; VP9 — "commonly used on YouTube and in
  WebM files"; AV1 — "broad decoder support; hardware encoder support is
  still limited"; HEVC — "significant gaps in browser support outside of
  Apple platforms". MDN also warns: "The WebCodecs specification supports a
  particular set of codecs, and individual devices and browsers may only
  support a subset of those" — and requires fully-specified codec strings
  (`avc1.4d0034`, not `h264`). Audio: Opus, AAC, MP3 (decode-only), PCM.
- **Browser availability (BCD `api.VideoDecoder`, re-fetched 2026-10-19,
  matching doc 09 §8.1)**: Chrome 94+, Edge 94+ (Evergreen WebView2 = same
  engine; WebCodecs *not* on the Edge-vs-WebView2 differences list —
  platform-windows.md §3), Firefox 130+ **desktop only** (Firefox Android:
  no), Safari 16.4+ (video portion; full WebCodecs incl. audio in Safari 26 —
  platform-macos.md §3). macOS floors: Safari 16.4 ≈ macOS 13.3.
- **Linux wrinkle (platform-linux.md §2.1)**: WebCodecs is compiled into
  Debian trixie's WebKitGTK 2.52.6 (in-binary flag strings
  `WebCodecsVideoEnabled`/`Audio`/`AV1`/`HEVC`), GStreamer-backed with
  `prefer-hardware` routing to gstreamer1.0-vaapi → VA-API; **default-enabled
  state unproven on hardware** [VERIFY] — runtime probing
  (`isConfigSupported`) is mandatory, MSE/ImageBitmap are the fallbacks.
- **Routing consequence for our matrix**: WebCodecs can carry H.264, HEVC
  (Apple-leaning), VP8, VP9, AV1 — it cannot help Theora, MPEG-1/2/4, VC-1,
  MJPEG, or any long-tail codec (not in the registry). It is a *decoder
  backend*, not a format program: our demux/parse stays engine-side, and
  EncodedVideoChunks are shipped to the browser decoder (doc 09 §4.2 frame
  path: `VideoFrame` → WebGPU `importExternalTexture`/canvas).

### 4.3 ADR-008 scope (what that ADR must decide) and its ADR-001 relationship

Per ROADMAP §6, ADR-008 = "WebCodecs-as-hw-analog on web (Phase 7, feeds
ADR-001)". **Numbering discrepancy to flag to the Lead**: doc 01 §7's ADR
table instead lists ADR-008 = optical disc and ADR-009 = WASM threading —
doc 01 and ROADMAP disagree on 008/009 assignments (follow-up F-12 below;
doc 01 is Lead-owned, not edited here).

The ADR's decision surface, as evidenced above [framing, not deciding]:
(i) accept `VideoDecoder` (and audio-side `AudioDecoder`) as the web analog
of ADR-001's OS-decoder exception under the same audited-OS-service (E2)
envelope — policy-clean per §4.1, with the `windows-rs`/`objc2`/libva
bindings as the desktop parallels; (ii) capability model — always probe
`VideoDecoder.isConfigSupported()` per codec+profile before promising
playback (mirrors `MFTEnumEx`/`VTIsHardwareDecodeSupported`/VA-API
entrypoint-query on desktop); (iii) truthful reporting —
`system.get-engine-info → decoders[].hardware = true, backend = "WebCodecs"`
(doc 09 §10's exact rule); (iv) fallback ladder — WebCodecs → engine software
(rav1d asm-off on wasm; H.264/HEVC candidates if adopted per §2) → typed
`blocked` with `blocked_reason` naming the missing substrate (doc 09 §6.3/§10
vocabulary; "no silent `<video>` fallback" is already decided there);
(v) platform floors — Safari 16.4/macOS 13.3, Firefox-130-desktop,
Firefox-Android excluded; (vi) sequencing — doc 09 puts the ADR in Phase 7
but §2.7's H.264 framing leans on it for the web leg of parity claims, so
0.D3 should at least scope it now (this section's input) even if the formal
acceptance lands with Phase 7 web work.

Relationship to ADR-001 in one sentence: **ADR-001 defines the audited
OS-decoder exception pattern on desktop; ADR-008 extends that same pattern to
the browser's decode services on web — same envelope, same truth rules,
different ABI (JS API vs FFI), and without it H.264/HEVC stay `blocked` on
wasm32 (doc 09 §12 item 6).**

---

## 5. Per-codec treatment (confirming/refining doc 04 §4 rows)

**AV1 (P0, partial).** rav1d 1.1.0 unchanged since doc 04's verification
(§3.1 finding 1); asm-off configuration and build.rs gate now proven by
inspection (§3); the remaining unknowns are the Phase 5 execution (conformance,
perf) and the arm32/nightly + wasm build legs. HW legs: all three desktop
platforms + WebCodecs cover AV1 (with the Windows Store-extension and
Safari-SoC-generation caveats, §1 row 1). rav1e (encoder) noted for the
future transcode path (doc 04 §4.4, unchanged). New competitive context:
`oxideav-av1` self-describes as scaffold **[RV]**; `zvidlib-av1-decoder` is
12 days old **[RV]** — neither displaces rav1d as the candidate.

**VP8 (P1, missing→candidates).** No usable decoder today: `vp8` name
NOT_FOUND **[RV]**; `oxideav-vp8` is an explicit scaffold **[RV]**;
`zvidlib-vp8` (2026-10-07) is days old **[RV]**. libvpx bindings
(`shiguredo_libvpx`) are C — forbidden **[RV]**. HW: VA-API Intel-class VP8 +
NVIDIA shim; Windows in-box status needs the MFTEnumEx probe [VERIFY];
macOS unclear [VERIFY]. Doc 04's XL self-build estimate stands; VP8 remains
the weakest leg of the WebM trio (AV1 covered, VP9 candidate-covered, VP8
not).

**VP9 (P0, missing→candidates).** The strongest of the new entrants by
evidence quality: `rusty_vp9` claims bit-exactness on **all 315 official
libvpx conformance vectors** across profiles 0–3 (8/10/12-bit) with
self-acknowledged slowness (~0.16–0.21× FFmpeg single-thread per the umbrella
project's own table, fetched 2026-10-19); `vp9dec` is an independent
clean-room zero-dependency implementation; `rvp9-decoder` claims
spec-conformance vs official vectors; `vp9-mt` adds tile-column threading.
All are 0.1.x and months old **[RV]**. HW: VP9 is the best-covered codec
across MF (two free Store packs, preinstall since Win10 1709), VT (recent
macOS, hw/sw split [VERIFY]), VA-API (all vendors), and WebCodecs
(`vp09.*`, broad). Recommended v1 path: hw-first + the §2.4-style gated
software track [PROPOSAL] — VP9 software would close the WebM/YouTube-class
gap that doc 04 §7 row 4 records.

**Theora (P1, missing).** Unchanged: no real implementation (`theora` name
reserved since 2017 **[RV]**; `oxideav-theora` scaffold **[RV]**; libtheora
is C — forbidden). No hw path on any OS. Doc 04's L self-build + "later"
stands; the 0.B4 usage-share data should decide whether Theora deserves more
attention than its P1 label suggests (legacy Ogg content).

**MPEG-1/2 (P1, missing→candidate).** `oxideav-mpeg12video` 0.0.13 is a
genuine (124-file, pure-Rust, MIT) implementation claiming the full MPEG-1 +
MPEG-2 decode pipeline **[RV]** — the DVD/DVB workhorse gap may now be a
verify-then-adopt instead of an L self-build. HW: Windows free Store
extension; VA-API Intel/AMD/NVIDIA-shim; macOS none. Recommended: gated
candidate track + hw where present [PROPOSAL].

**MPEG-4 Part 2 / ASP (P1, missing→candidate).** `oxideav-mpeg4video` 0.1.8
(decoder + encoder claims, pure-Rust, MIT) **[RV]**. HW: **in-box on Windows
MF** (record §4.1) — the only platform that hw-decodes ASP; VA-API driver
classes do not (NVIDIA shim explicitly excludes MPEG-4). Recommended:
Windows-hw + gated candidate elsewhere [PROPOSAL].

**MJPEG (P1, pure-rust-exists).** Confirmed: `jpeg-decoder` 0.3.2 and
`zune-jpeg` 0.5.16-rc2 both live and massive-download **[RV]** (93 M / 135 M).
Gaps stay as doc 04 lists them (interlaced/field-based MJPEG, AVI1 chunk
glue — S-sized). No WebCodecs string; irrelevant to hw.

**VC-1/WMV9 (P2, missing).** No new entrant found today **[RV]**. HW:
Windows in-box WMV family; VA-API Intel + NVIDIA shim; macOS none. Doc 04's
XL/later stands; the Windows+Intel hw path is the only v1-relevant slice.

**P3 rows.** ProRes: two new candidate families (`oxideav-prores`,
`veloster`/`veloce-prores`, RDD-36 claims) **[RV]** + VT decode on macOS —
still `later`, but the future record has a named candidate. DNxHD/HR:
`soundkit-dnx` 0.13.3 **[RV]** as candidate. FFV1: doc 04's
[VERIFY-P0] "none on crates.io" is now **closed**: `ffv1` name remains a
0.0.0 reservation **[RV]**, but `oxideav-ffv1` 0.0.9 (RFC 9043 claims) and
`codec_ffv1` 0.1.0 exist **[RV]** — young; `later` stands with candidates
recorded. Windows Screen/TSCC, Hap, Dirac, Cinepak, MS Video 1: unchanged,
no credible entrants **[RV]**; Indeo + RealVideo: permanently `blocked`
(doc 14 A5 rows).

---

## 6. Evidence appendix

### 6.1 URLs fetched this run (all 2026-10-19, User-Agent `lumen-phase0-audit` / `lumen`)

crates.io API (`https://crates.io/api/v1/crates/<name>` and
`.../<name>/<version>/dependencies`): rav1d, rav1d/1.1.0/dependencies,
rusty_h264, rusty_h264-decoder, rust_h264, h264-decoder, hibernia,
sillstone-h264, rust_h265, rusty_h265, rusty_vp9, vp9dec, rvp9-decoder,
vp9-mt, scuffle-h265, oxideav-mpeg12video, oxideav-mpeg4video,
oxideav-theora, oxideav-vp9, oxideav-vp8, oxideav-ffv1, oxideav-prores,
oxideav-av1, codec_ffv1, soundkit-dnx, veloster-prores, veloce-prores,
veloster-dnx, zvidlib, zvidlib-vp8, zvidlib-vp9-decoder,
zvidlib-av1-decoder, rff-codec-hevc, rff-codec-vp9, na_mpeg2_decoder,
rav1d-safe, rav1e, dav1d, openh264, x264, jpeg-decoder, zune-jpeg, theora,
vp8, vp9, h264, h265, hevc, de265, libhevc-rs, ffv1.
Searches (`?q=…`): ffv1, h264+decoder, hevc+decoder, h265, av1+decoder,
mpeg2+decoder, mpeg+video+decoder, vc1, vp9+decoder, theora+decoder, vp8,
dav1d, prores+decoder, dnxhd.
Tarballs (`static.crates.io`): rav1d-1.1.0, rusty_h264-0.16.1,
rusty_h264-decoder-0.16.1, rusty_h265-0.6.2, rusty_vp9-0.1.3,
rust_h264-0.4.0, rust_h265-0.1.0, oxideav-mpeg12video-0.0.13,
h264-decoder-0.2.5 (scanned: C/C++/asm file count, build.rs presence,
README, Cargo.toml features/deps; scratch in `tmp/c1/video/`, gitignored).
Web: `w3c.github.io/webcodecs/codec_registry.html` (Editor's Draft
2026-09-24) · `developer.mozilla.org/en-US/docs/Web/API/WebCodecs_API` ·
`raw.githubusercontent.com/mdn/browser-compat-data/main/api/VideoDecoder.json`
· `webkit.org/blog/13966/webkit-features-in-safari-16-4/` ·
`raw.githubusercontent.com/cisco/openh264/master/README.md` ·
`en.wikipedia.org/wiki/X264` ·
`api.github.com/repos/Remade-With-Rust/remade_ffmpeg_rs/readme` (rate-limit
hit afterward; README quote captured before) ·
`raw.githubusercontent.com/Remade-With-Rust/remade_ffmpeg_rs/main/README.md`
(404 — umbrella README lives at repo default branch path other than
`main/README.md`; API-readme used instead) ·
`doc.rust-lang.org/cargo/reference/build-scripts.html` +
`features.html` (checked for the cfg-in-build.rs question; no statement
found — resolved empirically instead, §3.1 finding 4/7).
Repo-internal sources (re-read, not re-fetched): doc 04 §1/§4/§7/§8;
docs/09-web-wasm-target.md §8/§10/§12; docs/inventory/platform-{windows,
macos,linux}.md hw-decode sections; docs/inventory/modules-linux.json
(video/hw decoder capability groups); docs/13-agent-wave-program.md 0.C1-I
card; docs/adr/ADR-002-evidence.md (format precedent only); sibling section
docs/feasibility/sections/audio-codecs.md (format precedent).

### 6.2 Standing [VERIFY] items carried forward (not re-verified today)

Doc 04 §1.3 usage-share estimates (P0 "≈80 %", "~half of real-world video")
— replaced only by 0.B4/audit item 7. Doc 04 §6.2 record-file contents
(per-format limits/corrupt-input rows) — Phase 5+ record work. All
platform-record [VERIFY]s inherited by this section's table: macOS
HEVC-software floor, VP9 hw/sw split, AV1-software-VT report, ProRes floor,
WKWebView WebCodecs parity (platform-macos.md); Windows HEVC
OEM-variant/preinstall matrix, WebView2-HEVC behavior (platform-windows.md);
VA-API E2 classification (Lead call), AMD per-ASIC matrix, WebKitGTK
WebCodecs default-enablement, VA→Vulkan DMABUF zero-copy
(platform-linux.md).

### 6.3 Remaining open items → numbered follow-up list for 0.D3

| # | Item | Owner/when |
|---|---|---|
| F-1 | Decide ADR-001 per-API escalation (MF/VT/VA-API) with §1 table + §2.7 framing as video input; VA-API E2 classification is the gating sub-question (platform-linux.md §6.2) | Lead, 0.D3 |
| F-2 | Approve/decline the conformance-gated software-adoption track (§2.4 conditions) for H.264 (`rusty_h264-decoder`, `rust_h264`) and HEVC (`rusty_h265`, `rust_h265`) — Phase 5 corpus + fuzz + perf gates before any status upgrade | Lead via ADR-001 conditions, 0.D3; execution Phase 5 |
| F-3 | rav1d Phase 5 proof plan adoption (§3.2 P5.1–P5.7) incl. the no-C-toolchain sandbox build and the asm-on/off benchmark that quantifies doc 04's speed gap | Phase 5 (5.A-rav1d tasks); arm32-nightly + wasm legs [VERIFY] |
| F-4 | Scope ADR-008 (WebCodecs-as-hw-analog) now per §4.3, even if formal acceptance lands Phase 7 (doc 09 §12 item 6) | Lead, 0.D3 |
| F-5 | VP8/VP9/Theora/MPEG-1/2/MPEG-4 candidate crates: schedule gated evaluations (same method as F-2) in Phase 5/6 breadth; VP8 needs a real implementation before WebM parity claims | Phase 5/6 planning |
| F-6 | HEVC distribution legality (patents) — ADR-010 trigger; also gates any HEVC *software* ship, not just hw | ADR-010 (user + counsel) |
| F-7 | Windows-specific probes for the matrix: VP8 in-box MFT status; MJPEG-in-VT on macOS; MPEG-2 VT absence confirmation — fold into the Phase 0 hardware runbook (platform records' [VERIFY] ledgers) | hardware runbook |
| F-8 | Fold this section's "changed since doc 04" findings (§1 rows 6,7,8,9,10,13,14,15 + §2.1) back into doc 04 §4 tables — doc 04 owner, after 0.D3 (AGENTS §3.7 scope discipline) | doc 04 owner |
| F-9 | remade_ffmpeg_rs / oxideav / zvidlib provenance review: single-org, AI-assisted-development signals ("not yet independently audited" self-label) — the dependency-provenance criteria (doc 11 method, 0.C4) should define how much weight self-reported conformance claims may carry | Lead + doc 11 method |
| F-10 | rav1d `cc`/`nasm-rs` declared-but-inert build-deps: allowlist wording (0.C4 format) + optional `[patch]`-out evaluation if policy demands graph cleanliness beyond invocation cleanliness | 0.C4 follow-on / Phase 5 |
| F-11 | FFV1 [VERIFY-P0] closure is recorded here (§5); doc 04 owner updates its row (candidates named, none adopted) | doc 04 owner |
| F-12 | ADR numbering conflict: doc 01 §7 (ADR-008 = optical disc, ADR-009 = WASM threading) vs ROADMAP §6 (ADR-008 = WebCodecs, ADR-009 = DVD/Blu-ray) — reconcile before 0.D3 references ADR-008 in decisions | Lead, pre-0.D3 |

---

*Bound by doc 04's closing rule: when reality disagrees with a row, the row
loses. Today, reality moved — five rows improved on paper and none of them
have earned trust yet.*
