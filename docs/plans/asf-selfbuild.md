# ASF / WMV / WMA Self-Build — Investigation & Unit Split (6.ASF-I)

Status: COMPLETE (investigation card — zero code changes; see §8).
Branch: `wip/p6-asf-i`. Card: 6.ASF-I (Phase 6, W3; the dead first dispatch did no work — this
is the fresh restart, all evidence re-measured this session).
Method: every claim below is either **[VERIFIED]** live in this session (commands + numbers
recorded) or explicitly labeled **[FORMAT KNOWLEDGE — verify in ASF-a/b]** against the spec
when that unit opens. file:line cites are from `wip/p6-asf-i` at base `565042d` (main,
lead(6.W2-STATUS-2)).

**The one-paragraph verdict.** The corpus has **zero** asf/wma/wmv files, but ffmpeg 7.1.5
generates the whole family cleanly (11 fixtures produced and ffprobe-verified this session,
§1), the container is a GUID+size object tree whose top level and every header object I
byte-verified against those fixtures (§2), and the existing `demux` crate's shape
(header → tracks → packets → index, typed `demux.<fmt>.*` errors, `SeekSupport`,
live-ffprobe differential tests) maps onto it with no new seams: ASF-a/b/c/d decompose into
header tree / data packets / index seeking / differentials, exactly as 6.WAVE-PLAN scheduled.
The value case is stronger than 5.K3-I recorded: **PCM, ADPCM-MS and MJPEG ride ASF fine in
ffmpeg** (§1.3), so the v1 wiring gets real decodable lanes through the *existing* PCM-facts,
`adpcm` WAVEFORMATEX-`extra_data` and `mjpeg` decoder paths — not just metadata plus typed
refusals. The two live-verified hazards that shape the split: ffmpeg's **play-duration lie**
and **zero-filled Simple Index entries** (§2.3), and **broadcast-mode files that ffprobe
itself refuses** (§2.5).

---

## 1. Corpus census + fixture generation (all [VERIFIED] this session)

### 1.1 Census: zero ASF-family lanes

`ls /tmp/my-project/corpus-v4/media | wc -l` → **143 files**; extension census
(`ls | sed 's/.*\.//' | sort | uniq -c`): avi 44, mkv 31, mp4 24, wav 12, ts 5, mov 5,
ogg 3, flac 3, mp3 2, mp2 2, mka 2, m4a 2, aac 2, webm 1, ogv 1, h264 1, flv 1, ac3 1,
`265` 1 — sums to 143. `ls | rg -i "\.(asf|wma|wmv)$"` → **no matches**; case-insensitive
name scan for asf/wma/wmv substrings → no matches. This confirms 5.K3-I's note
(docs/formats/container-asf.md:103, checked 2026-10-24) and open-path-wiring.md §5's census.
Every ASF test lane must therefore be **generated** (ffmpeg) or **acquired** (C-V6 corpus
item, corpus-plan.md `asf-wmv` row) — §4 carries the per-unit fixture strategy.

### 1.2 Fixture generation: the exact commands (reproduce verbatim)

ffmpeg/ffprobe version (both): `7.1.5-0+deb13u1`. The `asf` muxer exists
(`ffmpeg -muxers | rg asf`: `asf`, plus `asf_stream` = broadcast variant, plus the
unrelated Argonaut `argo_asf` — do not confuse). Muxer options: extensions `asf,wmv,wma`,
default video codec msmpeg4v3, default audio wmav2, `-packet_size` 100..65536 default 3200.

Working directory `/tmp/asf-i-fix` (not committed — investigation only). All commands
need `-y`: without it ffmpeg stops on an **interactive overwrite prompt** that hangs the
non-interactive shell (measured: 3 hung attempts before the flag; with `-y` each generation
completes in <1 s). wmav2 also refuses bitrates below 24000 (`bitrate too low: got 8000,
need 24000 or higher` — measured).

```sh
# audio-only (WMA family)
ffmpeg -f lavfi -i "sine=frequency=440:duration=1:sample_rate=8000"  -ac 1 -c:a wmav2 -b:a 32k wma_wmav2_mono8k.wma
ffmpeg -f lavfi -i "sine=frequency=440:duration=2:sample_rate=44100" -ac 2 -c:a wmav1 -b:a 32k wma_wmav1_st44k.wma
# video-only (WMV family)
ffmpeg -f lavfi -i "testsrc=size=64x48:rate=10:duration=1" -c:v wmv2        -b:v 100k wmv_wmv2_64x48.wmv
ffmpeg -f lavfi -i "testsrc=size=64x48:rate=10:duration=1" -c:v wmv1        -b:v 100k wmv_wmv1_64x48.wmv
ffmpeg -f lavfi -i "testsrc=size=64x48:rate=10:duration=1" -c:v msmpeg4v3   -b:v 100k asf_msmpeg4v3.asf
ffmpeg -f lavfi -i "testsrc=size=64x48:rate=10:duration=1" -c:v mjpeg       -b:v 200k asf_mjpeg.asf
# A/V muxed (the 5.K3-I probe-fixture shape)
ffmpeg -f lavfi -i "testsrc=size=64x48:rate=10:duration=1" -f lavfi -i "sine=frequency=440:duration=1:sample_rate=8000" \
       -ac 1 -c:v wmv2 -b:v 100k -c:a wmav2 -b:a 32k wmv_wmv2_wma_av.wmv
# muxer variants
… same inputs … -c:v wmv2 -b:v 100k -c:a wmav2 -b:a 32k -packet_size 8000 asf_pkt8000.wmv
… same inputs … -c:v wmv2 -b:v 100k -c:a wmav2 -b:a 32k -f asf_stream      asf_stream_broadcast.wmv
# decodable-payload probes (the §1.3 value case)
ffmpeg -f lavfi -i "sine=frequency=440:duration=1:sample_rate=8000" -ac 1 -c:a pcm_s16le asf_pcm_s16le.wma
ffmpeg -f lavfi -i "sine=frequency=440:duration=1:sample_rate=8000" -ac 1 -c:a adpcm_ms  -b:a 32k asf_adpcm_ms.wma
```

ffprobe summaries (`ffprobe -show_entries format=format_name,duration -show_entries
stream=codec_name,codec_type,…`): **every file above reports `format_name=asf`**; codecs
wmav2 / wmav1 / wmv2 / wmv1 / msmpeg4v3 / mjpeg / pcm_s16le / adpcm_ms respectively;
`wmv_wmv2_wma_av.wmv` = 2 streams (wmv2 64×48 + wmav2 8 kHz mono), duration 1.128000 s —
the same shape as the Phase-0 probe fixture `asf_wmv2_wmav2` (docs/baselines/raw/linux/
17-format-probes.txt:339–354, cited by container-asf.md:54). The broadcast variant is the
one ffprobe **rejects** (§2.5). Fixture sizes 6.9–26.3 KB; sha256 of all 11 recorded in
§4.4's table so the build units can pin byte-exact regeneration.

**Oracle note:** `ffprobe -show_packets` over `wmv_wmv2_wma_av.wmv` yields 26 payloads with
`pos` values 807 / 4007 / 7207 — i.e. 26 demuxed payloads across **3 container packets of
exactly 3200 bytes** each (multi-payload packets, §2.4). This is the differential oracle
shape ASF-b/d reuse (the `mkv_corpus.rs` live-ffprobe pattern, §3).

### 1.3 The value-case upgrade (measured, changes the v1 story)

5.K3-I recorded the ASF container as "container alone buys little" (docs/formats/
container-asf.md:118–119) — metadata + hw-WMV + typed-unsupported WMA. This session
measured a stronger fact: the `asf` muxer carries **payloads we already decode**:

| Payload | ffprobe codec | Our registry surface ([VERIFIED] cites) | Day-one playable? |
|---|---|---|---|
| pcm_s16le | `pcm_s16le`, tag `[1][0][0][0]` | `pcm_*` → `PcmDecoder::with_format(channels, rate)` from host facts (open-path-wiring.md §3; pipeline_host.rs:230–253) | **yes** |
| adpcm_ms | `adpcm_ms`, tag `[2][0][0][0]` | `adpcm_ms` → `AdpcmDecoder::configure_wav` parses the **WAVEFORMATEX blob** from `extra_data` (crates/codecs/src/audio/adpcm/mod.rs:302–358, 510) | **yes** (the blob is the SP type-specific data, §2.3) |
| mjpeg | `mjpeg`, fourcc `MJPG` | `mjpeg` → `MjpegDecoder::new()` (pipeline_host.rs:174–178) | **yes** |
| wmav1/wmav2/wmv1/wmv2/msmpeg4v3 | … | not in the landed set (`mjpeg, pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1` — apps/cli/src/pipeline_host.rs:308–315) | demux + typed `cli.codec_unavailable` |

So an MJPEG+PCM (or MJPEG+ADPCM) `.asf` is a full first-vertical twin of the AVI
D-018 lane, and the classic `.wma`/`.wmv` files still land in the honest typed-refusal
path per track (audio-wma.md's blocked-honest verdict, unchanged).

---

## 2. ASF container structure — byte-verified where possible

Everything in §2.1–§2.4 was measured on the §1.2 fixtures with `od`/python this session
(raw hex walks); **[FORMAT KNOWLEDGE]** marks the parts that come from the public ASF
specification's structure and still need the spec text in hand (ASF-a's first gate, §4.1)
before we encode them as parser truth.

### 2.1 Top-level object tree

ASF is a sequence of objects: **GUID (16 bytes) + size (8-byte LE QWORD, includes the
24-byte header) + payload**, with the size-0 and 0xFFFFFFFFFFFFFFFF sentinels declared
[FORMAT KNOWLEDGE]. Byte-verified walks of three fixtures:

**`wmv_wmv2_wma_av.wmv` (10,499 B):**

| Offset | Object | GUID (canonical) | Size |
|---|---|---|---|
| 0 | **Header Object** | `75B22630-668E-11CF-A6D9-00AA0062CE6C` | 757 |
| 757 | **Data Object** | `75B22636-668E-11CF-A6D9-00AA0062CE6C` | 9,650 |
| 10,407 | **Simple Index** | `33000890-E5B1-11CF-89F4-00A0C90349CB` | 92 |

Header children (6 declared in the header's count field at +24; walk matched exactly):
File Properties `8CABDCA1-A947-11CF-8EE4-00C00C205365` (104 B) @30, Header Extension
`5FBF03B5-A92E-11CF-8EE3-00C00C205365` (156 B) @134, Extended Content Description
`D2D0A440-E307-11D2-97F0-00A0C95EA850` (98 B) @290, **Stream Properties**
`B7DC0791-A9B7-11CF-8EE6-00C00C205365` ×2 (133 B video @388, 114 B audio @521), Codec
List `86D15240-311D-11D0-A3A4-00A0C90348F6` (122 B) @635.

**`wma_wmav2_mono8k.wma` (6,942 B):** Header 492 (5 children: FPO 104, Header Extension
46, Extended Content Description 98, Stream Properties 114, Codec List 100) → Data 6,450
→ **no trailing object: audio-only files carry NO Simple Index** [VERIFIED] — the index is
per-video-stream [FORMAT KNOWLEDGE]. This is the ASF-c policy input (§4.3).

GUID storage is little-endian for the first three fields, big-endian for the last 8 bytes:
the Header GUID appears on disk as `30 26 b2 75 8e 66 cf 11 a6 d9 00 aa 00 62 ce 6c`
[VERIFIED, `od`].

### 2.2 File Properties Object (the facts source)

Payload 80 bytes [VERIFIED sizes]. Decoded fields (offsets from payload start), with the
caveats that matter:

| Field (off) | `wmv_wmv2_wma_av.wmv` value | Note |
|---|---|---|
| File ID (0, 16) | all zeros | ffmpeg writes zeros — **do not use as an identity oracle** |
| File Size (16, 8) | 10,499 = actual | ✓ |
| Creation Date (24, 8) | 0x019DB1DED53E8000 | fixed constant in ffmpeg output |
| Data Packets Count (32, …) | 3 | **width ambiguity: bytes at +32..+40 are `03 00 00 00 00 00 00 00`** — reads as QWORD 3 or as DWORD 3 + 4 zero pad bytes. Arithmetic (§2.4) confirms the value is 3 either way; ASF-a must pin the spec field width (see §8) |
| Play Duration (40, 8) | 41,734,144 (100-ns) = **4.173 s** | ffprobe says duration 1.128 s — ffmpeg's asf muxer pads play duration (known writer behavior) ⇒ **the duration fact must NOT trust FPO Play Duration**; ffprobe-consistent duration comes from Send Duration / packet timestamps [VERIFIED values; interpretation pinned in ASF-a] |
| Send Duration (48, 8) | 10,647,936 = 1.065 s | close to real |
| Preroll (56, 8) | **3,100** | the same constant appears as the first payload's stored timestamp (§2.4) — payload stamps are preroll-offset; ffprobe's first audio pts is 0.000 ⇒ preroll is **subtracted**, and the units behave as **milliseconds** [VERIFIED value + behavior; unit wording to pin vs spec] |
| Flags (64, 4) | 0x00000002 | bit 1 = seekable; bit 0 = broadcast [FORMAT KNOWLEDGE for bit semantics; values cross-checked in §2.5] |
| Min/Max Data Packet Size (68/72, 4) | 3,200 / 3,200 | matches the measured 3,200-byte packets exactly ✓ |
| Maximum Bitrate (76, 4) | 132,256 | ≈ 100k video + 32k audio ✓ sanity |

### 2.3 Stream Properties Object (the track table source)

Payload layout [VERIFIED by exact arithmetic across 6 fixtures — 2 audio + 4 video, sizes
24+58+TSD+EC match to the byte]:

```
+0   Stream Type GUID (16)      audio F8699E40-5B4D-11CF-A8FD-00805F5C442B
                              video BC19EFC0-5B4D-11CF-A8FD-00805F5C442B   [VERIFIED bytes]
+16  Error Correction Type (16)  audio-spread BFC3CD50-618F-11CF-8BB2-00AAB4E220 observed
+32  Time Offset QWORD (8)
+40  Type-Specific Data Length DWORD (4)
+44  Error Correction Data Length DWORD (4)
+48  Stream Number WORD (2)     1 (video) / 2 (audio) — the TrackId source
+50  Stream Language Index WORD (2)
+52  two bytes observed zero — spec name (stream flags / encryption bit?) [FORMAT KNOWLEDGE,
                              pin in ASF-a; the content-encryption refusal keys off it]
+54  Type-Specific Data …       WAVEFORMATEX (audio) / VIDEOINFOHEADER-family (video)
+54+TSD  Error Correction Data  (8 bytes observed on audio streams)
```

- Audio TSD = **WAVEFORMATEX**, byte-verified on three files: `wma_wmav2_mono8k.wma`
  tag 0x0161 (wmav2), 1 ch, 8,000 Hz, avg 4,000 B/s, **block align 256** (= the 256-byte
  audio payloads ffprobe reports ✓), 16 bits, cbSize 10 + 10 extra bytes;
  `asf_pcm_s16le.wma` tag 0x0001, align 2, bits 16; `asf_adpcm_ms.wma` tag 0x0002,
  align 1024, bits 4, **cbSize 32 (the MS coefficient tail)** — exactly the blob
  `AdpcmDecoder::parse_waveformat` consumes (adpcm/mod.rs:302–326).
- Video TSD: 51–55 bytes containing `biWidth=64, biHeight=48, biPlanes=1, biBitCount=24,
  biCompression='WMV2'/'WMV1'/'MP43'/'MJPG'` (fourccs found at a fixed offset in each
  file's first SP [VERIFIED]) and `biSizeImage=9,216=64×48×3` ✓. The full
  VIDEOINFOHEADER field grammar (rcSource/rcTarget/dwBitRate/AvgTimePerFrame placement)
  did **not** match my from-memory model — the observed 55-byte TSD is a short form;
  ASF-a must decode it against the spec, not against recollection (§8).

Other header objects, verified present with sizes: **Codec List** = reserved GUID + count
+ per-entry {type(2), UTF-16 length-prefixed name/description, length-prefixed info} —
byte-read "Windows Media Audio V8" strings in it; **Extended Content Description** =
count(2) + {name_len, value_len, type, UTF-16 name, value} — observed carrying
`WM/EncodingSettings = "Lavf61.7.103"` (ffmpeg 7.1.5's libavformat version string —
a generation provenance marker); **Header Extension** = reserved GUID
`ABD3D211-A9BA-11CF-8EE6-00C00C205365` + reserved(2)=6 + data size(4)=110 + children
(Extended Stream Properties with language/name/payload-extension data — observed
`AspectRatioX`/`AspectRatioY` UTF-16 names; grammar = ASF-a scope). The [FORMAT KNOWLEDGE]
object set beyond these (Content Description, Content Branding, Language List, Mutual
Exclusion, Stream Bitrate Properties, Index/Media-Object-Index/Timecode-Index objects,
Padding, Script streams) is real but **not exercised by ffmpeg output** — v1 policy:
known-GUID → typed skip/ignore-list; unknown-GUID → skip-by-size (bounded) exactly like
the AVI junk-list walk.

### 2.4 Data Object and packets

Data Object header [VERIFIED]: GUID + size + File ID (16 zeros) + **Total Data Packets as
8 bytes = 3** + **reserved 0x0101** (the documented constant [FORMAT KNOWLEDGE], bytes
verified). Packet area then starts (offset 807 in the av fixture; ffprobe's first payload
`pos` is the same 807 ✓). Measured packet arithmetic: packets at 807 / 4,007 / 7,207, each
**exactly 3,200 bytes** (the FPO min=max packet size ✓), 3×3,200 = 9,600 = 10,407−807 ✓
closing the Data Object exactly. First packet's leading bytes (recorded for ASF-b):
`82 00 00 09 5d ec 00 00 00 00 a4 00 85 02 01 00 00 00 00 08 00 01 00 00 1c 0c 00 00 00 01 …`
— the EC-flags/property-flags/length/padding/sequence/send-time/duration/multi-payload bit
grammar is [FORMAT KNOWLEDGE — verify in ASF-b]; the constant `1c 0c` (3,100 = preroll)
appears in the first payload header and ffprobe reports that payload at pts 0.000 ⇒
**payload timestamps are stored preroll-offset in milliseconds** [VERIFIED behavior, §2.2].
Multi-payload framing is real: ffprobe shows 26 payloads over 3 packets (up to ~17 payloads
in one 3,200-byte packet), audio payloads exactly block-align-sized (256 B) and video
payloads whole frames (1,753 B first frame) ⇒ **no fragmentation observed at these sizes**;
large frames fragment across packets as media-object continuations [FORMAT KNOWLEDGE —
the ASF-b reassembly risk]. `-packet_size 8000` variant verified to generate a valid file
with 8,000-byte packets (16,899 B file, same content) — the packet-size fields must be
honored, not assumed 3,200.

### 2.5 Broadcast mode (the live-verified complication)

`-f asf_stream` output [VERIFIED]: (a) carries a **12-byte non-ASF prefix** before the
Header GUID (`24 48 2f 03 …` — the MMS `$H` framing family [FORMAT KNOWLEDGE]), so a
content probe anchored at offset 0 **cannot** match it; (b) FPO flags = 0x3 (broadcast bit
set), File Size = 0, packet count = 0, play duration = 31,000,000 (just the 3.1 s
preroll), Data Object size field = **50** (placeholder) — sizes are unknown by design;
(c) **no Simple Index**; (d) **ffprobe 7.1.5 refuses the file even with `-f asf`**
("Invalid data found when processing input" [VERIFIED twice]).
⇒ v1 policy: broadcast files are not parseable targets; the demuxer gates on FPO flags
bit 0 (typed `demux.asf.broadcast-unsupported`), and the probe signature naturally
misses the `$H`-prefixed form (honest probe-miss refusal). Recorded as an honest v1
exclusion, matching container-asf.md:31–33.

---

## 3. The demux crate we build into (all [VERIFIED] cites)

`crates/demux/src/lib.rs:39–50` — 11 container modules today; house rules at lib.rs:31–37
(std-only C5, no unsafe, no unwrap/expect/panic, MP §12 bounded memory). Module shape
precedent (mkv/mod.rs:1–30): layered natives — `ebml` (primitives) / `header` (tracks) /
`cluster` (packets) / `cues` (index) / `error` (typed constructors). ASF maps 1:1:
`guid`+`header` / `data` / `index` / `error`.

- **Trait surface**: `plugin_api::Demuxer` start/next_event/seek + `seekability`/`reanchor`
  (crates/plugin-api/src/demux.rs:52–121); `SeekSupport::{Fast{indexed}, SlowLinear,
  None}` at demux.rs:129–145. mkv's impl: `new` mkv/mod.rs:149, `track_entries` :190,
  `cue_index` :196, `start` :717, `next_event` :725, `seek` :811, `seekability` :823
  (cues → Fast), `reanchor` :834. ts refuses (`SeekSupport::None`, open-path-wiring.md
  §2) — the two ends of the seekability spectrum ASF-c sits between.
- **Typed errors**: mkv/error.rs is the template — constructors minting stage-`Demux`
  `EngineError`s with stable dotted codes (`demux.mkv.not-matroska`, `.truncated`,
  `.bad-vint`, `.bad-structure`, `.no-tracks`, `.cue-table-too-large`, `.seek.too-slow`…,
  mkv/error.rs:35–198), table-tested (:200–284). The per-module `error.rs` exists so
  container zones stay merge-disjoint (mkv/error.rs:1–11 rationale).
- **Facts accessors for the host harvest**: every container exposes its decoder-config
  truth (`mp4::sample_entry_bytes` mp4/mod.rs:617; `mkv::track_entries` → `TrackEntry`
  with `codec_private`; `wav::fmt_chunk_bytes`; open-path-wiring.md §2 table). ASF's will
  be the WAVEFORMATEX / VIDEOINFOHEADER bytes + codec descriptor map.
- **Probe**: `media_probe::ProbeRegistry::with_builtins()` lists exactly 10 signatures
  (crates/media-probe/src/signature/mod.rs:125–138); FormatIds at report.rs:37–85. Adding
  ASF = one signature struct in signature/phase5.rs (magic-anchored twin of the demuxer's
  own `probe`, mirror-pinned by tests/registry_roundtrip.rs — phase5.rs:1–24 doc) + one
  `FormatId::asf()` + one `register` line. Signature shape: Header GUID at window start
  (16 bytes), score `SCORE_MAGIC_ANCHOR` (70) — the Mkv/Wav/Ogg precedent in phase5.rs:57–131.
- **Open path**: `apps/cli/src/containers/mod.rs:40–55` is THE registry (wired: adts,
  aiff, avi, mp4, ogg, wav); wiring a family = `containers/<fmt>.rs` with
  `open(stream, opts)` (the wav.rs:45–88 pattern: construct → `start()` → harvest
  `TrackFact`s (demux_host.rs:137–160 — `codec`, `pcm`, `extra_data`, `dimensions`) →
  clamp/validate start-time via pure `seek` → wrap in the shared `SelectDemuxer`
  (demux_host.rs:398–416)) + one match arm. Unwired formats keep the typed refusal
  (containers/mod.rs:60–71). **6.OPW-asf is one file + two lines** once the demuxer exists.
- **Differential test convention**: crates/demux/tests/mkv_corpus.rs:1–40 — every corpus
  lane demuxed through the real access stack vs **live ffprobe** (stream count, codec
  identity, packet counts via `-count_packets`, timestamp ceilings), skipping loudly when
  `LUMEN_CORPUS`/the default dir is absent. In-tree fixture convention:
  tests/fixtures/generated/MANIFEST.toml (xtask-generated, sha256-pinned) + class-B
  malformed derivatives (trunc/flip/garbage, the avi/mp4 rows in tests/fixtures/).
- **Effort precedents** (`wc -l` per module tree, this session): mkv 6,048; mp4 3,751;
  avi 7,908; ts 4,055; wav 3,424; adts 2,417; ogg 2,725; ps 2,143; aiff 2,266; es 1,926;
  ivf 1,637. (mkv's split: header 919, cluster 538, ebml 663, cues 213, chapters 842,
  tags 704, attachments 559, editions 389, mod 853, error 284, fuzz 84.)

---

## 4. THE SPLIT — ASF-a / ASF-b / ASF-c / ASF-d

6.WAVE-PLAN (worklog.md:10036–10039) sequenced: W4 `6.ASF-a header tree`, W5 `6.ASF-b
data packets`, W6 `6.ASF-c index seeking`, W7 `6.ASF-d differential vs ffmpeg fixtures`,
W8 `6.ASF records` (records = the format-card updates, not in this doc's scope beyond
pointers). The 5.K3-I split proposal (container-asf.md:121–127) is the same three-way +
records; **differential-vs-fixtures got its own unit because the corpus is empty** — the
differential *is* the acceptance evidence. Each unit below: files, gates, fixtures,
error-vocabulary sketch, risks. Shared rules for all four: std-only, no unsafe, no
unwrap/expect/panic, `pub mod asf;` lands once (lib.rs:39–50 append — one line),
R1-disjoint files between concurrent wave siblings.

### 4.1 ASF-a — header object tree (W4)

**One-liner:** parse the GUID object walk + File Properties + Stream Properties + Codec
List + metadata objects into an `AsfDemuxer` that `start()`s with a real track table and
facts accessors; packets deliberately not yet demuxed (typed "not started" /
EndOfStream-only stub is dishonest — instead the unit lands `next_event` returning a typed
`Unsupported` until ASF-b; tests assert that exact refusal).

- **Files:** `crates/demux/src/asf/{mod.rs, guid.rs, header.rs, error.rs, tests.rs,
  test_support.rs (mem_stream reuse), fuzz.rs}` (new tree; `avi/test_support.rs:121`
  `mem_stream` is the pattern); `crates/demux/src/lib.rs` +1 line;
  `crates/media-probe/src/signature/phase5.rs` +`AsfSignature` (16-byte Header-GUID
  anchor, extensions `["asf","wmv","wma","nsc"→no, keep the three]`), `report.rs`
  +`FormatId::asf()`, `signature/mod.rs:125–138` +1 register line;
  `fuzz/fuzz_targets/fuzz_demux_asf.rs` + seeds (the fuzz_demux_avi.rs:1–25 wiring-status
  convention: compile-ready target + stable-loop survival test in-crate).
- **Scope:** GUID constants table (§2.1 verified set + the [FORMAT KNOWLEDGE] extended
  set); top-level walk (Header → Data → trailing index objects) with skip-by-size for
  unknown GUIDs and MP §12 ceilings on object count/size; FPO facts (§2.2 — including the
  QWORD/DWORD count-width pin and the **play-duration-vs-send-duration decision**: track
  spans come from ASF-b's packet stamps; FPO durations are advisory facts only); SP →
  tracks (stream number 1–127 → TrackId, type GUID → MediaKind, WAVEFORMATEX/VIDEOINFO
  TSD captured verbatim); stream-type GUID → registry descriptor vocabulary (§4.5 map);
  Codec List + Extended Content Description + Content Description → facts for
  `media.get-info`; Header Extension bounded child walk (ESP language/name data captured
  or honestly ignored, recorded); broadcast gate (§2.5) + encrypted-stream gate (§2.3
  flags) → typed refusals.
- **Gates (demux-surface):** module tests over the §1.2 fixtures (committed in-tree by
  ASF-d — until then, regenerate on the fly or commit minimal seeds in this unit;
  decision below); probe round-trip (signature ↔ demuxer `probe` mirror test, the
  registry_roundtrip.rs convention); header differential vs live ffprobe on all 11
  fixtures (stream count, codec identity, channels/rate/dimensions); `clippy
  --all-targets -D warnings` + `fmt --check` on `lumen-demux` + `lumen-media-probe`;
  fuzz target compiles + stable-loop survival on seeds; **spec acquisition gate**: the
  unit records the exact ASF Specification revision + URL + redistribution terms in
  `docs/formats/container-asf.md` (network is live — verified: curl reaches
  download.microsoft.com, though the URLs I recalled this session 404/403; locating the
  current spec home is part of the unit, [VERIFY-P0] per container-asf.md:61–62).
- **Error vocabulary sketch** (`asf/error.rs`, mkv/error.rs shape):
  `demux.asf.not-asf` (sniff), `demux.asf.truncated`, `demux.asf.bad-object-size{offset}`,
  `demux.asf.header-too-large` (ResourceLimit/Memory), `demux.asf.no-streams`,
  `demux.asf.unknown-stream-type` (typed passthrough, codec refuses later — the mkv
  unknown-codec precedent), `demux.asf.broadcast-unsupported` (Unsupported + FormatInfo),
  `demux.asf.encrypted-stream` (Unsupported), `demux.asf.bad-waveformatex`,
  `demux.asf.not-started`.
- **Risks:** FPO/SP field-width ambiguities (§2.2/§2.3 — spec pin required); the video
  TSD short form (55 B, not the full VIDEOINFOHEADER I expected — geometry/fourcc
  extraction must be tested against WMEncoder-style real files later, C-V6); Header
  Extension grammar depth (payload-extension systems) — v0 may capture-and-ignore;
  probe collision risk none known (16-byte GUID anchor is unambiguous vs all 10 current
  signatures [VERIFIED against the signature list]).

### 4.2 ASF-b — Data Object packets (W5)

**One-liner:** the packet/payload parse — EC flags, property flags, length/padding/
sequence, send time/duration, multi-payload headers, per-payload {stream id, media object
number, preroll-offset timestamp, replicated data}, sample reassembly — wired into
`next_event()` as real `Packet`s.

- **Files:** `crates/demux/src/asf/{data.rs (+tests in tests.rs)}`; touches nothing
  outside `asf/` (R1-disjoint from wave siblings by construction).
- **Gates:** packet-level differential vs `ffprobe -show_packets` on all fixtures:
  payload count (26 for the av file), per-stream counts, pts equality (after preroll
  subtraction), payload sizes, keyframe flags; the `-packet_size 8000` fixture (packet
  size honored from header fields, not constants); a class-B derivative set
  (trunc0–4 / flip1–16 / garbage0 of the av fixture — the avi/mp4 malformed convention,
  tests/fixtures/expectations/malformed/) asserting typed errors, zero panics, bounded
  memory; fuzz seeds grown from these; the typed-refusal removal test (ASF-a's
  `next_event` Unsupported becomes packets — the exact commit that flips it is this
  unit's headline).
- **Error vocabulary sketch:** `demux.asf.packet.bad-flags{offset}`,
  `demux.asf.packet.bad-length`, `demux.asf.packet.padding-bad`,
  `demux.asf.payload.bad-replication`, `demux.asf.payload.oversized` (ResourceLimit),
  `demux.asf.payload.unknown-stream` (skip + Recovered event, the Discontinuity/
  Recovered seam plugin-api/src/demux.rs:147–160), `demux.asf.timecode-overflow`
  (Corrupt::Timing, mkv's timecode_overflow precedent mkv/error.rs:120–127).
- **Risks (the register the task asked for):**
  1. **Packet header bit grammar** — no in-tree spec; the §2.4 first-packet bytes are the
     anchor but the EC/property flag decode must come from the spec text (ASF-a's
     acquisition gate feeds this).
  2. **Multi-payload frames + media-object reassembly** — payloads of one frame may span
     packets (media object number + fragment flags); grouping logic is the classic
     off-by-one source; ffmpeg fixtures at small sizes don't exercise it (§2.4 measured:
     no fragmentation at 64×48/8 kHz) ⇒ **generate a high-bitrate large-frame fixture**
     (`-b:v 2000k`, bigger testsrc) in this unit to force fragmentation lanes.
  3. **Padding** — padded packets (padding-length fields + 0x55 filler [FORMAT
     KNOWLEDGE]) and `-packet_size` interaction; the pkt8000 fixture covers size variety,
     padding needs a low-bitrate long file lane.
  4. **Replicated data** — compressed-payload replication field grammar [FORMAT
     KNOWLEDGE — verify in ASF-b].
  5. **Repacking** (the task's word): demuxers must hand the decoder one media object per
     packet — fragmented objects get reassembled, replicated data collapses; the
     mkv `PayloadSlots` bounded-pool discipline (mkv/mod.rs:28–33) is the memory model.

### 4.3 ASF-c — Simple Index seeking (W6)

**One-liner:** parse the Simple Index Object into a seek table, implement
`seek`/`seekability`/`reanchor` with the mkv-cues compute-then-commit discipline, and
decide the no-index policy (audio-only files, index-less captures) honestly.

- **Files:** `crates/demux/src/asf/index.rs` + the three trait methods in `asf/mod.rs`.
- **Verified inputs that shape it:** Simple Index = FileID(16) + TimeInterval(8, 100-ns;
  measured 10,000,000 = 1 s) + MaxPacketCount(4) + EntryCount(4) + entries × 6 bytes
  {packet number (4), packet count (2)} with keyframe flag bits in the count field's
  high bits [FORMAT KNOWLEDGE]; **ffmpeg-written entries are all zero** (6 × {0,0}
  measured) — the index carries no usable information in our own fixtures ⇒ (a) seek
  must sanity-check entries (all-zero ⇒ treat as absent), (b) the differential oracle
  must be ffmpeg's *seek behavior*, not its index bytes; audio-only `.wma` files have no
  index at all (§2.1).
- **Policy (proposed, Lead may amend):** sane index present → `SeekSupport::Fast
  {indexed: true}`, mapping time → entry → packet number → byte offset (packet-offset map
  built during ASF-b's walk or a bounded index-scan pass; packets are byte-sequential so
  offsets accumulate from parsed packet lengths — never trusted from the file); no index
  but parseable Data Object → `SlowLinear` bounded scan (the mkv no-cues precedent,
  `demux.mkv.seek.too-slow` mkv/error.rs:189–198); broadcast → already refused at open.
  Simple Index indexes **video streams only** — audio-packet positions derive from the
  video-entry packets' neighborhood (document the coarser granularity honestly).
- **Gates:** open→seek→play lanes (the apps/cli/tests/native_seek.rs convention, run via
  the demuxer's own seek tests since the CLI lane waits for 6.OPW-asf); seek-target
  differential vs ffmpeg (`-ss` on the fixtures, comparing landed keyframe timestamps);
  out-of-range/before-first/past-end typed errors (`demux.asf.seek.out-of-range`);
  invariant D4 (failed seek leaves position valid — plugin-api/src/demux.rs:100–108).
- **Risks:** zero-filled index entries (measured) make "indexed" seek untestable on
  ffmpeg fixtures alone — mitigations: hand-crafted index bytes in a unit fixture
  (class-B style byte-patch of the SI object), plus C-V6 real-world files later; index
  semantics vs preroll (entry times are preroll-offset too? [verify in unit]); packet
  number → byte offset requires walking packets (bounded; no O(1) shortcut exists in the
  format without the index carrying offsets).

### 4.4 ASF-d — differential vs ffmpeg fixtures (W7)

**One-liner:** the acceptance evidence — commit the fixture family in-tree, add the
corpus/fixt suites, print the honest per-file differential table, and propose the
corpus-v4 lane additions.

- **Files:** `crates/demux/tests/{asf_corpus.rs (skips loudly — zero corpus lanes today,
  §1.1 — becomes live when/if C-V6 lands), asf_fixtures.rs}`; fixtures under
  `crates/demux/fixtures/asf/` + `tests/fixtures/generated/` (xtask MANIFEST entries,
  sha256-pinned — the avi/mp4 convention); malformed derivatives +
  `tests/fixtures/expectations/{valid,malformed}/asf_*.toml`; worklog table.
- **Fixture table to commit** (sizes + sha256 measured this session, §1.2 commands):

| File | Bytes | sha256 (first 16 hex) | Lane |
|---|---|---|---|
| wma_wmav2_mono8k.wma | 6,942 | 2f1abf090401e5d4 | audio-only, no index |
| wma_wmav1_st44k.wma | 10,102 | 40e3e4b6eb3e707b | wmav1 variant |
| wmv_wmv2_64x48.wmv | 7,123 | f1803712b502c41a | video-only + SI |
| wmv_wmv1_64x48.wmv | 7,119 | 3cfac4e86fee00fc | wmv1 variant |
| asf_msmpeg4v3.asf | 7,129 | 66201b9205f26a90 | legacy fourcc (Lead re-pin 6.W4: regeneration deterministic, verified twice; investigation value was stale) |
| asf_mjpeg.asf | 26,321 | 1454c0a3a62b640e | **decodable video** |
| wmv_wmv2_wma_av.wmv | 10,499 | 5bd8c76fa14e31aa | A/V multi-payload |
| asf_pkt8000.wmv | 16,899 | e9ef0bc063ccc02f | packet-size variant |
| asf_pcm_s16le.wma | 26,106 | 29c7a469c3627a6f | **decodable audio (PCM)** |
| asf_adpcm_ms.wma | 6,936 | 02ca9cfcf394078b | **decodable audio (ADPCM)** |
| asf_stream_broadcast.wmv | 10,467 | a9db9175da47fe6c | broadcast refusal lane |

  (ASF-b additionally generates the large-frame/high-bitrate fragmentation fixture +
  padding lanes; ASF-d pins them.) The broadcast file is committed as a **refusal
  fixture** (expected: probe-miss or typed broadcast-unsupported) — the same role
  mp4's garbage0 rows play.
- **Gates:** the full differential print (stream/codec/packet-count/pts vs live ffprobe
  7.1.5 on every fixture); decode-exact lanes for pcm/adpcm/mjpeg (ffmpeg `-f md5`
  oracle, the parity-v4 convention); the honest-limits table (§2 ambiguities either
  resolved by then or carried as recorded gaps); records update in the same unit or the
  W8 `6.ASF records` sibling: container-asf.md status flips to partial/implemented,
  feature-matrix rows proposed (Lead-owned), audio-wma.md unchanged (blocked verdict
  stands — WMA tracks stay typed-unsupported).

### 4.5 Stream-type GUID → codec descriptor vocabulary (ASF-a's map)

[VERIFIED] = observed in ffmpeg output this session; the rest is [FORMAT KNOWLEDGE —
verify in ASF-a]. Descriptors target the registry vocabulary the host provider already
speaks (pipeline_host.rs:174–263).

| Signal | → descriptor | Decoder today |
|---|---|---|
| audio GUID + WAVEFORMATEX tag 0x0001 | `pcm_s16le`/`pcm_u8`/… by bits+channels | PcmDecoder (facts) ✓ |
| tag 0x0002 | `adpcm_ms` | AdpcmDecoder (WAVEFORMATEX extra_data) ✓ |
| tag 0x0011 | `adpcm_ima_wav` [FORMAT KNOWLEDGE] | AdpcmDecoder ✓ |
| tag 0x0160/0x0161/0x0162/0x0163 | `wmav1`/`wmav2`/`wmapro`/`wmalossless` | **typed `cli.codec_unavailable`** (audio-wma.md blocked verdict; the descriptor naming is ours to mint — recorded in ASF-a) |
| video GUID + fourcc `MJPG` | `mjpeg` | MjpegDecoder ✓ |
| `WMV1`/`WMV2` | `wmv1`/`wmv2` | typed refusal (video-wmv-vc1.md hw-first row) |
| `MP43`/`MP42`/`MP4S` | `msmpeg4v3`/… | typed refusal |
| `VC1`-family [FORMAT KNOWLEDGE] | `vc1` | typed refusal (ADR-001 hw path) |
| command/script GUID | — | typed `unknown`-class passthrough, track filtered at host (§3 subtitles precedent) |

---

## 5. Effort estimate vs the precedents (honest numbers)

Per-module precedents (§3 wc -l, tests included) and the ASF estimate — assuming the spec
acquisition gate succeeds and the packet grammar is spec-clear:

| Unit | Estimated LOC (code+tests) | Precedent anchor | Size call |
|---|---|---|---|
| ASF-a header tree | ~900–1,300 | wav 3,424 / aiff 2,266 are single-stream containers; mkv/header.rs is 919 but rides 663 of ebml — ASF's GUID+size frames are simpler than EBML vints | **S–M** (matches 5.K3-I's `6.?a` S–M) |
| ASF-b data packets | ~800–1,200 | mkv/cluster.rs 538 + lacing; avi's packet walk lives in mod.rs's 1,271 | **M** (5.K3-I's `6.?b` M) |
| ASF-c index seeking | ~250–450 | mkv/cues.rs 213 + seek arms | **S** (5.K3-I's `6.?c` S, optional → now scheduled) |
| ASF-d differentials | ~300–500 test code + fixtures | mkv_corpus/ogg_fixtures suites | **S–M** |
| **Total** | **~2,250–3,450** | vs mkv 6,048 / mp4 3,751 / avi 7,908 | four units, one wave apart |

Why lower than mkv/mp4: no variable-length integers, no EBML schema, no chapter/tag/
edition machinery in v1 scope, no fragmented-moov-style moving targets; the object walk is
fixed 24-byte frames + skip-by-size. Why not lower: the packet bit-grammar +
media-object reassembly has no mkv/avi analog (closest: mkv lacing), and the fixture
family must be generated + maintained because the corpus is empty. 5.K3-I's "M for
header+data core" (container-asf.md:59–60) is consistent with this table.

---

## 6. What a future 6.OPW-asf needs from each unit

The wiring unit itself is one file + two lines (§3 open-path cites). Its inputs:

- **From ASF-a:** `AsfDemuxer::new(Box<dyn SourceStream>)` + `start()` → L0 Tracks; facts
  accessors to harvest into `TrackFact`s (demux_host.rs:137–160): codec descriptor per
  stream (§4.5 map), `pcm: Option<(channels, rate)>` from WAVEFORMATEX for PCM streams,
  `extra_data: SharedBytes` = **the raw WAVEFORMATEX body** for ADPCM streams (the
  `fmt_chunk_bytes` precedent — wav.rs:107–109), video dimensions for `dimensions`;
  probe id `asf` registered (phase5.rs + report.rs) so `containers::open("asf", …)` is
  reachable; the refusal surfaces (broadcast/encrypted/unknown-codec) stay typed.
- **From ASF-b:** `next_event()` yielding `DemuxEvent::Packet` — without it the engine's
  `NoDecodableTrack`/decode flow has nothing to drive; preroll-corrected pts so the
  start-time clamp (`containers/*.rs` steps 2) is honest.
- **From ASF-c:** `seekability()` + pure `seek()` + `reanchor()` so the open-path
  start-time validation and in-session seeking work (the wav.rs:57–70 clamp pattern);
  the no-index SlowLinear policy must be surfaced (ts's `SeekSupport::None` typed
  refusal is the fallback shape if Lead prefers v0-simple).
- **From ASF-d:** the fixture lanes the CLI conformance lane runs over (incl. the three
  decodable payload families), plus the honest-limits table copied into the wiring
  unit's worklog (what this file's §2 ambiguities resolved to).
- **Sequencing note:** 6.OPW-asf can land after ASF-b without ASF-c (seek refusal is
  typed), and after ASF-a only if packet-stub refusal is acceptable — recommend
  post-ASF-b, matching the wave table's W5→W7 spacing.

---

## 7. Honest limits / unverified (this session's gaps)

- **Zero code changes** — this is a doc + worklog card; nothing compiled, no gates run
  (the worktree is clean main `565042d` plus docs; see the worklog for the commit).
- **Spec text not in hand.** The ASF Specification (publicly documented GUID/object
  structure) is the format's ground truth and this doc's [FORMAT KNOWLEDGE] labels mark
  everything that awaits it. Network is live (verified: HTTP responses from
  download.microsoft.com and microsoft.com) but the URLs I could recall 404'd / were
  bot-blocked (403) — the exact current spec home + revision + redistribution terms are
  ASF-a's [VERIFY-P0] gate, already flagged by container-asf.md:61–62.
- **Field-width ambiguities left open on purpose** (spec pin required): FPO
  Data-Packets-Count byte width (observed 8 bytes where I expected 4, §2.2); Header
  Extension reserved-2 width (observed arithmetic fits 2+4, §2.3); the 2 zero bytes at
  SP +52 (stream flags? encryption bit?); preroll unit wording (behavior = ms,
  subtracted from payload stamps, §2.2/§2.4); the video TSD's short 51–55-byte
  VIDEOINFOHEADER form (§2.3); Simple Index entry keyframe-flag bit positions (§4.3).
- **ffmpeg-only evidence.** Every parsed byte came from ffmpeg 7.1.5's muxer. Real-world
  WMEncoder/Windows-media files (different writer quirks: real indexes, non-zero file
  IDs, Content Description objects, DRM) are unmeasured — the C-V6 corpus acquisition
  (corpus-plan.md `asf-wmv` row; audio-wma.md:86–88) is the closing item; until then the
  differential claims are scoped "vs ffmpeg 7.1.5" explicitly.
- **wmvkit 0.1.0** (the 5.K3-I oracle candidate, container-asf.md:41–50) was NOT
  compiled or consulted this session; if used later it is a **behavioral oracle only**
  — it is MPL-2.0 and our tree is MIT (ADR-006), so no code-level derivation; ffmpeg
  remains the primary oracle.
- **VLC-side** behavior is cited from the existing inventory records (one green probe
  lane, container-asf.md:51–57), not re-measured here.
- The `-f asf_stream` broadcast bytes were generated and ffprobe-refused, but I did not
  attempt a full MMS-framing decode — out of v1 scope by policy (§2.5).
