# Feasibility §5 — Subtitles & captions (parse, timing, render)

Task ID: 0.C2-I (Wave 0.C, investigation). Date of verification: **2026-10-19**.
Feeds: `docs/feasibility-report.md` §5 (doc 02 §5 item 5). Source table: doc 04
§5; audit items: **6** (oxisynth LGPL — the MIDI synth lives here because
subtitle rendering and MIDI synthesis share the "no C libraries" wall), plus
doc 04 §8.6 item 1's subtitle side is out of scope (it is audio-only).
Risk linkage: R-09 (ASS styling), doc 04 §7 gaps 5 (ASS), 6 (PGS/DVB), 9 (MIDI).
Baseline scope: VLC 3.0.23 Linux census — subtitle category = **32 modules**:
`spu decoder` 22 + `sub source` 7 + `sub filter` 1 + `text renderer` 2
(docs/inventory/modules-linux.json; counting rule in modules-summary.md §G3).

**Method.** Crates re-checked today via crates.io API are marked **[RV]**
(URLs in §7); everything else keeps doc 04's 2026-10-19 marks. Rendering-path
analysis builds on doc 03 §4.11 (subtitle compositing [PROPOSAL]) and doc 03
§4.12 (timed-event model) — this file *refines* those, it does not restate
them. Font-stack/text-shaping crate selection is doc 01/03 scope
(`[VERIFY-P0]` in doc 04 §5) and is treated here only as a dependency edge.

---

## 1. Per-format verdict table

"Parser availability" = pure-Rust crate today or self-build size (S/M/L per
doc 04 §1.4). "Rendering path" = where the pixels come from: **text** (our
renderer-wgpu text stack via the CPU-rasterized RGBA atlas), **bitmap**
(composited RGBA layers, §3), **extraction** (caption bytes pulled out of the
container/elementary stream before any decode), **synth** (audio, §5 only).
"v1 path": software-decode (full parse+render in v1) / partial (ships with a
named subset) / blocked.

| # | Format | Prio | Parser availability (re-verified) | Rendering path | v1 path | Parity consequence |
|---|---|---|---|---|---|---|
| 1 | SubRip (.srt) + variants | P0 | `srt` 0.1.0 (tytouf/srt-rs, 2017-03-10, stale) **[RV]**; emk `subtitles-rs` family as reference (doc 04); `subparse` 0.7.0 (MPL-2.0, 2021-04-11) **[RV]** demonstrates the parse surface incl. SRT | text | software-decode (self-build S) | Full text parity; styling is SRT-trivial (bold/italic/underline runs) |
| 2 | ASS/SSA | P0 | `ass-parser` 0.2.3 (MIT OR Apache-2.0, 2025-04-11) **[RV]**; `ssa-parser` 0.1.0 (2024-11-12) **[RV]**; libass is C — forbidden (doc 04, unchanged) | text (styled runs → RGBA atlas, positioning, karaoke) | partial — plain text v1 Phase 2, styling subset Phase 5 (§2) | The R-09 gap: anime/fansub files render degraded until Phase 5; matrix `ass.styling = partial (subset)` |
| 3 | WebVTT | P1 | `webvtt-parser` 1.2.0 stable / 1.2.1-rc line (MIT, fframes repo, updated 2026-10-18) **[RV]**; `webvtt` 0.2.0 (2023-04-28, bare-bones) **[RV]** | text | software-decode (harden, S) | Cue settings (line/position/align, vertical writing) subset mapped to our position model; native `<v>`/`<c>` tags degrade to plain runs |
| 4 | SubViewer (.sub/.sbv) | P2 | none — self-build S (doc 04) | text | software-decode (Phase 5) | Trivial; only encoding oddities |
| 5 | MicroDVD (.sub) | P2 | none — self-build S; frame-number timing needs fps handshake (doc 04) | text | software-decode (Phase 5) | fps source (container or user override) must be explicit — [VERIFY default-fps policy] |
| 6 | MPL2 | P3 | none — self-build S (doc 04) | text | software-decode (later) | — |
| 7 | SAMI (.smi) | P2 | none — self-build S–M (HTML-ish; doc 04) | text | software-decode (Phase 5) | Language-sync (`SYNC` blocks) + class-based styling subset; full CSS degraded |
| 8 | TTML/DFXP (incl. IMSC subset) | P2 | none — self-build M (subtitle-semantics subset; doc 04) | text | partial (Phase 6) | IMSC-HRIT/ST styling subset; anime/broadcast TTML loses advanced styling (same class of gap as ASS, smaller audience) |
| 9 | PGS (Blu-ray) | P2 | none — self-build S parse (documented RLE segments; doc 04) | bitmap (§3) | software-decode parse in Phase 6 **iff** compositing contract lands (doc 01 dependency) | Full bitmap parity is *possible* — this is decode-only work plus renderer overlay, no styling gap |
| 10 | DVB subtitles (in TS) | P2 | none — self-build S–M (segment parsing inside MPEG-TS PES; doc 04) | bitmap (§3) | software-decode (Phase 6, pairs with TS work) | Same as PGS plus PMT-descriptor/subtitle-PID plumbing |
| 11 | VobSub (DVD .sub/.idx) | P2 | `vobsub` 0.2.3 (CC0-1.0, 2017-05-22, emk family) **[RV]** — decode path verified in doc 04 | bitmap (§3) | partial (harden, later) | Palette/idx-sync hardening; DVD-Video *navigation* stays blocked (separate ADR-008 scope) |
| 12 | CC/608/708 (CEA captions) | P1 | `cea608` 0.0.1 (MIT, 2026-03-28) is a **builder** ("pop-on caption sequence builder"), not a decoder **[RV]**; `cc708`/`eia608` NOT_FOUND **[RV]** → self-build from CEA-608-E/708-D (§4) | extraction → text (+608 roll-up windows / 708 window model) | partial — 608 in Phase 6, 708 M–L behind it (§4) | Broadcast/ATSC captures and US disc rips lose captions until then; matrix `cc608/cc708 = partial` per stage |
| 13 | Ogg Kate | P3 | none found (doc 04 `[VERIFY-P0]` stands — re-search not run today) | text + optional bitmap canvas | blocked/later | Niche; Kate's motion/scroll effects would degrade like ASS |
| 14 | 3GPP timed text (tx3g) | P3 | none — self-build S (in-MP4 track; doc 04) | text | software-decode (later) | iOS/MP4 captions; small feature set |

### 1.1 Census reconciliation (VLC's 32 subtitle modules vs the table)

VLC's subtitle category is broader than doc 04 §5's format list. Disposition of
every census module (modules-linux.json `by_capability`), so nothing is
silently dropped from parity accounting:

| VLC module(s) | Capability | v1 disposition |
|---|---|---|
| `subsdec` (text subtitle decoder) | spu decoder | covers SRT/SubViewer/MicroDVD/MPL2/SAMI/SSA-text — our rows 1–7 |
| `libass` | spu decoder | C — forbidden; replaced by our styling subset (§2). The single biggest parity-visible substitution in the category |
| `webvtt`, `ttml`, `substx3g`, `stl` (EBU STL), `kate`, `subsusf` (USF) | spu decoder | rows 3, 8, 14, + three below |
| `spudec`, `svcdsub` (Philips OGT), `cvdsub` | spu decoder | row 11 (VobSub) + SVCD/CVD variants — add as bitmap-parse rows S (self-build, later) |
| `dvbsub`, `aribsub` (ARIB STD-B24/B37) | spu decoder | row 10 + ARIB: Japanese broadcast, DRCS glyphs + large table machinery — **later/blocked [VERIFY spec access]**, matrix note |
| `cc` | spu decoder | row 12 (§4) |
| `scte27` (bitmap), `scte18` (EAS text), `textst` (HDMV TextST) | spu decoder | not in doc 04 §5 — add: S–M self-builds, `later`; scte18 is tiny (emergency-alert text extraction, S) |
| `zvbi` (VBI/Teletext) | spu decoder | C library (zvbi) — forbidden; Teletext = its own project (M–L). v1: `blocked` with reason; [VERIFY] whether any v2 appetite exists |
| `avcodec` (spu side) | spu decoder | covers PGS/DVB/xsub in VLC via FFmpeg — our rows 9–10 replace it in Rust |
| `freetype`, `tdummy` | text renderer | freetype is C — forbidden; replaced by the renderer-wgpu text stack (doc 03 §4.11, crate choice [VERIFY-P0]) |
| `marq`, `rss`, `logo`, `mosaic`, `audiobargraph_v`, `remoteosd`, `dynamicoverlay` | sub source | overlay-generator features, not subtitle formats; v1 takes none (marq/logo later as engine overlays); remoteosd/mosaic/audiobargraph = `later` |
| `subsdelay` | sub filter | delay/extend filter — S self-build, Phase 5+, engine-side not parser-side |
| `ddummy`×2, `stats` | spu decoder | test modules, no analog by design |

Census-derived additions for doc 04's table (Lead to fold in): SVCD/CVD subs,
`scte27`, `scte18`, `textst`, ARIB, Teletext — all recorded above with
dispositions; flagged in §6 [VERIFY] ledger as doc 04 table updates, not
silently decided here.

---

## 2. ASS/SSA gap quantification (R-09 — libass is C/forbidden)

### 2.1 What "full styling" means (the libass feature surface)

libass (used by VLC's `libass` module, by mpv, and by every fansub-grade
renderer) implements, at minimum:

- **Script/dialog model:** `Script Info` (PlayResX/Y, WrapStyle,
  ScaledBorderAndShadow, YCbCr Matrix), `Styles` (per-style font name/size,
  primary/secondary/outline/shadow colors + alpha — ASS uses BGR + alpha
  byte, often in &HAABBGGRR form, with a YCbCr-vs-RGB header ambiguity),
  alignment (1–9 numpad model), margins L/R/V, spacing, angle, scale X/Y,
  border style 1 (outline+shadow) vs 3 (opaque box), encoding, bold/italic/
  underline/strikeout incl. the `-1` "toggle" convention), `Events` with
  Layer, Start, End, Style, Margin overrides, Name, Text.
- **Override-tag zoo:** `\b \i \u \s` (formatting), `\fs \fn \fe` (font
  size/name/encoding), `\c \1c \2c \3c \4c` and `\alpha \1a…\4a` (colors),
  `\pos(x,y) \move(x1,y1,x2,y2,t1,t2) \org(x,y)` (positioning/camera),
  `\an \a` (alignment override, modern + legacy), `\fr \frx \fry \frz`
  (rotation), `\fax \fay` (shear), `\bord \shad \be \blur` (border/shadow/
  blur strength), `\fsp` (letter spacing), `\k \kf \ko` (karaoke timing —
  the secondary-color sweep), `\t(t1,t2,accel,tags)` (animated transforms of
  a *subset* of tags), `\clip \iclip` (rectangle and vector-drawing clip
  paths), `\r[style]` (style reset), `\fad(fadein,fadeout)` and
  `\fade(a1,a2,a3,t1,t2,t3,t4)` (fades), `\p1`/`\p0` (vector-drawing mode
  with `\pbo` baseline offset), `\q`-wrap overrides, and the drawing-command
  mini-language itself (`m l b s p c` with spline control points).
- **Layout engine:** line breaking per WrapStyle (0 smart, 1 end-of-line
  `\N` only, 2 no wrap, 3 smart+wide), margin collision, default-position
  math, karaoke word-timing highlight, transforms applied on a clock, nested
  `\t` interpolation, font fallback across families, embedded font
  attachments (MKV `fontttf` attachments, often obfuscated), fontconfig
  system discovery, strikeout/underline drawing, rotated/sheared layout,
  vector-clip rasterization, and 32-bit-per-pixel compositing with shadow
  blur.

No pure-Rust crate implements this; `ass-parser`/`ssa-parser` (both **[RV]**
today) parse the *syntax* (sections, styles, events, tags-as-strings) and
explicitly do not render. The gap is therefore a **typesetting-engine**
build, not a parser build.

### 2.2 What our pure-Rust subset realistically covers

Refining doc 04 §5's proposal into a **tag-coverage matrix** (the conformance
artifact R-09's trigger consumes; disposition vocabulary: **render** =
visually correct, **degrade** = visible but not libass-identical (counted,
matrix note), **drop** = ignored with a per-event counter, never a parse
failure):

| Class | Tags/features | v1 (Phase 2, plain-text) | v2 (Phase 5, styling subset) | Disposition notes |
|---|---|---|---|---|
| Text formatting | `\b \i \u \s` | degrade (plain) | **render** | font-family synthesis for bold/italic if the face lacks them |
| Font | `\fs \fn` | degrade (default face+size) | **render** | per-run font/size in the atlas pipeline; fallback chain [VERIFY text-stack crate] |
| Colors | `\c \1c \2c \3c \4c \alpha \1a…\4a` | degrade (1c only) | **render** \c/\1c/\alpha; outline/shadow colors with §3 atlas alpha | header YCbCr-vs-RGB ambiguity resolved per Script Info field |
| Positioning | `\pos`, `\an`, `\a`, margins, alignment | degrade (bottom-center default, `Style` alignment respected) | **render** static `\pos`/`\an` | per-cue dest-rect in the overlay contract (§3) |
| Motion | `\move`, `\org` | drop (counted) | degrade (linear `\move`, no `\org` camera) | `\org` affects rotation perspective — drops cleanly |
| Rotation/shear | `\fr \frx \fry \fz \fax \fay` | drop | degrade (`\frz` only, 2-D) | 3-D rotation needs transform in the text raster pass |
| Border/shadow | `\bord \shad \be \blur`, BorderStyle 3 | drop (counted) | degrade (bord/shad as stroke+shadow runs; no `\blur` gaussian) | stroke via atlas dilation [VERIFY perf] |
| Spacing | `\fsp` | drop | render | simple per-run tracking |
| Karaoke | `\k \kf \ko` | drop (counted) | **render** \k/\kf (secondary-color sweep timed to the run); `\ko` degrade | the highest-value fansub tag; timing data already in parse |
| Animation | `\t(...)` | drop | degrade (linear interpolation of `\fs \c \alpha \frz \bord` subset) | nested/accel curves degrade to linear |
| Clip | `\clip \iclip` | drop | degrade (axis-aligned rect only) | vector clip needs a rasterizer — v3 candidate |
| Reset | `\r` | drop | **render** | cheap, high frequency |
| Fades | `\fad \fade` | degrade (hard on/off) | render \fad; \fade degrade | per-event alpha ramp in the compositing pass |
| Drawing mode | `\p1` drawings, `\pbo` | drop | degrade (line/polygon fills, no bezier smoothing) | reuses bitmap path (§3) if shapes rasterize CPU-side |
| WrapStyle | 0–3 + `\q` | degrade (0) | render 0/2, degrade 1/3 | line breaker lives in the text stack |
| Style line fields | fonts/colors/alignment/margins | partial (font/size/alignment/color only) | **render** all except angle/scaleX/Y | — |
| Script Info | PlayRes scaling, ScaledBorderAndShadow, YCbCr Matrix | degrade | render PlayRes scaling + SBS; matrix [VERIFY] | PlayRes→video-frame coordinate mapping is mandatory for correct positions |

Estimated coverage after Phase 5: **~95 % of real-world styling value** is the
doc 04 `[VERIFY]` claim — we keep it marked [VERIFY] but now make it
*measurable* (§2.3); the residual (vector clip, `\org`, 3-D rotation, blur,
embedded-font edge cases) is precisely what the counter surfaces.

### 2.3 Conformance measurement plan (per doc 06 §7)

- The subtitle-parser conformance report (`conformance-<id>.toml`, doc 06
  §7.8) gains an **unsupported-tag counter** block: per-tag counts
  (`tags_dropped`, `tags_degraded`), per-event flag
  (`events_with_degradation`), and corpus-level share — this is the R-09
  trigger metric (doc 14: "trigger: unsupported-tag count in conformance").
- **Corpus:** legally-distributable ASS files with escalating tag density
  (self-authored + community-licensed CC samples; no fansub rips) — corpus
  list + licenses in the format record (doc 04 §6 template).
- **Oracle:** VLC 3.0.23 renders the same files through libass; our
  differential harness compares *event timing* exactly (parse-level) and
  renders *visual* comparisons as screenshot diffs with a documented
  tolerance (libass is the reference *implementation*, consulted only via
  its output — same clean-room method as the VLC oracle; no libass source
  is read) [PROPOSAL].
- Matrix linkage: `ass = partial (styling subset)` permanently in v1 with
  `notes` carrying the measured tag-degradation share per release; full
  libass parity explicitly **not claimed** (doc 14 A9).

### 2.4 Font-stack dependency (flag, not decide)

The subset above assumes the renderer text stack can do: style runs, per-run
fonts/sizes/colors, stroke+shadow, per-cue positioning, and embedded-font
(MKV attachment) loading. The crate choice (rustybuzz/swash/cosmic-text
class) is doc 01/03 scope (`[VERIFY-P0]` per doc 04 §5); **this section's
commitment is only that the ASS subset is designed against that interface**,
so the text-stack decision cannot later shrink the subset silently — any
shrink shows up as a moved matrix row, not a hidden regression.

---

## 3. PGS / DVB / VobSub bitmap compositing via renderer-wgpu

Doc 03 §4.11 [PROPOSAL] establishes: subtitle events rasterize CPU-side into
an **RGBA atlas** uploaded per event (~Hz, not per frame), the video shader
composites video → color-convert → tone/scale → alpha-blend the subtitle
layer at its position. That is the right architecture; this section **refines**
it for the bitmap-subtitle family (doc 04 §5 "bitmap compositing dependency"
tracking item):

1. **Unified layer type.** PGS, DVB, VobSub, ASS drawings, and 708 windows all
   reduce to the same contract: one `OverlayLayer { texture (premultiplied
   RGBA8), dest_rect (video-frame space), alpha, blend }`. The renderer never
   knows the format; the parser produces layers. Text events already use this
   path — bitmap formats are *not* a second compositing code path (this is
   the refinement of doc 03's "one compositing code path for all subtitle
   formats" made concrete).
2. **Decode side (CPU, per format):**
   - **PGS:** display-set state machine (PCS/WDS/ODS/END segments); ODS RLE
     decode → 8-bit indexed plane; CLUT segments supply YCbCrCr palette
     entries → CPU-side convert to premultiplied RGBA (per HDMV spec the
     palette is YCbCr — [VERIFY exact colorimetry/limited-range handling]);
     epoch semantics = a display set flushes the previous one; window
     cropping/offsets applied CPU-side into the layer rect.
   - **DVB:** page/region/object model from PES subtitling segments (needs
     the TS demuxer to expose subtitle PIDs from the PMT subtitling
     descriptor); region CLUTs also YCbCr [VERIFY]; multiple regions per
     page compose into one layer; page timeouts define event ends.
   - **VobSub:** `.idx` (palette, timestamps) + `.sub` (MPEG-program-stream
     wrapped) — `vobsub` 0.2.3 [RV] already decodes; we harden sync +
     palette (doc 04 row 11).
3. **Timing correctness (the actual risk):** PGS epochs must survive seeks
   (a seek flushes the partial display set — a stale half-decoded overlay is
   the classic bug); DVB page acquisition-time vs PTS subtleties; VobSub
   sparse timestamps (interpolation rule documented in the format record).
   These are conformance-kit cases (doc 06 §7.1), not renderer cases.
4. **Memory budget [PROPOSAL]:** full-HD worst case is one 1920×1080 RGBA8
   layer ≈ 8.3 MB per active event; cap simultaneous layers (4) and evict
   atlas regions on epoch flush; upload is per *changed* event only —
   bitmap subs change at ~Hz so the per-frame cost is one textured quad.
5. **CI:** headless wgpu offscreen render (doc 03 §4.11f) comparing
   video+overlay compositing against a CPU reference compositor on
   synthetic paletted fixtures; property tests: dest-rect never inverted,
   palette conversion exhaustively sampled, epoch flush on seek asserted.
6. **Ownership:** the overlay-plane contract must be specified in doc 01
   **before Phase 6** lands any bitmap sub (doc 04 §5's tracking item,
   retained; this file supplies the technical content for that insert).

---

## 4. CC/608/708 extraction + rendering

**Where the bytes live (extraction surface):** NTSC line-21 VBI analog
heritage; in file formats, 608/708 caption data rides (a) MPEG-2 picture
header `user_data` (ATSC A/53 Part 4), (b) H.264 SEI
`user_data_registered_by_itu_t_t35_provider` (ATSC provider code, cc_type
0–3), (c) container-level side channels (e.g., some TS/MP4 caption tracks).
Our demuxers must expose these as a caption elementary stream feeding the
subtitle-parser plugin — extraction is *demuxer* work, rendering is *ours*.

**Spec availability (clean-room):** CEA-608-E / CEA-708-D (now CTA-608-E /
CTA-708-E) are published standards — **publicly purchasable, not free
full-text** from CTA [VERIFY current storefront/policy]; ATSC A/53 Part 4
(carriage in MPEG-2, incl. the 708 channel framing) is a **free download**
from ATSC; H.264 SEI carriage is in ITU-T H.264 (free). Specs are public ⇒
clean-room legal; only CTA purchase friction remains.

**608 complexity — honest size: M.** Two fields × two channels (CC1–CC4,
T1–T4 + XDS on field 2); three display modes (pop-on with buffer swap,
paint-on, roll-up with 2–15 row history); preamble/address codes; mid-row
attributes (colors, italics, underline); PACs (color+underline+indent);
special characters; extended character sets (FR/DE/ES/PT/…); the
non-obvious parts are the NTSC timing heritage (field/line parity rules)
and the pop-on buffer semantics. Rendering maps onto our text layers with a
TV-style fixed-grid look; roll-up is a scrolling window — representable in
the timed-event model as successive events [PROPOSAL: cap roll-up history
per spec].

**708 complexity — honest size: M–L.** 708 is closer to a small windowing
text system than a subtitle format: up to 63 services, a window model
(create/delete/define/toggle/setattributes/setpen), pen color/size/font/
attributes, per-window row/column addressing, deferred scroll,
`SetWindowAttributes` with border/word-wrap/justification/printing
direction, bitmap-plus-text command set, and a multi-plane glyph encoding
(G0/G1/G2/G3 with modified UTF-8). VLC's `cc` module handles both;
a faithful 708 renderer is a terminal-emulator-class build. **Plan
[PROPOSAL]:** 608 in Phase 6 (with extraction); 708 decode (command stream →
window state → our overlay layers) as its own follow-on, M–L, gated on
demand evidence [VERIFY usage share — doc 04 §8.6 item 7].

**Ecosystem check (2026-10-19):** `cea608` 0.0.1 is a caption *builder*
(verified: "Minimal CEA-608 pop-on caption sequence builder", MIT, 53 total
downloads) **[RV]** — useful only as a test-vector generator; `cc708`,
`eia608` NOT_FOUND **[RV]**; no maintained pure-Rust CEA decoder found ⇒
self-build from spec, as doc 04 row 12 already says (kept).

---

## 5. oxisynth LGPL classification (doc 04 §8.6 item 6)

**What it is (re-verified 2026-10-19):** OxiSynth — "pure safe Rust
SoundFont™ synthesizer, inspired by the widely known `FluidSynth`"
(repo README, master branch). crates.io: `oxisynth` **0.1.0**, published
**2025-05-25**, license field **LGPL-2.1**, 32 k total downloads **[RV]**.
Workspace also ships `soundfont` (SoundFont parsing), `oxisynth-chorus`,
`oxisynth-reverb` (independent effect crates); the `sf3` feature (compressed
SoundFont samples) adds a dependency on **`lewton`** (Vorbis) [RV, master
Cargo.toml]. Users: Neothesia (its origin), `microwave`, and `xsynth` (which
reuses the `soundfont` crate). WASM is a first-class target (README demo).
It is *only* the synthesizer — it consumes MIDI events; it does not parse
SMF files.

**Why it is in the subtitle section at all:** doc 04 §3 row "MIDI /
synthesized" named oxisynth alongside rustysynth; the audit item is ours, and
the MIDI *playback path* shares the subtitle wall (VLC's chain is SMF →
FluidSynth, both C today: `fluidsynth` plugin + freetype rendering of
nothing — the synth is audio-side, but the format entry lives with the
"decode" families). Recorded here once so doc 11 has a single anchor.

**LGPL-2.1 in an MIT repo — classification only, decision is doc 11's:**

- *Static linking (the Rust default):* LGPL §2 requires the ability to
  relink against a modified library (object files or source for the whole
  work, or an exemption). Rust's static compilation model makes the
  practical compliance story "ship source or object files for the combined
  binary" — a real burden our simple MIT distribution does not carry today.
- *Dynamic linking:* the classic LGPL boundary — load `oxisynth` as a
  `cdylib` at runtime (via `libloading`) so the app is a "work that uses"
  the library. Workable, but it complicates distribution (per-platform
  dylib shipping, symbol stability across an LGPL crate with no
  semver-stable C ABI [VERIFY]), conflicts with our single-binary
  simplicity, and needs unsafe loading code (AGENTS §3.3 — adapter-ADR
  territory).
- *Our repo policy:* ADR-006 fixes MIT. Whether LGPL *can* enter at all
  (either mode) is a **doc 11 dependency-policy decision** — flagged with a
  pointer, not decided here (task instruction; doc 04 §3.2's "license review
  before any use" stands).

**v1 recommendation [PROPOSAL]:** MIDI = `later`, **not** `blocked`, with
**rustysynth as the default v2 path** and **no oxisynth dependency**:

- `rustysynth` 1.3.7 (MIT, published 2026-10-01, actively maintained) is
  "a SoundFont MIDI synthesizer written in pure Rust… **Supports standard
  MIDI files** with additional features including dynamic tempo changing…
  **No dependencies other than the standard library**" **[RV]** — this
  *closes the SMF-parser gap* doc 04 estimated as an S self-build (audit
  item 6's "rustysynth SMF needs": answer = included in-crate; remaining
  open need = **SoundFont policy** only).
- SoundFont policy [PROPOSAL]: user-supplied `.sf2` (exactly like VLC's
  fluidsynth plugin requires a soundfont; VLC itself does not ship one);
  no bundled default font in v1 — bundling would add a licensing question
  (which font, what license) for zero parity value. `.sf3` support
  [VERIFY rustysynth sf3 status] is optional follow-on.
- If an earlier-than-v2 "something plays" is wanted: a tone-generator
  placeholder (S: oscillator + envelope + SMF timing via rustysynth's
  parser, square/sine voices, honestly labeled `midi = partial
  (placeholder synth)`) — offered as an option, not recommended for v1.
- oxisynth: remains on the **not-in-tree list pending doc 11's LGPL
  classification**; revisit only if rustysynth proves insufficient
  (e.g., chorus/reverb quality) after real use.

---

## 6. Remaining [VERIFY] ledger (this section)

1. **[VERIFY-P0]** Full transitive-graph + tarball purity audit of every
   crate named here (`ass-parser`, `ssa-parser`, `webvtt-parser`, `vobsub`,
   `subparse`, `rustysynth`, …) — doc 04 §8.6 item 10; all [RV] marks are
   existence/version/license checks only.
2. **[VERIFY-P0]** Renderer text-stack crate selection (rustybuzz/swash/
   cosmic-text class) — doc 01/03 scope; gates §2.2's Phase 5 subset and
   the ASS degrade-vs-render split.
3. **[VERIFY]** The "~95 % of styling value" share in doc 04 §5 — becomes
   measurable via §2.3's counters; replace the estimate with corpus data
   when the conformance kit runs (doc 04 §8.6 item 7 dependency).
4. **[VERIFY]** PGS/DVB palette colorimetry details (YCbCr limited-range
   conversion, HDMV vs DVB differences) against the specs during Phase 6
   implementation.
5. **[VERIFY]** MicroDVD default-fps policy (container hint vs user
   override) — needs a UX decision before the S self-build.
6. **[VERIFY]** CTA-608-E/708-D spec purchase/obtainability (current
   storefront policy) — gates nothing structural (A/53 Part 4 + H.264 are
   free), but the 608/708 self-build can't start without the documents in
   hand.
7. **[VERIFY]** 708 demand evidence (usage share) for the M–L build order.
8. **[VERIFY]** Ogg Kate pure-decoder search (doc 04 `[VERIFY-P0]` stands;
   not re-run today).
9. **[VERIFY]** ARIB STD-B24/B37 spec access + DRCS handling cost (census
   module `aribsub` disposition "later/blocked").
10. **[VERIFY]** Teletext (`zvbi` replacement) appetite — currently
    `blocked` with reason; needs an explicit v2 go/no-go recorded.
11. **[VERIFY]** rustysynth `.sf3` (compressed samples) support status —
    affects the optional follow-on in §5.
12. **[VERIFY]** doc 04 §5 table updates for census-derived formats
    (SVCD/CVD, scte27/scte18, textst, ARIB, Teletext) — Lead folds §1.1
    into doc 04 so the roadmap and census reconcile 1:1.

## 7. Evidence appendix (all external URLs fetched 2026-10-19)

crates.io API (`https://crates.io/api/v1/crates/<name>`; `/versions` for
license fields):

1. `srt` — 0.1.0, 2017-03-10, tytouf/srt-rs (stale).
2. `ass-parser` — 0.2.3, 2025-04-11, MIT OR Apache-2.0, Aavtic/ass_parser.
3. `ssa-parser` — 0.1.0, 2024-11-12, mikai233/subtitle_parser.
4. `subparse` — 0.7.0, 2021-04-11, MPL-2.0, kaegi/subparse.
5. `webvtt-parser` — stable 1.2.0 / 1.2.1-rc line, updated 2026-10-18, MIT,
   dmtrKovalenko/fframes.
6. `webvtt` — 0.2.0, 2023-04-28, bare-bones, no repo field.
7. `vobsub` — 0.2.3, 2017-05-22, CC0-1.0, emk/subtitles-rs.
8. `cea608` — 0.0.1, 2026-03-28, MIT, "Minimal CEA-608 pop-on caption
   sequence **builder**" (53 downloads).
9. `cc708`, `eia608` — NOT_FOUND (re-checked today).
10. `oxisynth` — 0.1.0, 2025-05-25, **LGPL-2.1**, PolyMeilex/oxisynth.
11. `rustysynth` — 1.3.7, 2026-10-01, MIT, sinshu/rustysynth.

GitHub raw fetches:

12. https://raw.githubusercontent.com/PolyMeilex/oxisynth/master/README.md —
    "pure safe Rust SoundFont™ synthesizer, inspired by FluidSynth"; WASM
    demo; users Neothesia/microwave/xsynth. (Note: default branch is
    `master`; `main` does not exist — citation hygiene.)
13. https://raw.githubusercontent.com/PolyMeilex/oxisynth/master/Cargo.toml
    and …/master/oxisynth/Cargo.toml — workspace layout (soundfont,
    chorus, reverb); `sf3 = ["lewton"]` feature; `license = "LGPL-2.1"`.
14. https://raw.githubusercontent.com/sinshu/rustysynth/main/README.md —
    "SoundFont MIDI synthesizer written in pure Rust, ported from
    MeltySynth"; "Supports standard MIDI files…"; "No dependencies other
    than the standard library".

Repo-internal evidence:

15. docs/inventory/modules-linux.json — subtitle category source (spu
    decoder 22 / sub source 7 / sub filter 1 / text renderer 2; §1.1).
16. docs/inventory/modules-summary.md §G3 — counting rule + 16-kind mapping
    (subtitle-parser ← spu decoders; subtitle-renderer ← text renderers +
    sub sources/filters).
17. docs/03-engine-plan.md §4.11 (RGBA-atlas compositing [PROPOSAL]) and
    §4.12 (timed-event model) — refined in §3 here, not restated.
18. docs/06-plugin-architecture.md §7.1/§7.7/§7.8 — conformance corpora,
    fuzz registry (`fuzz_sub_*` naming), report format where the
    unsupported-tag counters land (§2.3).
19. docs/14-risk-register.md — R-09 rows (trigger: unsupported-tag count),
    A9 acceptance (subset-not-parity), §7 R-09 narrative.
20. docs/04-codec-format-roadmap.md §5 table + gap 5/6/9 rows — the
    baseline this file audits; formats not re-searched today keep its
    2026-10-19 marks (notably Kate `[VERIFY-P0]`).

*Honesty statement: every [RV] claim was fetched live on 2026-10-19 with the
URL recorded above; capability descriptions are quoted from crate metadata or
READMEs, never inferred. Clean-room discipline held: no VLC, libass, or
FluidSynth source consulted — reference implementations appear only as
behavioral oracles in proposals. No git commit was made (AGENTS §3.8).*
