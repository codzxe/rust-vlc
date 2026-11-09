# deps/records/oxideav-ac3-0.0.11.md — oxideav-ac3 0.0.11 (purity + differential, card 6.VTA-oxideav-ac3)

> **Class:** P0 validate-then-admit SPIKE (Phase 6 W1). Candidate AC-3
> (Dolby Digital, ATSC A/52) decoder — the Phase-5 exit gate named
> oxideav-ac3 the ac3-decode candidate. This is NOT a wiring card:
> oxideav-ac3 was not added to `crates/codecs`. Everything below was
> measured in this session (2026-10-29) in the build sandbox against
> oxideav-ac3 **0.0.11** (latest) with a harness-only AC-3 syncwalker
> (0x0B77 + A/52 Table 5.18 frame-size formula — never production code)
> supplying the frames. Harness: `tools/vta/ac3/` (standalone,
> non-workspace; ARCHIVED-NOT-ADMITTED).

## R1 — Version, language & source

- **oxideav-ac3 0.0.11** (crates.io, published 2026-09-01; release
  line 0.0.1 → 0.0.11 over 2026-04-23 → 2026-09-01, 11 releases in
  ~4.5 months; 7,142 downloads / ~6,310 recent). Author **Mark
  Karpeles**, org OxideAV (github.com/OxideAV/oxideav-ac3). License
  **MIT** (manifest `license` key + LICENSE in tarball). Edition 2021,
  `rust-version = "1.80"` — but the graph's effective MSRV is
  **1.95** (oxideav-core 0.1.42 is edition 2024, `rust-version =
  "1.95"`); ≤ our 1.99.1 pin. Maintenance signals: active line, huge
  CHANGELOG (188 kB), ships INVESTIGATION_R12/13/14 engineering
  docs, 12 examples + 9 integration tests incl. ffmpeg-differential
  and robustness tests, plus an ENCODER and E-AC-3 (Annex E) decode.
- Pure Rust: **0 native files** (scan for .c/.cpp/.h/.S/.asm/.a/.so
  → none), **no build.rs** (`build = false`), no `links=`, no
  `extern "C"`/`extern "system"`/`#[link]` (grep → 0), no blobs.
- **unsafe: 0 in oxideav-ac3** (the single `rg unsafe` hit is a doc
  comment asserting "no unsafe code"); **11 in oxideav-core**, all in
  `arena/{mod,sync}.rs` (a bump allocator: 2× `unsafe impl
  Send/Sync`, `alloc_zeroed`/`dealloc`/`write_bytes`, 4 pointer
  casts). The ac3 decode path does **not reference the arena** (grep
  → 0 hits in oxideav-ac3/src) — it exchanges `Vec`-backed
  Packet/Frame only.

## R2 — Transitive graph

`oxideav-ac3 0.0.11 → oxideav-core 0.1.42` (non-optional, normal) →
bytemuck 1.25.2, serde_json 1.0.154 (itoa, memchr, serde_core, zmij),
thiserror 2.0.21 (+ thiserror-impl, proc-macro2, quote, syn,
unicode-ident). **11 locked packages, no default-feature trap** (the
crate has no feature flags at all; oxideav-core's only feature
`json-options` is a no-op alias — serde_json is a hard dep because
core's FilterFactory signature uses `serde_json::Value`). Licenses:
MIT ×3 (oxideav-ac3, oxideav-core, zmij), MIT OR Apache-2.0 (thiserror
family, serde_json family), Unlicense OR MIT (memchr), Zlib OR
Apache-2.0 OR MIT (bytemuck), (MIT OR Apache-2.0) AND Unicode-3.0
(unicode-ident) — all permissive, no copyleft.

Supply-chain weight note: a decoder pulling serde_json + proc-macro
syn chain for a `FilterFactory` type it never uses on the decode path
is heavier than the ruopus precedent (zero deps), but there is no
feature combination that avoids it.

## R3/R4 — Build scripts & native links

None. No build.rs, no links=, no extern declarations, no blobs, no
pkg-config. F2/F3/F4/F6 all NO.

## R5 — Targets

Pure Rust, no `cfg(target_*)` gates in the decode path, no_std not
claimed. wasm32 build NOT measured (gap).

## R6 — API surface (relevant to a would-be wiring unit)

- `oxideav_ac3::decoder::make_decoder(&CodecParameters) ->
  Result<Box<dyn Decoder>>` (oxideav-core `Decoder` trait:
  `send_packet`/`receive_frame`/`flush`/`reset`). Same struct decodes
  AC-3 and E-AC-3, dispatching on per-packet bsid. Variants:
  `make_decoder_ltrt` (§7.8.2 LtRt downmix), `make_decoder_with_drc`
  (DRC regime; default = `DrcSettings::line_out` — full `dynrng`, no
  dialnorm normalisation), `make_eac3_decoder_with_joc`.
- `CodecParameters::audio(CodecId::new("ac3"))` with `channels=None`
  → passthrough of the bitstream's native channel count (measured 2ch
  and 6ch lanes).
- Output: **s16le interleaved only** (`Frame::Audio.data[0]`, 1536
  samples/ch per syncframe — §8.2.1.2). Packing is
  `(x * 32_767.0).clamp(-32_768, 32_767) as i16` — **truncating
  quantisation at scale 32767**, vs ffmpeg's fltp→s16
  `lrintf(x*32768)` round-to-nearest: even a bit-identical float
  decode would show ±1 LSB from this alone. No f32 output API.
- Channel order on the 5.1 lane matched ffmpeg directly (FL FR FC LFE
  BL BR) — no permutation needed (`wave_order.rs`).
- 44.1 kHz streams: frame size alternates 417/418 words (frmsizecod
  rows 20/21) — handled transparently by the decoder; the syncwalker
  measured 834/836-byte strides, 0 resyncs on every lane.

## D — Conformance differential (measured this session)

Pipeline: harness syncwalker → `make_decoder` (passthrough channels)
→ s16le; reference `ffmpeg 7.1.5 -i F -map 0:a -c:a pcm_s16le -f
s16le` (native ac3 decoder; `-drc_scale` default 1 — measured
byte-identical output at drc_scale 0 on the corpus lane, so DRC is
not a confound there). Full-stream compare, min-length prefix (all
lanes produced identical total byte counts — no length skew).

| Lane | ch / rate / bitrate | smp/ch | dec err | exact | differing | **max abs Δ (LSB)** | verdict |
|---|---|---|---|---|---|---|---|
| **corpus 0001-audio-only.ac3** (white noise, 44.1k) | 2 / 44100 / 192k | 49,152 | 0 | 2,759 | 95,545 (97.2%) | **316** | **DIVERGENT** |
| gen g_st192_44k (dual sine, 44.1k) | 2 / 44100 / 192k | 265,728 | 0 | 225,672 | 305,784 (57.5%) | **5** | ±1-LSB float class |
| gen g_st384_48k_noise (white noise) | 2 / 48000 / 384k | 288,768 | 0 | 218,071 | 359,465 (62.3%) | **4** | ±1-LSB float class |
| gen g_51_448_48k (5.1 sine) | 6 / 48000 / 448k | 288,768 | 0 | 1,554,410 | 178,198 (6.2%) | **2** | ±1-LSB float class |
| gen g_mono96_48k (sine) | 1 / 48000 / 96k | 288,768 | 0 | 122,627 | 166,141 (28.8%) | **3** | ±1-LSB float class |
| gen g_st96_48k (sine+noise mix) | 2 / 48000 / 96k | 288,768 | 0 | 130,726 | 446,810 (77.2%) | **202** | **DIVERGENT** |
| gen g_51_256_48k (5.1 sine) | 6 / 48000 / 256k | 288,768 | 0 | 1,563,876 | 168,732 (5.8%) | **2** | ±1-LSB float class |
| gen g_noise192_48k (white noise) | 2 / 48000 / 192k | 53,760 | 0 | 3,664 | 103,856 (96.6%) | **431** | **DIVERGENT** |

No lane is bit-exact (raw MD5s differ everywhere; representative:
corpus mine `2d06755425357b97…` vs ff `c330a49513069688…`).

**Characterisation of the divergent class** (measured with the
`cplprobe` side-info census + per-frame delta walk): the 3 divergent
lanes are exactly the **low-bitrate (≤192k) stereo broadband** lanes
(white noise @192k ×2 rates, sine+noise mix @96k); every divergent
lane has coupling in use (`cplinu` on every block) AND broadband
content. Coupling alone does NOT predict divergence — the sine lanes
@192k/256k/448k also run fully coupled yet stay within ±5 LSB, and
the noise @384k lane is uncoupled and clean. `dithflag` is 1 on
every block of every lane (non-discriminating). RMS parity on the
corpus lane is exact (mine 7752.39 vs ff 7753.09) → no gain/DRC/dialnorm
skew; deltas grow mid-frame from identical leading samples. Root
cause **not definitively isolated** this session; the evidence
pattern is consistent with (a) the spec-permitted dither freedom —
A/52 §7.3.4 "any reasonably random sequence may be used" and the two
implementations use different PRNGs (crate: 16-bit LFSR seed
0x1234; ffmpeg: its own LCG) whose difference is large only where
bap=0 mantissas carry big quantiser steps (low bitrate + broadband),
and/or (b) a coupling-path (§7.4.3 coordinate recomposition)
rounding difference. Either way it exceeds the house ±1-LSB
differential tolerance on 3 of 8 lanes, including **the corpus
lane**.

## H — Hostile input (measured this session)

Warm decoder + `catch_unwind`, 6 cases: truncated frame →
`Err(InvalidData "ac3: packet short: frame_length=834 pkt_len=417")`;
garbage bytes mid-stream (200 B XOR 0xA5) → **Ok, full 1536-sample
frame** (CRC is not enforced on the default path — §6.1.2 permits
lenient either-CRC policy; a wrapper wanting strict mode must use
`verify_packet_crc`); zero-length → `Err(InvalidData "packet too
short for syncinfo")`; reserved fscod (=3) → `Err` (surfaced as
eac3 strmtyp reserved after the AC-3 header reject); frmsizecod=63 →
`Err`; syncword + 4000×0xEE garbage → `Err`. **0 panics / 6 cases.**

## Verdict: **BLOCKED**

Purity is clean (pure Rust, 0 unsafe in the ac3 crate, 11 confined
unsafe lines in core's unused-on-this-path arena, no build scripts,
all-permissive 11-package graph, no feature traps) and the hostile
contract is fully Result-based — but the **differential gate
fails**: the corpus lane (and 2 of 7 generated lanes) decode
202–431 LSB away from ffmpeg 7.1.5 with 77–97% of samples
differing — three orders of magnitude past the ±1-LSB house bar
(mp3/symphonia precedent) and far beyond the ruopus float-decoder
class (≤1.2e-05 ≈ 0.4 LSB). Until the divergence is root-caused and
either fixed upstream or shown to be spec-permitted implementation
freedom **and** the Lead ratifies a dither-tolerant AC-3 admission
bar, this crate must not be wired into `crates/codecs`.

Unblock conditions (all three required for re-evaluation):
1. Root-cause the ≥200-LSB class on low-bitrate coupled broadband
   lanes (dither PRNG vs coupling recomposition) — the crate's own
   ffmpeg-differential test suite is the natural starting point.
2. If spec-permitted freedom (dither): Lead decision + a
   re-measured differential with a dither-blind metric (e.g. spectral
   difference), documented in this record's format.
3. Pin `oxideav-ac3 = "=0.0.11"` + `oxideav-core` transitively
   (graph MSRV 1.95 must stay ≤ our pin; re-verify on any bump).

Honest gaps: E-AC-3 (Annex E) decode not measured (bonus capability,
separate card if ever needed); the crate's encoder not measured;
32 kHz streams not in the lane set; DRC modes beyond default
(line_out) and LtRt/downmix paths not differentially tested;
wasm32 not measured; single-author 0.0.x line (API churn risk);
f32 output unavailable (s16-only, truncating pack — any consumer
wanting float must accept the crate's quantisation); root cause of
the divergent class not definitively isolated (see §D).

---

## Addendum (2026-11-09) — wave assessment: remains BLOCKED; conformance-gated unblock plan (issue #7, W16-EE)

Assessed in the issue #7 admissions wave alongside the four admitted
codecs. Outcome: **BLOCKED stands — no admission, no Cargo.toml edit,
no `crates/codecs` wiring.** Nothing in this session changes §D: the
record's own differential numbers (ffmpeg 7.1.5 reference) fail the
house bar on 3 of 8 lanes, including the corpus lane —
**202–431 LSB max deviation with 77.2–97.2 % of samples differing**
(vs the ±1-LSB mp3/symphonia precedent and the ruopus ≤1.2e-05 float
class); five lanes sit in a ±5-LSB tolerance class that no Lead
policy row currently ratifies for AC-3.

**Conformance-gated unblock plan [PROPOSAL — Lead] (admission stays
Lead-gated per the wave rules):**

1. **Root-cause gate (hard):** isolate the ≥200-LSB class on
   low-bitrate coupled broadband lanes — dither-PRNG freedom
   (A/52 §7.3.4) vs coupling-path (§7.4.3) recomposition rounding —
   using the crate's own ffmpeg-differential suite as the starting
   point (§Verdict condition 1). No re-evaluation before this lands.
2. **Ratification gate:** if (and only if) root cause is
   spec-permitted freedom, the Lead ratifies a dither-blind AC-3
   admission metric (e.g. spectral-difference envelope) as a doc-11
   §3.x policy row; a re-measured differential in this record's
   format is the evidence artifact (§Verdict condition 2).
3. **Pin gate on any future admission:** `oxideav-ac3 = "=0.0.11"` +
   transitive `oxideav-core` (graph MSRV 1.95 ≤ our pin), re-verified
   on every bump (§Verdict condition 3).
4. **Cost note for planning:** supply chain is heavier than the
   admitted-codec precedent (11 packages incl. a serde_json/syn chain
   unused on the decode path, §R2) — worth weighing if gate 2 passes.

Re-trigger rule: none — nothing was admitted; a new oxideav-ac3
release re-opens the record per doc 11 §3.7 only if the Lead asks
for a re-evaluation.
