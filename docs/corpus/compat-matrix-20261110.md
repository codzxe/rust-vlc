# Full-corpus compatibility matrix — 2026-11-10 (issue #53, cards 8.B1–8.B2)

| Field | Value |
|---|---|
| Schema | `compat-matrix/report/1` (this file is the human rendering; machine JSON alongside) |
| Run | `2026-11-10T19:27:42Z` on linux / x86_64 (arch recorded; the native lane decodes on exactly one host) |
| Commit | `3dfe1c0b19c593011a5d238074fc4f67eb390882` |
| Corpus | 46 committed + 54 malformed (error-contract) + 66 regenerable (manifest-anchored, `tests/compat/regen-corpus-manifest.json`) |
| Oracle | live run: the pinned VLC VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd) (version-asserted before use; invocation record in the config + docs/corpus/vlc-diff-report-20261023.md §Oracle run 2026-11-09) |
| Method | real decode through the player's open path (`lumen_cli::demux_host::open_source` → container registry → `HostDecoderProvider` → `codec_api::Decoder`); MD5-prefix + ratified f32 tolerance vs the pinned VLC 3.0.23 (see notes) |

## 1. Headline counts (the native lane)

- Corpus files: **166** (84 decoded, 22 decoded-partial, 6 refused, 0 demux-fatal; 54 malformed all-typed / zero crashes)
- Oracle differential (lossless-comparable family): **29 bit-exact** (MD5-prefix), **0 f32 tolerance-pass**, **6 f32 tolerance-FAIL**, **0 not-bit-exact**, 0 no-comparison
- Lanes: `linux-native` (native-decode), `web-wasm` (steps), `windows` (blocked), `macos` (blocked) — native decode on this host; web/wasm via recorded build steps; Windows/macOS blocked-by-external (user-owned)

## 2. The lanes and their evidence

### `linux-native` — Linux native decode (this host — the lane that actually ran) (native-decode)


### `web-wasm` — Web/wasm (recorded build + lane-proof steps; the wasm module ships decoder-less) (steps)

- `the wasm32 engine-core module builds for real (probe -> demux -> engine; codecs edge target-gated off — apps/web/Cargo.toml)` — `cargo build -p lumen-web --target wasm32-unknown-unknown` → **exit 0** (19.9s)
- `the web crate's host-side lane proof: capability snapshot + first-vertical MJPEG/PCM decode with the real decoders linked natively (the wasm module reports the decoder split honestly)` — `cargo test -p lumen-web` → **exit 0** (22.7s)

### `windows` — Windows (blocked-by-external — no result inferred) (blocked)

- **blocked-by-external**: 3-OS runner machines do not exist (live CI is one lean Linux job; the 3-OS matrix is parked) and CI billing for the full matrix is not provisioned
  - ref: docs/plans/phase8-cards.md section 1 (blocker table, rows 3-4: 3-OS runner machines + CI billing); risk register R-20 refresh (issue #21); doc 10 section 10.4 budget line
  - next step: user provisions Windows/macOS runner machines and unblocks CI billing, then the compat-matrix CI job (.github/workflows/compat-matrix.yml, this wave) runs this same runner on all three OS and replaces these blocked cells with real ones
  - tracking: issue #53 (blocked-by-external cells); umbrella issue #23 (Phase-8 gate)

### `macos` — macOS (blocked-by-external — no result inferred) (blocked)

- **blocked-by-external**: 3-OS runner machines do not exist (live CI is one lean Linux job; the 3-OS matrix is parked) and CI billing for the full matrix is not provisioned
  - ref: docs/plans/phase8-cards.md section 1 (blocker table, rows 3-4: 3-OS runner machines + CI billing); risk register R-20 refresh (issue #21); doc 10 section 10.4 budget line
  - next step: user provisions Windows/macOS runner machines and unblocks CI billing, then the compat-matrix CI job (.github/workflows/compat-matrix.yml, this wave) runs this same runner on all three OS and replaces these blocked cells with real ones
  - tracking: issue #53 (blocked-by-external cells); umbrella issue #23 (Phase-8 gate)

## 3. Container formats × lanes

| probe format | claimed (linux) | corpus files | linux-native | web-wasm | windows | macos |
|---|---|---|---|---|---|---|
| `avi` (AVI (incl. OpenDML)) | avi=L:implemented/W:implemented/M:implemented/B:not_started | 43 | decoded | engine-core-only | blocked | blocked |
| `matroska` (Matroska / WebM / MKA) | matroska-webm=L:implemented/W:implemented/M:implemented/B:not_started | 11 | decoded | engine-core-only | blocked | blocked |
| `mp4` (MP4 / MOV / 3GP / M4A (ISO-BMFF)) | mp4-mov-3gp=L:implemented/W:implemented/M:implemented/B:not_started | 14 | decoded | engine-core-only | blocked | blocked |
| `ogg` (Ogg (incl. Ogv)) | ogg=L:implemented/W:implemented/M:implemented/B:not_started | 6 | decoded | engine-core-only | blocked | blocked |
| `mpegts` (MPEG transport stream) | mpegts=L:implemented/W:implemented/M:implemented/B:not_started | 10 | decoded | engine-core-only | blocked | blocked |
| `mpegps` (MPEG program stream / VOB) | mpegps=L:implemented/W:implemented/M:implemented/B:not_started | 1 | decoded | engine-core-only | blocked | blocked |
| `asf` (ASF / WMV / WMA) | asf-wmv=L:implemented/W:implemented/M:implemented/B:not_started | 1 | refused | engine-core-only | blocked | blocked |
| `wav` (WAV (RIFF/WAVE)) | wav=L:implemented/W:implemented/M:implemented/B:not_started | 10 | decoded | engine-core-only | blocked | blocked |
| `flac` (FLAC native stream) | flac-native=L:not_started/W:not_started/M:not_started/B:not_started | 1 | refused | engine-core-only | blocked | blocked |
| `aiff` (AIFF (COMM/SSND)) | aiff=L:implemented/W:implemented/M:implemented/B:not_started | 4 | decoded | engine-core-only | blocked | blocked |
| `adts` (ADTS / raw AAC stream) | adts-aac=L:implemented/W:implemented/M:implemented/B:not_started | 5 | decoded | engine-core-only | blocked | blocked |
| `mp3` (Raw MP3 stream) | mp3-raw=L:implemented/W:implemented/M:implemented/B:not_started | 1 | decoded | engine-core-only | blocked | blocked |
| `mp2` (Raw MP2 stream (MPEG audio ES)) | mp3-raw=L:implemented/W:implemented/M:implemented/B:not_started | 1 | decoded | engine-core-only | blocked | blocked |
| `h264` (Raw H.264 (Annex B)) | h264-annexb=L:implemented/W:implemented/M:implemented/B:not_started | 1 | decoded | engine-core-only | blocked | blocked |
| `hevc` (Raw HEVC (Annex B)) | hevc-annexb=L:implemented/W:implemented/M:implemented/B:not_started | 1 | decoded | engine-core-only | blocked | blocked |
| `ivf` (IVF (AV1/VP9 raw container)) | ivf=L:implemented/W:implemented/M:implemented/B:not_started | 2 | decoded | engine-core-only | blocked | blocked |

Cell details (native lane):

- `avi`: decoded — 41/41 files open+demux (41 fully decoded, 0 with a codec-side NoDecoder track); error-contract: 9 malformed variants, all typed (zero crashes)
- `matroska`: decoded — 11/11 files open+demux (5 fully decoded, 6 with a codec-side NoDecoder track); error-contract: 9 malformed variants, all typed (zero crashes)
- `mp4`: decoded — 14/14 files open+demux (4 fully decoded, 10 with a codec-side NoDecoder track); error-contract: 14 malformed variants, all typed (zero crashes)
- `ogg`: decoded — 6/6 files open+demux (5 fully decoded, 1 with a codec-side NoDecoder track)
- `mpegts`: decoded — 10/10 files open+demux (9 fully decoded, 1 with a codec-side NoDecoder track)
- `mpegps`: decoded — 1/1 files open+demux (0 fully decoded, 1 with a codec-side NoDecoder track)
- `asf`: refused — 1/1 corpus files REFUSED at the open path — unsupported (sniffed 30 26 b2 75 8e 66 cf 11 a6 d9 00 aa 00 62 ce 6c) (probe.no-demuxer): probe hints: extension=wmv, size=10329
- `wav`: decoded — 8/8 files open+demux (8 fully decoded, 0 with a codec-side NoDecoder track)
- `flac`: refused — 1/1 corpus files REFUSED at the open path — format `flac` probes (the fLaC marker) but the raw-FLAC elementary-stream demuxer has not landed: crates/demux/src/es covers h264/hevc/mp3/aac (card 5.I3a's slice) — the landing card harvests the 34-byte STREAMINFO body as extra_data for the already-wired FLAC decoder and flips this one registry arm
- `aiff`: decoded — 4/4 files open+demux (4 fully decoded, 0 with a codec-side NoDecoder track)
- `adts`: decoded — 5/5 files open+demux (4 fully decoded, 1 with a codec-side NoDecoder track)
- `mp3`: decoded — 1/1 files open+demux (1 fully decoded, 0 with a codec-side NoDecoder track)
- `mp2`: decoded — 1/1 files open+demux (1 fully decoded, 0 with a codec-side NoDecoder track)
- `h264`: decoded — 1/1 files open+demux (0 fully decoded, 1 with a codec-side NoDecoder track)
- `hevc`: decoded — 1/1 files open+demux (0 fully decoded, 1 with a codec-side NoDecoder track)
- `ivf`: decoded — 2/2 files open+demux (2 fully decoded, 0 with a codec-side NoDecoder track)

## 4. Codec families × lanes

| codec | claimed (linux) | corpus files | linux-native | web-wasm | windows | macos |
|---|---|---|---|---|---|---|
| `pcm_` (audio) | pcm=L:implemented/W:implemented/M:implemented/B:not_started | 48 | decoded | engine-core-only | blocked | blocked |
| `mjpeg` (video) | mjpeg=L:implemented/W:implemented/M:implemented/B:not_started | 33 | decoded | engine-core-only | blocked | blocked |
| `flac` (audio) | flac=L:implemented/W:implemented/M:implemented/B:not_started | 3 | decoded | engine-core-only | blocked | blocked |
| `mp3` (audio) | mp3=L:implemented/W:implemented/M:implemented/B:not_started | 1 | decoded | engine-core-only | blocked | blocked |
| `mp2` (audio) | mp2=L:implemented/W:implemented/M:implemented/B:not_started | 1 | decoded | engine-core-only | blocked | blocked |
| `vorbis` (audio) | vorbis=L:implemented/W:implemented/M:implemented/B:not_started | 4 | decoded | engine-core-only | blocked | blocked |
| `opus` (audio) | opus=L:implemented/W:implemented/M:implemented/B:not_started | 4 | decoded | engine-core-only | blocked | blocked |
| `aac` (audio) | aac-lc=L:implemented/W:implemented/M:implemented/B:not_started | 23 | decoded-damaged | engine-core-only | blocked | blocked |
| `aac-lc` (audio) | aac-lc=L:implemented/W:implemented/M:implemented/B:not_started | 5 | no-decoder | engine-core-only | blocked | blocked |
| `alac` (audio) | alac=L:implemented/W:implemented/M:implemented/B:not_started | 2 | no-decoder | engine-core-only | blocked | blocked |
| `adpcm_` (audio) | adpcm=L:implemented/W:implemented/M:implemented/B:not_started | 2 | decoded | engine-core-only | blocked | blocked |
| `av1` (video) | av1=L:implemented/W:implemented/M:implemented/B:not_started | 2 | decoded | engine-core-only | blocked | blocked |
| `av01` (video) | av1=L:implemented/W:implemented/M:implemented/B:not_started | 0 | not-covered | engine-core-only | blocked | blocked |
| `vp9` (video) | vp9=L:implemented/W:implemented/M:implemented/B:not_started | 3 | no-decoder | engine-core-only | blocked | blocked |
| `dnxhd` (video) | prores-dnxhd=L:partial/W:partial/M:partial/B:not_started | 0 | not-covered | engine-core-only | blocked | blocked |
| `h264` (video) | h264=L:partial/W:partial/M:partial/B:not_started | 12 | no-decoder | engine-core-only | blocked | blocked |
| `hevc` (video) | hevc=L:partial/W:partial/M:partial/B:not_started | 2 | no-decoder | engine-core-only | blocked | blocked |
| `mpeg4` (video) | mpeg4-asp=L:blocked/W:blocked/M:blocked/B:blocked | 0 | not-covered | engine-core-only | blocked | blocked |
| `mpeg2video` (video) | mpeg1-2=L:blocked/W:blocked/M:blocked/B:blocked | 1 | no-decoder | engine-core-only | blocked | blocked |
| `wmv2` (video) | wmv-vc1=L:not_started/W:not_started/M:blocked/B:unsupported | 0 | not-covered | engine-core-only | blocked | blocked |
| `vp8` (video) | vp8=L:blocked/W:blocked/M:blocked/B:blocked | 1 | no-decoder | engine-core-only | blocked | blocked |
| `theora` (video) | theora=L:blocked/W:blocked/M:blocked/B:blocked | 1 | no-decoder | engine-core-only | blocked | blocked |

Cell details (native lane):

- `pcm_`: decoded — 49 tracks decoded across 48 files (0 concealed packets); sample format u8
- `mjpeg`: decoded — 33 tracks decoded across 33 files (0 concealed packets)
- `flac`: decoded — 3 tracks decoded across 3 files (0 concealed packets); sample format s16
- `mp3`: decoded — 1 tracks decoded across 1 files (0 concealed packets); sample format f32
- `mp2`: decoded — 1 tracks decoded across 1 files (0 concealed packets); sample format f32
- `vorbis`: decoded — 4 tracks decoded across 4 files (0 concealed packets); sample format f32
- `opus`: decoded — 4 tracks decoded across 4 files (0 concealed packets); sample format f32
- `aac`: decoded-damaged — 23 tracks decoded across 23 files (142 concealed packets); sample format f32
- `aac-lc`: no-decoder — 4 tracks decoded across 5 files (0 concealed packets); sample format f32; 1 tracks REFUSED: no decoder in this build
- `alac`: no-decoder — 1 tracks decoded across 2 files (0 concealed packets); sample format s16; 1 tracks REFUSED: no decoder in this build
- `adpcm_`: decoded — 2 tracks decoded across 2 files (0 concealed packets); sample format s16
- `av1`: decoded — 2 tracks decoded across 2 files (0 concealed packets)
- `av01`: not-covered — no corpus track of this codec family
- `vp9`: no-decoder — 2 tracks decoded across 3 files (0 concealed packets); 1 tracks REFUSED: no decoder in this build
- `dnxhd`: not-covered — no corpus track of this codec family
- `h264`: no-decoder — 0 tracks decoded across 12 files (0 concealed packets); 12 tracks REFUSED: no decoder in this build
- `hevc`: no-decoder — 0 tracks decoded across 2 files (0 concealed packets); 2 tracks REFUSED: no decoder in this build
- `mpeg4`: not-covered — no corpus track of this codec family
- `mpeg2video`: no-decoder — 0 tracks decoded across 1 files (0 concealed packets); 1 tracks REFUSED: no decoder in this build
- `wmv2`: not-covered — no corpus track of this codec family
- `vp8`: no-decoder — 0 tracks decoded across 1 files (0 concealed packets); 1 tracks REFUSED: no decoder in this build
- `theora`: no-decoder — 0 tracks decoded across 1 files (0 concealed packets); 1 tracks REFUSED: no decoder in this build

## 5. The VLC 3.0.23 differential (lossless-comparable family)

| corpus file | afile fmt | lumen B | lumen md5 | vlc B | vlc md5 | source | verdict |
|---|---|---|---|---|---|---|---|
| `tests/compat/.regen-cache/0022-avi-av.avi` | float32 | 557336 | `c5f5a2856f4d…` | 556976 | `78df10d1a4cb…` | live run | f32 tolerance **FAIL** (rms 1.25e-2 > 1e-4 or peak 2.50e-2 > 1e-3, 139244 samples) |
| `tests/compat/.regen-cache/0040-avi-av.avi` | u8 | 350976 | `19615789b111…` | 350400 | `d4646917919e…` | live run | **bit-exact prefix** (tail 576 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0041-avi-av.avi` | u8 | 2112384 | `a528cd0177f3…` | 2112000 | `ed5dafa08241…` | live run | **bit-exact prefix** (tail 384 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0096-multivideo-av.avi` | s16 | 94208 | `44cec9ebf7e4…` | 93712 | `a9e08d88b494…` | live run | **bit-exact prefix** (tail 496 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0116-avi-av.avi` | u8 | 51728 | `f9091177a617…` | 51600 | `7b3260357fa2…` | live run | **bit-exact prefix** (tail 128 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0121-multivideo-av.avi` | s16 | 216096 | `36355267afc2…` | 216000 | `910fe36f8723…` | live run | **bit-exact prefix** (tail 96 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0134-avi-av.avi` | u8 | 178176 | `9d62cb4522ae…` | 177600 | `2d22ce916d4f…` | live run | **bit-exact prefix** (tail 576 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0155-avi-av.avi` | u8 | 1175880 | `e6bb46494f3c…` | 1174158 | `da214c472362…` | live run | **bit-exact prefix** (tail 1722 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0156-avi-av.avi` | u8 | 261984 | `66958319ced9…` | 261600 | `a3c8e3633e1f…` | live run | **bit-exact prefix** (tail 384 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0161-multivideo-av.avi` | s16 | 3022848 | `7d7ad50a3f50…` | 3018752 | `b96286a26cb1…` | live run | **bit-exact prefix** (tail 4096 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0177-multivideo-av.avi` | s16 | 340808 | `6ffbb0979393…` | 339568 | `4934d804d067…` | live run | **bit-exact prefix** (tail 1240 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0180-multivideo-av.avi` | s16 | 331776 | `40dd84593db9…` | 331200 | `9d74265e479c…` | live run | **bit-exact prefix** (tail 576 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0187-avi-av.avi` | u8 | 547720 | `34bcf94ce530…` | 546840 | `bc76bdf18701…` | live run | **bit-exact prefix** (tail 880 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0194-multivideo-av.avi` | s16 | 2998272 | `fbf9a9b3a986…` | 2981888 | `838928ee4447…` | live run | **bit-exact prefix** (tail 16384 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0201-multivideo-av.avi` | s16 | 1970176 | `ba0dc70bb09e…` | 1968000 | `1e35d870cd26…` | live run | **bit-exact prefix** (tail 2176 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0214-avi-av.avi` | s16 | 1144306 | `e6665f8344b9…` | 1142784 | `f968a60eb330…` | live run | **bit-exact prefix** (tail 1522 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0227-avi-av.avi` | s16 | 472032 | `79120dbdb263…` | 471040 | `e3dbede247a3…` | live run | **bit-exact prefix** (tail 992 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0233-multivideo-av.avi` | s16 | 870400 | `649d0232dba0…` | 868770 | `c8db9843ba6a…` | live run | **bit-exact prefix** (tail 1630 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0236-multivideo-av.avi` | s16 | 137216 | `2fa047d2f34c…` | 136710 | `d96aab94f910…` | live run | **bit-exact prefix** (tail 506 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0247-avi-av.avi` | float32 | 745728 | `ab5198c688a8…` | 745392 | `5fa583f66850…` | live run | f32 tolerance **FAIL** (rms 1.23e-2 > 1e-4 or peak 4.82e-2 > 1e-3, 186348 samples) |
| `tests/compat/.regen-cache/0278-avi-av.avi` | s16 | 356858 | `28a9e2652675…` | 356352 | `9763ff8dcdb5…` | live run | **bit-exact prefix** (tail 506 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0280-avi-av.avi` | float32 | 935976 | `18e044f04e63…` | 934760 | `598126d7a114…` | live run | f32 tolerance **FAIL** (rms 8.68e-3 > 1e-4 or peak 3.52e-2 > 1e-3, 233690 samples) |
| `tests/compat/.regen-cache/0297-avi-av.avi` | float32 | 3520224 | `0f368aaa880d…` | 3518528 | `1796324a7db6…` | live run | f32 tolerance **FAIL** (rms 8.22e-5 > 1e-4 or peak 4.63e-2 > 1e-3, 879632 samples) |
| `tests/compat/.regen-cache/0305-avi-av.avi` | float32 | 885440 | `06a68cd135e2…` | 884656 | `c389e0160fc6…` | live run | f32 tolerance **FAIL** (rms 1.22e-1 > 1e-4 or peak 4.94e-1 > 1e-3, 221164 samples) |
| `tests/compat/.regen-cache/0307-multivideo-av.avi` | s16 | 450560 | `e2bce5d5744e…` | 448000 | `d785c2f38598…` | live run | **bit-exact prefix** (tail 2560 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0308-avi-av.avi` | s16 | 4096000 | `01269e9f433b…` | 4089600 | `a436890a0b7b…` | live run | **bit-exact prefix** (tail 6400 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0318-avi-av.avi` | s16 | 1331712 | `0385e591ec22…` | 1331200 | `fd2e674edf84…` | live run | **bit-exact prefix** (tail 512 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0324-avi-av.avi` | u8 | 822908 | `1767da988df2…` | 820260 | `2c669806afcb…` | live run | **bit-exact prefix** (tail 2648 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0330-avi-av.avi` | float32 | 410880 | `bb76a78b3c77…` | 409520 | `c0e894718e50…` | live run | f32 tolerance **FAIL** (rms 6.43e-6 > 1e-4 or peak 1.20e-3 > 1e-3, 102380 samples) |
| `tests/compat/.regen-cache/0335-multivideo-av.avi` | s16 | 97216 | `91f7a4e28881…` | 96800 | `8d875f66915d…` | live run | **bit-exact prefix** (tail 416 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0336-multivideo-av.avi` | s16 | 425984 | `ca97c302d2e1…` | 425600 | `c06837c134cd…` | live run | **bit-exact prefix** (tail 384 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0342-avi-av.avi` | s16 | 1846464 | `0d1d39cbb040…` | 1843200 | `ce235467c545…` | live run | **bit-exact prefix** (tail 3264 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0367-multivideo-av.avi` | s16 | 4030464 | `b51e70202da7…` | 4018176 | `af367bbb6901…` | live run | **bit-exact prefix** (tail 12288 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0386-avi-av.avi` | s16 | 187292 | `37e3ad34ab0d…` | 186368 | `6cfd15ca4347…` | live run | **bit-exact prefix** (tail 924 B unflushed on VLC's side) |
| `tests/compat/.regen-cache/0394-avi-av.avi` | s16 | 1092096 | `14f6510ff52b…` | 1084800 | `c9b442b6b561…` | live run | **bit-exact prefix** (tail 7296 B unflushed on VLC's side) |

Committed-record cross-checks (2026-11-09 run):

- tests/compat/.regen-cache/0022-avi-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0040-avi-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0041-avi-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0096-multivideo-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0116-avi-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0121-multivideo-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0134-avi-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0155-avi-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0156-avi-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0161-multivideo-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0177-multivideo-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0180-multivideo-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0187-avi-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0194-multivideo-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0201-multivideo-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0214-avi-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0227-avi-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0233-multivideo-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0236-multivideo-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0247-avi-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0278-avi-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0280-avi-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0297-avi-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0305-avi-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0307-multivideo-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0308-avi-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0318-avi-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0324-avi-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0330-avi-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0335-multivideo-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0336-multivideo-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0342-avi-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0367-multivideo-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes
- tests/compat/.regen-cache/0386-avi-av.avi: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same realization) — the record's vlc_md5 is applicable
- tests/compat/.regen-cache/0394-avi-av.avi: fresh realization (random source or encoder drift) — the committed record's md5s do not apply to these bytes

## 6. Gap triage (8.B2) — the transitions outbox

5 transition proposals (applied by `cargo xtask compat-matrix apply-transitions --outbox docs/corpus/compat-matrix-transitions-20261110.json` — the parity tool re-checks each `from` status against the live matrix before writing):

- **container-asf-wmv** (linux): `implemented` → `partial` — compat-matrix run (linux-native lane): the open path REFUSES the corpus files of probe format `asf` — unsupported (sniffed 30 26 b2 75 8e 66 cf 11 a6 d9 00 aa 00 62 ce 6c) (probe.no-demuxer): probe hints: extension=wmv, size=10329. The container claim cannot be exercised end-to-end through the player's open path.
  - next step: wire the missing probe signature / container-registry arm for `asf` (the ivf/mpegps open-path wiring precedent), then re-run the matrix
  - evidence: docs/corpus/compat-matrix-20261110.md (this run's matrix table + raw rows)
- **codec-audio-aac-lc** (linux): `implemented` → `partial` — compat-matrix run (linux-native lane): 142 concealed (damaged) packets on VALID corpus files of codec `aac` — the decode is not clean
  - next step: triage the concealment (bounded fix, no correctness trade); add the corpus file as a regression fixture; re-run
  - evidence: docs/corpus/compat-matrix-20261110.md (this run's matrix table + raw rows)
- **codec-audio-aac-lc** (linux): `implemented` → `partial` — compat-matrix run (linux-native lane): 1 corpus tracks of codec `aac-lc` REFUSED (no decoder in this build) vs 4 decoded
  - next step: land the missing decoder lane (bounded fix) or split the row's claimed descriptor set; re-run the matrix
  - evidence: docs/corpus/compat-matrix-20261110.md (this run's matrix table + raw rows)
- **codec-audio-alac** (linux): `implemented` → `partial` — compat-matrix run (linux-native lane): 1 corpus tracks of codec `alac` REFUSED (no decoder in this build) vs 1 decoded
  - next step: land the missing decoder lane (bounded fix) or split the row's claimed descriptor set; re-run the matrix
  - evidence: docs/corpus/compat-matrix-20261110.md (this run's matrix table + raw rows)
- **codec-video-vp9** (linux): `implemented` → `partial` — compat-matrix run (linux-native lane): 1 corpus tracks of codec `vp9` REFUSED (no decoder in this build) vs 2 decoded
  - next step: land the missing decoder lane (bounded fix) or split the row's claimed descriptor set; re-run the matrix
  - evidence: docs/corpus/compat-matrix-20261110.md (this run's matrix table + raw rows)

## 7. Regenerable-corpus outcomes

- `0022-avi-av.avi`: present, sha256 verified
- `0040-avi-av.avi`: present (prior realization, sha256 841e7667b79bfc54289dee4f5df2e5812669d66fff3dac64b4584f2f1a85ba0d)
- `0041-avi-av.avi`: present (prior realization, sha256 1869bae76b4baf9afe82c647bd03536c28928700c2807df6680a3d10f04c5be1)
- `0096-multivideo-av.avi`: present, sha256 verified
- `0116-avi-av.avi`: present (prior realization, sha256 15d1ff12e37e19f6bee7221277b69256d3ca1ea3d30e9635ca03e546d4385cfa)
- `0121-multivideo-av.avi`: present, sha256 verified
- `0134-avi-av.avi`: present (prior realization, sha256 affc3a6a75c5d5d23302b8534d6d9b31a1ffe845bfcabe8c86f01ba92c5c06d4)
- `0155-avi-av.avi`: present, sha256 verified
- `0156-avi-av.avi`: present, sha256 verified
- `0161-multivideo-av.avi`: present, sha256 verified
- `0177-multivideo-av.avi`: present (prior realization, sha256 8439b529da3aea8188eba59f79a998807bf0dd7c4469b5be996c54bb4260f0ef)
- `0180-multivideo-av.avi`: present (prior realization, sha256 64325efb33d541348c83ab8f24d015b391b1c35a14fc942a085e1f35b8d521b3)
- `0187-avi-av.avi`: present, sha256 verified
- `0194-multivideo-av.avi`: present (prior realization, sha256 835a4293c0bb0339a8d356f22c0ad1bdfda3f9010784a2ed9a90cc3e533be38b)
- `0201-multivideo-av.avi`: present (prior realization, sha256 de8ba6781d1b02892251d923dad083bb6f07cac192e6a60c608871b468da5aa4)
- `0214-avi-av.avi`: present (prior realization, sha256 4dc672ec9b884111e8ecbb79a051c0ce8040c571ab7e82639ced80650ec649d2)
- `0227-avi-av.avi`: present (prior realization, sha256 5e73f00cc70402262d4bf48428312ca4fa2c9edada66b5bb693ac913118c4514)
- `0233-multivideo-av.avi`: present, sha256 verified
- `0236-multivideo-av.avi`: present, sha256 verified
- `0247-avi-av.avi`: present (prior realization, sha256 4dbc68cf248ec97c624aa2fe9d8002d9c8b719788932a5641df329db22781683)
- `0278-avi-av.avi`: present, sha256 verified
- `0280-avi-av.avi`: present (prior realization, sha256 18c5bdf11bb05193729c25154acadc0a422e82bdc7dc24c45bdb96cc973a09dc)
- `0297-avi-av.avi`: present, sha256 verified
- `0305-avi-av.avi`: present (prior realization, sha256 1ab47065185b54b8641a8ca00c25ddaf3f1436a7e89887644e2101dfc14281b0)
- `0307-multivideo-av.avi`: present, sha256 verified
- `0308-avi-av.avi`: present, sha256 verified
- `0318-avi-av.avi`: present (prior realization, sha256 d50f4ed6d8c1b3e31ab9457eb1186ad8fbaade5665bc7a289704b7c36298eee8)
- `0324-avi-av.avi`: present, sha256 verified
- `0330-avi-av.avi`: present, sha256 verified
- `0335-multivideo-av.avi`: present (prior realization, sha256 bb8b3662a683f619e647ace896b525759f564076c1e9cbd229140ce1f34d968a)
- `0336-multivideo-av.avi`: present (prior realization, sha256 55381045f6c416ac1b3990cbc2edff968a11a8d873aa153dc5791d56b61a9b9d)
- `0342-avi-av.avi`: present (prior realization, sha256 b44161d196c0e6413991783c557a89d4ed7277492a31a88a8f45898c9afa2785)
- `0367-multivideo-av.avi`: present (prior realization, sha256 5e2f307aa02820b46b4b6d38713af9b8da81ddbf956110890eb09764868a56e6)
- `0386-avi-av.avi`: present, sha256 verified
- `0394-avi-av.avi`: present (prior realization, sha256 01d00f53602ef74e8a376aedc5194bd55a7874fe9d0e385131002a6bf6eb8bf3)
- `0360-audio-only.mp3`: present, sha256 verified
- `0051-audio-only.mp2`: present, sha256 verified
- `0313-audio-only.aac`: present, sha256 verified
- `0371-muxed-av.ts`: present (prior realization, sha256 bf1bb72fe10b65e2e7107c4d44b0edf6aeb51a221b3408417136c76e1dd1690c)
- `0269-vfr-av.mkv`: present (prior realization, sha256 34a6ed2449490306461d9a74a99091f63ddc8bc57e631d5e570a12db45a55db9)
- `0105-mkv-video.mkv`: present (prior realization, sha256 c7bb2f9010a9b02036260a2113e62b647d0e72d129e4733d5120539be3c55e56)
- `0009-audio-only.mka`: present (prior realization, sha256 9999d86669d4409f9e68c68192746b9c1afca3813506a3faa658836643c13765)
- `0396-audio-only.mov`: present, sha256 verified
- `0210-audio-only.ogg`: present (prior realization, sha256 311e4c9f1304234a03f1a722aac647e2dec198bdb036e19332e03734ad618b00)
- `0115-audio-only.ogg`: present (prior realization, sha256 ef9a74b6694dc44f8f8a03198ef39ac7097612d25a3103131c5114fe9b2b9fbc)
- `0220-audio-only.wav`: present, sha256 verified
- `0331-audio-only.wav`: present (prior realization, sha256 606e00924f6eb4ae5d3bd72f92494078cda1e5f6367917555b2c0c983cb07923)
- `0370-audio-only.wav`: present, sha256 verified
- `0241-audio-only.wav`: present (prior realization, sha256 dc9a82ba11d45c02a313a18ecf24e87396c21fe8bacc1cbe4bcef0e56d4aff4c)
- `0316-audio-only.flac`: present, sha256 verified
- `0126-raw-video.h264`: present, sha256 verified
- `0099-raw-video.265`: present, sha256 verified
- `0132-meta-bomb-av.mkv`: present (prior realization, sha256 bfe4e0ae569379f7812fad68486051fedf46db2c6859681999d35ec35000d195)
- `0136-audio-only.mp4`: present, sha256 verified
- `0028-audio-only.webm`: present (prior realization, sha256 b0e2dab12d183d3ac97b4838a1c7f93f318567ac5443070befc0ba086fa714fd)
- `0065-webm-video.webm`: present (prior realization, sha256 9b700bd16591d1188a0b951d6d80035b19b52ada69d4307f6723e85334a85997)
- `0089-ogv-video.ogv`: present (prior realization, sha256 96f58b808f8f4ef89263dff3d429fe9b2c155c04cfd52ea780982742103d68fe)
- `0385-dynres-video.mkv`: **FAILED**: ffmpeg step failed for 0385-dynres-video.mkv (argvs: 1 recorded in the manifest; the run's raw-rows file carries the step record)
- `0257-avi-video.avi`: present, sha256 verified
- `0146-avi-video.avi`: present, sha256 verified
- `0190-audio-only.m4a`: present, sha256 verified
- `0007-vfr-av.mov`: present (prior realization, sha256 073b2eb1235ab44a628ee28c9b65e939488abcaa51ceddd5bf29f676c46f88c8)
- `0068-fmp4-video.mp4`: present (prior realization, sha256 ec7b993321113b6b15c7f06df8524d563a0a4c187e1913bfc6b2ce1e4a46a7ce)
- `0267-audio-only.mkv`: present (prior realization, sha256 43979413747374ffd8ee2a7c7d505e42e5ce5a91dd19d9df196cb1cdaa9c9b12)
- `0344-muxed-av.mp4`: present (prior realization, sha256 b0b615b9314961a18bec2da64b396c3e532e5668b35dddb75ce4ecb58520fcca)
- `mpegps-dash.mpg`: present, sha256 verified
- `asf-head.wmv`: present, sha256 verified

## 8. Honest limits of this run

- The native-decode lane (`linux-native`) is the only lane that decoded media in this run. Windows/macOS cells are blocked-lane citations (3-OS runner machines + CI billing are user-owned blockers — docs/plans/phase8-cards.md §1); NO result is inferred for platforms that did not run.
- The web/wasm lane evidence is the recorded build/test steps: the wasm32 module is real (engine core: probe → demux → engine), but it ships DECODER-LESS on wasm32 — the codecs crate does not build for the target (rav1d 1.1.0 asm-gated, measured 2026-11-10; apps/web/Cargo.toml target gate; apps/web/src/capabilities.rs reports the split honestly). No decode is claimed or exercised on the web lane.
- Oracle comparisons cover the lossless-comparable family only (PCM-in-AVI, the documented first vertical): integer PCM via the MD5-prefix method (VLC's afile dump is a bit-exact prefix; the shortfall is the unflushed tail), pcm_f32le via the ratified RMS ≤ 1e-4 / peak ≤ 1e-3 tolerance path (docs/corpus/vlc-f32-tolerance-20261109.json). Lossy codecs are not byte-compared — decoder implementations legitimately differ; their rows carry decode-count evidence only.
- Transitions are PROPOSALS in the outbox (5); only `cargo xtask compat-matrix apply-transitions` writes the matrix, after re-checking each `from` status against the live file. Nothing here self-verifies: no transition proposes `verified` (CI billing is blocked; MP §15).

