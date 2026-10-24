# Codec record — screen-capture codecs (Screen video, FRAPS, TechSmith, Windows Screen 1/2, …)

Status: **deferred (later) — decision record (card 5.P2b-I part B)**.
Family per doc 04 §4.1 row "Windows Screen 1/2, TSCC-ish — P3, later,
none, S–M (simple codecs)": the capture/desktop codecs that appear in
screen-recording files, not in camera/broadcast media. Members named by
the card and covered here: **Screen video** (Flash Screen Video, FSV1
"Screen video" / FSV2 "Screen video 2", in FLV), **FRAPS** (FPS1, in
FRAPS-captured AVIs), **TechSmith Screen Codec** (TSCC/TSC2, Camtasia
AVIs), **Windows Screen 1/2** (MSS1/MSS2, WMV screen sessions), plus
the adjacent tail the census lumps together (CamStudio CSCD-class,
MS Video 1-adjacent legacy) which stays under
`codec-video-legacy-blocked`'s umbrella row.

**Matrix-row PROPOSAL (recorded in-file for the Lead, per the 5.K3-I
investigation precedent):** no dedicated feature-matrix row exists
today (only `codec-video-legacy-blocked` for Cinepak/Hap/Dirac/MS Video
1). PROPOSE `codec-video-screen-capture` (video-codec, P3, phase 8,
vlc_modules `["avcodec"]`, inventory_ref `video-census-tail`, status
not_started ×4, notes citing this record) so the family is visible as
its own truth-unit. Feature-matrix.toml deliberately untouched
(investigation scope).

- Date of evidence: **2026-10-24** (crates.io API, this card).
- Supersedes-cites: doc 04 §4.1 row, feasibility video-codecs §1 row 12
  (2026-10-19), formats.json `video-census-tail`.

## Parse / Decode / Encode

Nothing implemented. Family shape (why "S–M simple codecs"): mostly
RLE + zlib variants over RGB(A) frames — FSV1 = zlib-compressed
rectangular RLE tiles; FSV2 = block-diff + zlib; FRAPS FPS1 = RLE with
lossy/lossless variants; TSCC = zlib RLE; MSS1/MSS2 = WMV-adjacent
(zlib + motion-compensated screen codec, the hardest of the set).
Decode-only if ever scheduled; typed-unsupported stream semantics
meanwhile.

## Evidence (what was checked, 2026-10-24)

- **crates.io direct names:** `fraps`, `fraps-decoder`, `tscc`,
  `techsmith`, `screen-video`, `fsv1`, `flash-screen-video`, `mss1`,
  `mss2`, `windows-screen`, `camstudio`, `cscd` — **all NOT_FOUND**.
  Keyword searches (`fraps`, `techsmith`, `screen video codec`) surface
  nothing relevant — the closest hits are unrelated
  (`frap` = APRS parser; `syeve` = its own lossless streaming codec,
  not a TSCC-class decoder). **Confirms feasibility §1 row 12
  "none (doc 04, unchanged)" on 2026-10-24.**
- **VLC handling (oracle):** VLC has **no dedicated decoder modules**
  for any of these — modules-linux.json's 14 video-decoder
  registrations (png, theora, oggspots, cdg, avcodec×2, rawvideo,
  dav1d, ddummy×2, jpeg, xwd, svgdec, stats) contain no screen-codec
  entries; they all ride `avcodec` (ffmpeg's codec zoo) and the
  census-tail is `not-probe-verified` (formats.json
  `video-census-tail`). Not mirrorable; behavioral oracle only.
- **Container adjacency (real dependencies if ever scheduled):**
  FSV1/2 live in **FLV** (no FLV demuxer in tree; `oxideav-flv` 0.0.5
  exists on crates.io as a young container-only candidate — noted for
  the Phase-6/7 container backlog, not adopted); FRAPS/TSCC/CSCD live
  in **AVI** (AVI demuxer exists in tree — fourcc pass-through works
  today, decode refused); MSS1/2 live in **ASF** (Phase-6 proposal,
  `container-asf.md`).

## Bit depths / layouts

N/A (deferred). Family note: mostly RGB(A) 8-bit; FSV2/MSS2 also carry
palette/16-bit variants (doc 04 §4.1 "everything" gap column).

## Tracks / Seeking / Timestamps

Codec-level record; container records own mechanics. AVI fourcc
pass-through already surfaces these tracks as typed-unsupported
streams (AVI landed in tree).

## Corrupt-input behavior

N/A (no parser ships). Screen captures in the wild are often truncated
mid-recording (crash-killed captures) — any future parser must treat
truncation as end-of-stream, recorded here as a design input.

## Limits / Platforms

Deferred uniformly (would be not_started ×4 on the proposed row).
Patents: MSS1/2 are Microsoft codecs (patent haze, Indeo-class per doc
04 §4.1's blocked-neighbors column) `[VERIFY at any future card]`;
FSV1/2 (Adobe/On2 lineage) believed unencumbered `[VERIFY]`.

## Corpus

None — no screen-codec lane in corpus-v4; fixtures require generation
(FSV1/2 via ffmpeg `-c:v flashsv`, TSCC/FRAPS need original tooling or
archived samples `[VERIFY at C-V6]`).

## Results

None (no code). Next step: **no card scheduled; re-evaluate only if
doc 04 §8.6 item 7 usage-share data surprises (screen-recording files
in user libraries) or at the Phase-7 R-10 window re-check of crates.io
(currently zero candidates).** If scheduled: S–M self-builds against
public reverse-engineering docs, gated by the proposed
`codec-video-screen-capture` matrix row.
