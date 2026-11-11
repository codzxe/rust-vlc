# deps/records/oxideav-mpeg4video-0.1.8.md — oxideav-mpeg4video 0.1.8 (default features)

> **Class:** P0 validate-then-admit SPIKE record (card
> 6.VTA-oxideav-mpeg4video, Phase 6 Wave 1; doc 11 §2 format, fields
> R1–R10 — rusty_vp9-0.1.3 is the closest analogue and this record
> follows its shape). MPEG-4 Part 2 / ASP (DivX/Xvid) is a top
> blocked-format (the DivX-era library) named by the Phase-5 exit gate.
> Evidence gathered 2026-10-26 in the build sandbox: crates.io API
> queries, tarball download + sha256 match, vendored-source scans,
> scratch `cargo tree`, an **eight-lane differential against ffmpeg
> 7.1.5's default `mpeg4` decoder driven through OUR OWN mp4 demuxer**
> (§D), and a hostile no-panic battery (§H). Same-author family as the
> just-BLOCKED oxideav-ac3 — measured on its own merits, not the
> family's. **Final acceptance and the allowlist row remain Lead
> decisions (doc 11 §3.1).**

## The pin (the whole point of this record)

- **Candidate shape: `oxideav-mpeg4video = { version = "=0.1.8" }`,
  default features (i.e. none — the only other feature is `nightly`,
  OFF).** NOT admitted this round: see R10.
- Version pin `=0.1.8` (exact) if ever admitted: 0.x crate, 13 releases
  in 5 months (R8) — API-churn risk until 0.2/1.0.

## R1 — Language & source (F1, F7)

- Pure Rust (manifest keywords: `pure-rust`; description: "Pure-Rust
  MPEG-4 Part 2 Video (ISO/IEC 14496-2) decoder and encoder"). Clean-room
  per README ("orphan-rebuild scaffold … under the workspace clean-room
  policy"). Vendored tarball: 71 `.rs` files, 44,919 code lines (+5,462
  comment lines); 0 `.c`/`.h`/`.S`/`.asm`/`.a`/`.so`, no binary blobs
  (only `.gitignore`, CI yml, `.cargo_vcs_info.json`).
- **No `build.rs`** (`build = false`). F2 vacuously clean.
- **0 `extern "C"`**; no FFI in either direction.
- `unsafe` surface: **0 tokens in the whole crate** (`rg -o unsafe
  src/ | wc -l` → 0) — stricter than rav1d (98) and rusty_vp9 (98);
  this crate IS effectively `forbid(unsafe)`-clean without the lint.
- Repo: https://github.com/OxideAV/oxideav-mpeg4video (publisher Mark
  Karpelès / OxideAV). LICENSE: **MIT**.

## R2 — Transitive graph (F5)

Scratch `cargo tree --edges normal` with the exact pin (harness at
`tools/vta/mpeg4`, committed):

```
oxideav-mpeg4video v0.1.8
└── oxideav-core v0.1.42          ← the ONE dependency
    ├── bytemuck v1.25.2
    ├── serde_json v1.0.154 → itoa, memchr, serde_core, zmij
    └── thiserror v2.0.21 (proc-macro; already in our graph)
```

New lockfile nodes vs our workspace graph: **oxideav-mpeg4video,
oxideav-core, bytemuck, serde_json, itoa, memchr, zmij** (≈7; thiserror/
serde_core/proc-macro chain already present via plugin-api). No `-sys`,
no git deps, no C toolchain anywhere in the new subgraph. serde_json is
a HARD dep of oxideav-core (same shape the ac3 record flagged — no
feature shape avoids it; it is not on the decode path we measured).

## R3 — Build scripts (F2)

Absent (R1). No `cc`, no codegen, no downloads at build time.

## R4 — Native links (F3, F4, F6)

No `links=`, no `#[link]`, no `-l`, no pkg-config, no binaries. F3/F4/F6
all NO.

## R5 — Targets

Pure Rust, no target-specific gates seen in the decode path (no
`core::arch` at all — 0 unsafe). x86_64 evidence host; other arches
compile the same code. `[VERIFY-P1: wasm32 build at the first wasm gate
run — standing item shared with the symphonia/rav1d/rusty_vp9
admissions.]`

## R6 — Actual feature coverage (against this card's needs)

- Used surface: `decoder::Mpeg4VideoDecoder::{new, with_options,
  decode, decode_with_pts, flush, vol}` +
  `framestore::DecodedFrame` (luma/cb/cr_samples, width/height —
  macroblock-padded — coding_type, pts, pts_ticks) + `compat::
  DecodeOptions::{spec, ecosystem}`. The direct `make_decoder` registry
  endpoint (extradata priming, packet send/receive, pixel-count DoS cap)
  was NOT exercised this round — the harness uses the direct
  `Mpeg4VideoDecoder` endpoint with explicit extradata priming.
- Measured end-to-end: I-VOP, P-VOP, B-VOP (display reorder + §6.3.5
  tick times), packed bitstream (DivX N-VOP interleave — see §D for the
  found defect), odd display dims (1×1 lane, macroblock padding +
  chroma ceil), mid-stream VOL parameter change (dims + config, lane
  0093).
- Explicitly NOT used: the whole `encoder/` half, short-video-header
  (H.263-style) mode, `nightly` feature, ecosystem-compat beyond one
  probe lane (§D note).

## R7 — License

**MIT** (LICENSE + manifest + index) — on doc 11 §3.2's allowlist.
Transitive: oxideav-core MIT; bytemuck (MIT/Apache/Zlib); serde_json +
itoa + memchr + zmij (MIT/Apache); thiserror (MIT/Apache). All
permissive. Unsafe note: 0 in this crate; 11 `unsafe` tokens in
oxideav-core 0.1.42, all in `src/arena/{mod,sync}.rs` (Send/Sync impls
+ allocator ptr casts) — the arena is NOT referenced by the decode path
we measured (same finding as the ac3 record, same core version).

## R8 — Maintenance & security record

- 0.1.8 published **2026-09-12**; `rust_version` 1.80 (manifest) — but
  the graph-effective MSRV is oxideav-core 0.1.42's edition-2024 →
  **1.85** (our pin 1.99.1 ≥ 1.85 ✓). Not yanked.
- Release history: 0.1.0 2026-04-17 → 0.1.8 2026-09-12, 13 versions in
  5 months — actively iterating. Downloads: 2,349 total (1,368 recent)
  — low uptake, single-author crate (bus-factor risk, same honest note
  as ac3/rusty_vp9; the exact pin + re-trigger rule is the mitigation).
- OSV API (POST /v1/query, 2026-10-26):
  `{"package":{"name":"oxideav-mpeg4video","ecosystem":"crates.io"}}` →
  `{}` — zero advisories (also true of the 7 new transitive nodes,
  checked as a graph: none carry advisories at these versions).
- Upstream claims bit-exactness vs "a black-box reference decode using
  the ideal (floating-point) Annex A.1 IDCT" over 24 fixture streams,
  with a documented ±1 "near-tie" budget — **NOT taken on faith; §D is
  what we measured ourselves** (against ffmpeg's integer-IDCT oracle,
  a different and stricter-for-us reference than theirs).

## R9 — Provenance

Tarball fetched via the crates.io API; downloaded-file sha256
`d97ea47f6613029b60f1f743dda10dddc4bc993f7259f2ec52577e6c3dbeb384`
recomputed in-sandbox == index `cksum` byte-for-byte. Index confirms
`deps: [oxideav-core ^0.1]`, `features: {default: [], nightly: []}`,
`build = false`. `[VERIFY-P1: publisher identity cross-check at
admission time, cpal-record convention.]`

## R10 — Verdict

**BLOCKED (differential).** Purity is the strongest of any codec
candidate measured so far (pure Rust, ZERO unsafe in the crate, no
build.rs, no FFI, MIT, 1-dep graph) and the hostile battery is
panic-free (§H) — but the admission gate is the differential, and it
fails the house bit-exact bar (rusty_vp9 precedent) twice over:

1. **No lane except the degenerate 1×1 is bit-exact.** 7/8 lanes land
   in a tolerance class of max Δ 1–3 with 0.6–4.7 % of samples
   differing (overwhelmingly Δ=1). A negative control kills the easy
   excuse: ffmpeg's own `-idct int|simple|xvid` variants are
   **byte-identical to each other** on these files (0 differing bytes),
   so this envelope is crate-vs-ffmpeg deviation, not ffmpeg-internal
   IDCT variance. It is consistent with the crate's ideal-IDCT rounding
   vs ffmpeg's integer IDCT — and MPEG-4 Part 2's IDCT is famously
   implementation-defined — but a tolerance envelope is a Lead
   decision, not a spike agent's (the ac3 BLOCKED precedent at ±316 LSB
   vs this crate's ±3 are different magnitudes, yet the bar is the bar).
2. **Packed bitstream emits duplicate frames** (§D lane 0117, the
   historically-hard case): 9 extra byte-identical P frames (37 output
   vs ffmpeg's 28) — the DivX N-VOP placeholder is decoded as an extra
   output on top of the correctly-reordered anchor. Packed DivX is the
   era-dominant encoding; this is a presentation-layer correctness bug
   for exactly the library MPEG-4 admission is for.

**Unblock conditions (re-evaluation path):**
- (a) Lead ratifies a documented per-pixel tolerance envelope for
  MPEG-4 Part 2 (proposed from this evidence: max Δ ≤ 3, ≤ 5 % of
  samples, with the ffmpeg-idct-variant negative control recorded) —
  a deliberate doc-11 §3.x policy row, not a default.
- (b) The packed-bitstream N-VOP double-emission is fixed upstream, OR
  the demuxer/codec wrapper adopts an `mpeg4_unpack_bframes`-equivalent
  plus a duplicate-frame policy, and lane 0117 re-measures 28/28.
- (c) Tool coverage widens before any wider claim: qpel, GMC/S-VOP,
  interlaced (§7.7.x), `mpeg_quant` method-1, data-partitioned, and
  RVLC lanes (the crate's own README documents root-caused spec-vs-
  ecosystem divergences in interlaced-direct and mpeg-quant paths —
  unmeasured by us).
- A version bump or feature change re-triggers this record per doc 11
  §3.7.

---

## D — Differential evidence (the honest core)

Method (every command re-usable from `tools/vta/mpeg4/README.md`):
lanes demuxed by **OUR** `crates/demux/src/mp4` (`Mp4Demuxer` over
`mem_stream`) — video packets fed to `oxideav-mpeg4video` 0.1.8
literal-spec mode, frames cropped to the VOL display dims and packed
tightly (Y/Cb/Cr rows — byte-layout identical to ffmpeg rawvideo
`yuv420p`). Reference: ffmpeg 7.1.5 default `mpeg4` decoder, `-map
0:v:0 -f rawvideo -pix_fmt yuv420p`, plus `-f framehash` md5s.

**Container honesty (three demuxer facts recorded for the M-wave):**
1. Our AVI demuxer's v0 codec filter (`vids`+`MJPG` / `auds`+PCM only)
   FILTERS OUT mpeg4 tracks — `0117-packed-bs.avi` direct returns
   "no-supported-streams". The AVI lanes therefore ran as honest
   `-c copy` remuxes AVI→MP4 (elementary-stream-preserving; proven by
   ffmpeg decoding original and remux to byte-identical raw output on
   0117: 28 frames, identical md5s) through OUR mp4 demuxer.
2. Our mp4 demuxer does not yet surface codec config (esds), so the
   VOL run was primed from ffprobe-extracted `extradata` hex
   (`--extradata`); the VIDEO SAMPLES still came through our demuxer.
   0117 needed no extradata (in-band VOL in the first keyframe).
3. `Mpeg4VideoDecoder` consumes start-code-scanned units per packet,
   so mp4 sample framing vs AVI chunk framing is immaterial to it.

| lane | source | dims | frames mine=ref | identical | differing samples | max Δ | stream md5 |
|---|---|---|---|---|---|---|---|
| A g_base | testsrc2 3 s `-c:v mpeg4` | 320×240 | 75 = 75 | 0/75 | 404,340 / 8,640,000 (**4.68 %**) | **3** (Δ1: 403,849 Δ2: 478 Δ3: 13) | NO |
| B g_bf2 | testsrc2 3 s `-bf 2 -g 12` (b-frame reorder) | 320×240 | 75 = 75 | 0/75 | 388,697 (4.50 %) | **2** | NO |
| C 0117 | corpus `0117-packed-bs.avi` remux (PACKED BS) | 160×120 | **37 ≠ 28** | 0/28¹ | 27,212 / 806,400 (3.38 %)¹ | **2**¹ | NO |
| D 0097 | corpus `0097-mp4-video.mp4` | 64×48 | 37 = 37 | 0/37 | 1,702 (0.998 %) | **1** | NO |
| E 0045 | corpus `0045-mp4-video.mp4` | 176×144 | 37 = 37 | 0/37 | 8,762 (0.623 %) | **1** | NO |
| F 0132 | corpus `0132-avi-av.avi` remux | 120×90 | 41 = 41 | 0/41 | 5,610 (0.845 %) | **2** (Δ2: 3) | NO |
| G 0107 | corpus `0107-odd-dims.avi` remux | 1×1 | 25 = 25 | **25/25** | 0 | **0** | **YES** |
| H 0093 | corpus `0093-param-midchange.mp4` (VOL change mid-stream) | 240×180 | 63 = 63 | 0/63 | 91,714 (2.25 %) | **2** | NO |

¹ Lane C measured against **ffmpeg decoding the ORIGINAL AVI** (28
frames); the crate emitted **37 = 28 + 9 byte-identical duplicate P
frames** (indices 4, 8, 12, … — one per GOP: the packed-bitstream
N-VOP placeholder decoded as an extra output frame ON TOP of the
correctly-reordered anchor; 6-byte N-VOP samples visible in the packet
dump). After dropping the 9 consecutive duplicates: 28/28 frame-count
parity, content in the same ±2 tolerance class as every other lane.
The remux itself decodes to 28 frames under ffmpeg (byte-identical to
the original), so the extras are the crate's. Also recorded: with
`decode_with_pts` on packed samples the container pts attaches to
every frame of the sample (packed semantics), and the emitted
container-pts sequence is non-monotone (B 3072, B 4096, P 3072 …);
the §6.3.5 `pts_ticks` sequence is non-decreasing once the duplicates
are dropped. **B-VOP reordering itself is correct** on lane B (75/75
count parity, monotone pts, tolerance-class content).

**Classification: TOLERANCE (not bit-exact) on 7/8 lanes — max Δ 1–3,
0.6–4.7 % of samples differing, Δ=1-dominated — plus a frame-count
divergence (duplicate emission) on the packed-bitstream lane.** One
degenerate lane (1×1) is bit-exact. Negative control: ffmpeg
`-idct int` vs `-idct simple` vs `-idct xvid` on lanes A/D are
byte-identical to each other (0 differing bytes) — the tolerance
envelope is NOT ffmpeg-internal IDCT variance. Ecosystem-compat mode
probed on lane D: byte-identical result to spec mode (same 1,702
Δ=1 samples) — the two documented compat divergences (interlaced
direct, mpeg-quant) are not exercised by these lanes.

Harness honesty: two harness bugs were caught and fixed during the
measurement, both recorded because the discipline is the point — (1)
odd-dims chroma was cropped with floor-division (`dw/2`), producing
0-size chroma planes on the 1×1 lane (fixed to ceil; lane G then
measured bit-exact); (2) the driver's max-delta accumulator was never
updated (reported 0 while deltas existed; caught by a direct python
byte-compare cross-check, fixed, all lanes re-measured). Decode errors:
**0 across all 8 lanes** (every packet Ok; frame counts equal ffmpeg's
everywhere except the packed lane's duplicates).

## H — Hostile input (no-panic contract)

Battery (`--hostile`, extradata-primed where the lane carries it;
catch_unwind + Result), lanes B (g_bf2), C (0117 remux, in-band VOL),
H (0093): truncated-keyframe **off-by-one sweep 0/9595, 0/2872, 0/6617
prefixes panicked**; zero-length → Ok(0 frames); garbage-64B → Ok(0);
corrupted VOP header (start-code+1 byte flipped) → Ok(1 frame) on the
VOL-primed lanes (typed-lenient class — the corrupted byte lands in
header bits the decoder tolerates; flagged for the M-wave differential
to bound); garbage-then-real → **recovers** (6 frames 0 errs primed /
8 frames in-band-VOL lane); truncated mid-stream → Ok(4–5 frames) + 1
typed Err. **0 panics anywhere (18 case-runs + 3 full truncate
sweeps).**

## G — Honest gaps (what this record does NOT claim)

- No lane ran through our AVI demuxer (v0 codec filter excludes
  mpeg4 — R2/D honesty #1); AVI corpus lanes are `-c copy` remuxes
  with ES-preservation proven on 0117 only. MKV mpeg4 lanes (3 in
  corpus, incl. 77 s/71 s drift files) not run at all.
- Extradata primed from ffprobe (our mp4 demuxer doesn't surface esds
  config yet) — an M-wave demuxer follow-up either way.
- Unmeasured MPEG-4 tools: qpel, GMC/S-VOP (sprite), interlaced
  (§7.7.1/§7.7.2 incl. the crate's documented interlaced-direct spec
  divergence), `mpeg_quant` method-1, data-partitioned + RVLC,
  short-video-header (H.263) mode, S(S)-VOP resync-marker streams,
  >8-bit `not_8_bit`, newpred, scalability. The crate's 24-fixture
  conformance claim is upstream's number (vs THEIR ideal-IDCT oracle).
- B-frame `pts_ticks`/display order verified on generated lane B and
  (post-dedupe) lane C only; no seek/reset (`Mpeg4PacketDecoder`)
  testing; `make_decoder` registry endpoint unexercised.
- Decode throughput NOT benchmarked (rav1d convention: a bench card
  owns the numbers; lanes here are 1–4 s CPU-cheap files).
- Maintenance lens: 0.1.x, single author, 2,349 downloads — young;
  the `=0.1.8` pin + re-trigger rule is the mitigation if ever
  admitted.

---

## Addendum (2026-11-09) — wave assessment: remains BLOCKED; conformance-gated unblock plan (issue #7, W16-EE)

Assessed in the issue #7 admissions wave alongside the four admitted
codecs. Outcome: **BLOCKED stands — no admission, no Cargo.toml edit,
no `crates/codecs` wiring.** The record's own differential evidence
(§D/§R10) fails the house bit-exact bar twice over: (1) **no lane
except the degenerate 1×1 is bit-exact** — 7/8 lanes at max Δ 1–3
with 0.6–4.7 % of samples differing (the ffmpeg-idct-variant negative
control pins this as crate-vs-ffmpeg deviation, consistent with
MPEG-4 Part 2's implementation-defined IDCT); (2) the packed-bitstream
lane (0117, DivX N-VOP) **double-emits — 37 output frames vs
ffmpeg's 28** — a presentation-layer correctness bug in exactly the
era-dominant encoding this codec would be admitted for.

**Conformance-gated unblock plan [PROPOSAL — Lead] (admission stays
Lead-gated per the wave rules):**

1. **N-VOP gate (hard):** the packed-bitstream double-emission is
   fixed upstream, OR the wrapper adopts an
   `mpeg4_unpack_bframes`-equivalent + duplicate-frame policy, and
   lane 0117 re-measures 28/28 (§R10 condition b). No admission
   without this.
2. **Tolerance gate:** the Lead ratifies a documented per-pixel
   tolerance envelope for MPEG-4 Part 2 (proposed from this record's
   evidence: max Δ ≤ 3, ≤ 5 % of samples, with the idct-variant
   negative control recorded) as a deliberate doc-11 §3.x policy row
   (§R10 condition a) — not a default.
3. **Coverage gate:** qpel, GMC/S-VOP, interlaced, `mpeg_quant`
   method-1, data-partitioned and RVLC lanes measured before any
   wider claim (§R10 condition c; the crate's README documents
   root-caused divergences in the interlaced-direct and mpeg-quant
   paths — unmeasured by us).
4. **Pin gate on any future admission:** `oxideav-mpeg4video =
   "=0.1.8"` + transitive `oxideav-core`, re-verified on every bump.

Mitigating fact kept on record: purity is the strongest of any codec
candidate measured (zero unsafe, 1-dep graph, MIT) — if all four
gates clear this is the natural ProRes-era companion candidate; the
blocker is conformance, not supply chain.
