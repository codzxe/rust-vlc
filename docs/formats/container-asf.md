# Container record — ASF / WMV (Advanced Systems Format)

Status: **not_started — decision record (card 5.K3-I)**. Verdict:
**self-build, LATER, Phase 6** (was "Phase 5/6 TBD" in the seeded
feature-matrix row; doc 04 §2 table already said Phase 6). This file
records feasibility evidence and the split-card proposal; no code exists.

- Layer planned: L2 demux (`crates/demux/src/asf/`, future), C5 std-only,
  zero external dependencies, no `unsafe` — same posture as the MKV/PS/AVI
  self-builds.
- Oracle: ffmpeg/ffprobe 7.1.5 + VLC 3.0.23 (`asf` demux module).
- Date of evidence: **2026-10-24** (crates.io API re-checks, this card).

## Parse / Decode / Encode

Nothing implemented. Planned scope (self-build, doc 04 §2 "M–L"):

1. **Header object tree** — GUID-identified objects (the format is a
   sequence of length-prefixed GUID boxes, publicly documented in
   Microsoft's ASF specification; header walk, File Properties, Stream
   Properties incl. WAVEFORMATEX/VIDEOINFO payload descriptors, Header
   Extension, Content Description + extended metadata, Content Branding,
   language/mutual-exclusion/bitrate-exclusion objects).
2. **Data Object packetization** — multi-payload packets, packet-level
   flags, replicated data, padding, per-stream sample reassembly.
3. **Simple Index Object** seeking (the documented seek index).

Encode: out of scope (VLC's `asf` *muxer* is sout-side; feasibility §6).

Unsupported constructs (honest v1 exclusions, to be refined by the build
cards): broadcast/live-mode ASF (no seek index), DRM ("Protected"
ContentEncryption paths → typed refusal), script streams (type 5-adjacent)
unless trivially pass-through.

## Evidence (what was checked, 2026-10-24)

- **crates.io direct names:** `asf`, `asf-rs`, `asf_demux`,
  `symphonia-format-asf` — all **NOT_FOUND** (API `crate does not exist`).
  Confirms doc 04 §2 row ("none found", verified 2026-10-19) and that
  Symphonia's format set (isomp4/mkv/ogg/riff/caf) has no ASF member.
- **NEW since the 2026-10-19 verification:** search surfaces
  **`wmvkit` 0.1.0** — "Pure-Rust ASF (.wmv) demuxer", MPL-2.0,
  published 2025-12-24, **single release, never updated** (10 months
  stale), 428 downloads, repo `github.com/xmoezzz/wmvkit`. README scope:
  "ASF Header Object + Data Object", stream separation into
  `DemuxedSample`s — **no claimed seek-index, broadcast, or metadata
  coverage**. Assessment: below the adoption bar (single-release 0.1.0;
  same precedent as feasibility §1.3's young-crate rule — harden-or-oracle,
  never adopt-and-trust). Recorded as a **reference/oracle candidate** for
  the Phase-6 build, not a dependency.
- **VLC parity (what we mirror):** VLC 3.0.23 demuxes ASF with its own
  `asf` module (docs/inventory/modules-linux.json:769 "ASF/WMV demuxer";
  also an `asf` sout muxer, out of v1 scope). Baseline probe
  docs/baselines/raw/linux/17-format-probes.txt:339–354: the
  `asf_wmv2_wmav2` fixture (ffmpeg-generated WMV2 64×48 + WMAv2 8 kHz
  mono, 1.128 s) exits 0 with `using demux module "asf"` — VLC-side
  container evidence is one green probe lane, not a corpus.
- **Spec availability:** the ASF container structure is publicly
  documented (GUID/object tree) — this is what makes self-build **M** for
  the header+data core, in contrast to the WMA *bitstream* (see
  `audio-wma.md`: no open decoder spec). `[VERIFY-P0 when the build card
  opens: exact spec revision/URL and its redistribution terms]`.

## Bit depths / layouts

N/A (container). Audio payloads arrive as WAVEFORMATEX-described frames
handed to the codec layer — including the existing
`parse_waveformat`/ADPCM seam already in `crates/codecs` (adpcm_ms and
adpcm_ima_wav descriptors accept ASF-style WAVEFORMATEX facts; the
WMA tags 0x160/0x161/0x162 remain codec-layer blocked, see below).

## Tracks (selection, default, languages)

Planned: Stream Properties → track table (number 1–127, type, language
list object, mutual exclusion). Gap recorded: none implemented.

## Seeking

Planned: Simple Index Object (per-video-stream time-keyed index); absent
index (broadcast files) → duration-estimate seek, same policy class as
MKV-without-Cues. Not implemented.

## Timestamps

Planned: File Properties `Preroll` + 100-ns units (doc'd offset: payload
timestamps are offset by preroll); discontinuity behavior TBD by the build
card. Not implemented.

## Corrupt-input behavior (policy, not yet code)

Object-size/GUID bounds checks → typed `Corrupt` refusals; junk objects
skipped by size; truncation mid-object → `Corrupt::Truncated`; ceilings on
object count/size per the §12 bomb policy. Zero panics, one `fuzz_demux_asf`
target before any `implemented` status (doc 04 §8.5 registry rule).

## Limits / Platforms / Results

To be filled by the Phase-6 build cards. Matrix row today:
`not_started` on all four platforms — unchanged by this card.

## Corpus (honest gap)

**corpus-v4/media carries no `.wmv`/`.asf` lane** (checked 2026-10-24).
The only in-tree evidence is the Phase-0 probe fixture referenced by
docs/inventory/formats.json (`asf_wmv2_wmav2`, ffmpeg-generated). The
Phase-6 build cards must add generated lanes (ffmpeg `asf` muxer exists —
corpus-plan.md row `asf-wmv`: self-build GUID-object writer for class-B
adversarial lanes, G-FF for carriers) plus the C-V6 curated WMA/VC-1
sources acquisition item.

## Verdict + split-card proposal (the 5.K3-I decision)

1. **Container: self-build LATER, Phase 6.** Rationale: (a) publicly
   documented structure → M-sized, same class as our AVI/MKV self-builds;
   (b) no adoptable crate (`wmvkit` recorded as oracle candidate only);
   (c) v1 value case is real but P2: metadata/library listing +
   hw-decodable WMV video on Windows/Linux (gated by the
   `codec-video-wmv-vc1` row's hw-first path) + **typed-unsupported WMA
   audio** — "container alone buys little" (doc 04 §2) is sharpened, not
   contradicted.
2. **Split-card suggestion for the Phase-6 wave proposal:**
   - `6.?a` (S–M): ASF header object tree + track table + metadata
     objects; typed refusals; unit tests + fuzz seeds.
   - `6.?b` (M): Data Object packetization + demux-trait wiring +
     differential vs ffprobe on generated lanes.
   - `6.?c` (S, optional): Simple Index seeking + language/exclusion
     objects.

## Matrix-row proposal (for the Lead; feature-matrix.toml not touched by
this investigation card)

- `container-asf-wmv`: `phase_target` 5 → **6** (align with doc 04 §2; the
  seeded 5 predates this decision); status stays `not_started` ×4;
  `evidence` ← "docs/formats/container-asf.md (5.K3-I, 2026-10-24)"; notes
  updated to: demux-only value case (metadata + hw WMV video Win/Linux +
  typed-unsupported WMA audio); `wmvkit` 0.1.0 oracle candidate, not a dep.
