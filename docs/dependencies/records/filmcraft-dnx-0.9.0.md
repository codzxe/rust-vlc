# deps/records/filmcraft-dnx-0.9.0.md — filmcraft-dnx 0.9.0 (default-features OFF)

> **Class:** P0 validate-then-admit SPIKE record (card 6.VTA-filmcraft-dnx,
> Phase 6 Wave 1; doc 11 §2 format, fields R1–R10 + F1–F7 — rusty_vp9-0.1.3
> is the closest analogue and this record follows its shape). DNxHD was the
> pro-editing format gap named by the Phase-5 exit gate ("filmcraft-dnx").
> Evidence gathered 2026-10-26 in the build sandbox: crates.io sparse index
> + API queries, tarball download + sha256 match, vendored-source scans, a
> scratch `cargo tree`, a **five-lane differential against ffmpeg 7.1.5
> `dnxhd`** (raw VC-3 ES, `-f dnxhd` muxer), and a hostile-input battery.
> **Final acceptance and the allowlist row remain Lead decisions (doc 11
> §3.1; `boundaries.toml` + `deny.toml` are Lead-owned and untouched by
> this card — same posture as every codec admission since claxon).**

## The pin (the whole point of this record)

- **Admission shape: `filmcraft-dnx = { version = "=0.9.0",
  default-features = false }`.** The crate's one feature is `threads`
  (default ON, adds `rayon` — macroblock scan-line parallelism). We pin it
  OFF: the measured decode path is single-threaded (`decode_frame`,
  `DecodeOptions { threads: false }` semantics when the feature is
  absent), and the pin adds **no `rayon` node** to our lockfile. If the
  M-wave later wants threaded decode, that is a feature change and
  re-triggers this record (doc 11 §3.7).
- Version pin `=0.9.0` (exact): first-and-only release, 0.9.x line (R8) —
  API-churn risk until 1.0, mitigated by the exact pin.

## Candidate search (measured, cited — card scope item 1)

Queries run 2026-10-26 against the crates.io search API (`q=dnxhd`,
per_page=20; 9 total results) + GitHub org listing. Every result triaged:

| candidate | version | verdict of triage |
|---|---|---|
| **filmcraft-dnx** | 0.9.0 | "Clean-room Avid DNxHD / DNxHR (SMPTE ST 2019-1 VC-3) decoder and encoder" — **SELECTED** (permissive licence, `unsafe_code = "forbid"`, oracle-tested upstream) |
| soundkit-dnx | 0.13.3 | decoder, but **LGPL-2.1-or-later** (wavey-ai/soundkit) — off our permissive-only allowlist class; not pursued |
| veloster-dnx | 0.1.10 | header parser / compression-ID analyzer only — NOT a decoder |
| veloce-dnx | 0.1.7 | header parser only — NOT a decoder |
| ff-encode / ff-format / oximedia-transcode / mediainfo-rust / revelo-parsers-video | — | ffmpeg wrappers / metadata / parsers — not VC-3 decoders |

Additional queries with zero usable hits: crates.io `q=vc3`, `q=vc-3`.
GitHub: no `filmcraft` org visible via the public repos API (crate lists
no repository URL — see R9 provenance note); the crate is published from
a private source per its clean-room README.

## R1 — Language & source (F1, F7)

- Pure Rust; **`[lints.rust] unsafe_code = "forbid"`** in the manifest —
  lint-enforced, not just convention. Vendored tarball: **9 `.rs` files in
  `src/`, 0 `.c`/`.h`/`.S`/`.asm`/`.a`/`.so`**, no binary blobs
  (`src/ffmpeg_probe.rs` shells out to ffmpeg only in `#[ignore]`d
  hand-run probes — not on the decode path).
- **No `build.rs`** (`build = false`) — F2 vacuously clean.
- **0 `unsafe` tokens and 0 `extern "C"`** across `src/` (rg), for BOTH
  `filmcraft-dnx` and `filmcraft-bitstream`. This is stricter than the
  rav1d/rusty_vp9 precedent (those admitted intrinsic-only `unsafe`).
- Repo: none published (R9). LICENSE: MIT OR Apache-2.0.

## R2 — Transitive graph (F5)

Scratch `cargo tree --edges normal` with our exact pin (harness at
`tools/vta/dnx`, committed):

```
vta-dnx
└── filmcraft-dnx v0.9.0            [default-features off]
    ├── filmcraft-bitstream v0.9.0  ← LEAF. ZERO dependencies.
    └── thiserror v2.0.21
        └── thiserror-impl (proc-macro) → proc-macro2, quote, syn,
            unicode-ident          [all standard, already-familiar nodes]
```

No `-sys` crates, no git deps, no new native nodes. With default features
ON the graph would additionally pull `rayon` (+crossbeam-deque etc.) —
avoided by the pin. `[VERIFY-P1: merged lock diff must show only
filmcraft-dnx + filmcraft-bitstream + thiserror family.]`

## R3 — Build scripts (F2)

Absent (R1). No `cc`, no codegen, no downloads at build time. F2: NO
native compilation of any kind.

## R4 — Native links (F3, F4, F6)

No `links=`, no `#[link]`, no `-l` flags, no pkg-config, no binaries, no
downloads. F3/F4/F6 all NO.

## R5 — Targets

Pure safe Rust, no arch gates in source (no SIMD intrinsics at all — the
IDCT is portable `f32`). Upstream README claims wasm32-unknown-unknown
builds; **not measured by us** (standing shared VERIFY-P1 item at the
first wasm gate run, symphonia/rav1d/rusty_vp9 convention).

## R6 — Actual feature coverage (against the card's needs)

- Used surface: `decode_frame(&[u8]) -> Result<Frame>` (one call per
  CBR frame), `Frame { width, height, chroma (Yuv422/Yuv444/Yuv420),
  rgb, bit_depth (8/10/12), y/cb/cr: Vec<u16>, interlace, cid, … }`.
  Measured this session: 8-bit AND 10-bit 4:2:2 decode end-to-end (§D).
- Upstream status table: DNxHD HD CIDs 1235/1237/1238/1241–1244/1250–1253/
  1258/1259 + DNxHR 1270–1274 done and oracle-tested; 12-bit, 4:4:4,
  interlaced, alpha implemented (oracle coverage varies — upstream's
  claim, our evidence is §D).
- Explicitly NOT used: the whole `encode/` half (VC-3 encoder), alpha
  planes, `spec_dequant` option (we decode with the DEFAULT
  ffmpeg-compatible reconstruction, `spec_dequant = false`).

## R7 — License

**MIT OR Apache-2.0** (manifest + index; both files absent from tarball —
the crates.io index is licence-bearing of record). On doc 11 §3.2's
allowlist class. Transitive: filmcraft-bitstream MIT OR Apache-2.0,
thiserror MIT OR Apache-2.0 — all permissive.

## R8 — Maintenance & security record

- 0.9.0 published **2026-10-23** (3 days before this record) — the
  FIRST and only release; `rust_version` 1.95 (our pin 1.99.1 ≥ 1.95 ✓,
  compiled clean on it); not yanked. **9 downloads** — honesty:
  brand-new, zero-uptake, single-publisher crate with no public repo;
  bus-factor and provenance risk are real, mitigated by the `=0.9.0`
  exact pin and this record's re-trigger rule. Same risk class as
  rusty_vp9 (young, low downloads), plus the no-repo deviation recorded
  in R9.
- OSV API (POST /v1/query, 2026-10-26): `filmcraft-dnx` → `{}` and
  `filmcraft-bitstream` → `{}` — zero advisories (the thiserror family
  is already in our graph).
- Upstream ships oracle tests against ffmpeg (`tests/oracle_decode.rs`)
  and documents **two measured places where ffmpeg's VC-3 decoder
  deviates from SMPTE ST 2019-1 text** (CID 1271 quant weights;
  reconstruction offset) — the default decode path targets
  ffmpeg-compatibility. Their claims are upstream's; §D is what we
  measured ourselves.

## R9 — Provenance

Tarball fetched from static.crates.io; downloaded-file sha256
`a3a59d559caf74b9281578f06015702bd402192198f71f5d41e7c6a9ec3592ab`
recomputed in-sandbox == sparse-index `cksum` byte-for-byte (cargo
re-verifies on every fetch). Index confirms `deps:
[filmcraft-bitstream, rayon(optional), thiserror]`, `features:
{threads: [dep:rayon]}`, not yanked. **No repository URL published** —
source-of-record is the crates.io tarball only; `[VERIFY-P1: publisher
identity cross-check at admission time, cpal-record convention — no
GitHub org found under "filmcraft" via the public API.]**

## R10 — Verdict

**conditional PASS — pure safe Rust (`unsafe_code = "forbid"`, zero
unsafe tokens in the whole measured graph), minimal permissive dep
graph, hostile battery panic-free, and a five-lane differential vs
ffmpeg 7.1.5 classified TOLERANCE: every frame within ±2 LSB (bulk ±1,
balanced signs; 0.04–6.6 % of samples differing, content-dependent),
root cause identified as IDCT rounding class (see §D) — NOT bit-exact.**
Conditions: (a) **Lead explicitly ratifies the ±2 LSB tolerance** for
VC-3 before the allowlist row — the rusty_vp9 bit-exact bar is NOT met
and is unreachable in principle without cloning ffmpeg's integer IDCT
(the crate implements SMPTE eq. 8.3 exactly in f32; ffmpeg approximates
it in integers); (b) the admission M-wave wires `codecs` behind a
differential test of the §D shape with a maxΔ≤2 + sign-balance gate;
(c) fuzz-lite `fuzz_codec_dnx` at the usual cadence is cheap insurance
despite the zero-unsafe surface; (d) any version bump, feature change
(esp. enabling `threads`/rayon), or new publisher re-triggers this
record per doc 11 §3.7.

---

## D — Differential evidence (the honest core)

Method (every command re-runnable from `tools/vta/dnx/README.md`): lanes
generated with **ffmpeg 7.1.5's own `dnxhd` encoder** (the corpus holds
ZERO dnxhd lanes — Lead census confirmed; DNxHD is intra-only so
frame-level comparison is exact) as raw elementary streams via the
`-f dnxhd` muxer (muxer+encoder both present, `ffmpeg -formats` shows
`DE dnxhd`). DNxHD is CBR: the harness splits the ES into
filesize/N frames (N from generation, verified by byte-count parity with
ffmpeg's decode output) and calls `decode_frame` per chunk, repacking
the planar `Frame` tightly (Y then Cb then Cr; u16LE for 10-bit) —
byte-layout identical to ffmpeg rawvideo `yuv422p`/`yuv422p10le`.
Reference: `ffmpeg -i lane.dnxhd -f rawvideo` (decoder `dnxhd`,
native pix_fmt). Comparison: per-frame MD5 + per-plane max|Δ| and
differing-sample counts (python, `tools/vta/dnx/evidence/`).

Encoder-side profile combos (strict, per ffmpeg's own valid-profile
table): 1080p yuv422p ∈ {36,45,75,90,115,120,145,175,185,220,240,290,
365,440}M; 10-bit 1080p ∈ {175,185,365,440}M; 720p yuv422p ∈ {60,75,90,
110,120,145,180,220}M.

| lane | source | dims / fmt (CID) | frames | byte parity | maxΔ (Y/Cb/Cr) | differing samples | per-frame MD5 |
|---|---|---|---|---|---|---|---|
| L1 | testsrc2, 175 Mbps | 1920×1080 yuv422p 8-bit (CID 1238) | 3 = 3 | exact | 2 / 1 / 2 | 1.26–1.43 % | all 3 differ |
| L2 | smptehdbars, 36 Mbps | 1920×1080 yuv422p 8-bit (CID 1237) | 3 = 3 | exact | 1 / 1 / 1 | 0.04–0.21 % | all 3 differ (identical to each other — bars are static) |
| L3 | mandelbrot, 145 Mbps | 1920×1080 yuv422p 8-bit (CID 1242) | 3 = 3 | exact | 2 / 2 / 2 | 4.96–6.61 % | all 3 differ |
| L4 | testsrc2, 90 Mbps | 1280×720 yuv422p 8-bit (CID 1251) | 3 = 3 | exact | 2 / 1 / 2 | 1.45–1.81 % | all 3 differ |
| L5 | testsrc2, 185 Mbps | 1920×1080 yuv422p10le 10-bit (CID 1235) | 3 = 3 | exact | 2 / 1 / 2 (of 1023) | 1.16–1.42 % | all 3 differ |

**Classification: TOLERANCE (IDCT rounding class) — 15/15 frames decoded,
byte-count parity exact on all 5 lanes, every frame within maxΔ 2 LSB
(10-bit lane: 2 of 1023), NO frame bit-exact.** Delta histogram (L1
frame 0, 4.1 M samples): `{-1: 24848, +1: 29408, +2: 2}` — balanced ±1
with two +2 outliers per frame; same shape on the 10-bit lane. Root
cause (upstream-documented + consistent with our balanced histogram and
the content dependence — smptebars 0.04 % vs mandelbrot 5 %): the crate
evaluates the exact orthonormal 8×8 IDCT (SMPTE eq. 8.3) in `f32`,
ffmpeg uses its integer approximated IDCT; ties round differently. The
crate's DEFAULT decode path already tracks ffmpeg's two other documented
deviations (reconstruction offset, CID 1271 weights — `spec_dequant =
false`), which is why the residual is pure rounding noise and not a
systematic offset.

Corpus honesty: no corpus lane exists or was used — all lanes generated
this session by ffmpeg's encoder (recorded per card scope). Lane byte
sizes: L1/L5 917 504 B/frame, L2 188 416, L3 606 208, L4 458 752 — all
exact CBR multiples.

Harness honesty: two harness bugs were caught and fixed before any
comparison number was trusted — an arg-parsing slip (returned usage
instead of decoding) and an ffmpeg misuse (`-c:v dnxhd` on the REFERENCE
side re-encodes instead of decodes; caught because it emitted ffmpeg's
valid-profile table and a zero-byte output). The first successful run
then produced the numbers above; nothing was re-run selectively.

## H — Hostile input (no-panic contract)

Battery (`--hostile`, lanes L1 + L4 + L5, catch_unwind + Result):
zero-length → `Err(Truncated)`; 64 B garbage-truncation → `Err(Truncated)`;
0xAA fill → `Err(Invalid("unknown header version"))`; deterministic LCG
garbage → `Err(Invalid("unknown header version"))`; header-kept +
zeroed payload → `Err(Row(0))` (typed corrupt-scan-line error);
half-truncated real frame → `Ok(frame)` (DNxHD scan-line indices stay
in-bounds past mid-frame — **concealment-class note, same class as
rusty_vp9's truncated-keyframe observation; wrapper must treat Ok on
truncation as suspect, flagged for the M-wave**). Sampled off-by-one
truncation sweep (256 cut points across the ~0.9 MB frame): **0 panics**
(208 Ok / 48 typed Err on L1; 201/55 L4; 206/50 L5). **Zero panics
anywhere — 3 lanes × (6 cases + 256 sweep points).**

## G — Honest gaps (what this record does NOT claim)

- **Not bit-exact vs ffmpeg** — tolerance class only; the ±2 LSB
  tolerance is a Lead decision, not a fait accompli (R10 condition a).
- No real-corpus DNxHD bitstream (corpus holds none); all lanes are
  ffmpeg-generated. No Avid-produced or camera-produced sample decoded.
- Not exercised: DNxHR (RI profile, CIDs 1270–1274 — the modern
  variable-raster form; ffmpeg's `dnxhd` encoder can emit it with
  `-profile:v dnxhr_hq`-style settings but our lanes are all classic
  DNxHD HD CIDs), 12-bit, 4:4:4 / RGB (CID 1256), 4:2:0, interlaced
  (field-coded / MACF), alpha, VBR, low-delay, MOV/MXF container
  plumbing through OUR demuxers (the raw-ES lane sidesteps containers;
  `AVdn`/`AVdh` sample-entry mapping to the demux layer is M-wave work).
- Full per-byte truncation sweep not run (sampled 256-point sweep only —
  a full sweep is ~1 M whole-frame decodes per lane; recorded as a
  sampled method, not hidden).
- wasm32 build not measured (upstream claims it builds); no throughput
  benchmark (bench-card convention); upstream's oracle-test claims are
  upstream's — our independent evidence is §D/§H.
- Maintenance lens: 3-day-old first release, 9 downloads, no public
  repo — youngest crate we have measured; pin + re-trigger rule is the
  mitigation, and R9's publisher cross-check stands.

---

## Addendum (2026-11-09) — admission status: ADMITTED IN-TREE (issue #7, W16-EE)

The spike evidence above is now backed by an in-tree admission; this
addendum closes the spike-era gap in this record (issue #7,
admissions-wave follow-up):

- **Cargo.toml pin (Lead admission):** `filmcraft-dnx = { version =
  "=0.9.0", default-features = false }` — `crates/codecs/Cargo.toml`
  (cards 6.M-dnx + 6.VTA-DNX-RATIFY; the Lead-ratified ±2-LSB
  tolerance class of §Verdict above).
- **ADR-010:** the "Software decoders implemented" table carries the
  DNxHD/DNxHR (VC-3) row — `crates/codecs/src/video/dnx/**` +
  `filmcraft-dnx =0.9.0` (cards 6.M-dnx + 6.VTA-DNX-RATIFY; this
  record).
- **In-tree differential evidence (this session, 2026-11-09, branch
  w15/07-codec-admissions at main 62453fb):** `cargo test -p codecs`
  → 643 test cases across the lib + all 51 integration targets:
  **624 pass, 18 ignored (bench/throughput-class), 0 fail.**
  DNxHD-specific: `dnx_differential` — **6/6 pass** (9.39 s; the
  tolerance-class lanes of §D).
- **Matrix:** the `codec-video-prores-dnxhd` row in
  `docs/feature-matrix.toml` is flipped `not_started → partial` in
  this same commit: **DNxHD/HR decode is implemented** (evidence
  above); **ProRes is NOT** — a repo-wide grep for prores code under
  `crates/codecs` returns nothing, and no ProRes decoder or record
  exists (the §Notes candidates oxideav-prores 0.1.1 / veloster-prores
  remain unmeasured). The row's honest state is DNxHD implemented /
  ProRes not started.
- Honest gaps carried forward (§G above): no Avid/camera-produced
  sample, DNxHR/12-bit/4:4:4/interlaced unexercised, MOV/MXF
  sample-entry plumbing is later-wave work.
- Per doc 11 §3.7: any version bump re-triggers this record.
