# deps/records/rusty_vp9-0.1.3.md — rusty_vp9 0.1.3 (default features)

> **Class:** P0 validate-then-admit SPIKE record (card 6.VTA-rusty-vp9,
> Phase 6 Wave 1; doc 11 §2 format, fields R1–R10 + F1–F7 — rav1d-1.1.0
> is the closest analogue and this record follows its shape). VP9 was a
> top blocked-format (YouTube/WebM) named by the Phase-5 exit gate.
> Evidence gathered 2026-10-26 in the build sandbox: crates.io sparse
> index + API queries, tarball download + sha256 match, vendored-source
> scans, a scratch `cargo fetch`/`cargo tree`, and a **six-lane
> bit-exactness differential against ffmpeg `libvpx-vp9` driven through
> OUR OWN ivf demuxer** (§D). **Final acceptance and the allowlist row
> remain Lead decisions (doc 11 §3.1; `boundaries.toml` + `deny.toml`
> are Lead-owned and untouched by this card — same posture as every
> codec admission since claxon).**

## The pin (the whole point of this record)

- **Admission shape: `rusty_vp9 = { version = "=0.1.3" }`, default
  features (i.e. none).** The crate has exactly one feature,
  `prometheus-telemetry` (an offline coefficient-entropy tap for the
  upstream's private telemetry campaign — `#[cfg(feature)]`-gated
  module, no dependencies, default OFF). We do not enable it: the
  production decode path never touches telemetry.
- Version pin `=0.1.3` (exact): 0.x crate with 4 releases in 3 months
  — API churn risk until 0.2/1.0 (R8).

## R1 — Language & source (F1, F7)

- Pure Rust ("in-house VP9 decoder + encoder, pure Rust with no
  C/FFI" — lib.rs header). Vendored tarball: **38 `.rs` files, 0
  `.c`/`.h`/`.S`/`.asm`/`.a`/`.so`**, no binary blobs.
- **No `build.rs` at all** (`build = false` in Cargo.toml) — F2 is
  vacuously clean; there is no build script to sabotage-test (the
  rav1d R3 sabotage experiment has no equivalent surface here, by
  construction).
- **0 `extern "C"`** across `src/` (rg). No FFI in either direction.
- `unsafe` surface: **98 tokens**, all of them
  `core::arch::x86_64` SIMD intrinsics (`__m256i`/`__m128i` AVX2/SSE
  kernels), `#[cfg(target_arch = "x86_64")]`-gated with
  `#[cfg(not(target_arch = "x86_64"))]` safe fallbacks (verified in
  `src/transform.rs`; `src/inter.rs` kernels likewise). Breakdown:
  decode-path 76 (`inter.rs` 46, `transform.rs` 23, `loopfilter.rs`
  5, `decode.rs` 1, `prof.rs` 1), encoder-path 22 (`encode/*`) — the
  encoder side is dead weight we compile but never call. NOT
  `#![forbid(unsafe_code)]` — same honest R7/R8 deviation class as
  rav1d (policy deviation, not a purity violation: F7 "bundled
  non-Rust engine" is NO — the compiled engine is 100 % Rust).
- Repo: https://github.com/Remade-With-Rust/remade_ffmpeg_rs
  (publisher "Mata Network"). LICENSE: Apache-2.0.

## R2 — Transitive graph (F5)

Scratch `cargo fetch` + `cargo tree --edges normal` with our exact
pin, harness at `tools/vta/vp9` (committed):

```
vta-vp9
├── demux v0.1.0 (path) → media-types, plugin-api → codec-api,
│   serde, thiserror, toml        [all already in our workspace lock]
└── rusty_vp9 v0.1.3              ← LEAF. ZERO dependencies.
```

**`rusty_vp9` adds ZERO new lockfile nodes** — no `-sys` crates, no
proc-macros, no git deps. The new-graph question that rav1d's record
had to answer in detail does not exist here: there is nothing to
add. `[VERIFY-P1: the merged lock diff must show only the rusty_vp9
node itself.]`

## R3 — Build scripts (F2)

Absent (R1). No `cc`, no `nasm`, no codegen, no downloads at build
time. F2: NO native compilation of any kind.

## R4 — Native links (F3, F4, F6)

No `links=`, no `#[link]`, no `-l` flags, no pkg-config, no
binaries, no downloads. F3/F4/F6 all NO.

## R5 — Targets

x86_64 compiles the AVX2/SSE intrinsic kernels (this session's
evidence host); every other arch compiles the safe fallbacks
(`cfg(not(target_arch = "x86_64"))`). No wasm32-specific gates seen.
`[VERIFY-P1: wasm32 build from our tree at the first wasm gate run —
shared standing item with the symphonia/rav1d admissions.]`

## R6 — Actual feature coverage (against card 6.VTA-rusty-vp9's needs)

- Used surface: `Vp9Decoder::new` / `push(data, pts)` /
  `next_frame() -> Result<DecodedFrame>` / `flush()`, plus
  `DecodedFrame { width, height, bit_depth, subsampling_x/y, planes,
  strides, pts }`. `Error::Again`/`Error::Eof` mirror ffmpeg's
  send/receive convention. `push` splits VP9 superframes internally
  and (libvpx-convention) emits only the top displayable layer.
- 8-bit AND 10-bit decode both measured end-to-end this session
  (§D lane F, profile 2 / yuv420p10le). The `DecodedFrame` contract
  covers 8/10/12-bit and 4:2:0/4:4:0/4:4:4 subsampling.
- Explicitly NOT used: the whole `encode/` half (VP9 encoder),
  `prometheus-telemetry`, the `prof` instrumentation taps.

## R7 — License

**Apache-2.0** (LICENSE + manifest + index) — on doc 11 §3.2's
allowlist. Transitive: nothing (R2). Unsafe note carried from R1:
98 intrinsic-only `unsafe` tokens, a *policy* deviation from the
forbid-unsafe engine precedent, not a license one — the reason this
record is differential-tested and names a fuzz-lite obligation
before any wider claim (rav1d precedent, doc 04 §4.1).

## R8 — Maintenance & security record

- 0.1.3 published **2026-10-17** (9 days before this record);
  `rust_version` 1.85 (our pin 1.99.1 ≥ 1.85 ✓); not yanked.
  Release history: 0.1.0 2026-07-29 → 0.1.1 2026-08-01 → 0.1.2
  2026-10-07 → 0.1.3 2026-10-17 — **young but actively iterating**
  (0.1.3 fixed clippy/fmt CI breakage). Total downloads 3,784 (248
  on 0.1.3) — low-uptake, single-publisher-org crate; honesty:
  bus-factor and API-churn risk are real, mitigated by the `=0.1.3`
  exact pin and this record's re-trigger rule.
- OSV API (POST /v1/query, 2026-10-26): `{"package":{"name":
  "rusty_vp9","ecosystem":"crates.io"}}` → **`{}` — zero advisories**
  (vacuously also true of a zero-dep graph: no transitive surface to
  advise on).
- The upstream claims "bit-exact against all 315 official libvpx
  conformance vectors" — NOT taken on faith; see §D for what we
  measured ourselves.

## R9 — Provenance

Tarball fetched through the pinned sparse index; downloaded-file
sha256 `9bf32821db20be4ea56ce1f8feb757bf55f146f2685ba98822c3bd2d43a33889`
recomputed in-sandbox == index `cksum` byte-for-byte (cargo
re-verifies on every fetch). Index also confirms `deps: []`,
`features: {"prometheus-telemetry": []}`. `[VERIFY-P1: publisher
identity cross-check at admission time, cpal-record convention.]`

## R10 — Verdict

**conditional PASS — pure Rust, zero-dependency, default-features
pin `rusty_vp9 = "=0.1.3"`; differential-measured BIT-EXACT against
ffmpeg `libvpx-vp9` on all six lanes including 10-bit profile 2;
hostile battery panic-free.** Conditions: (a) Lead's allowlist row
(doc 11 §3.1) with the exact pin above; (b) the admission M-wave
wires `codecs` behind a differential test of the §D shape (live
ffmpeg oracle, per-frame compare) before any wider claim; (c)
fuzz-lite (`fuzz_codec_vp9`, doc 04 §4.1) as for rav1d, given the
98-token unsafe surface; (d) a version bump or feature change
re-triggers this record per doc 11 §3.7.

---

## D — Differential evidence (the honest core)

Method (every command re-runnable from `tools/vta/vp9/README.md`):
lanes demuxed by **our** `crates/demux/src/ivf` (`IvfDemuxer` over
`mem_stream`), packets fed to `rusty_vp9` 0.1.3, output repacked
tightly per `DecodedFrame` (Y then U then V rows; u16LE for bd>8) —
byte-layout identical to ffmpeg rawvideo `yuv420p`/`yuv420p10le`.
Reference: `ffmpeg -c:v libvpx-vp9 -f rawvideo` (ffmpeg 7.1.5,
libvpx-vp9). Comparison: per-frame MD5 of frame-sized chunks both
sides (python hashlib) + whole-stream md5sum cross-check. Encode
side of all generated lanes: `-c:v libvpx-vp9 -deadline good
-cpu-used 8` (fast encode; decode is what is under test).

| lane | source | dims / fmt | frames (mine=ref) | identical frames | stream MD5 equal? |
|---|---|---|---|---|---|
| A | testsrc2 5 s, default GOP | 320×240 I420 8-bit | 125 = 125 | **125/125** | YES (7b5cbbc70b04132c8c74fcd07576fd48) |
| B | testsrc2 3 s, `-g 30` forced keyframes | 640×480 I420 8-bit | 90 = 90 | **90/90** | YES (356c84a819e60fcfc3bc73048de20c70) |
| C | odd dims (asked 127×95; libvpx coded render 126×94) | 126×94 I420 8-bit | 75 = 75 | **75/75** | YES (5d879407bc72e7ea105bc214c64c2256) |
| D | corpus-derived: `0126-webm-video.webm` (VP8!) re-encoded VP8→VP9 | 480×360 I420 8-bit | 41 = 41 | **41/41** | YES (9621dcf981bfc3b52efa40c66c66a543) |
| E | VP9 `.webm` remuxed `.ivf` (`-c copy`, ES byte-preserving) | 320×240 I420 8-bit | 50 = 50 | **50/50** | YES (88bb0b3ce56155bf0edb85bce4374ef4) |
| F | profile 2, `-pix_fmt yuv420p10le` | 320×240 I420 10-bit | 50 = 50 | **50/50** | YES (2391b4241617d81cf05d50efd54781d4) |

**Classification: BIT-EXACT — 6/6 lanes, 431/431 frames
byte-identical to ffmpeg `libvpx-vp9` (0 differing frames anywhere,
so 0 pixel deltas to report).** Not tolerance-class, not divergent.

Corpus honesty (recorded per card): the corpus holds **zero VP9
lanes** — its single `.webm` (`0126-webm-video.webm`, 480×360
yuv420p 25 fps 1.64 s) is **VP8** (libvpx encoder tag). Lane D is a
**re-encode** of that file VP8→VP9 (not byte-preserving — it cannot
be; recorded as such). Lane E's `.webm`→`.ivf` remux IS
elementary-stream byte-preserving (`-c copy`, no re-encode), proven
by the whole-stream decode output matching ffmpeg decoding the
`.webm` directly.

Harness honesty: the first in-harness hand-rolled MD5 was WRONG —
caught by the external whole-stream md5sum cross-check (raw bytes
were always identical); digests are computed externally ever since
and the harness only demuxes/decodes/writes. The 10-bit lane's
first comparison also mismatched due to a harness byte-pairing bug
(u16 samples read byte-wise); fixed, re-measured, bit-exact. Both
incidents are recorded here because the point of a VTA card is the
measurement discipline, not the verdict.

## H — Hostile input (no-panic contract)

Battery (`--hostile`, lanes A + D + F, catch_unwind + Result):
truncated keyframe packet → Ok(frame) (see note); 64 B garbage →
`Err(invalid data: bad frame marker)`; zero-length → `Err(bit
reader past end)`; corrupted frame header → `Err(bad frame
marker)`; corrupted mid-bitstream → `Err(frame dimensions out of
range)`; **off-by-one truncation sweep: 0/125 (A), 0/41 (D), 0/50
(F) panics**; garbage-then-real stream → recovers, 10 frames out.
**Zero panics anywhere.** Note (typed, not disqualifying): a
truncated keyframe packet decodes to a full frame instead of a
typed Err — VP9's boolean decoder end-fills like libvpx's error
concealment does; flagged for the M-wave differential to pin down
the exact concealment-equivalence class.

## G — Honest gaps (what this record does NOT claim)

- No VP9 bitstream from the real corpus (there are none) — lane D
  is a re-encode of the corpus VP8 file.
- Not exercised: profile 3 (12-bit), profile 1 4:4:4, lossless
  mode, SVC/superframe-index lanes (alt-ref visibility), 
  resolution-change mid-stream, 4:2:2-equivalent chroma sampling;
  `.webm` through OUR mkv demuxer end-to-end (V_VP9→"vp9" codec-id
  mapping verified in `crates/demux/src/mkv/header.rs:772` but the
  lane ran via the ivf remux; mkv-direct is a 5-minute follow-up
  for the M-wave).
- Decode throughput NOT benchmarked (rav1d-record convention: a
  bench card owns the numbers; VP9 on this 2-core box is
  measurably CPU-bound — the six lanes took O(seconds) each).
- The crate's own 315-vector conformance claim is upstream's
  number; our independent evidence is the six lanes above.
- Maintenance lens: 0.1.x, single publisher org, 3,784 downloads —
  young; the `=0.1.3` pin + re-trigger rule is the mitigation.

---

## Addendum (2026-11-09) — admission status: ADMITTED IN-TREE (issue #7, W16-EE)

The spike evidence above is now backed by an in-tree admission; this
addendum closes the spike-era gap in this record (issue #7,
admissions-wave follow-up):

- **Cargo.toml pin (Lead admission):** `rusty_vp9 = "=0.1.3"` —
  `crates/codecs/Cargo.toml` (card 6.M-vp9; the spike's exact-pin +
  re-trigger mitigation).
- **ADR-010:** the "Software decoders implemented" table carries the
  VP9 row — `crates/codecs/src/video/vp9/**` + `rusty_vp9 =0.1.3`
  (card 6.M-vp9; this record) on our contract surface.
- **In-tree differential evidence (this session, 2026-11-09, branch
  w15/07-codec-admissions at main 62453fb):** `cargo test -p codecs`
  → 643 test cases across the lib + all 51 integration targets:
  **624 pass, 18 ignored (bench/throughput-class), 0 fail.**
  VP9-specific: `vp9_differential` — **4/4 pass** (21.98 s; ffmpeg
  reference lanes incl. the re-encoded corpus VP8 lane of §D).
- **Matrix:** the `codec-video-vp9` row in `docs/feature-matrix.toml`
  is flipped `blocked → implemented` in this same commit (was stale
  since 5.R2-V / 3116aec — ADR-010 "matrix drift" note; the old
  `blocked_reason` text named the "validation spike pending", which
  this record + the in-tree differential supersede).
- Honest gap carried forward: profile 3 (12-bit), 4:4:4, SVC and
  mkv-direct lanes remain unexercised (§G above) — the admission is
  for the measured lane class, not the whole VP9 surface.
- Per doc 11 §3.7: any version bump re-triggers this record.
