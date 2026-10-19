# 04 — Format & Codec Coverage Roadmap

Status: planning (Wave 1, Task 1.A4). Owner: engine/codec track.
Scope: which containers, codecs and subtitle formats Lumen will support, from
which pure-Rust building blocks, with what gaps, in which phase, with which fuzz
targets. This is the input to the Phase 0 codec-coverage audit and the
feature-matrix (`docs/feature-matrix.toml`, built in Phase 0).

Verification date for all crate claims in this document: **2026-10-19**, via the
crates.io HTTP API and by downloading and inspecting the published crate
tarballs (`static.crates.io`) for C/C++ sources, assembly, and `build.rs`
scripts. Claims not verified that way are marked `[VERIFY]` or `[VERIFY-P0]`.

---

## 1. Method, status vocabulary and priority scheme

### 1.1 Status vocabulary

Every format in this roadmap gets exactly one status:

| Status | Meaning |
|---|---|
| `pure-rust-exists` | A named, maintained pure-Rust crate implements the format and passes the §9 policy audit (no C/C++/asm sources, no `build.rs` that invokes a non-Rust toolchain, acceptable license, transitive graph clean). The audit itself still runs in Phase 0 — the designation here means "verified promising", not "approved". |
| `partial` | A pure-Rust implementation exists but with material gaps (subset of profiles, in-work status, no published release, or policy friction such as an optional assembly path that must be disabled). |
| `missing` | No compliant implementation exists; Lumen must self-build or defer. |
| `hw-only` | Feasible for Lumen only via ADR-001 OS hardware decoders (VideoToolbox / Media Foundation / VA-API); no compliant software path exists today. |
| `blocked` | No compliant path at all (policy, legal, or a C-only ecosystem with no credible port). Reported as blocked in the feature matrix with reason and next step. |
| `later` | Post-v1 target; tracked but not scheduled before Phase 5 breadth work ends. |

Note the distinction between `hw-only` and `blocked`: an `hw-only` codec can
still be *played* on desktop platforms through the audited OS-decoder
exception, with the software path honestly reported as absent. The web build
(wasm32) has no OS decoder of its own — see §4.4 for the WebCodecs question.

### 1.2 How purity was and will be verified

For this document: (a) crates.io API lookup for existence, version, date,
license, repository; (b) tarball download and scan for `*.c`, `*.cpp`, `*.h`,
`*.S`, `*.asm` and `build.rs`. In Phase 0 the same checks become automated
(`tools/dependency-audit`, `cargo-deny`, `cargo vet`, `cargo tree` transitive
graph) per §9 of the MASTER-PROMPT. A Rust API never proves purity; bindings to
libavcodec/libopus/libvorbis/libass/openh264/dav1d are **forbidden** and are
listed here only to be explicitly rejected.

### 1.3 Priority scheme (P0–P3)

Priority is derived from VLC's real-world usage share: which demuxers, codecs
and subtitle formats actual users actually open. The authoritative source is
the **Phase 0 inventory** built from VLC 3.0.23's plugin list and option
categories (`docs/02-phase0-audit-plan.md`). Until that inventory lands, the
shares below are **planning estimates marked `[VERIFY]`** — they order work but
must not be quoted as measurements.

- **P0** — the long tail's head: files a general-purpose player meets daily.
  Estimate ≈ 80 % of real-world files `[VERIFY]`: MP4/MOV, Matroska/WebM,
  H.264, AAC, MP3, FLAC, Vorbis, Opus, WAV/PCM, MPEG-TS, SRT, VP9/AV1.
- **P1** — common but skews by region/source: MPEG-PS, MPEG-1/2 video, MPEG-4
  ASP, HEVC, AC-3/E-AC-3, ALAC, AIFF, Ogg container oddities, ASS/SSA, WebVTT,
  AVI+ADPCM, Motion-JPEG.
- **P2** — real but niche: DTS, TrueHD, WMA, VC-1, Theora, PGS/DVB subtitles,
  CC/608/708, ASF/WMV container, Musepack, Speex, WavPack, MP2 radio streams.
- **P3** — legacy/edge: RealMedia, ATRAC, ProRes, DNxHD, screen codecs, MIDI,
  3GPP timed text, CAF, and everything else in VLC's long tail.

### 1.4 Effort scale used in self-build estimates `[PROPOSAL]`

S = ≤ 2 person-weeks; M = 2–8; L = 8–26 (a focused quarter);
XL = 26–104 (a multi-quarter project of its own). These are planning numbers
for a competent Rust engineer with spec in hand, including tests and a fuzz
target, excluding conformance-corpus licensing work.

---

## 2. Containers / demuxers

### 2.1 Summary table

| Format | Prio | Status | Candidate pure-Rust crates (verified 2026-10-19) | Main gaps | Self-build | Phase | Fuzz target |
|---|---|---|---|---|---|---|---|
| WAV (RIFF WAVE) | P0 | pure-rust-exists | `symphonia-format-riff` 0.6.1 (WAV "Excellent"), `hound` 3.5.1 [github.com/ruuda/hound] | exotic WAVE_FORMAT_EXTENSIBLE corners; RF64 > 4 GB | none | 2 | `fuzz_demux_wav` |
| FLAC (native, Ogg-FLAC) | P0 | pure-rust-exists | `symphonia-bundle-flac` 0.6.1 ("Excellent", gapless); `claxon` 0.4.3 [github.com/ruuda/claxon] (tarball: 16 .rs, no build.rs) | claxon unmaintained since 2020; Ogg-FLAC pairing | none | 2 | `fuzz_demux_flac` |
| MP4/MOV/3GP (incl. fragmented) | P0 | partial | `mp4parse` 0.17.0 [github.com/mozilla/mp4parse-rust] (tarball: no C, no build.rs, MPL-2.0); `symphonia-format-isomp4` 0.6.1 ("Great") | MOV legacy atoms; mp4parse is Firefox-oriented and its player-facing seek/index API is thin; ftyp/quirk breadth | M | 2 | `fuzz_demux_mp4` |
| Matroska / WebM | P0 | partial | `symphonia-format-mkv` 0.6.1 ("Good", tarball: pure, `build = false`); `matroska` 0.30.1 [github.com/tuffy/matroska] (metadata-only, mkvinfo-like) | Cues-based seek index handling; streaming EBML; malformed-block tolerance; attachments, ordered chapters | M | 2 | `fuzz_demux_mkv` |
| Ogg | P0 | pure-rust-exists | `ogg` 0.9.2 [github.com/RustAudio/ogg] (BSD-3, tarball: 9 .rs, no build.rs; decode+encode); `symphonia-format-ogg` 0.6.1 ("Great") | chained streams, skeleton, Opus-preskip edge cases | none | 2 | `fuzz_demux_ogg` |
| MPEG-TS | P0 | pure-rust-exists | `mpeg2ts-reader` 0.18.2 [github.com/dholroyd/mpeg2ts-reader] (tarball: pure); `mpeg2ts` 0.6.1 [github.com/sile/mpeg2ts] (MIT) | program mapping/zapping, teletext/DVB-sub extraction, PCR timing, corruption tolerance for broadcast captures | S (wrap + harden) | 5 | `fuzz_demux_ts` |
| MP3 raw stream | P0 | pure-rust-exists | `symphonia-bundle-mp3` 0.6.1 (MP1/MP2/MP3, "Excellent") | ID3v2 edge tags, Xing/VBRi seek tables, resync-on-garbage | none | 2 | `fuzz_demux_mp3stream` |
| AAC ADTS / LOAS | P0 | partial | ADTS framing inside `symphonia-codec-aac` glue; no standalone ADTS demuxer crate found | LOAS/LATM missing; ADTS variable frame sync robustness | S | 5 | `fuzz_demux_adts` |
| MPEG-PS (VOB-adjacent) | P1 | pure-rust-exists | `mpeg-ps` 0.6.0 [github.com/fishloa/rust-broadcast] (tarball: 14 .rs, no build.rs) | pack/system header + PES only `[VERIFY-P0]` scope; DVD-VOB IFO navigation out of scope (ADR) | M (harden) | 5 | `fuzz_demux_ps` |
| AVI | P1 | missing | none (crates.io: no maintained AVI demuxer; `riff` 2.0.0 [github.com/frabert/riff] is RIFF primitives only) | full AVI: index idx1/indx, OpenDML > 2 GB, interleaving | M | 5 | `fuzz_demux_avi` |
| AIFF / AIFF-C | P1 | pure-rust-exists | `symphonia-format-riff` 0.6.1 ("Great" for AIFF) | AIFF-C `sowt`/uLaw corners `[VERIFY-P0]` | none | 5 | `fuzz_demux_aiff` |
| ASF / WMV/WMA container | P2 | missing | none found (crates.io `asf` NOT_FOUND) | header/object tree, packet fragmentation, seek index | M–L | 6 | `fuzz_demux_asf` |
| RealMedia (.rm/.ra/.ram) | P3 | missing | none (crates.io `realmedia` NOT_FOUND) | legacy container; decode side is the real blocker (see §4) | M | later | `fuzz_demux_rm` |
| MPEG-4 elementary (.m4v) | P1 | missing | none standalone | VOSH/VOL + SL packet headers | S | 5 | `fuzz_demux_m4v` |
| 3GPP timed text (tx3g sidecar) | P3 | missing | none | small sidecar format; only when MP4 tx3g tracks show up in the wild | S | later | `fuzz_demux_tx3g` |
| CAF | P3 | pure-rust-exists | `symphonia-format-caf` 0.6.1 ("Good") | niche; fine as-is | none | later | `fuzz_demux_caf` |

Legend for the "candidate" column: every named crate was confirmed to exist on
crates.io on 2026-10-19 with the stated version, and — where noted — tarball-
inspected for purity. Repositories are cited inline; full transitive-graph
audits happen in Phase 0.

### 2.2 Container records (notes per format)

**WAV.** Symphonia rates WAV "Excellent" and PCM "Excellent"; `hound` is the
fallback/reference and gives encode support for the eventual transcode feature.
Gaps to own: WAVE_FORMAT_EXTENSIBLE channel masks, 24-bit and float layout
quirks, RF64. Self-build: none. Record in `docs/formats/container-wav.md`.

**FLAC.** Native FLAC is a container+codec bundle; both Symphonia (bundle) and
claxon (decode-only, frozen since 2020 but stable and widely used) are pure.
Seeking uses the STREAMINFO/SEEKTABLE; claxon exposes seekpoints. Encode later
via `flacenc` 0.5.1 [github.com/yotarok/flacenc-rs] (pure Rust, Apache-2.0,
verified on crates.io) for the transcode path.

**MP4/MOV/3GP.** Two credible pure-Rust parsers exist. `mp4parse` is
Mozilla's Firefox parser — production-grade for the atoms Firefox needs,
MPL-2.0, no build script (tarball-verified). Gaps: it is a metadata/sample
parser, not a full player-facing demuxer API; fragmented MP4 (moof) support
exists but its completeness for arbitrary progressive-download patterns needs
corpus testing `[VERIFY-P0]`; QuickTime-era atoms (e.g. old `stsd` entries,
`cmov` compressed MOV) are not guaranteed. Symphonia's `isomp4` is rated
"Great" and already player-shaped. Plan: use Symphonia isomp4 as the engine's
MP4 demuxer, keep mp4parse as a cross-check oracle in tests and for atoms
Symphonia rejects. QuickTime-specific quirks (compressed atoms, colr/gama
interpretation, aperture modes) get their own record section
`docs/formats/container-mp4.md` §QuickTime.

**Matroska/WebM.** Symphonia's MKV demuxer is "Good" (default feature). Gaps
we must own regardless of crate: cues-based seeking with byte-offset caching,
live/streaming EBML (unknown-size segments), attachment handling, broken-
BlockGroup tolerance, and AV1-in-BlockAdditional mappings `[VERIFY-P0]`. The `matroska` crate is
metadata-only (its README: "reading metadata… such as title, duration, track
information") — useful for the "media information" panel, not for demuxing.

**Ogg.** Two independent pure implementations; use `ogg` (container) +
Symphonia/lewton codecs, with `symphonia-format-ogg` as alternative. Own:
chained/stream boundary events, OggMIDI? no — Ogg Kate and Ogg Speex headers
`[VERIFY-P0]` (Kate is a subtitle codec; VLC supports it; no pure-Rust Kate
decoder found → later).

**MPEG-TS.** `mpeg2ts-reader` is a zero-copy push parser, pure, with decent
test culture; `mpeg2ts` (sile) adds muxing for the future stream-output
feature. Gaps: SI table breadth (PAT/PMT/SDT/EIT), teletext and DVB subtitle
PES extraction (needed by §5), PCR→STC clock reconstruction (Phase 3 work),
and tolerance for the corrupt captures real tuner users have. Fuzzing is
non-negotiable here.

**MPEG-PS.** `mpeg-ps` 0.6.0 (fishloa/rust-broadcast) is a young parser
covering pack header, system header, program stream maps `[VERIFY-P0]` exact
scope. DVD-Video navigation (IFO/VOB menus) is a separate, C-library-bound
problem — ADR territory per §14, not in this roadmap's v1 scope.

**AVI.** No maintained crate; self-build M: RIFF chunk walker, `hdrl`/`strl`
interpretation, idx1 + OpenDML indx seeking, `rec ` interleaving, and the
notorious junk-tolerance rules (AVI files in the wild are crime scenes).

**ASF/WMV.** Self-build M–L: GUID-object tree, data packet fragmentation,
language/streams, seek index. Pairs with the missing WMA codecs (§3), so the
container alone buys little — schedule both together in Phase 6.

**RealMedia.** Container is self-build M; the interesting payloads (RV30/40,
Cook, ATRAC3) have no pure-Rust decoders (crates.io `rv40`, `cook`,
`realmedia` all NOT_FOUND). Marked `later`; probable end state: demux+metadata
yes, decode blocked.

**AIFF.** Symphonia covers it ("Great") via the RIFF-format crate. AIFF-C
compressed variants (`sowt` is trivial; others rare) `[VERIFY-P0]`.

**Raw elementary streams (MP3 stream, ADTS AAC, .m4v, AC-3/E-AC-3 .ac3,
DTS .dts, MLP).** VLC probes and plays bare streams. Pattern: a small
`elementary-stream` wrapper crate with per-format sync/framing logic and one
fuzz target each. Sizes S each; schedule with their codecs.

---

## 3. Audio decoders

### 3.1 Summary table

| Codec | Prio | Status | Candidates (verified 2026-10-19) | Gaps | Self-build | Phase | Fuzz target |
|---|---|---|---|---|---|---|---|
| PCM: s16/s24/s32/float/u8, planar | P0 | pure-rust-exists | `symphonia-codec-pcm` 0.6.1 ("Excellent") | rare channel orders; WASM endianness | none | 2 | `fuzz_codec_pcm` |
| MP3 (MPEG-1/2/2.5 Layer III) | P0 | pure-rust-exists | `symphonia-bundle-mp3` 0.6.1 ("Excellent", gapless, also MP1/MP2) | MAD-class conformance breadth; free-format bitrate streams `[VERIFY-P0]` | none | 2 | `fuzz_codec_mp3` |
| MP2 | P1 | pure-rust-exists | same crate ("Great") | broadcast-grade corruption tolerance | none | 5 | `fuzz_codec_mp3` |
| AAC-LC | P0 | partial | `symphonia-codec-aac` 0.6.1 ("Great") | HE-AAC (SBR) and HE-AACv2 (PS) rated "—" (in work) upstream; see honesty note | M (SBR/PS) | 2 (LC only) | `fuzz_codec_aac` |
| AAC in MP4 vs ADTS | P0 | partial | same decoder; framing differs | ADTS header parser + ASC-in-esds unwrapping glue | S | 2/5 | `fuzz_demux_adts` |
| FLAC | P0 | pure-rust-exists | `symphonia-bundle-flac` ("Excellent"), `claxon` 0.4.3 | none material; encode via `flacenc` | none | 2 | `fuzz_codec_flac` |
| ALAC | P1 | pure-rust-exists | `symphonia-codec-alac` 0.6.1 ("Great", gapless) | old `alac` crate 0.5.0 [github.com/ebarnard/alac.rs] kept only as oracle | none | 5 | `fuzz_codec_alac` |
| Vorbis | P0 | pure-rust-exists | `symphonia-codec-vorbis` ("Excellent"); `lewton` 0.10.2 [github.com/RustAudio/lewton] (tarball: pure, MIT/Apache) | lewton unmaintained since 2021 — Symphonia is primary | none | 2 | `fuzz_codec_vorbis` |
| Opus | P0 | partial | **no published pure-Rust decoder**: `opus`/`audiopus` crates are libopus bindings (forbidden); Symphonia lists Opus "—" in work, `symphonia-codec-opus` not yet on crates.io | the whole decoder, realistically | L–XL | 5–6 | `fuzz_codec_opus` |
| WMA 1/2 / Pro | P2 | missing | none (crates.io `wma` NOT_FOUND) | everything | L | later | `fuzz_codec_wma` |
| AC-3 / E-AC-3 | P1 | missing | none (crates.io `ac3` NOT_FOUND; liba52 is C) | everything | L | 6 | `fuzz_codec_ac3` |
| DTS (Coherent Acoustics) | P2 | missing | none (no decoder found; `dts` crate is unrelated tooling) | everything | L | later | `fuzz_codec_dts` |
| TrueHD (MLP) | P2 | partial | `truehd` 0.7.2 [github.com/truehdd/truehdd] — "research implementation of Dolby TrueHD parser/decoder", tarball-verified pure (28 .rs, no build.rs, Apache-2.0) | self-described research-grade; needs conformance corpus + seeking | M (harden) | 6 | `fuzz_codec_truehd` |
| ADPCM (MS/IMA/OKI/Yamaha…) | P1 | pure-rust-exists | `symphonia-codec-adpcm` 0.6.1 ("Good", default-on) | some exotic variants; correctness across the ADPCM zoo | none | 5 | `fuzz_codec_adpcm` |
| WavPack | P3 | partial | Symphonia lists WavPack "—" in work; `wavpack` crate on crates.io is C bindings (forbidden) | decoder | M | later | `fuzz_codec_wavpack` |
| Speex | P3 | missing | none (crates.io `speex` NOT_FOUND) | everything | L | later | `fuzz_codec_speex` |
| Musepack (SV7/SV8) | P3 | missing | none (crates.io `musepack` NOT_FOUND) | everything | M | later | `fuzz_codec_mpc` |
| MIDI / synthesized | P3 | later | `rustysynth` 1.3.7 [github.com/sinshu/rustysynth] "SoundFont MIDI synthesizer written in pure Rust" (MIT); `oxisynth` 0.1.0 [github.com/PolyMeilex/oxisynth] (⚠ LGPL-2.1 — license review) | needs SMF parser (S) + SoundFont bundling policy `[PROPOSAL]` | S (SMF) | later | `fuzz_codec_midi` |
| ATRAC1/3, ATRAC-X | P3 | later | none | everything | L | later | — |

### 3.2 Audio notes

**AAC honesty note.** AAC-LC via Symphonia is rated "Great" upstream — good
enough for the common MP4/M4A and ADTS files. HE-AAC (SBR, "aacPlus") and
HE-AACv2 (PS) are *not done* upstream (status "—"). This matters because HE-AAC
is common in broadcast captures and low-bitrate streams; playing them at
half-sample-rate LC audio is audible degradation, and silently doing so would
violate the truthful-reporting rule. Plan: Phase 2 ships AAC-LC only, the
feature matrix says `partial (LC)`, and SBR/PS is a self-build M sized against
the spec once Phase 5 starts — or the format is honestly listed unsupported
until then. Licensing note: AAC decoder patents are a §14 question for doc 11,
independent of code availability.

**MP3 quality.** Symphonia is the only maintained pure-Rust MP3 decoder found
(`minimp3`-family crates are C; forbidden). Its upstream status is "Excellent"
against their corpus; MAD-class edge cases (free-format streams, heavily
corrupted headers, MPEG-2.5) get our own differential tests against VLC-as-
oracle in Phase 5. We do not promise MAD parity; we promise measured parity on
our published corpus.

**Opus is the biggest audio gap.** libopus is C and its bindings are forbidden;
Symphonia's Opus work is unpublished. Options: (a) wait/contribute upstream
(track `symphonia-codec-opus`), (b) self-build an Opus decoder (L–XL — the
CELT+SILK hybrid is subtle), (c) ship v1 with Opus `missing` and let WebM
fall back to its Vorbis tracks. Recommendation `[PROPOSAL]`: contribute to or
vendor-forward Symphonia's in-work decoder rather than starting parallel;
decision point at the Phase 0 audit.

**MIDI `[PROPOSAL]`.** VLC plays MIDI through FluidSynth (C — forbidden).
rustysynth is a credible pure-Rust SoundFont synthesizer; pairing it with a
self-built SMF parser (S) gives MIDI playback without SoundFont shipping (user
supplies .sf2, like VLC's own fluidsynth plugin requires). oxisynth's LGPL-2.1
needs the license review before any use. Status: `later`, not `blocked`.

**PCM variants.** "PCM" is many formats: integer 8/16/24/32, float 32/64,
signed/unsigned, planar vs interleaved, big/little endian (AIFF is BE).
Symphonia's PCM codec covers the matrix ("Excellent"); our records must map
each container's PCM fourcc/encoding to the exact layout (see per-format
records, §6).

---

## 4. Video decoders

### 4.1 Summary table

| Codec | Prio | Status | Candidates (verified 2026-10-19) | Gaps | Self-build | Phase | Fuzz target |
|---|---|---|---|---|---|---|---|
| AV1 | P0 | partial | `rav1d` 1.1.0 [github.com/memorysafety/rav1d] — Rust port of dav1d (BSD-2). Tarball: contains ARM `.S` and x86 asm, `build.rs` with `cc` + `nasm-rs` → **policy-compliant only with `asm` feature off**; no-asm Rust fallbacks exist | speed without asm; upstream stability | none (integrate no-asm first) | 5 | `fuzz_codec_av1` |
| Motion-JPEG | P1 | pure-rust-exists | `jpeg-decoder` 0.3.2 [github.com/image-rs/jpeg-decoder] (tarball ecosystem pure); `zune-jpeg` 0.5.x [github.com/etemesi254/zune-image] | field-based MJPEG (interlaced), AVI1 chunk format | S (glue) | 2 | `fuzz_codec_mjpeg` |
| Uncompressed / RGB(YUV) video | P1 | missing (trivial) | none needed — own format defs | fourcc zoo (v210, v308, I420, BGRx…) | S | 2 | `fuzz_codec_rawvideo` |
| Theora | P1 | missing | none (crates.io `theora` is a reserved name; libtheora is C) | everything | L | later | `fuzz_codec_theora` |
| VP8 | P1 | missing | none (crates.io `vp8` NOT_FOUND; libvpx is C) | everything | XL | later | `fuzz_codec_vp8` |
| VP9 | P0 | missing | none (crates.io `vp9` NOT_FOUND) | everything | XL | later | `fuzz_codec_vp9` |
| H.264/AVC | P0 | hw-only (see §4.2) | forbidden: `openh264` (C bindings); crates.io `h264` is an empty reserved name; **no production pure-Rust decoder exists today** `[VERIFY-P0]` | software decode entirely | XL (26–104 pw, multi-quarter) | hw: 5–7 | `fuzz_codec_h264` (parser self-build) |
| HEVC/H.265 | P1 | hw-only | forbidden: libde265 etc.; no pure-Rust decoder found (crates.io `hevc`, `de265`, `libhevc-rs` NOT_FOUND) | software decode; patents | XL | hw: 7 | `fuzz_codec_hevc` (parser self-build) |
| MPEG-1/2 video | P1 | missing | none (libmpeg2 is C) | everything (DVD/DVB workhorse) | L | 6 | `fuzz_codec_mpeg2` |
| MPEG-4 Part 2 / ASP (DivX/Xvid) | P1 | missing | none | everything (incl. B-frames, QPel, GMC) | L–XL | later | `fuzz_codec_mpeg4asp` |
| VC-1 / WMV9 | P2 | missing | none (no crate; reference decoder unavailable) | everything | XL | later | `fuzz_codec_vc1` |
| Windows Screen 1/2, TSCC-ish | P3 | later | none | everything | S–M (simple codecs) | later | — |
| Apple ProRes | P3 | later | none | everything | L | later | — |
| DNxHD/HR | P3 | later | none | everything | L | later | — |
| FFV1 (lossless) | P3 | later | none on crates.io verified `[VERIFY-P0]` | everything | M | later | — |
| Indeo 3/4/5 | P3 | blocked | none; legacy + patent haze | — | — | — | — |
| RealVideo RV10–RV40 | P3 | blocked | none | — | — | — | — |
| Hap, Dirac, Cinepak, MS Video 1… | P3 | later | none | long tail | varies | later | — |

### 4.2 H.264/AVC — the single biggest risk, honestly

Facts (verified 2026-10-19): there is **no production-grade pure-Rust H.264
decoder**. The crates.io name `h264` is an empty reserved placeholder;
`openh264` and `x264` crates are C bindings (forbidden by §9). Symphonia is
audio-only. rav1d covers AV1 only. The realistic options:

- **(a) ADR-001 hardware-decode-first (recommended `[PROPOSAL]`)**: VideoToolbox
  (macOS), Media Foundation (Windows), VA-API (Linux) decode H.264 on the OS
  side as the *primary* path, behind the audited OS-ABI exception, with
  truthful UI/matrix reporting whenever the software path is absent. Coverage:
  good on desktop for H.264-in-MP4/MKV/TS. Consequences: the **web build has
  no OS decoder** — H.264 on wasm32 is `blocked` unless WebCodecs is accepted
  as the browser-OS analog (it is a browser API, not a C dependency — but it
  is a non-Rust decode path and needs its own ADR `[PROPOSAL]`, plus it breaks
  the "same engine everywhere" story); hardware-less desktops and exotic
  profiles (10-bit Hi10P anime encodes famously) fall over.
- **(b) Long-term self-build**: a spec-driven clean-room H.264 decoder is an
  XL project (26–104+ person-weeks for Baseline→Main→High). It is the only way
  to make the parity claim stand without hardware. Sequence after Phase 5 core
  breadth, if ever, gated by an explicit ADR.
- **(c) Profile-subset decode**: Baseline-only software decoder (M-sized) covers
  low-complexity streams (some web/legacy content, most camera footage) and
  gives a fallback for hw-less machines; interlaced, CABAC, 10-bit stay out.
  Honest `partial (Baseline)` reporting required.

Consequence for parity per §1: H.264 is a P0 codec; until one of (a)/(b)/(c)
lands, the feature matrix records H.264 software decode as `blocked`/`hw-only`
and the parity percentage is computed against that truth. No overclaiming.
What we *can* self-build cheaply regardless: a **pure-Rust H.264 parser**
(NAL/SPS/PPS/slice-header, MP4 avcC and Annex-B unwrapping) — needed anyway to
feed hardware decoders and to probe/profile streams. That parser is M-sized
and gets `fuzz_codec_h264` (parser-level fuzzing even where decode is hw).

### 4.3 HEVC — same shape, worse

Same options as H.264, plus: patent/licensing assessment for *distributing an
HEVC decoder at all* is a §14 item (doc 11's job, not ours); hardware support
is broader on recent OSes; the pure-Rust self-build is XL and lower value than
H.264's. Default plan: ADR-001 hw-only, parser self-build M, software decode
`blocked` with next-step recorded.

### 4.4 AV1 and the rav1d nuance

rav1d (memorysafety/Prossimo project, v1.1.0, 2025) is a genuine Rust port of
dav1d — the most credible pure-Rust video decoder in existence. Two caveats
found by tarball inspection: (1) it ships hand-written assembly (ARM `.S`,
x86 nasm) built through `cc`/`nasm-rs` — **default build fails §9**; the
`asm` feature can be disabled, leaving Rust fallbacks `[VERIFY-P0: confirm
zero non-Rust build activity with asm off, and benchmark the speed cost]`;
(2) it is a young port — conformance corpus + differential tests against
dav1d/VLC needed before we claim anything. Plan: integrate no-asm first as the
Phase 5 AV1 path; revisit the asm question as an ADR (assembly is not C, but
it is non-Rust native code and must not sneak in by default). Note rav1e
(pure-Rust AV1 *encoder*, Xiph, BSD-2) exists for the future transcode path.

### 4.5 The "video codec hole" summary

WebM's flagship codecs are VP8/VP9 (missing, XL each) and AV1 (rav1d, partial).
MPEG-land's are H.264 (hw-only), HEVC (hw-only), MPEG-2 (missing, L),
MPEG-4 ASP (missing, L–XL). This means v1's software video story is:
**AV1 + Motion-JPEG + uncompressed video**, with H.264/HEVC via hardware where
the OS provides it. That is the honest ceiling until the Phase 5/6 self-build
program (§8) fires. The parity report says exactly this.

---

## 5. Subtitle & caption formats

Text rendering itself (font selection, shaping) is a renderer concern
(`renderer-wgpu`, doc 03); this section covers parsing, timing models, and the
compositing contract. Bitmap subtitles additionally require the renderer's
YUV/RGB overlay path — a hard dependency to track.

| Format | Prio | Status | Candidates / plan | Self-build | Phase | Fuzz target |
|---|---|---|---|---|---|---|
| SubRip (.srt) + variants | P0 | partial | spec is trivial; `srt` 0.1.0 [github.com/tytouf/srt-rs] (2017, stale) — self-build S, using the emk `subtitles-rs` family [github.com/emk/subtitles-rs] as reference | S | 2 | `fuzz_sub_srt` |
| ASS/SSA | P0 | partial | **libass is C — forbidden.** Text/timing parse via `ass-parser` 0.2.3 [github.com/Aavtic/ass_parser] and `ssa-parser` 0.1.0 [github.com/mikai233/subtitle_parser] (both verified on crates.io); styling subset self-build `[PROPOSAL]` | M (styling subset) | 2 (plain text) → 5 (styling) | `fuzz_sub_ass` |
| WebVTT | P1 | partial | `webvtt-parser` 1.2.1-rc [github.com/dmtrKovalenko/fframes] (RC — audit); `webvtt` 0.2.0 (emk family); spec is small | S (harden) | 5 | `fuzz_sub_vtt` |
| SubViewer (.sub/.sbv) | P2 | missing | self-build S | S | 5 | `fuzz_sub_subviewer` |
| MicroDVD (.sub) | P2 | missing | frame-number-based timing; self-build S | S | 5 | `fuzz_sub_microdvd` |
| MPL2 | P3 | missing | self-build S | S | later | `fuzz_sub_mpl2` |
| SAMI (SMI) | P2 | missing | HTML-ish; self-build S–M | S–M | 5 | `fuzz_sub_sami` |
| TTML/DFXP (incl. IMSC) | P2 | missing | XML; self-build M (subtitle semantics subset) | M | 6 | `fuzz_sub_ttml` |
| PGS (Blu-ray) | P2 | missing | parse is documented RLE segments; render → renderer-wgpu overlay | S parse + M composite | 6 | `fuzz_sub_pgs` |
| DVB subtitles (in TS) | P2 | missing | segment parsing inside MPEG-TS (pairs with §2 TS work) | S–M | 6 | `fuzz_sub_dvb` |
| VobSub (DVD .sub/.idx) | P2 | pure-rust-exists | `vobsub` 0.2.3 [github.com/emk/subtitles-rs] (CC0, 2017) — decode verified; sub/idx sync + palette | S (harden) | later | `fuzz_sub_vobsub` |
| CC/608/708 (CEA captions) | P1 | missing | crates.io: `cea608` 0.0.1 is a builder, not a decoder; `cc708`/`eia608` NOT_FOUND → self-build from CEA-608-E/708-D specs; 708 is the larger job | M | 6 | `fuzz_sub_cc` |
| Ogg Kate | P3 | missing | no pure decoder found `[VERIFY-P0]` | M | later | — |
| 3GPP timed text (tx3g) | P3 | missing | in-MP4 track; self-build S | S | later | `fuzz_demux_tx3g` |

**ASS styling decision `[PROPOSAL]`.** libass is the industry stylizer and is
C. Plan: (1) Phase 2 ships ASS/SSA as plain text with basic positioning (Style
font/colour/alignment) — the `subparse` crate (0.7.0, MPL-2.0,
[github.com/kaegi/subparse]) demonstrates the parse surface; (2) Phase 5 adds a
self-built styling subset: \b, \i, \fn, \fs, \c/\1c-\4c, \pos, \move, \an, karaoke
timing tags — the tags that carry ~95 % of real-world styling value
`[VERIFY]`; (3) unsupported tags degrade gracefully and are counted in the
feature matrix (`partial (styling subset)`); (4) full libass parity is
explicitly *not claimed*. Font rendering comes from the renderer's text stack
(pure-Rust shaping via a Rust text crate is doc 03/01 scope `[VERIFY-P0]`).

**Bitmap compositing dependency.** PGS, DVB, VobSub all produce paletted
bitmaps that renderer-wgpu must composite with correct timing on the video
timeline. That contract (overlay plane format, palette, alpha, coordinates,
epoch/segment semantics) must be specified in doc 01 before Phase 6 lands any
bitmap sub. Tracking item, not a codec problem.

---

## 6. Per-format record template (§8 of MASTER-PROMPT)

Every format Lumen claims gets a record file `docs/formats/<kind>-<name>.md`
(`container-*`, `audio-*`, `video-*`, `sub-*`). One record = one feature-matrix
key family. The template is:

```markdown
# <Format name> (<kind>)
TOML front-matter keys:
  id            = "container-mp4"          # matches feature-matrix.toml
  status        = "verified|implemented|in_progress|not_started|blocked"
  phase         = 2                         # first phase that ships it
  fuzz_target   = "fuzz_demux_mp4"          # required when status != not_started
  crates        = ["symphonia-format-isomp4 0.6.1 (MPL-2.0)"]
  self_built    = []                        # crates we own
  last_audit    = "2026-10-19"              # dependency-audit date

## Parse / Decode / Encode
What is implemented, in which crate, behind which trait. Explicit list of
unsupported constructs.

## Bit depths / layouts
## Tracks (selection, default, languages)
## Seeking (index source, granularity, cost, edge cases)
## Timestamps (units, time base, discontinuity behavior)
## Corrupt-input behavior (what is skipped, what errors out, known crashes)
## Limits (resolution, channels, duration, memory ceilings)
## Platforms (windows/linux/macos/wasm32 results)
## Corpus (fixture list with licenses + source)
## Results (differential vs VLC 3.0.23, fuzz stats, CI evidence links)
```

### 6.1 Filled example — strong case: `docs/formats/audio-flac.md`

> **Both examples below are illustrative mock records.** The numbers, corpora
> and results shown are placeholders demonstrating the required *shape* of
> evidence, not actual measurements — real records cite CI artifacts and are
> only written when the evidence exists (AGENTS.md rule 4).

```markdown
---
id = "audio-flac"
status = "verified"
phase = 2
fuzz_target = "fuzz_codec_flac"
crates = ["symphonia-bundle-flac 0.6.1 (MPL-2.0)", "claxon 0.4.3 (Apache-2.0, oracle)"]
self_built = []
last_audit = "2026-10-19"
---
# FLAC (decode)

## Parse / Decode / Encode
Decode: symphonia-bundle-flac behind `codec-api::AudioDecoder`. Native FLAC
(STREAMINFO + frames) and Ogg-FLAC via the Ogg demuxer. Encode: not in v1;
flacenc 0.5.1 audited for the Phase 7 transcode path.

## Bit depths / layouts
4–32-bit integer at 1–655350 Hz, mono to 8.0 channels; output normalized to
i16/i24/f32 planar per codec-api. Channel order follows the Vorbis channel
mapping that FLAC inherits.

## Tracks
Single audio track per file. Metadata: Vorbis comments + PADDING + picture
blocks surfaced as track metadata events.

## Seeking
Native SEEKTABLE when present; otherwise duration/estimate seek (binary search
over frames — claxon exposes seekpoints; Symphonia implements sample-accurate
seek [VERIFY-P0 on exact API]). Target: ≤ 150 ms on 100 MB files.

## Timestamps
Sample-count domain; gapless (upstream "Excellent", gapless=yes). Time base =
1/sample_rate.

## Corrupt-input behavior
CRC-8/CRC-16 mismatch → drop frame, emit silence event, continue; malformed
STREAMINFO → unsupported error at open (never a hang). Fuzzer: 24 h, 0
panics; 4-frame corpus of truncated/mid-header files green.

## Limits
Max blocksize 65535 samples; memory bounded by max-frame policy (§12).

## Platforms
windows/linux/macos: CI decode-diff vs VLC 3.0.23 on 42-file corpus, 100 %
match (bit-exact, ±0 samples). wasm32: same corpus, bit-exact [dates+links].

## Corpus
tests/fixtures/flac/: 12 generated (ffmpeg flac -vary via external oracle —
generated, CC0), 30 from public-domain music set (see CORPUS.md).

## Results
Differential harness `tools/parity-report run audio-flac` green since
<commit>; parity key flac.decode = verified.
```

### 6.2 Filled example — gap case: `docs/formats/video-h264.md`

```markdown
---
id = "video-h264"
status = "partial"          # truthful: parser+hw path exist, software decode does not
phase = 5                    # hw-first integration
fuzz_target = "fuzz_codec_h264"
crates = []
self_built = ["codecs-h264-parse (NAL/SPS/PPS/slice header, avcC↔Annex-B)"]
last_audit = "2026-10-19"
---
# H.264/AVC (decode)

## Parse / Decode / Encode
Parse: self-built bitstream parser (M effort, Phase 5) — profiles/levels, SPS/
PPS extraction, Annex-B ↔ MP4 avcC conversion, SEI surfacing. Decode: NO
software decoder (no production pure-Rust implementation exists — §4.2).
Hardware: via ADR-001 platform decoders (VideoToolbox / Media Foundation /
VA-API), reported as active in UI and matrix when used. Web (wasm32): blocked
unless WebCodecs ADR passes. Encode: out of scope v1.

## Bit depths / layouts
8-bit 4:2:0 via hw paths (10-bit/4:2:2/4:4:4 = hw-dependent; Hi10P unsupported
everywhere → blocked, stated in UI error).

## Tracks
Per-container; parser feeds track selection with profile/level info.

## Seeking
Index from container (mp4 stss/sync samples, mkv cues); parser supplies IDR
boundaries. Frame-accurate seek only where container index allows.

## Timestamps
POC/pts handling in hw wrappers; discontinuities surfaced as events.

## Corrupt-input behavior
Parser must be panic-free under fuzz (24 h target). Damaged NALs → drop to
next IDR with a decode-error event; never fabricate frames (MASTER-PROMPT §8).

## Limits
Level 5.2 streams gated by hw caps; software fallback = none (honest error).

## Platforms
windows: MF H.264 ✓ [matrix link]; linux: VA-API ✓ (Intel/NVIDIA tested, AMD
[VERIFY]); macos: VideoToolbox ✓; wasm32: blocked (no WebCodecs ADR yet).

## Corpus
tests/fixtures/h264/: 18 files (encoder-generated CC0 + Sintel/Big Buck Bunny
CC-BY trailers), 6 malformed NAL cases.

## Results
Differential: N/A (no software decode to diff) — hw output validated by
playback conformance suite instead; clearly labeled, not counted as decode
parity. matrix key h264.software_decode = blocked; h264.hw_decode = partial.
```

These two examples set the honesty bar: a "gap" record is still a *complete*
record — what exists, what does not, and how the matrix reports it.

---

## 7. Honest gap register (top risks, ranked)

| # | Gap | Consequence to parity | Mitigations | Truthful reporting rule |
|---|---|---|---|---|
| 1 | **H.264 software decode** (no pure-Rust decoder exists) | Largest single-format hole; ~half of real-world video `[VERIFY]` | (a) ADR-001 hw-first; (b) XL self-build; (c) M-sized Baseline subset; hw-parsers regardless | `h264.software_decode = blocked`; `h264.hw_decode = partial/hw-only`; UI states when hw is active |
| 2 | **HEVC software decode** | Growing share (UHD/broadcast) `[VERIFY]`; patents compound it | ADR-001 hw; parser self-build; XL self-build unlikely v1 | same pattern as H.264; `blocked` on wasm32 |
| 3 | **Opus decoder** | WebM/YouTube-class audio unplayable; Opus is P0 | contribute to Symphonia in-work crate; L–XL self-build | `opus = missing` until it passes conformance; Vorbis-track fallback noted |
| 4 | **VP9 (and VP8) software decode** | Second web-video pillar | XL self-build each (post-v1 candidates); AV1 via rav1d covers newer content only | `vp9/vp8 = missing/later` |
| 5 | **ASS styling** | Styled subtitles render plain | self-built subset `[PROPOSAL]`; graceful degradation | `ass.styling = partial (subset)`; unsupported tags counted |
| 6 | **PGS/DVB bitmap subs** | Blu-ray/DVB caption holes | S–M parse self-build + renderer-wgpu overlay contract (doc 01 dependency) | `pgs/dvbsub = missing` until compositing proven |
| 7 | **DTS / TrueHD / AC-3 family** | Surround-audio gap for movie files | TrueHD: truehdd crate hardening (M); AC-3/DTS: L self-builds Phase 6 | `dts = missing`, `truehd = partial`, `ac3 = missing` — no silent silence |
| 8 | **MPEG-1/2 + MPEG-4 ASP** | Legacy/DVD/DivX hole | L–XL self-builds queued Phase 6+ | `missing/later` |
| 9 | **MIDI** | Niche but VLC-listed | rustysynth `[PROPOSAL]` + SMF parser; SoundFont = user-supplied | `midi = later` |
| 10 | **Real/Indeo/legacy C-only codecs** | Long tail | no compliant path — permanently `blocked` with reason | `blocked` rows stay visible in matrix forever |

Rule that binds all of them: the feature matrix computes parity only from
verified rows; every row here is `blocked` or `partial` until its record file
shows automated evidence (§17). "Plays on my machine" is not a status.

---

## 8. Phase assignment, sequencing, and the fuzz-target registry

### 8.1 Phase 2 — first vertical (the one file that must play end-to-end)

Recommended pair `[PROPOSAL]`, chosen because every layer is
`pure-rust-exists` today, no ADR needed:

- **Primary: MP4 + PCM audio + Motion-JPEG video** — Symphonia isomp4 (Great),
  symphonia-codec-pcm (Excellent), jpeg-decoder/zune-jpeg (pure). Exercises
  real demux, timestamp, queue, clock, command layer, Tauri + video-in-webview
  prototype, with zero policy exceptions.
- **Stretch: Ogg + Vorbis audio** (ogg + symphonia-codec-vorbis, both
  Excellent) and **FLAC + WAV** as the audio-only warmups in week 1.

H.264-in-MP4 is deliberately *not* the first vertical: it drags ADR-001 in
before the pipeline exists. It becomes the Phase 5 hw-first flagship.

### 8.2 Phase 3–4

No new formats by design (per §16); Phase 3 hardens seeking/AV-sync across
the Phase 2 set; Phase 4 makes them product-grade. The per-format records for
the Phase 2 set reach `verified` here.

### 8.3 Phase 5 — breadth order (P0 first)

1. FLAC/WAV/PCM records to `verified` (if not already); MP3 (excellent crate
   status, high user value).
2. MP4+AAC-LC (matrix `partial (LC)`), Matroska full (cues seeking, WebM
   payload set minus VP9), Ogg polish.
3. **AV1 via rav1d (asm-off)** — the flagship software video codec; conformance
   corpus + differential vs VLC.
4. **H.264 hw-first** (ADR-001): parser self-build + three platform decoders;
   HEVC follows.
5. MPEG-TS (with AC-3 passthrough/decode pending), MPEG-PS, AVI+ADPCM,
   AIFF, ALAC, Motion-JPEG polish, elementary-stream wrappers.
6. Subtitles wave 1: SRT, ASS plain text, WebVTT.
7. MP2, ADPCM breadth, TrueHD hardening.

### 8.4 Phases 6–7

AC-3 (self-build L), Theora (L) if prioritized by Phase 0 share data, Speex/
Musepack/WavPack per demand, ASF+WMA decision point, DTS decision point, PGS/
DVB/VobSub + CC/608/708 with the renderer overlay contract, TTML, MIDI
`[PROPOSAL]`, ProRes/DNxHD/screen codecs `later`. Web build parity column gets
its own pass (WebCodecs ADR decision).

### 8.5 Fuzz-target registry (§12: every parser gets one)

Directory `fuzz/`, cargo-fuzz, one target per parser named exactly as in the
tables above (`fuzz_demux_*`, `fuzz_codec_*`, `fuzz_sub_*`). Registry rules:

- A parser crate may not reach `implemented` until its target exists, runs 10
  minutes clean in CI (release mode), and has a seed corpus committed.
- Corpora: generated malformed files (our generators are Rust, CC0), truncated
  fixtures, plus legally-distributable real files listed in
  `tests/fixtures/CORPUS.md` with license per file.
- Coverage gates: new parser without corpus = CI failure (feature-audit tool).
- Targets also build on wasm32 where feasible `[VERIFY-P0]`.

### 8.6 Phase 0 audit list (everything `[VERIFY-P0]` in this doc)

1. Symphonia AAC/HE-AAC/Opus real coverage on our corpus; `symphonia-codec-opus`
   publication status. 2. rav1d asm-off purity proof + speed benchmark. 3.
   mp4parse fragmented-MP4 & MOV-quirk corpus pass. 4. `mpeg-ps` scope. 5.
   `truehd` (truehdd) conformance vs reference streams. 6. oxisynth LGPL
   review; rustysynth SMF needs. 7. Usage-share data to replace `[VERIFY]`
   estimates (from VLC 3.0.23 inventory + telemetry-free corpus stats). 8.
   WebCodecs-as-OS-decoder ADR scope. 9. Kate/Ogg FFV1/DVD-Video ADRs. 10.
   Full transitive-graph audit of every crate named here (cargo-deny/vet).

---

*This roadmap is the contract the feature matrix will be audited against. When
reality disagrees with a row, the row loses.*


