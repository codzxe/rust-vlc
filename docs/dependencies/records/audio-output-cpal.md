# deps/records/audio-output-cpal.md — cpal admission spike (issue #1, wave 14)

> **Class:** admission spike — validate-then-admit evidence for ADR-005's
> pending real-backend decision. The assignment pinned `cpal = "0.15"`
> (resolved **0.15.3**; the newer **0.18.2** already carries the card-2.C3
> P0 record, `records/cpal-0.18.2.md` — cross-referenced throughout).
> Every graph/license/MSRV/unsafe statement below is **real cargo output**
> gathered 2026-11-09 in a throwaway `/tmp/cpal-spike` project (never this
> workspace — **cpal is not admitted and not a workspace dependency**; the
> 1.E5-R negative-fixture convention). Toolchain: cargo 1.99.1
> (5f94df478 2026-08-27).
> **Admission is Lead-gated (AGENTS §3.2/§3.3): this record ends in a
> `[PROPOSAL]`, not an allowlist row.**

## R1 — Language & source

- cpal 0.15.3, repo `https://github.com/rustaudio/cpal`, crates.io
  description: *"Low-level cross-platform audio I/O library in pure
  Rust."* — the "pure Rust" claim covers **the cpal crate only**: no
  vendored/native source ships in cpal itself, but every desktop host
  binds a **C OS API** through `-sys` crates (R4). "Rust API" ≠ "pure
  Rust graph" — exactly AGENTS §3.2's warning.
- License (cargo metadata): **Apache-2.0**. MSRV: `rust-version = "1.70"`
  (real manifest field). No `links=` key in cpal itself.

## R2 — Transitive graph (real `cargo tree`, resolved from the index)

Linux (host, default features, `cargo tree -e normal,build`):

```
cpal v0.15.3
├── alsa v0.9.1
│   ├── alsa-sys v0.3.1          [links = "alsa"; MIT]
│   │   ├── libc v0.2.191
│   │   [build-dependencies]
│   │   └── pkg-config v0.3.34
│   ├── bitflags v2.13.2
│   ├── cfg-if v1.0.5
│   └── libc v0.2.191
├── dasp_sample v0.11.0
└── libc v0.2.191
```

macOS (`--target aarch64-apple-darwin`):

```
cpal v0.15.3
├── core-foundation-sys v0.8.7
└── coreaudio-rs v0.11.3
    ├── bitflags v1.3.2
    ├── core-foundation-sys v0.8.7
    └── coreaudio-sys v0.2.18     [MIT; RustAudio]
        [build-dependencies]
        └── bindgen v0.72.1       → clang-sys 1.9.1, libloading,
                                    regex, nom, cexpr, … (C-toolchain
                                    build chain)
```

Windows (`--target x86_64-pc-windows-msvc`):

```
cpal v0.15.3
├── dasp_sample v0.11.0
└── windows v0.54.0               [metadata-based, no bindgen]
    └── windows-core v0.54.0
        ├── windows-result v0.1.2
        └── windows-targets v0.52.6 → windows_x86_64_msvc 0.52.6
```

Optional hosts are OFF by default (jack, pulseaudio, pipewire, asio…) —
the default Linux build is **ALSA-only**, matching the 0.18.2 record's
finding. Version delta vs the P0 record: 0.15.3 → alsa 0.9.1 /
alsa-sys 0.3.1, while 0.18.2 → alsa 0.11 / alsa-sys 0.4; the latest
`alsa` on crates.io is 0.12.1 (2026-07-31). `[VERIFY-P1: regenerate the
graph from our own Cargo.lock at admission — admission should target the
newest cpal, not 0.15.]`

## R3 — Build scripts

- **cpal 0.15.3 `build.rs` (read in full):** emits
  `rerun-if-env-changed=CPAL_ASIO_DIR` + a `rustc-cfg=asio` when that env
  var is set. No toolchain, no network, no OUT_DIR binaries. Benign.
- **alsa-sys 0.3.1:** `pkg-config` build dep probing the **system
  `libasound`** (`links = "alsa"`). No bindgen in 0.3.1 (committed
  bindings — the R2 graph shows none).
- **coreaudio-sys 0.2.18:** an **unconditional `bindgen` build dep** on
  the macOS target (R2 graph) — bindgen invokes **libclang/clang at build
  time** to generate CoreAudio FFI. This is the heaviest build-script
  surface in the graph and a macOS-admission consideration the 0.18.2
  refresh must re-check (newer coreaudio-sys may ship pre-generated
  bindings) `[VERIFY-P1]`.
- **windows 0.54.0:** metadata/import-lib based — no compiler toolchain
  in the build graph.

## R4 — Native links & unsafe exposure (E-class deviation evidence)

The native surface is the **OS's own audio service** — no vendored C, no
third-party media C library: ALSA `libasound` (pkg-config, shared), Apple
CoreAudio/CoreFoundation frameworks, Windows WASAPI via the `windows`
metadata crate. This is doc 11 §1.2's **E2 exception class** — the
ADR-003/ADR-004 exception pattern (audited OS-service bindings; unsafe
confined to the adapter layer, documented invariants per ADR).

`unsafe` exposure, counted in the real extracted sources (rg):

- cpal 0.15.3 `src/`: wasapi/device.rs **28**, wasapi/stream.rs **15**,
  coreaudio/macos/mod.rs **11**, lib.rs **9**, alsa/mod.rs **8**,
  asio/stream.rs **7**, coreaudio enumerate/property_listener **9** —
  every host is a C-FFI wrapper; unsafe is structural, not incidental.
- alsa 0.9.1 `src/`: pcm.rs **49**, ctl_int.rs **54**, rawmidi.rs **22**,
  hctl.rs **9** — the "thin but safe wrappers" still carry the FFI seam.
- alsa-sys 0.3.1 `src/generated.rs`: **1,301 `extern "C"` functions** —
  the raw C ABI surface cpal's Linux host sits on.

Consequence for us (AGENTS §3.3): cpal can never be imported by an
L0–L4 core crate; it may only sit behind an **approved-ADR platform
adapter** (ADR-005 decision 2: `platform-*`, L5) — which is the current
architecture.

## R5 — Targets & the sandbox build attempt

- Resolution/graph capture (`cargo tree` for all three targets above)
  **succeeded** — index resolution needs no audio dev packages; this is
  how R2 was obtained without admitting anything.
- **Build attempt (real):** `cargo build` in `/tmp/cpal-spike` fails with
  **exit 101** — `error: failed to run custom build command for alsa-sys
  v0.3.1` … `Package alsa was not found in the pkg-config search path`
  (the sandbox ships no `alsa.pc`/ALSA dev package) — reproducing the
  0.18.2 record's R5 finding on the 0.15 chain. No audio device exists
  here either: **a cpal-backed crate can neither compile nor be exercised
  in this sandbox**; admission additionally requires a CI image with the
  target's audio dev packages (ADR-005 decision 3).

## R6 — Feature coverage vs the seam

cpal's model (device enumeration, `Stream` with a `&mut [f32]`/`&[f32]`
callback, pause/resume, default-device selection) maps onto the
implemented `AudioBackend` trait (`enumerate/open/pause/resume` +
`clock_source` + `underruns`); our `render_into` is the callback body a
cpal adapter would install. cpal provides **no sample-position API on
all hosts** — the clock truth stays our `SampleCounter` advanced inside
the callback (the design the seam already ships). Device behavior:
**unverifiable here** (no device).

## R7 — Licenses (real `cargo tree -f "{p} [{l}]"`)

cpal Apache-2.0; alsa Apache-2.0/MIT; alsa-sys MIT; libc, dasp_sample,
bitflags, cfg-if, core-foundation-sys, windows: MIT OR Apache-2.0;
coreaudio-rs MIT/Apache-2.0; coreaudio-sys MIT. All inside the deny.toml
allow set. MSRVs: cpal 0.15.3 **1.70**, windows 0.54 **1.62**, alsa /
alsa-sys **undeclared** (risk noted); our toolchain 1.99.1 satisfies all.

## R8 — Maintenance

- cpal: RustAudio org; 0.15.3 is old (0.18.2 is current, released
  2026-08-16 per the P0 record) — admission targets latest. Small
  effective maintainer circle (recorded risk in the P0 record).
- alsa (diwic/alsa-rs): max 0.12.1, updated **2026-07-31**, ~28.6M
  downloads (crates.io API) — active.
- RustSec: `[VERIFY-P1: query at admission date]`.

## R9 — Provenance

Registry `index.crates.io-1949cf8c6b5b557f` via cargo 1.99.1; sources
inspected in `~/.cargo/registry/src` (manifests, build.rs, generated.rs
counts above are from the extracted tarballs). `[VERIFY-P1: tarball
sha256 + publisher pins at admission — the 0.18.2 record's R9 procedure.]`

## R10 — Alternatives survey (what the crate pages/API actually say)

- **`alsa` (alsa-rs, diwic/alsa-rs):** *"Thin but safe wrappers for ALSA
  (Linux sound API)"* — active (0.12.1, 2026-07-31). **Not pure Rust**:
  binds system libasound through alsa-sys (E2, same class); it is
  literally cpal's own Linux substrate. Choosing it directly = the
  "direct-API" path of ADR-005 decision 4 (finer PCM control, one host
  only).
- **`libpulse-binding`:** *"A Rust language binding for the PulseAudio
  libpulse library"* (2.30.1, updated 2025-04-19, ~6.9M dl). Binds the
  **C client library of a userspace sound *server*** — not a kernel OS
  service: adds a runtime daemon dependency and sits outside the
  E2 "OS-provided service" reading. Weaker fit; recorded, not
  recommended. `[VERIFY: current maintenance status before any use.]`
- **`rodio`:** *"Audio playback and recording library"* (0.23.0,
  2026-10-11) — **not a backend alternative**: its normal dependencies
  include **`cpal ^0.18.2`** (crates.io dependencies API, fetched
  2026-11-09). It is a higher-level convenience layer on top of the same
  substrate.
- **Pure-Rust conclusion:** no pure-Rust path to OS audio output exists
  on any desktop target — every OS exposes audio through C ABIs (ALSA C
  library, CoreAudio C frameworks, WASAPI COM). The E-class exception is
  **inherent** to audio output; the only pure-Rust layer possible is the
  seam we already ship (L4, std-only).

## Verdict & recommendation

**NOT admitted** (Lead-gated; nothing changed in the workspace).
`[PROPOSAL — Lead sign-off]` — the recommended path, mirroring ADR-005's
decision 3:

1. **Linux-first**: admit cpal at the **latest** version (currently
   0.18.2 per the P0 record; refresh the R2 graph from the admission
   lockfile) as the `platform-linux` (L5) substrate implementing the
   existing `AudioBackend` trait — default features only (ALSA host; no
   jack/pulse/pipewire features), unsafe confined to that adapter with
   documented invariants (AGENTS §3.3, ADR-003/004 pattern).
2. **Preconditions**: CI image carrying `libasound2-dev` (alsa.pc); the
   P0 record's `[VERIFY-P1]` set (lockfile graph, RustSec, publisher
   pins, dasp_sample license); ADR-005 amended with the substrate choice
   + safety review per doc 11 §1.2 E2.
3. **Per-target follow-ups** (separate admissions, decision 4's
   per-target flip): macOS — weigh cpal's coreaudio-sys **bindgen build
   chain** (R3) against direct coreaudio-rs/metadata bindings;
   Windows — cpal's `windows`-crate path is metadata-based (no bindgen)
   and the cleanest of the three.
4. The flip point is already coded: `audio_output::cpal` carries the
   typed `BackendNotAdmitted` constructor marker + the
   `AUDIO_ADR005_ADMITTED` const (the streaming TLS seam pattern) — the
   Lead's admission change flips it in the same commit that lands the
   adapter.

*Evidence posture (AGENTS §3.4): every graph, license, MSRV, unsafe
count and build failure above was produced by real cargo/crates.io API
runs in the build sandbox on 2026-11-09 (`/tmp/cpal-spike`, exit codes
recorded un-piped where they matter: build = 101). Nothing ran against a
real audio device; nothing here asserts device behavior.*
