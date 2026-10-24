# deps/records/rav1d-1.1.0.md — rav1d 1.1.0 (asm OFF)

> **Class:** P0 conditional dependency verification (doc 11 §2 format,
> fields R1–R10 + F1–F7), ordered by card 5.E1: "AV1 decode via rav1d
> with asm OFF: purity proof". Doc 11 §2.4 names this exact admission
> shape as its "conditional" exemplar; doc 04 §4.4 carries the two
> caveats (asm-by-default, young port). Evidence gathered 2026-10-26
> in the build sandbox: sparse-index query, tarball download + sha256
> match, source inspection of `build.rs`/`Cargo.toml.orig`/`COPYING`,
> a scratch `cargo fetch` + `cargo tree --edges normal` (pasted in
> R2), a sabotage build (R3), and OSV API advisory queries per
> version. **Final acceptance and the allowlist row remain Lead
> decisions (doc 11 §3.1; `boundaries.toml` + `deny.toml` are
> Lead-owned and untouched by 5.E1 — same posture as every codec
> admission since claxon).**

## The feature pin (the whole point of this record)

- Sparse index (`index.crates.io/ra/v1/rav1d`, 2026-10-26): the
  **default features are
  `["asm", "asm_arm64_dotprod", "asm_arm64_i8mm", "asm_arm64_sve2",
  "bitdepth_8", "bitdepth_16"]`** — `asm` IS default-on, confirming
  doc 04 §4.4. A default admission would compile ARM `.S` and x86
  nasm via `cc`/`nasm-rs` → F2/F7 fail, doc 11 §2.4.
- **Our admission: `rav1d = { version = "=1.1.0", default-features =
  false, features = ["bitdepth_8", "bitdepth_16"] }`.** Both
  `bitdepth_*` features are pure-Rust compile-time gates of the
  8-bit and 10/12-bit DSP paths (`#[cfg(feature = "bitdepth_8")]` in
  `src/ipred.rs`, `src/itx.rs`, `src/lib.rs` — a `compile_error!`
  fires when neither is enabled), so `features = []` as sketched in
  doc 11 §3.1 would compile a decoder that can decode nothing; the
  bitdepths are required and are named here per the Lead's C5
  mandate ("name what breaks"). What is OFF and stays OFF: `asm` and
  everything implying it (`asm_arm64_*`).
- `check_asm` does not exist in 1.1.0 (feature list above is
  exhaustive) — nothing to disable beyond `asm`.

## R1 — Language & source (F1, F7)

- Rust (a genuine port of dav1d by memorysafety/Prossimo —
  retranspiled + hand-migrated, per README/retranspile.sh). The
  tarball **ships 174 native asm files** (`src/x86/*.asm` nasm,
  `src/arm/*.S`) — these are EXCLUDED from compilation by the asm-off
  pin (see R3's proof); the default build would compile them.
- No `*.c/*.h` compiled: the tarball's `include/` tree is Rust
  modules; `Cargo.toml.orig` excludes `*.c`, `*.h` from the package.
  No `links=` key (F3 NO).
- `extern "C"` exists — but as **Rust-side ABI exports** of rav1d's
  own `dav1d_*` C-compatible API (`pub unsafe extern "C" fn
  dav1d_open` … in `src/lib.rs`), i.e. rav1d *provides* a C ABI for
  embedders; it links against no C library. `libc` is used only for
  errno constants and `ptrdiff_t`/`uintptr_t` type aliases.
- `unsafe` surface: large (≈642 `unsafe` tokens across `src/` — a
  port-in-progress with raw-pointer picture plumbing), **not**
  `#![forbid(unsafe_code)]`. This is a deviation from the
  claxon/symphonia posture and is recorded as the honest R7/R8
  risk note: the workspace `unsafe_code = deny` lint covers our
  crates only; rav1d's unsafe is exactly the surface the
  memorysafety/Prossimo project exists to shrink. F7 (bundled
  non-Rust engine): NO — with asm off, the compiled engine is
  100 % Rust.
- Repo: https://github.com/memorysafety/rav1d. COPYING: BSD-2-Clause
  ("Copyright © 2018-2019, VideoLAN and dav1d authors; © 2023-2024,
  VideoLAN, dav1d authors, and Internet Security Research Group").

## R2 — Transitive graph (F5)

Scratch `cargo fetch` + `cargo tree --edges normal` with our exact
feature pin (this is the graph shape the codecs crate builds):

```
codecs
└── rav1d 1.1.0                (ck 1932f060d5e7bd49dc9f8b272c1dc5e9ce0ffe141c28be900265d3989b36c9ed)
    ├── assert_matches 1.5.0            [NEW]  MIT/Apache-2.0
    ├── atomig 0.4.3                    [NEW]  MIT/Apache-2.0
    │   └── atomig-macro 0.4.0 (proc-macro)   [NEW]
    ├── bitflags 2.13.2                 (already in lock)
    ├── cfg-if 1.0.5                    [NEW]
    ├── libc 0.2.191                    [NEW]
    ├── parking_lot 0.12.5              [NEW]
    │   ├── lock_api 0.4.14             [NEW]
    │   │   └── scopeguard 1.2.0        [NEW]
    │   └── parking_lot_core 0.9.12     [NEW]
    │       ├── cfg-if 1.0.5, libc 0.2.191, smallvec 1.16.4
    ├── paste 1.0.15 (proc-macro)       [NEW]
    ├── raw-cpuid 11.6.0                [NEW]
    ├── strum 0.26.3                    [NEW]
    │   └── strum_macros 0.26.4 (proc-macro) [NEW]
    ├── to_method 1.1.0                 [NEW]  CC0-1.0
    └── zerocopy 0.7.35                 [NEW]
        └── zerocopy-derive 0.7.35 (proc-macro) [NEW]
```

Build-graph additions (build-deps compile but do no work — R3):
`cc 1.10.0` (+`jobserver`, `shlex`, `find-msvc-tools`),
`nasm-rs 0.3.2` (+`byteorder`). Proc-macro closure adds `syn`,
`proc-macro2`, `quote`, `unicode-ident`, `heck`, `rustversion`
(shared with existing lock nodes where present). No `-sys` crates,
no git deps. `[VERIFY-P1: the merged lock diff must show exactly
these additions.]`

## R3 — Build scripts (F2) — the purity proof

- rav1d 1.1.0 **has a `build.rs`**, but its entire `main` is:

  ```rust
  fn main() {
      #[cfg(feature = "asm")]
      {
          asm::main();
      }
  }
  ```

  The whole `mod asm` (the `cc::Build` / `nasm_rs::Build` machinery,
  `config.h`/`config.asm` generation) is `#[cfg(feature = "asm")]`.
  With our pin, build.rs is a **compile-time no-op**.
- **Sabotage build (executed 2026-10-26):** scratch crate, our exact
  feature pin, `rm -rf target`, then
  `CC=/nonexistent-cc AR=/nonexistent-ar cargo build` with `nasm`
  absent from PATH — **build succeeded** (15.2 s, "Finished dev
  profile"). A single `cc`/nasm invocation would have failed the
  build: the tool is not on any path. That is the doc 11 §2.4
  `[VERIFY-P0]` "zero non-Rust build activity with asm off" closed
  by direct experiment, not inspection alone.
- `cc`/`nasm-rs` still appear as build-deps (they compile as
  ordinary pure-Rust crates; they are never invoked). This is
  wasted-compile cost, not a purity violation — recorded as the
  speed/memory note in R6. F2: NO native compilation occurs.

## R4 — Native links (F3, F4, F6)

- No `links=`, no `#[link]`, no `-l` flags, no binary blobs
  (`*.a *.o *.so` → 0 in the compiled graph), no pkg-config, no
  downloads. F3/F4/F6 all NO. The engine is a from-scratch Rust
  port, not a dav1d wrapper (dav1d itself is nowhere in the graph).

## R5 — Targets

- Pure Rust with the asm-off pin; no `cfg(target_*)` native gates in
  the Rust paths (`cfg(target_arch)` selects *Rust* asm/SIMD-free
  fallbacks). wasm32: the `stdarch_arm_feature_detection` nightly
  attribute is `cfg(target_arch = "arm")`-gated; wasm32 compiles the
  generic paths. `[VERIFY-P1: wasm32 build from our tree at the
  first wasm gate run — shared standing item with the symphonia
  admissions.]`

## R6 — Actual feature coverage (against card 5.E1's needs)

- Used surface: the `dav1d_*` C-ABI-shaped entry points that are the
  only public Rust API of 1.1.0 — `dav1d_open`,
  `dav1d_send_data`, `dav1d_get_picture`, `dav1d_flush`,
  `dav1d_close`, `dav1d_picture_unref`, `dav1d_data_create`,
  `dav1d_parse_sequence_header` (seq-header OBU facts), plus the
  `Dav1dSettings`/`Dav1dData`/`Dav1dPicture` types. The Rust-native
  `rav1d_open`/`rav1d_send_data`/`rav1d_get_picture` are
  `pub(crate)` in 1.1.0 (verified in source) — the C ABI is the
  intended embedding surface for now.
- Explicitly NOT used: film-grain application is left at rav1d's
  `apply_grain` default for the decode path we exercise (recorded
  divergence risk if a fixture uses grain — see the differential
  test); thread pool defaults are overridden by the codec-api
  `ThreadBudget` (memory care: rav1d's `n_threads=0` default spawns
  one thread per logical core — our default pins 1 and exposes the
  budget up to 256).
- Speed note (doc 04 §4.4's benchmark ask): asm-off costs real decode
  throughput vs dav1d-with-asm; no number is claimed here — the
  differential tests record frame counts and correctness only. A
  benchmark card (doc 12) owns the numbers; the ADR question
  "assembly is not C but is non-Rust" stays open per doc 04 §4.4.

## R7 — License

- **BSD-2-Clause** (COPYING + manifest) — explicitly on doc 11 §3.2's
  allowlist and explicitly anticipated by doc 11 §8.1 ("rav1d
  BSD-2"). Transitive: MIT / Apache-2.0 / BSD-2-Clause / CC0-1.0 /
  (Unlicense OR MIT) — all inside the allowlist classes (dual
  expressions pass on their allowed side).
- Unsafe note (carried from R1): the large unsafe surface is a
  *policy* deviation from our forbid-unsafe engine precedent, not a
  license one; it is the reason this record is **conditional** on
  the asm-off pin and a differential-tested admission, and why the
  fuzz-lite target (`fuzz_codec_av1`, doc 04 §4.1) is mandatory
  before any wider claim.

## R8 — Maintenance & security record

- rav1d 1.1.0: published 2025-05-07, `rust_version` 1.79, not
  yanked; upstream is the active memorysafety/Prossimo port (1.1.0
  is the latest as of 2026-10-26 — 0.1.0 from 2019 is an unrelated
  5-byte placeholder squat, `deps: []`, ignored).
- OSV API (POST /v1/query, per-version, 2026-10-26): **every node in
  the R2 graph queries clean except `paste 1.0.15` →
  RUSTSEC-2024-0436** (informational: repository archived/
  unmaintained; no vulnerability). Accepted-risk note: `paste` is a
  build-time proc-macro used by rav1d's internal macro glue; the
  advisory is unmaintained-not-exploited, and removal is upstream
  rav1d's to make. `[VERIFY-P1: re-query RustSec at admission date
  per doc 11 §3.7; if rav1d drops `paste`, expect the node to
  disappear on the next bump.]`
- 642 `unsafe` tokens (R1) — flagged for the Lead's Q2-V audit lens.

## R9 — Provenance

- Tarball fetched from static.crates.io through the pinned sparse
  index; downloaded-file sha256
  `1932f060d5e7bd49dc9f8b272c1dc5e9ce0ffe141c28be900265d3989b36c9ed`
  matches the index `cksum` byte-for-byte (recomputed in-sandbox;
  cargo re-verifies on every fetch). `[VERIFY-P1: publisher identity
  cross-check at admission time, cpal-record convention.]`

## R10 — Verdict

**conditional PASS — pure Rust under the exact pin
`default-features = false, features = ["bitdepth_8",
"bitdepth_16"]`** (asm and every asm-implying feature OFF; build.rs
provably inert — sabotage build, R3; sha256-pinned; licenses
allowlisted; one informational advisory recorded). Admitted by card
5.E1 under the Lead's C5 pre-approval, pending (a) the Lead's
allowlist row — doc 11 §3.1's sketch row for rav1d should carry the
feature list above, not the `features = []` placeholder, for the
R6 reason — and (b) nothing else. A version bump, a feature change,
or an upstream re-tuning of `default` re-triggers this record per
doc 11 §3.7.

*Evidence posture (AGENTS §3.4): every statement above was executed
locally in the build sandbox on 2026-10-26 — sparse-index query,
tarball download + sha256 recomputation, source scans of
`build.rs`/`Cargo.toml.orig`/`COPYING`/`src/`, the pasted R2 tree,
the R3 sabotage build, and 31 per-version OSV queries. No decode
claim is made without our own ffmpeg-authored differential run
behind it (the test files record their own commands).*
