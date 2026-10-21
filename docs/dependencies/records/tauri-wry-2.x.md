# deps/records/tauri-wry-2.x.md — tauri 2.12.4 + wry 0.57.0 (pre-audit record, the ADR-004 admission vehicle)

> **Class:** Phase 0 pre-audit record (doc 11 §2 format, fields R1–R10 —
> quick-pass level: crates.io API metadata + dependency listing per
> `docs/dependencies/audit-method.md`; the full tarball/graph scan happens at
> admission time). Data fetched 2026-10-19 by task 0.C4 (raw JSON in
> `tmp/preaudit2/`, gitignored; consolidated in
> `docs/dependencies/preaudit-foundation.md` §1/§2). Fields that need the
> workspace/lockfile or a desktop build environment are `[VERIFY-P1]`.
> **Exception class: E1 — OS webview, UI shell** (doc 11 §1.2), reserved to
> **ADR-004** — now accepted (`docs/adr/ADR-004-os-webview.md`).
> **Final acceptance and the allowlist row are Lead decisions (doc 11 §3.1).**
> This record's verdict: **exception-ADR-004, NOT imported** — the desktop
> skeleton implements the typed seam without the dependency.

## What and why

- **tauri 2.12.4** (Apache-2.0 OR MIT, updated 2026-10-14) — the desktop app
  shell framework: window lifecycle, IPC, sidecar plumbing. Rust core hosting
  OS webviews.
- **wry 0.57.0** (Apache-2.0 OR MIT, updated 2026-09-08) — the webview host
  abstraction: **WebView2** (Windows), **WKWebView** (macOS), **WebKitGTK
  4.1/GTK3** (Linux — the 0.C5-a finding that feeds ADR-003's R-04
  assessment).
- **tao** rides the same exception envelope (tauri's windowing crate;
  preaudit §2 note).
- Consumed **only** as the L6 `apps/desktop` UI shell under ADR-004. They are
  not engine code: **the engine never links them** (preaudit §2;
  `boundaries.toml` apps/desktop entry). Referenced by:
  `apps/desktop/Cargo.toml` (header comment), `apps/desktop/src/{lib,main,
  transport,error}.rs`, `apps/desktop/README.md`, `docs/gates/phase2-exit.md`
  §3, `docs/dependencies/allowlist.md` §5, ADR-003/ADR-004.

## R1 — Language & source

- Rust crates; the native surface is the **OS webview engine itself**
  (WebView2 / WKWebView / WebKitGTK) — this is the E1 exception, not an E2
  OS-service binding and not a vendored third-party C media library.
- Tarball-level purity scan (0 `*.c/*.h`, `extern "C"` census, blobs,
  `links=`): **[VERIFY-P1 at admission]** — the 0.C4 quick pass did not
  extract tarballs for this pair; the method is `audit-method.md` §4.

## R2 — Transitive graph

- Known from the preaudit: tauri pulls **wry + tao** (same exception
  envelope). The full per-target closure (GTK bindings, windows/macos crates)
  is **[VERIFY-P1: `cargo tree -e normal,build` at the pinned lockfile]** —
  expected to be large; the record at admission must enumerate it and confirm
  no deny.toml-banned name (`libloading`, `bindgen`, codec wrappers) enters
  through it.

## R3 — Build scripts

- **[VERIFY-P1]**: build-script census (pkg-config probes for
  webkit2gtk-4.1 are expected on Linux — same class as cpal's `alsa-sys`
  probe, recorded not hidden).

## R4 — Native links

- The OS webview libraries, linked per target by wry. F3 (third-party C
  *media* engine wrapper): **NO** — the webview is the OS-provided UI surface
  sanctioned by MP §2's React+TS fixed decision and doc 11 §1.2 E1. Scope,
  versions and the security review are owned by ADR-004 (its decision 1/4).

## R5 — Targets

- Desktop triples only (x86_64/aarch64 × Windows/Linux/macOS); no wasm32 role
  (apps/web is its own target, doc 09).
- **Empirical environment fact (2026-10-21, card 2.D3):** this sandbox has no
  `webkit2gtk-4.1` dev packages and no display server — a tauri/wry build is
  not attempted here by design; the crate compiles std-only instead
  (`apps/desktop/Cargo.toml` header comment).

## R6 — Feature coverage (against the seam's needs)

The 2.D3 seam (`apps/desktop/src/transport.rs`) names three host kinds; the
admitted dependency must serve them:

| Seam (`WebviewHostKind`) | Need from tauri/wry | Status |
|---|---|---|
| `TauriChannel` | tauri IPC as the LUF1 byte transport | designed; wire protocol v1 works over any ordered byte channel (2.D3 proof) |
| `CustomProtocol` (`lumen://`) | custom protocol registration for stream delivery | pending wire protocol v2 (origin handshake) — ADR-004 decision 4 |
| `SharedMemory` | SharedArrayBuffer ring under host-controlled COOP/COEP | pending wire protocol v2 |

Also required at Phase 4: transparent/composited windows for ADR-003
candidate A on macOS/Windows (the `WS_CLIPCHILDREN` and `drawsBackground`
items recorded in ADR-003 §Context). None of this is exercised yet — the
prototype proved the **protocol core** (variant B) at the deterministic
virtual-time level, verdict **PROVISIONAL-B**
(`docs/prototypes/video-in-webview.md`); per-OS browser verdicts are
PENDING a desktop machine.

## R7 — License

- tauri 2.12.4: **Apache-2.0 OR MIT**; wry 0.57.0: **Apache-2.0 OR MIT**
  (crates.io version endpoints, 2026-10-19). Both inside deny.toml's allow
  set and compatible with the MIT repo (ADR-006). Transitive licenses:
  **[VERIFY-P1: cargo-deny licenses at lockfile]**.

## R8 — Maintenance & security record (fetched 2026-10-19)

- tauri 2.12.4 released 2026-10-14; wry 0.57.0 released 2026-09-08 — both
  active, frequent-release projects (the Tauri org, multiple maintainers).
- RustSec/OSV query at the admission date: **[VERIFY-P1]**. Bus factor:
  healthy at org level; the standard R-43 re-audit cadence applies once
  admitted.

## R9 — Provenance

- crates.io API (UA `lumen-preaudit/0.1`), 2026-10-19; raw JSON preserved in
  `tmp/preaudit2/` (gitignored). Tarball sha256 pins + sparse-index
  cross-check + publisher identity: **[VERIFY-P1 at admission]** (the wasmi
  record's §R9 pattern).

## R10 — Verdict (pre-audit)

**exception-ADR-004 — and NOT imported.** The exception ADR is accepted
(2026-10-22, `docs/adr/ADR-004-os-webview.md`), but the dependency is
pre-audited only: nothing in the lockfile, no row in allowlist §3 (it is
listed in §5 "Approved-but-not-imported"). The honest state:

1. `apps/desktop` (2.D3) implements the **engine-side half** — the variant-B
   protocol core + the typed `WebviewHost` seam — std-only, zero deps, 32
   tests green; `open_webview()` refuses with `HostError::WebviewPending`
   (`src/host.rs`), `active_presentation_path()` reports the truth
   (ADR-003 decision 4).
2. The **prototype verdict is PROVISIONAL-B** (protocol proven; per-OS webview
   matrix PENDING — `docs/prototypes/video-in-webview.md`).
3. No webview runtime exists in the sandbox; per-OS claims are unmade.

## Admission criteria (the gate this record enforces)

1. A desktop build environment: webkit2gtk-4.1 dev packages + a display
   server (or the per-OS equivalents), plus CI matrix images carrying the
   webview runtimes.
2. The `[VERIFY-P1]` refreshes above: full tarball scan, locked-graph
   enumeration + deny.toml cross-check, build-script census, RustSec,
   publisher pins, cargo-deny licenses.
3. The ADR-004 security review (doc 11 §1.2 E1 scope: webview sandbox
   posture, IPC surface, navigation/content isolation — TB3) executed against
   the real import.
4. A Lead allowlist row (allowlist.md §3) with a pinned version range +
   feature set, and the same-PR rule for any `Cargo.lock` delta.
5. The ADR-003 per-OS prototype runs recorded (the to-run list in
   `docs/prototypes/video-in-webview.md`) so the shell lands with per-OS
   verdicts, not expectations.

*Evidence posture (AGENTS §3.4): everything above is either a recorded
crates.io fetch (2026-10-19, preaudit §4), a repository artifact
(apps/desktop, ADR-004, the prototype report), or an explicit [VERIFY-P1].
Nothing here has been built or executed against a webview.*
