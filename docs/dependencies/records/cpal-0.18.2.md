# deps/records/cpal-0.18.2.md — cpal 0.18.2 (P0 verification record, card 2.C3)

> **Class:** P0 dependency verification (doc 11 §2 format, fields R1–R10),
> ordered by doc 13 card 2.C3 ("cpal `[VERIFY-P0]` purity; fallback:
> platform adapters per doc 01 §2.7"). Evidence gathered 2026-10-21 in the
> build sandbox: crates.io API (`/crates/cpal`, `/crates/cpal/0.18.2`,
> `/crates/cpal/0.18.2/dependencies`), static tarball inspection of cpal
> 0.18.2 + the Linux-default transitive closure (alsa 0.11.0, alsa-sys
> 0.4.0), and one **build attempt of a throwaway /tmp project** (not this
> workspace — the 1.E5-R negative-fixture convention).
> **Final acceptance and the allowlist row are Lead decisions (doc 11
> §3.1).** This record's verdict: **conditional (E2), NOT admitted in
> 2.C3 — the card's platform-adapter fallback is executed instead.**

## R1 — Language & source

- Rust, **no native source files** in the cpal tarball (sha256
  `6f02e8d0327b42d3e2e4ab2119af397344eb9fc54a34bf0ddeaa1277af8681f1`,
  233,947 bytes, 59 `.rs` files): no `*.c *.cc *.cpp *.cxx *.h *.hpp *.m
  *.mm *.S *.s *.asm` — the only non-Rust source is
  `src/host/audioworklet/worklet.js` (the wasm32 AudioWorklet JS shim,
  feature-gated `audioworklet`, web target only).
- `extern "C"` blocks exist **only** in the two web hosts
  (`src/host/webaudio/mod.rs`, `src/host/audioworklet/dependent_module.rs`)
  — JS interop on wasm32, dead on desktop targets.
- No `links=` key. No binary blobs (`.a .lib .o .obj .so .dll .dylib`).
- Repo: https://github.com/RustAudio/cpal (crates.io `repository` field).
  Description: "Low-level cross-platform audio I/O library."
- License (version endpoint, 0.18.2): **Apache-2.0** — inside deny.toml's
  allow set. `rust-version = "1.85"`.

## R2 — Transitive graph (Linux, default features)

Per the crates.io dependencies API (fetched 2026-10-21) + tarball
manifests, `cpal 0.18.2` default-features on
`cfg(linux)`:

- **cpal → (target-independent)** `dasp_sample ^0.11` (pure Rust sample-
  conversion traits);
- **cpal → (linux)** `alsa ^0.11` + `libc ^0.2` (required);
- `alsa 0.11.0` (Apache-2.0/MIT; tarball sha256
  `812947049edcd670a82cd5c73c3661d2e58468577ba8489de58e1a73c04cbd5d`)
  → `alsa-sys ^0.4.0`, `bitflags ^2.4`, `cfg-if ^1.0`, `libc ^0.2.178`;
- `alsa-sys 0.4.0` (MIT; tarball sha256
  `ad7569085a265dd3f607ebecce7458eaab2132a84393534c95b18dcbc3f31e04`)
  → `libc` (normal) + `pkg-config ^0.3.7` (**build** dep).
- Optional and OFF by default: `jack`, `pipewire`, `pulseaudio`,
  `asio-sys` (+`num-traits`), `audio_thread_priority` (`realtime`),
  `alsa-sys` (the `realtime` feature's direct use), wasm/android/apple
  target families. Every optional native surface is therefore opt-in;
  the default Linux build is **ALSA-only**.
- `[VERIFY-P1: regenerate the exact graph from our own Cargo.lock if cpal
  is ever admitted]` — this record walked API + tarballs, not a lockfile.

## R3 — Build scripts

- **cpal `build.rs` (read in full, 6 lines):** emits
  `cargo:rerun-if-env-changed=CPAL_ASIO_DIR` and a `rustc-cfg=asio` when
  that env var is set. **No toolchain invocation, no network, no OUT_DIR
  binaries.** Benign.
- **alsa 0.11.0: no build.rs.**
- **alsa-sys 0.4.0 `build.rs` (read in full):** runs
  `pkg_config::Config::new().statik(false).probe("alsa")` — i.e. it
  **invokes the system `pkg-config` tool to locate the system ALSA
  library (libasound) and emits link flags for it**; on probe failure it
  panics with an install-hint message (observed live, R5 below). Under
  the non-default `use-bindgen` feature it would additionally invoke
  bindgen + clang to regenerate bindings — **not used** (the bindings are
  committed at `src/generated.rs`; `wrapper.h` exists only for that
  regeneration path).
- F2 tokens (`cc clang gcc cl.exe nasm yasm masm cmake make xcrun`):
  **none anywhere in the default graph.** `pkg-config` is a build-time
  system-tool execution — recorded, not hidden.

## R4 — Native links

- The native surface is **the OS's own audio service**: on Linux the
  system `libasound` (ALSA), located via pkg-config and linked as a
  shared library — **no vendored C, no compiled C, no assembly** (F1/F4/
  F7 all NO). This is exactly doc 11 §1.2's **E2 exception class**
  ("audited OS-service bindings … call OS-provided services, not vendored
  C code"), whose registered ADR slot for audio output is **ADR-005**
  (not yet written). F3 (third-party C media engine wrapper): **NO** —
  matching the preaudit-foundation finding ("every native dependency is
  an OS audio service binding, no third-party media C library").

## R5 — Targets (and the sandbox build attempt)

- Desktop triples via per-target OS deps (ALSA / CoreAudio+AudioToolbox /
  WASAPI via `windows` 0.62 with a minimal feature list; android AAudio;
  wasm32 WebAudio). `docs.rs` metadata confirms the matrix.
- **Empirical sandbox evidence (2026-10-21):** a throwaway /tmp project
  depending on `cpal = "0.18.2"` **fails to build** in this sandbox —
  exit 101, alsa-sys build.rs: *"The system library `alsa` required by
  crate `alsa-sys` was not found. The file `alsa.pc` needs to be
  installed…"* The sandbox ships the runtime `libasound.so.2` but no ALSA
  development package (no `alsa.pc`, no headers), so pkg-config cannot
  resolve it. There is also **no audio device** in the sandbox (no ALSA
  pcm devices to open). Consequence: in this environment a cpal-backed
  crate cannot even compile, let alone be exercised against a device.
- CI-side build on a fuller image: `[VERIFY-P0: not runnable here]`.

## R6 — Actual feature coverage (against doc 03 §4.9's needs)

- cpal's callback model (`Stream` with a user data callback receiving
  `&mut [f32]`/`&[f32]`) aligns with doc 03 §4.9's `RenderCallback`
  contract (the preaudit made the same observation). Device enumeration,
  pause/resume, and default-device selection exist in the API. Sample
  counters are NOT provided by cpal (no position API on all hosts) — the
  clock truth source would still be our own atomic fed by the callback
  (the design this card ships behind the null adapter). Coverage against
  real devices: **unverifiable in this sandbox** (no device).

## R7 — License

- cpal 0.18.2: Apache-2.0 (version endpoint). alsa 0.11.0:
  Apache-2.0/MIT. alsa-sys 0.4.0: MIT. dasp_sample: per preaudit, MIT OR
  Apache-2.0 (quick-pass row class). All inside deny.toml's allow set —
  no license-set change would be needed. `[VERIFY-P1: re-check dasp_sample
  at the pinned version at admission time.]`

## R8 — Maintenance & security record

- RustAudio org, `actively-developed` badge, release 0.18.2 dated
  2026-08-16 (crates.io `updated_at`). Bus factor: the RustAudio org has
  multiple maintainers but cpal's effective maintainer circle is small —
  recorded as risk, consistent with the preaudit.
  `[VERIFY-P1: RustSec query at admission date.]`

## R9 — Provenance

- Tarballs fetched from static.crates.io with sha256 recorded above;
  publishers per crates.io API `[VERIFY-P1: publisher identity +
  index-hash cross-check at admission time]`.

## R10 — Verdict

**conditional (E2) — and NOT admitted by card 2.C3.** The card's own
decision procedure resolves this way on four independent grounds:

1. **Policy vehicle missing.** E2 admission requires an ADR with the
   exact API surface + safety review (doc 11 §1.2); audio output's slot
   is ADR-005, which is Lead-owned and unwritten. The preaudit
   (§3.3) explicitly reserves "cpal vs direct-API" for ADR-005.
2. **Architecture places OS backends in L5.** Doc 01 §2.6's
   audio-output row: deps `media-types, audio-engine`, "OS-specific
   backends are implemented in `platform-*`" (§2.7: "audio backend (API
   per ADR-005)"); preaudit policy consequence 1: L0–L4 core crates
   depend only on `pass` crates — anything native lives behind a
   platform adapter. Wiring cpal into L4 `audio-output` would contradict
   the registry the card is implementing.
3. **The sandbox cannot honestly validate it.** No ALSA dev package ⇒
   alsa-sys cannot build (empirical exit 101 above); no audio device ⇒
   no device path can run. Even the card's reduced bar ("compiles + is
   exercised against a mock/virtual device where possible") is
   unreachable for the cpal graph here.
4. **Wave-2 dependency discipline.** D-018's recorded logic (zero new
   dependencies for the first vertical) is the standing precedent; the
   fallback the card names — platform adapters per doc 01 §2.7 — is the
   architecture-conformant v0.

What ships instead: `audio-output` defines the `AudioBackend` trait seam
(extends audio-engine's `AudioSink` with enumerate/open/pause/resume +
the sample counter), a fully implemented + tested **null backend**
(CI-safe, carries the card's ACs), and **typed not-supported stubs** for
the real-device backends (stable error code naming the target and
ADR-005). When ADR-005 decides cpal-vs-direct, the implementation lands
in `platform-*` (L5) against this record + a fresh
`[VERIFY-P1]` pass (lockfile graph, RustSec, publisher pins) — or in an
audio-output feature if the Lead explicitly amends the registry row.

*Evidence posture (AGENTS §3.4): every statement above was executed
locally in the build sandbox on 2026-10-21 (API fetches, sha256-summed
tarball extractions, full build.rs reads, the /tmp build attempt with
un-piped exit code 101 — the DISK-INCIDENT-1 lesson applied). Nothing
was run against a real audio device; nothing here asserts device
behavior.*
