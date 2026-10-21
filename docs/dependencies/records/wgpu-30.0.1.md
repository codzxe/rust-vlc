# deps/records/wgpu-30.0.1.md — wgpu 30.0.1 (deferral decision record, card 2.D1)

> **Class:** dependency **deferral** decision record — NOT a full P0
> verification. Card 2.D1's honest paths offered (1) wgpu on a software
> backend if the environment supports it, (2) the full renderer
> architecture behind an internal backend seam with the AC met by the
> software path and the wgpu import deferred with a recorded decision,
> (3) wgpu vendored + compiling with the AC on the software path anyway.
> This record is the evidence for choosing **(2)**. A full R1–R10
> verification (doc 11 §2, the `records/cpal-0.18.2.md` depth) is owed
> by whoever admits the import; §6 lists what it must add beyond this.
> Evidence gathered 2026-10-20/21 in the build sandbox: environment
> probes, the crates.io API (`/crates/wgpu`, `/crates/wgpu/30.0.1`,
> `/crates/wgpu-hal/30.0.1/dependencies`), and one **empirical probe of
> a throwaway /tmp project** (not this workspace — the 1.E5-R
> negative-fixture convention). **Final acceptance and the allowlist row
> are Lead decisions (doc 11 §3.1). This record's verdict:
> conditional (E2), NOT admitted in 2.D1 — the card's software-path
> fallback is executed instead.**

## 1 — The environment: no GPU stack at all (verified)

- `/dev/dri` does not exist (probe exit 2 — no DRI device nodes).
- `vulkaninfo` / `glxinfo` are not installed.
- The Vulkan **loader** is present (`libvulkan.so.1` in the linker map)
  but there are **no ICDs**: `/etc/vulkan/icd.d/` exists and is empty;
  `/usr/share/vulkan/icd.d/` does not exist. No `lvp_*`/lavapipe ICD
  JSON, no `swrast`/`llvmpipe` driver library anywhere in the loader
  paths — honest path (1) (wgpu over a software Vulkan/GL backend) is
  **not available** in this sandbox.
- `libEGL` is absent from the linker map (only `libwayland-egl.so.1`, a
  Wayland client helper, not EGL); `libGL`/`libGLX_mesa` exist but no
  display server runs.

**Empirical wgpu 30.0.1 probe (throwaway /tmp project, default
features, release build — compiles clean, exit 0):**
`Instance::request_adapter(&RequestAdapterOptions::default())` returns
`Err(RequestAdapterError)`:

```
No suitable graphics adapter found; noop support not compiled in,
vulkan drivers/libraries could not be loaded, metal support not
compiled in, dx12 support not compiled in, gl drivers/libraries could
not be loaded, webgpu support not compiled in
```

So even a compiled-in wgpu (honest path 3) would carry **zero
executable renderer evidence** in this environment — every backend
enumerates to nothing. The 2.C3 cpal situation (cannot even build) is
stricter, but the consequence for an honest gate is the same: no run,
no AC.

## 2 — The graph: banned crate in the default Linux closure (verified)

- wgpu 30.0.1 is the newest crates.io release (2026-10). Default
  features = `std, parking_lot, dx12, metal, gles, vulkan, wgsl, webgpu`.
- The default-features normal-edge graph on Linux resolves to **80
  unique packages** (`cargo tree -e normal --prefix none | sort -u`).
- **`libloading v0.8.9` is in that closure** — entered via `ash v0.38`
  (the **Vulkan** backend's loader dlopen) and `dlib v0.5` (the GL/X11
  family), not only via the GL backend. (The crate docs' earlier
  "Linux GL backend pulls libloading" phrasing is thus *understated*;
  corrected here.) `deny.toml` bans `libloading` unconditionally — the
  workspace's own cargo-deny gate would fail the import as-is.
  Manifest-level fact (crates.io dependencies API): wgpu-hal 30.0.1
  declares `libloading ^0.8` as a normal optional dependency targeted at
  `cfg(not(target_arch = "wasm32"))`.
- `cargo-deny` is not installed in this sandbox session (the 1.E5-R
  pinned binary is gone), so the ban violation is established at the
  manifest level above, not by a live gate run — recorded honestly.
  `[VERIFY-P0: run the pinned cargo-deny bans check against the exact
  admitted feature set at admission time.]`

## 3 — The envelope and the architecture

- Doc 13 card 2.D1 and doc 01 §2.6's renderer-wgpu registry row grant
  `media-types, video-engine` — no external crates. Pinning an
  80-package E2 graphics stack is the preaudit's "conditional, full
  R1–R10 record at pin time" path (doc 11 §2; allowlist §5 lists wgpu
  as conditional-E2, pre-audited, rows added only when actually
  imported).
- Doc 01 §2.6/§2.7: windowed surface creation and OS integration are
  L5 platform-adapter territory (ADR-003 decides the raw-window-handle
  strategy); the renderer crate's v0 job is the render pipeline behind
  the `Presenter` seam.
- Gate economics (DISK-INCIDENT-1 sensitivity): `full_gate.sh` runs
  `--all-features`, so a feature-gated wgpu would compile the graphics
  stack on every future gate for code that cannot run here (the probe's
  check artifacts alone were 241 MB).

## 4 — The decision (path 2, executed)

`renderer-wgpu` v0 ships the **complete renderer architecture behind
the internal `Compositor` seam, std-only**:

- the **software compositor** (CPU point-sampled rasterizer over the
  `RenderSurface` write protocol) is linked and carries the card's AC —
  the planned 7.C1 "software present" fallback, arrived early;
- every wgpu integration point is typed and shape-pinned before the
  dependency exists: `PresentMode::wgpu_present_mode()` (the exact
  `wgpu::PresentMode` variant names), `SurfaceFormat::
  wgpu_texture_format()` (`Bgra8UnormSrgb`/`Rgba8UnormSrgb`), the
  `Renderer::with_backend` injection point (surface + compositor pair),
  and the `RenderSurface` contract a wgpu surface implements (with a
  GPU compositor free to bypass the CPU write protocol with its own
  draw path);
- the hardware backend joins when the Lead ratifies the import — the
  2.D3 prototype is the evidence vehicle; ADR-003 owns window/surface
  strategy.

## 5 — What the AC actually ran (the honesty bottom line)

The card's acceptance criterion — "headless render of a synthetic
frame produces the expected PNG hash" — is met by the **software path
in this sandbox**: synthetic frame → `Presenter::present` → measured
staging → placement → software composite → sRGB surface → readback →
PNG → committed SHA-256 (`1466e74a…eb30bf`, sha256sum + Python-zlib
oracle verified; `tests/png_hash.rs`). The wgpu path has no executable
evidence here and none is claimed.

## 6 — Owed by the admission-time record (Lead-gated)

1. Full R1–R10 record at the pinned version + feature set (tarball
   inspection incl. every build script, F1–F7 scans, sha256
   provenance pins, publisher identity) — the cpal record's depth.
2. The `libloading` disposition: a deny.toml exception ADR, a
   libloading-free feature set (Vulkan-only via ash's static entry?
   `[VERIFY]`), or a Lead-ratified ban carve-out — the current ban
   makes default-features wgpu unadmittable as-is.
3. The window/surface story: ADR-003 (raw-window-handle vs webview
   frame streaming) and which layer owns surface creation (L5).
4. A CI image with GPU drivers (or lavapipe) + hardware-labeled runs —
   the analog of 2.C3's audio-device caveat; until then the wgpu
   backend's ACs run only where a backend enumerates.
5. Fresh `[VERIFY-P1]` pass on lockfile graph, RustSec and publisher
   pins at the actual pin time.

*Evidence posture (AGENTS §3.4): every environment statement above was
probed live in this sandbox (2026-10-20/21); the graph claims come from
the crates.io API + a real `cargo tree` of a throwaway project; the
adapter refusal is a real program run with an un-piped exit code. The
one tool that could NOT be run (cargo-deny) is named as such. The wgpu
tarball itself was NOT inspected — that is §6's job, not this record's.*
