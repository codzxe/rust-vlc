# deps/records/h264-software-decoder-audits.md — H.264 software-decoder admission audits (issue #2, wave 15)

> **Class:** admission audits — the candidate-evidence half of ADR-001's
> conformance-gated track (feasibility video-codecs §2.4/§2.5) for R-01
> (doc 14, exposure 9). Three throwaway `/tmp/w14dd-a` projects (`pa`–`pf`)
> produced every graph/build-script/unsafe count below on **2026-11-09** —
> never this workspace: **no decoder crate is admitted and none is a
> workspace dependency** (the 1.E5-R negative-fixture convention). Toolchain:
> cargo 1.99.1. Sources inspected in
> `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f`.
> **Admission is Lead-gated (AGENTS §3.2/§3.3): this record ends in a
> `[PROPOSAL]`, not an allowlist row.** The hardware half of the issue-#2
> decision set is drafted separately as ADR-013 (VA-API lane).

## Candidate summary

| Candidate | Version | License | Graph | Build script | `unsafe` | Verdict |
|---|---|---|---|---|---|---|
| `rust_h264` | 0.6.1 | MIT OR Apache-2.0 | **zero deps** (default `std`) | none | 3 `unsafe impl` + 26 `unsafe fn` + 31 blocks, all in `simd/`, `once.rs`, `picture_sync.rs` | **RECOMMEND** `[PROPOSAL — Lead sign-off]` |
| `rusty_h264` | 0.17.2 | BSD-2 | 4-crate family + bytemuck/once_cell/rusty_alloc→libc | `common`'s is **cfg-only** (read in full) | codec core `#![forbid(unsafe_code)]`; unsafe lives only in `rusty_h264-accel` (safe-wrapped SIMD, default-on `asm`) | viable fallback, narrower coverage |
| `openh264` | 0.9.9 | BSD-2-Clause | `openh264-sys2` + **cc 1.11.1, nasm-rs 0.3.2** as build-deps | **compiles vendored Cisco C++/NASM** | FFI throughout | **REJECT** (MP §9 forbidden-bindings) |
| `dav1d` | — | — | — | — | — | not H.264 (AV1; already our rav1d lane) |
| `va` | 0.0.1 | Apache-2.0 | — | — | — | **false friend**: a Vapory-blockchain smart-contract language stub, not VA-API |
| `libva` / `libva-sys` | 0.1.5 / 0.1.2 | BSD-3-Clause | thiserror/bitflags/log/libloading; libc | bindgen+pkg-config / cc+pkg-config | FFI by design | hw lane only → **ADR-013** |

## R1 — Language & source

- **rust_h264 0.6.1**, repo `https://github.com/roticv/rust_h264`, crates.io
  description: *"Pure Rust H.264/AVC video decoder."* Manifest (read):
  `license = "MIT OR Apache-2.0"`, `rust-version = "1.89"`, edition 2021,
  `exclude = ["testdata/", "examples/", "fuzz/", "BENCHMARK.md"]` — the
  conformance corpus ships in-repo upstream, not in the tarball.
- **rusty_h264 0.17.2** — a facade over a 4-crate workspace
  (`-common`, `-decoder`, `-encoder`, `-accel`); manifest description:
  *"A ground-up, pure-Rust H.264 encoder and decoder. Decoder bit-exact vs
  Cisco openh264 (Baseline + B-slices + most of High profile) … forbid(unsafe)
  codec core; portable Rust SIMD (SSE2/AVX2/NEON, no assembler) on by
  default."* The NASM heritage is gone (ripped upstream 2026-08-12 per the
  build.rs history note) — today's `asm` feature is Rust-intrinsic-only.
- **openh264 0.9.9** — Rust wrapper around Cisco's C++ library; the crate's
  own two consumption modes both fail us: vendored-source build or runtime
  blob loading.
- **`va` 0.0.1** — extracted tarball read: *"Va is an emerging smart contract
  language for the Vapory blockchain"* (Tetcoin Developers); `src/lib.rs`
  contains one `2 + 2 == 4` test. Pure name collision; recorded so no future
  wave chases it.

## R2 — Transitive graph (real `cargo tree`, resolved from the index)

`pa` (rusty_h264, default features):

```
rusty_h264 v0.17.2
└── rusty_h264-common v0.17.2
    ├── bytemuck v1.25.2
    ├── once_cell v1.21.4
    └── rusty_alloc-api v2.2.6 → rusty_alloc v2.4.0 → libc v0.191
```

(the facade also pulls `-encoder`/`-decoder`/`-accel` — all same-version
workspace kin; no third-party code beyond the four utility crates above)

`pb` (rust_h264, default features):

```
rust_h264 v0.6.1        # end of graph — zero dependencies
```

`pc` (openh264, default features):

```
openh264 v0.9.9
└── openh264-sys2 v0.9.9
    [build-dependencies]
    ├── cc v1.11.1 (find-msvc-tools, jobserver→libc, shlex)
    └── nasm-rs v0.3.2 (log)
```

`pe`/`pf` (VA-API access, hw-lane inputs for ADR-013):

```
libva v0.1.5 → bitflags, thiserror, log, libloading v0.9 (default `dlopen`)
               [build-deps] bindgen v0.72, pkg-config, regex
libva-sys v0.1.2 → libc      [build-deps] cc, cfg-if, pkg-config
```

## R3 — Build scripts

- **rust_h264 0.6.1: no `build.rs` at all** — cleanest possible posture.
- **rusty_h264-common 0.17.2 `build.rs` (read in full):** emits
  `cargo::rustc-check-cfg=cfg(accel)`/`cfg(accel_x86)` plus two
  `rustc-cfg` lines when the `asm` feature is on and the target is
  x86-64/aarch64. No toolchain invocation, no network, no OUT_DIR binaries.
  Benign — cfg-only, exactly as claimed.
- **openh264-sys2 0.9.9:** build-deps **cc + nasm-rs** compile the vendored
  Cisco C++ sources (and x86 assembly) at build time. This is the
  forbidden-bindings list verbatim (MP §9; doc 04; risk-register R-01
  contingency: "never wrap openh264/x264"). The alternative `libloading`
  mode swaps a compile-time C++ toolchain for an **unaudited runtime binary
  blob** — worse for provenance, not better. Both paths rejected.

## R4 — `unsafe` exposure (counted in the extracted tarballs, rg)

- **rust_h264 0.6.1:** 3 `unsafe impl` — `OnceLock<T>` `Send`/`Sync`
  (`once.rs` ×2) and `SharedPlane: Sync` (`picture_sync.rs` ×1); 26
  `unsafe fn` — `simd/x86.rs` 17, `simd/neon.rs` 6, `picture_sync.rs` 3;
  31 `unsafe {` blocks total. Every occurrence sits in three files:
  target-gated SIMD intrinsics and the threading primitives they serve. No
  `unsafe` anywhere in the parse/decode core (`nal/sps/pps/slice/cabac/…`).
  MSRV 1.89, `no_std + alloc` capable (manifest feature `std`, default on).
- **rusty_h264 0.17.2 family:** the decoder and common crates are
  `#![forbid(unsafe_code)]` (common: `cfg_attr(not(feature = "profile"),
  forbid(unsafe_code))` — the shipped build forbids). The family's unsafe is
  **deliberately concentrated in `rusty_h264-accel`** ("the one place
  `unsafe` lives, behind safe wrappers, so the codec core stays
  `forbid(unsafe)`" — its own lib.rs), reached via the default-on `asm`
  feature. `--no-default-features` gives a pure-scalar, zero-unsafe codec.
- **openh264:** FFI wrapper by construction — moot after R3's rejection.

## R5 — Claimed conformance & profile coverage (crate-stated; NOT yet our evidence)

- **rust_h264 0.6.1** (README, read): decodes **Baseline, Main, High,
  High 10, High 4:2:2** — 8- and 10-bit, 4:2:0 and 4:2:2, monochrome;
  **MBAFF and field pictures**; CAVLC + CABAC; B-frames, multi-reference,
  weighted prediction, long-term references, multi-slice; NEON on aarch64,
  optional nightly `portable-simd`. Conformance claims: *"Output is
  byte-identical to FFmpeg on the test and benchmark streams"* and *"Every
  8- and 10-bit 4:2:0, 4:2:2 and monochrome JVT conformance vector passes
  (199 of 204, the same as FFmpeg; the rest use FMO or SP/SI)"* — i.e. the
  misses are FMO/SP/SI, exactly the subsets FFmpeg's own decoder also fails.
  This is the **strongest candidate claim set** seen in any wave: JVT-corpus
  numbers at FFmpeg parity. It remains **self-validated by the upstream
  author** — ADR-001's "none has independent conformance validation" stands
  until our own gate runs (Verdict below).
- **rusty_h264 0.17.2** (decoder lib.rs, read): Constrained Baseline
  **+ B-slices + most of High profile** (8×8 transform, scaling matrices,
  CABAC I/P/B incl. High 8×8 residual; not yet `I_PCM`); validated
  bit-exact vs Cisco `h264dec` on **35/35 clean streams** of openh264's
  corpus, CABAC gated pixel-exact vs ffmpeg. Hard limits verified in source:
  `chroma_format_idc != 1` → `Unsupported` (4:2:0 only), bit depth > 8 →
  `Unsupported`, and **MBAFF/field pictures refused at the SPS**
  (`params.rs`: `Unsupported("interlace / field coding (MBAFF)")`).
  Fuzzed to never panic or hang on malformed input. 8-bit 4:2:0 only.
- HEVC side (issue #2's other half): **no HEVC candidate was audited this
  window** — `rust_h265 0.6.2` / `rust_h265 0.1.0` remain
  young/unproven on feasibility-doc authority (video-codecs §2), to be
  audited under this same framework before any HEVC software admission.

## R6 — Feature coverage vs the seam

The implemented hw-first seam (`crates/codecs/src/video/h264/**`, cards
5.F1/5.F2) already parses NAL/SPS/PPS/slice/SEI and fails decode with the
typed `h264.hw.required` `Unsupported`. A software decoder admitted under
this track slots in as the fallback `Decoder` impl behind the same contract:
avcC/Annex-B input the parser already normalizes, planar YUV out (rust_h264
emits FFmpeg-layout `yuv420p10le`-style planes for 10-bit — matches our
frame model; rusty_h264 is 8-bit 4:2:0 `YuvFrame`). Profile coverage is the
differentiator: rust_h264 covers the Hi10P/4:2:2/MBAFF long tail doc 04
§4.2 names as the hw-less-machine gap; rusty_h264 does not (and refuses
MBAFF outright).

## R7 — Licenses (manifests, read)

rust_h264 **MIT OR Apache-2.0**; rusty_h264 family **BSD-2**; openh264
BSD-2-Clause with Cisco's patent grant — irrelevant post-rejection. All
inside the deny.toml allow set. Patent exposure is **unchanged by any of
this**: R-03/ADR-010 own distribution; a pure-Rust H.264 decoder is still
an H.264 decoder (ADR-010 Tier B row, Q1).

## R8 — Maintenance

- rust_h264: single maintainer (roticv), 0.6.1 current, MSRV 1.89,
  CI-claimed JVT corpus + msrv job; benchmark tooling and a
  `verify_vs_ffmpeg.py` differential harness ship in the repo (tools/).
  Single-maintainer risk = R-43 class; `[VERIFY-P1: RustSec + release
  cadence at admission date]`.
- rusty_h264: actively versioned family (accel split, NASM removal
  2026-08-12 documented in-build), encoder+decoder scope. `[VERIFY-P1:
  RustSec + publisher pins at admission.]`
- openh264: upstream Cisco, mature — irrelevant post-rejection.

## R9 — Provenance

Registry `index.crates.io-1949cf8c6b5b557f` via cargo 1.99.1; manifests,
build.rs, README and all `src/` counts above are from the extracted
tarballs. `[VERIFY-P1: tarball sha256 + publisher pins at admission — the
R9 procedure every record re-runs.]`

## R10 — Alternatives survey (what the crate pages/API actually say)

- **dav1d:** AV1's reference decoder — wrong codec; our AV1 lane is rav1d
  1.1.0 (admitted, asm-off). Listed only to close the false lead.
- **`va` 0.0.1:** blockchain-language stub (R1) — dead end, recorded.
- **`libva` 0.1.5** (cros-libva lineage, BSD-3, rust-version 1.88) /
  **`libva-sys` 0.1.2:** the userspace VA-API C library from Rust —
  hardware-lane substrate candidates, **out of scope for software decode**;
  carried into ADR-013 as its audited access-path set (dlopen vs linked vs
  raw sys).
- **`openh264` runtime-blob mode:** rejected with the vendored-build mode
  (R3) — swapping a compile toolchain for an unauditable prebuilt blob.

## Verdict & recommendation

**NOT admitted** (Lead-gated; nothing changed in the workspace).

1. **`[PROPOSAL — Lead sign-off]` rust_h264 0.6.1 is the recommended
   candidate** for the H.264 software-decode fallback lane (R-01
   mitigation (c)): zero-dependency pure Rust, no build script, unsafe
   confined to target-gated SIMD + threading glue, and the strongest
   conformance claim set available (JVT 199/204 at FFmpeg parity, Hi10P /
   4:2:2 / MBAFF coverage). rusty_h264 0.17.2 recorded as the audited
   fallback (forbid(unsafe) core, BSD-2, but 8-bit 4:2:0, MBAFF refused,
   "most of High" only). openh264: **REJECT** on both consumption modes.
2. **Admission is conformance-gated, not claim-gated** (ADR-001's track;
   the rust_h264 numbers are upstream-authored until we reproduce them):
   (i) JVT conformance vectors in CI — target = the crate's 199/204 set,
   misses limited to FMO/SP/SI, byte-compared per frame; (ii) differential
   vs an **FFmpeg oracle** on our randomized real-media corpus (D-11) —
   the crate ships `tools/verify_vs_ffmpeg.py`; we run our corpus through
   an equivalent harness, byte-exact expected; (iii) profile-coverage
   report (Baseline/Main/High/Hi10P/4:2:2/MBAFF pass/fail matrix) feeding
   the feature-matrix row honestly; (iv) tarball sha256 + RustSec +
   publisher pins (R9). Gate failure ⇒ row stays `blocked`, crate out.
3. **Admission order stays Lead-owned** and lands as its own change
   (workspace Cargo.toml + deny.toml + this record's follow-up), *after*
   ADR-010's distribution posture is faced (an admitted H.264 decoder is
   still a Tier-B patent row — Q1).
4. **HEVC software candidates get the same framework, not a pass**: the
   rust_h265/rust_h265 audit is the recorded next step (R-02 stays
   mitigating; ADR-013 carries the hw half for both codecs).

*Evidence posture (AGENTS §3.4): every graph, manifest field, build
script and unsafe count above came from real cargo runs and extracted
tarball reads in `/tmp/w14dd-a` on 2026-11-09. No decoder ran on real
media in this window — all conformance numbers in R5 are quoted
upstream claims, which is precisely why the Verdict gates admission on
our own corpus.*
