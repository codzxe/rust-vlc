# AUDIT: apps/desktop-tauri/tauri.conf.json
| Field | Value |
|---|---|
| Auditor | W14-A07 |
| Date | 2026-11-21 |
| Lines / functions | 34 lines / 18 config keys (per-key table §4) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | COMPILE-TIME-ONLY, currently VOID. The conf is embedded by `tauri_build::build()` (apps/desktop-tauri/build.rs:11) and read by `tauri::generate_context!()` (app.rs:321) — but the consuming crate cannot compile: cross-file E0425, app.rs:301 calls `spawn_session_pump`, the definition is `spawn_event_pump` at app.rs:190 (verified by `sed -n '188,192p;298,304p'`; repo-wide rg: exactly one hit for `spawn_session_pump`, the call site). Primary ownership W14-A01; also recorded by W14-A02 main.rs.audit.md F-001. No `lumen-tauri` binary exists on any host; the `lumen` CLI does not read this file. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 3 |
| Commands run | `python3 -m json.tool apps/desktop-tauri/tauri.conf.json` → OK (well-formed JSON). `sed -n` on every quoted range in this audit (verbatim verification). `rg -n "spawn_session_pump\|spawn_event_pump" apps/desktop-tauri` → call :301 vs def :190. `ls ui/dist` → absent (CI builds it: ci.yml:719-724). `rg "__TAURI__" ui/src` → 0 hits (withGlobalTauri:false consistent). `rg "createObjectURL\|new Worker\|data:image/" ui/src` → only CSS `mask: url("data:image/svg+xml,...")` (img-src data: needed; blob:/worker allowances unused). `rg "dev.lumen.desktop"` repo-wide → conf:5 + crates/product-config/src/runtime.rs:32,111 (comments) + runtime.rs:162 (test asserts the DERIVED string, never reads the conf). `rg -n "tauri" .github/workflows/ci.yml` → tauri-check lane 680-775; `rg "cargo tauri"` repo-wide → tools/xtask/src/cmd/gate.rs:260 (the argv), gate.rs:1001 (a test pinning it), docs/10-testing-ci-quality.md:85 (doc row 9) — no CI/release lane executes tauri-cli. `file icons/icon.ico icons/icon.png` → real placeholder rasters. |

## 1. One-paragraph verdict

A real, well-formed Tauri 2 config whose security half is mostly sound
(`script-src 'self'`, no `assetProtocol`, `withGlobalTauri:false`, hard
`form/object/frame` locks, correct `connect-src ipc: http://ipc.localhost` for
the IPC) and whose frameless-shell wiring is internally consistent
(`decorations:false` ↔ UI banner controls via `plugin:window|*` ↔ the four
window grants in capabilities/default.json). But it is unexecutable as the
tree stands — its only consumer, the `lumen-tauri` crate, fails E0425
(`spawn_session_pump`, app.rs:301) — its `bundle` half has NO working consumer
anywhere (tauri-cli is never invoked by CI/release; the one `cargo tauri
build` in `xtask gate` stage 9 runs in `apps/desktop`, a directory with no
tauri.conf.json), its first-paint window `height: 800` drifts from the REAL
config `product.toml height = 720` that `apply_window_spec` re-asserts at
setup (a visible resize on every launch), and its CSP `media-src` provably
blocks the one thing the UI's `<video>` render seam ever mounts a `src` for
(remote url sources) while its `blob:`/`data:` allowances serve no code path.
Answering the wave's core question from this file's vantage: even once the
E0425 is fixed, a user who opens a playable file sees frames on the canvas
(LUF1 raw-binary IPC → `putImageData`, never CSP-governed) and hears NOTHING —
audio renders engine-side into the null device (`audio.null-device`, baseline
lumen-engine-info.txt) and no config key or IPC channel in this file carries
audio to the webview.

## 2. What actually works (verified)

— verified by `python3 -m json.tool` → JSON PARSE OK (the only executed check;
no build possible — standing UNVERIFIED-BUILD, baseline run #2
build-desktop-tauri.log.txt exit 101 at gdk-3.0 pkg-config).
— verified by rg/sed tracing (static):
  - `frontendDist: "../../ui/dist"` (conf:7) resolves from apps/desktop-tauri
    to the repo-root ui/dist; absent from the tree by design, and the CI lane
    builds it first and fails hard if missing (ci.yml:719-724: `npm --prefix ui
    run build; test -f ui/dist/index.html || … exit 1`; release.yml:194-201
    "ORDER IS LOAD-BEARING"). Baseline run #22 (ui-build.log.txt, EXIT=0)
    proves the UI builds.
  - `decorations: false` (conf:19) ↔ the banner's −/□/× controls invoke
    `plugin:window|minimize|toggle_maximize|close` (ui/src/client/
    window-controls.ts:70-72) ↔ capabilities/default.json:8-11 grants exactly
    those four `core:window:allow-*` (+ `core:default` covering
    `is_maximized`, `plugin:event|listen`). Chain intact across all three
    files.
  - `withGlobalTauri: false` (conf:10) ↔ the UI binds
    `window.__TAURI_INTERNALS__` directly (tauri-transport.ts:112-118,
    frame-channel.ts shim) and never reads `window.__TAURI__` (rg, 0 hits).
  - `connect-src 'self' ipc: http://ipc.localhost` (conf:23) covers the fetch
    IPC both the single-command transport and the raw frame channel ride
    (frame_channel.rs:15-22 "raw bytes ride the fetch-based IPC path").
  - `minWidth: 940, minHeight: 600` (conf:17-18) == product.toml
    min_width/min_height 940/600 (product.toml:70-71) — the w28 fix-5 floor
    agrees.
  - `identifier: "dev.lumen.desktop"` (conf:5) == product.toml's derived
    `{reverse_dns}.desktop` (`dev.lumen`, product.toml:37) and
    crates/product-config/src/runtime.rs:162 asserts the derived string.
  - `bundle.icon` entries exist and are real rasters: icons/icon.ico
    (256×256 PNG-in-ICO, 687 B), icons/icon.png (512×512 RGBA, 1819 B) —
    generated placeholders per scripts/release/gen-ico.py:4-7 ("tauri-build
    Windows resource step requires `icons/icon.ico`").
  - `productName: "Lumen"`/`version: "0.1.3"` == product.toml identity.name /
    display_version and Cargo.toml `version = "0.1.3"` (all verified strings).

## 3. Findings

### F-001 [S2]
Where: apps/desktop-tauri/tauri.conf.json:15-16 vs product.toml:64-65; runtime
override at apps/desktop-tauri/src/app.rs:255-258
Code:
```json
        "width": 1280,
        "height": 800,
```
```toml
# product.toml:63-65
[window]
width = 1280                  # schema 480..=3840
height = 720                  # schema 320..=2160
```
```rust
// app.rs:255-258 (setup, AFTER the config window already painted 800 tall)
        window.set_size(tauri::LogicalSize::new(
            f64::from(spec.inner_size.0),
            f64::from(spec.inner_size.1),
        ))?;
```
What is wrong: the conf's first-paint height (800, a 16:10 shape) disagrees
with the REAL window config product.toml [window] height=720 (16:9, "same 16:9
shape, one step down" per product.toml:58). Tauri creates config windows
before the setup hook, so every launch paints 1280×800 and then visibly
resizes to 1280×720. app.rs:234-236 claims the conf is "the first-paint
fallback, not a second source of truth" — the fallback has drifted, which is
exactly what a second source of truth does.
Why it matters: a window jump on every GUI launch, and a lying invariant: any
reader trusting either file gets a different answer for "how tall is the
Lumen window".
What correct looks like: height 720 in the conf (or delete the conf's
width/height and let apply_window_spec own all geometry), plus a drift-tie
test (none exists: no test reads tauri.conf.json — rg over crates/, apps/:
only two comments in product-config/src/runtime.rs:32,111).
Confidence: MEDIUM — Verification: read (static chain: conf→tauri window
creation→setup resize; the E0425 blocker prevents executing it).

### F-002 [S3]
Where: apps/desktop-tauri/tauri.conf.json:23 (media-src) vs
ui/src/components/player/MediaSurface.tsx:62,85 and
ui/src/screens/network-stream/NetworkStreamScreen.tsx:151
Code:
```json
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' blob: data:; font-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost; worker-src 'self' blob:; base-uri 'self'; form-action 'none'; object-src 'none'; frame-src 'none'"
```
```tsx
// MediaSurface.tsx:62 — the ONLY src ever mounted on the <video>:
  const src = source !== null && source.kind === "url" ? source.url : null;
```
What is wrong: `media-src 'self' blob: data:` blocks remote (http/https)
media, and a remote URL is the only value MediaSurface ever puts on the
`<video>`'s `src` (file sources mount src-less by design). The shipped
NetworkStreamScreen (routed lazily from App.tsx:45) and OpenMediaDialog
(OpenMediaDialog.tsx:84 classifies any `scheme://` paste as a url source) feed
exactly that. Meanwhile the allowance side is dead weight: no
`URL.createObjectURL`, no `data:` media, no bundled media assets exist in
ui/src (rg, 0 hits) — real desktop playback rides the raw LUF1 binary IPC
channel into `canvas.putImageData` (MediaSurface.tsx:198-227), which CSP
never governs.
Why it matters: the CSP contradicts the shipped UI's url render seam. Today it
is masked because the engine's input layer typed-rejects every non-`file`
scheme (crates/input/src/uri.rs:59-60 `scheme_not_served`) — but the moment an
http access plugin lands, the `<video>` half stays dark with a console-only
CSP violation, and INFRA-BLOCKED.md:37's rationale "media/blob for playback"
is already false (playback never traverses media-src in this shell).
What correct looks like: either drop the `<video>` url seam from the shell
build or allow `media-src https: http:` deliberately — and delete the unused
`blob:`/`data:` allowances either way.
Confidence: HIGH on the CSP semantics and the dead allowances (read);
HIGH that the engine rejects url sources today (uri.rs read).

### F-003 [S3]
Where: apps/desktop-tauri/tauri.conf.json:26-32 (bundle) vs
tools/xtask/src/cmd/gate.rs:257-263 and
apps/desktop-tauri/INFRA-BLOCKED.md:103-106
Code:
```json
  "bundle": {
    "active": true,
    "targets": "all",
```
```rust
// tools/xtask/src/cmd/gate.rs:258-263 — step 9 of the local gate
        Step {
            idx: 9,
            name: "tauri",
            argv: &["cargo", "tauri", "build", "--debug", "--no-bundle"],
            cwd_rel: "apps/desktop",
```
What is wrong: `bundle.active/targets` are consumed only by tauri-cli
bundling, which no CI/release lane invokes (release.yml:219 builds via plain
`cargo build --manifest-path apps/desktop-tauri/Cargo.toml`; INFRA-BLOCKED.md
§C: "tauri-cli is NOT used"). The single repo tool that DOES run
`cargo tauri build` — xtask gate step 9 — does so with cwd `apps/desktop`, a
directory that contains NO tauri.conf.json (the tauri app moved to
apps/desktop-tauri at ADR-004; rebrand.rs:669 got the path update, gate.rs —
mirroring docs/10-testing-ci-quality.md §2.1 row 9 verbatim — did not). So
step 9 can never pass (missing config or missing cargo-tauri tool → FAIL per
gate.rs's own spawn rule), and the bundle half of this config has zero
working consumers. Additionally `targets: "all"` includes macOS app/dmg,
which this icon list cannot serve (no .icns).
Why it matters: the config advertises a bundling posture the repo cannot
execute; the local gate is silently broken at step 9 for every host with
`apps/desktop/` present; a reader of the conf is misled about who bundles.
What correct looks like: gate step 9 cwd → `apps/desktop-tauri` (and doc 10
row 9 with it), or step 9 deleted; `bundle.targets` reduced to what the real
pipeline (custom scripts reading product.toml [brand]) owns, or a comment
naming bundle.icon as the only live key (it IS load-bearing: tauri-build's
Windows resource step, gen-ico.py:4-7).
Confidence: HIGH — Verification: read (all three sites quoted; no gate run
executed — REQUEST-ORCH-RUN candidate in log).

### F-004 [S3]
Where: apps/desktop-tauri/tauri.conf.json:4-5 vs
apps/desktop-tauri/Cargo.toml:25 and product.toml:27
Code:
```json
  "productName": "Lumen",
  "version": "0.1.3",
  "identifier": "dev.lumen.desktop",
```
```toml
# Cargo.toml:24-25
[package]
name = "lumen-desktop-tauri"
version = "0.1.3"
```
What is wrong: the version now lives in THREE hand-edited places (conf,
Cargo.toml, product.toml display_version) with no tie for the conf: no test,
gate, or validator reads tauri.conf.json (rg across crates/ and apps/: only
comments in product-config/src/runtime.rs:32,111; xtask config-validator
validates product.toml only). The identifier tie is equally unenforced —
runtime.rs:162 asserts the DERIVED string "dev.lumen.desktop" but never reads
the conf; the match survives only because nobody has edited one side.
(`cargo xtask rebrand` does rewrite conf productName/title/identifier —
rebrand.rs:15,669 — but not `version`.)
Why it matters: a release bump that updates Cargo.toml + product.toml (the
product-config test forces those two) can silently ship a tauri bundle whose
embedded metadata still says 0.1.3 — exactly the drift class the
"single source of truth" rule (product.toml §2.1 header) exists to prevent.
What correct looks like: delete `version` from the conf (tauri-build falls
back to Cargo.toml's package version) and add a drift-tie test asserting
conf.identifier == runtime's derived desktop_bundle_identifier.
Confidence: HIGH — Verification: read (rg evidence; tauri's fallback behavior
for a missing version key is UNVERIFIED — no build possible).

### F-005 [S4]
Where: apps/desktop-tauri/INFRA-BLOCKED.md:37,39-40,78-79 vs
tauri.conf.json:23,29-32 and capabilities/default.json:6-12
Code:
```markdown
- **Capabilities**: unchanged `core:default` only (no plugins — the
  4.H1 posture).
```
```markdown
3. **Icons** (`bundle.icon: []`) + per-OS bundle metadata — a packaging
   follow-up (the existing lanes' icon pipeline owns it).
```
What is wrong: the admission doc that this file's audit trail hangs on is
stale in three rows: "Capabilities: unchanged core:default only" (the
capabilities file now carries four window-control grants, w28 fix 3), "Icons
(`bundle.icon: []`)" (the conf lists icons/icon.png+icon.ico and both files
exist), and "media/blob for playback" (see F-002 — nothing in playback
traverses media-src).
Why it matters: INFRA-BLOCKED.md is the cited authority for the security
posture of this config (wave prompts point at it); each stale row is a claim
about MY file that the file contradicts.
What correct looks like: refresh §A/§B.3 to the w28/w31 state, or date-stamp
each row.
Confidence: HIGH — Verification: read (both sides quoted).

### F-006 [S4]
Where: apps/desktop-tauri/tauri.conf.json:23 (style-src) with rationale
INFRA-BLOCKED.md:35-37
Code:
```json
      "csp": "… style-src 'self' 'unsafe-inline'; …"
```
```markdown
  `style-src 'unsafe-inline'` for the UI's dynamic inline styles (sized
  sliders, positioned overlays), …
```
What is wrong: the documented reason does not require the allowance. The
UI's dynamic styles are React `style={{}}` props (23 sites in 14 files, e.g.
ui/src/components/slider/Seekbar.tsx), which React applies through the CSSOM
(`el.style.*`), not through parsed style attributes — CSP does not gate
CSSOM writes. There is no `createElement("style")`, no
`dangerouslySetInnerHTML`, and no style attribute in ui/index.html (verified
by rg; index.html read). The only CSP-relevant style surface is the
stylesheet files themselves ('self', already allowed).
Why it matters: a security header weakened by a rationale nobody re-checked;
if the allowance is truly needed (some webview quirk), that reason — not
"sized sliders" — should be recorded.
What correct looks like: drop `'unsafe-inline'` from style-src and let a
launch lane prove it (or document the real requirement).
Confidence: MEDIUM — Verification: read (UI-side rg complete; no webview run
possible to prove the negative — UNVERIFIED at runtime).

### F-007 [S4]
Where: apps/desktop-tauri/tauri.conf.json:23 (img-src/worker-src allowances)
Code:
```json
      "csp": "… img-src 'self' data: blob:; … worker-src 'self' blob:; …"
```
What is wrong: `img-src … blob:` and `worker-src … blob:` serve no code path:
rg over ui/src finds zero `new Worker` and zero `URL.createObjectURL`. The
live image-adjacent use is CSS `mask: url("data:image/svg+xml,…")`
(video.css:412, setting-field.css:62), which needs only the `data:` already
granted.
Why it matters: every allowance in a hardened header should trace to a use;
unused allowances are permanent unreviewed attack surface for whatever the
webview loads later.
What correct looks like: `img-src 'self' data:`; `worker-src 'self'` (or
delete worker-src and inherit default-src 'self' until a worker exists).
Confidence: HIGH — Verification: read (rg, 0 hits).

## 4. Per-function (per-config-key) table

| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `$schema` | 2 | REAL | — | Official tauri v2 schema URL; schema download UNVERIFIED (no network) — compile-time validation by tauri-build never ran (E0425, baseline #2). |
| `productName` | 3 | REAL | — | "Lumen" == identity.name (product.toml:19); rebrand-covered (rebrand.rs:15). |
| `version` | 4 | REAL | S3 (F-004) | 3rd hand-copied version site; no tie/gate; tauri embeds it in bundle metadata. |
| `identifier` | 5 | REAL | S3 (F-004) | Matches derived `dev.lumen.desktop`; tie unenforced (runtime.rs:162 asserts derivation, never reads conf). |
| `build.frontendDist` | 6-8 | REAL | — | ../../ui/dist correct; dir absent by design; CI order guard ci.yml:719-724; no beforeBuildCommand (tauri-cli unused — consistent). |
| `app.withGlobalTauri` | 10 | REAL | — | false; UI uses `__TAURI_INTERNALS__` only (rg `__TAURI__` 0 hits). |
| `windows[0].label` | 13 | REAL | — | "main" == app.rs:249 `get_webview_window("main")` == capabilities windows ["main"]. |
| `windows[0].title` | 14 | REAL | — | First-paint title; product.toml title_template/idle_key never applied at runtime (app.rs:74-76 admits binding pending) — dead product config, live conf key. |
| `windows[0].width` | 15 | REAL | — | 1280 == product.toml width (agrees). |
| `windows[0].height` | 16 | DRIFTED | S2 (F-001) | 800 ≠ product.toml 720; setup resize jump every launch. |
| `windows[0].minWidth/minHeight` | 17-18 | REAL | — | 940/600 == product.toml min floor (w28 fix 5 agrees). |
| `windows[0].decorations` | 19 | REAL | — | false ↔ banner controls ↔ 4 capability grants — chain verified (§2). No macOS-specific traffic-light handling anywhere (UI banner is the only chrome on all 3 OSes). |
| `security.csp: default-src/script-src/base-uri/form-action/object-src/frame-src` | 23 | REAL | — | Hard, correct; script-src 'self' safe (no wasm in ui/src, no inline scripts; tauri auto-nonces its injections). |
| `security.csp: style-src` | 23 | REAL | S4 (F-006) | 'unsafe-inline' rationale does not hold for CSSOM-applied React styles. |
| `security.csp: img-src` | 23 | PARTIAL | S4 (F-007) | data: needed (CSS masks); blob: unused. |
| `security.csp: media-src` | 23 | DRIFTED | S3 (F-002) | Blocks the <video> url seam; blob:/data: unused; "for playback" rationale false. |
| `security.csp: connect-src` | 23 | REAL | — | ipc: + http://ipc.localhost — exactly the fetch-IPC pair the transport + frame channel use. |
| `security.csp: worker-src` | 23 | OVER-PERMISSIVE | S4 (F-007) | No workers exist in ui/src. |
| `bundle.active` | 27 | DEAD | S3 (F-003) | No tauri-cli consumer; gate step 9 points at wrong cwd. |
| `bundle.targets` | 28 | DEAD | S3 (F-003) | "all" includes macOS (.icns absent from icon list). |
| `bundle.icon` | 29-32 | REAL | — | Files exist (placeholder rasters); load-bearing for tauri-build's Windows resource step (gen-ico.py:4-7). |

## 5. Test honesty (if tests exist in/for this file)

No test anywhere targets this file: `rg "tauri.conf" crates/ apps/` → only two
comments (product-config/src/runtime.rs:32,111). tests/single_command.rs
audits app.rs's handler registration only. The sole validation the conf ever
receives is tauri-build's compile-time schema check — which has never executed
(baseline #2 died in a dependency build script; the crate's own E0425 would
kill it later anyway). Nothing would fail if `height` or the CSP changed
arbitrarily — no mutation protection exists.

## 6. Doc claims touching this file

| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Real CSP … media/blob for playback" | INFRA-BLOCKED.md §A:34-38 | FALSE (rationale) | Playback rides raw IPC→canvas (MediaSurface.tsx:198-227) + engine-side null-device audio (baseline lumen-engine-info.txt); media-src governs only the <video> url seam — which it blocks (F-002). |
| "Capabilities: unchanged `core:default` only (no plugins)" | INFRA-BLOCKED.md §A:39-40 | FALSE (stale) | capabilities/default.json:8-11 adds 4 window grants (w28 fix 3). |
| "Icons (`bundle.icon: []`) … a packaging follow-up" | INFRA-BLOCKED.md §B.3:78-79 | FALSE (stale) | conf:29-32 lists 2 icons; files exist (file(1) verified). |
| "the crate compiles in its dedicated CI lane (tauri-check)" | INFRA-BLOCKED.md header:4-5 | FALSE (this tree) | app.rs:301 E0425 (`spawn_session_pump` undefined) — W14-A01/A02 F-001, re-verified by sed. |
| "tauri-cli is NOT used (no dev-side tool dep)" | INFRA-BLOCKED.md §C:103-106 | FALSE (contradicted) | tools/xtask gate step 9 runs `cargo tauri build` (gate.rs:260) — with a stale cwd (F-003). |
| "the first-paint tauri.conf.json values are fallbacks only … never a second source of truth" | app.rs module docs:71-76 | DRIFTED | height 800 (conf) vs 720 (product.toml) — F-001. |
| "tauri.conf.json first-paint minWidth/minHeight … 940x600" | product.toml:66-67 (w28 fix 5) | REAL | conf:17-18 == 940/600. |
| "window `title`" rebrand-covered | tools/xtask rebrand.rs:15 | REAL | conf:14 title present; rebrand surface list includes it. |
| "ORDER IS LOAD-BEARING … frontendDist is ../../ui/dist and tauri-codegen embeds it AT COMPILE TIME" | release.yml:196-201 / ci.yml:719-724 | REAL | conf:7 matches; ui-build.log.txt EXIT=0; ui/dist absent from tree as stated. |

## 7. Dead / duplicated / unreachable code

- `bundle.active`/`bundle.targets` (conf:27-28): no consumer (F-003).
- `media-src blob: data:`, `img-src blob:`, `worker-src blob:` (conf:23):
  unused allowances (F-002/F-007).
- `height: 800` (conf:16): superseded at runtime by product.toml 720 — dead
  on arrival, but not harmless (first-paint flash, F-001).
- `version` (conf:4): duplicated truth (F-004).
- product.toml `[window] title_template`/`title_idle_key`/`remember_geometry`:
  declared, parsed by window_config.rs, never applied by the shell (title is
  the conf's; no geometry persistence exists) — dead on the consumer side,
  noted here because the conf's `title`/window keys are the operative ones.

## 8. Dependencies & flags

- Consumed by `tauri-build = "2"` (Cargo.toml:64-68) via build.rs:11; runtime
  read site `tauri::generate_context!()` (app.rs:321). No feature flags
  involve this file. The tauri closure lives in this crate's own Cargo.lock
  (workspace-excluded; A06's lane). Rebrand surface: rebrand.rs:669 includes
  apps/desktop-tauri/tauri.conf.json (productName, title, identifier tokens).
- CSP interplay verified against the UI bundle: no wasm (script-src 'self'
  safe), local fonts only (fonts.css @font-face, font-src 'self' fits), data:
  SVG masks (img-src data: needed), IPC fetch pair (connect-src exact).

## 9. Open questions for the next agent

- W14-A01: after fixing the E0425 (`spawn_session_pump`→`spawn_event_pump`),
  decide the height question (F-001): conf 800 or product 720 — one must die.
- W14-A08: capabilities — confirm `core:window:allow-is_maximized` /
  `plugin:event|listen` really ride inside `core:default` (my chain assumes
  it; window-controls.ts:21-22 asserts it) — that's your file's grant table.
- Tools/xtask owner (gate.rs + docs/10 §2.1 row 9): stage 9's cwd
  `apps/desktop` predates ADR-004 and can never succeed — fix or delete; also
  INFRA-BLOCKED.md §C "tauri-cli is NOT used" must pick a side.
- Docs owner: INFRA-BLOCKED.md rows stale vs this file (F-005).
- Any future launch lane (tauri-launch.yml): verify at runtime (a) no CSP
  violation on the frame channel's `plugin:__TAURI_CHANNEL__|fetch` dance,
  (b) whether `style-src 'unsafe-inline'` can be dropped (F-006) — both
  UNVERIFIABLE without a webview.
