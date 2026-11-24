# CLAIM AUDIT: docs/inventory/platform-windows.md
| Field | Value |
|---|---|
| Auditor | W18-A07 |
| Date | 2026-11-24 |
| Claims extracted | 22 concrete claims re-verified live (crates.io API, Microsoft Store displaycatalog API, MS docs pages, published wry 0.57.0 tarball source) + repo-relevance claim |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | REAL 17 (external research verified live) / UNVERIFIED 4 (GitHub-issue bodies, caniuse/MDN internals, price, tao/winit/cpal/wgpu tarball internals) / PAPER-ONLY 1 as a product claim (the Windows platform story it feeds has zero shipped implementation) |
| Internal contradictions found | 0 in-doc; 1 doc-vs-repo (inventory describes a stack the release lane never builds) |

## 1. Blunt verdict
This is the rare Lumen doc whose factual content survives hostile live re-verification:
I re-fetched crates.io, the Microsoft Store catalog API, learn.microsoft.com and the actual
published wry-0.57.0 tarball, and the record's research checks out line after line —
7 of 13 crate-version rows are byte-exact even today, the three Store product ids resolve
to the exact claimed titles, the old HEVC id is dead exactly as described, the
non-existent `MFCreateVideoDecoder` 404s, the wry windowed-hosting/no-composition/
transparent-background source claims are all in the real tarball, and the alpha-0/255
quote matches the live MS page verbatim. What the record does NOT say — and what the
ledger must — is that none of this describes shipped Lumen code: the platform-windows
crate is an orphan (W08-A06: no workspace Cargo.toml depends on it), the release lane
(`cargo build --release --locked -p lumen-cli -p desktop`) contains no WASAPI, no Media
Foundation, no wgpu-DX12, no WebView2 — the workspace Cargo.lock has no tauri/wry/tao/
winit/wgpu/cpal/windows entries at all (only transitive `windows-sys 0.52.0`), and the
tauri closure lives only in the workspace-EXCLUDED, sandbox-infra-blocked
`apps/desktop-tauri` (own lockfile: tauri 2.12.5 / wry 0.57.2 / tao 0.37.2 — a patch
refresh past this record's 2.12.4/0.57.0/0.37.1). As an inventory of *planned* platform
technology it is genuine; as evidence of platform capability it is PAPER-ONLY.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "wry 0.57.0 … 2026-09-08 … webview abstraction used by Tauri" | :19 | REAL (drifted) | crates.io API (this session): `wry claim_max=0.57.0 actual_max=0.57.3 … updated=2026-11-18`. Correct at research date; now 0.57.3. |
| 2 | "tauri 2.12.4 / 3.0.0-alpha.5 / 2026-10-14" | :20 | REAL (drifted) | API: `tauri claim_max=2.12.4 actual_max=2.12.7 actual_new=2.12.7`. Correct 2026-10-19; drifted. |
| 3 | "tao 0.37.1 … 2026-09-26" | :22 | REAL (drifted) | API: `tao actual_max=0.38.0 updated=2026-11-24`. |
| 4 | "winit 0.30.13 / 0.31.0-beta.3" | :23 | REAL (drifted) | API: max 0.30.13 exact; newest now 0.31.0-beta.4. |
| 5 | "raw-window-handle 0.6.2 … 2024-05-17" | :24 | REAL (exact today) | API: `claim_max=0.6.2 actual_max=0.6.2 … updated=2024-05-17`. |
| 6 | "webview2-com 0.39.1 … 2026-03-11" | :25 | REAL (exact today) | API row: exact match. |
| 7 | "windows 0.62.2 … 2025-10-06" / "windows-sys 0.61.2" | :26-27 | REAL (exact today) | API rows: exact match both. |
| 8 | "windows-core 0.100.0 … 2026-09-03" / "windows-webview 0.100.0" | :28-29 | REAL (exact today) | API rows: exact match both. |
| 9 | "wgpu 30.0.1 … 2026-08-22" | :30 | REAL (drifted) | API: `wgpu actual_max=30.0.2 updated=2026-11-12`. |
| 10 | "cpal 0.18.2 … 2026-08-16" | :31 | REAL (exact today) | API: exact match. |
| 11 | wry implements windowed hosting "CreateCoreWebView2ControllerWithOptions(hwnd…) / CreateCoreWebView2Controller"; "no composition code present" | :51-53 | REAL | Published tarball (downloaded this session, `crates.io/api/v1/crates/wry/0.57.0/download`): `src/webview2/mod.rs:420` `env10.CreateCoreWebView2ControllerWithOptions(hwnd, &controller_opts, &handler)?;` `:422` `env.CreateCoreWebView2Controller(hwnd, &handler)?`; rg `CompositionController|IDCompositionVisual` in webview2/mod.rs → zero matches. |
| 12 | wry "with_transparent(true) … sets background (0,0,0,0) … guarded by !is_windows_7()" | :81-83 | REAL | Tarball `src/webview2/mod.rs:464-465`: `if attributes.transparent && !is_windows_7() { unsafe { set_background_color(controller, (0, 0, 0, 0))? };` |
| 13 | "**Alpha is binary**: 'The only supported alpha values are 0 and 255, all other values will result in E_INVALIDARG.'" | :75-76 | REAL | Live page fetch (this session): `curl …/icorewebview2controller2 \| rg` → `The only supported alpha values are 0 and 255, all other values will result in E_INVALIDARG.` — byte-exact with the quote. |
| 14 | "**HEVC Video Extensions — USD 0.99** — current product id `9nmzlz57r3t7`" | :184-185 | REAL (id verified; price UNVERIFIED) | Store API live: `9NMZLZ57R3T7 \| "HEVC Video Extensions"`. MSRP field returned None in my parse — USD 0.99 UNVERIFIED this session. |
| 15 | "AV1 Video Extension — free — `9mvzqvxjbq9v`" | :191 | REAL | Store API live: `9MVZQVXJBQ9V \| "AV1 Video Extension"`. |
| 16 | "VP9 Video Extensions — free — `9n4d0msmp0pt`" | :192 | REAL | Store API live: `9N4D0MSMP0PT \| "VP9 Video Extensions"`. |
| 17 | "the long-standing id `9N4WGH0Z6VHM` now returns … 'Product not found'" | :186-188 | REAL | Live: `displaycatalog…bigIds=9N4WGH0Z6VHM` → HTTP 200 with body `{"Products":[]}` — the empty-Products[] half of the claim reproduced exactly. |
| 18 | "**`MFCreateVideoDecoder` does not exist** (learn.microsoft.com 404 …)" | :209-210 | REAL | Live this session: `curl -o /dev/null -w … nf-mfapi-mfcreatevideodecoder` → `HTTP 404`. |
| 19 | "`WebViewBuilder::build_as_child` — webview as a child window" / "trait WebViewExtWindows … hwnd()" | :115-123 | REAL | Tarball: `src/lib.rs:1571 pub fn build_as_child<W: HasWindowHandle>(…)`; `src/lib.rs:2340 pub trait WebViewExtWindows`. |
| 20 | wry #1540/#1658/#408 field-issue bodies (WS_CLIPCHILDREN fix, decorations artifact, backdrop-filter) | :88-102 | UNVERIFIED | GitHub issue bodies not re-fetched this session (issue numbers + resolutions plausible; the WS_CLIPCHILDREN mechanism is consistent with the MSDN hosting docs). |
| 21 | caniuse/MDN facts (WebCodecs Chromium 94; WebView2 disabled-features list) | :150-158 | UNVERIFIED | Not re-fetched; no internal contradiction. |
| 22 | "tao 0.37.1 … grep of tao source shows no WS_CLIPCHILDREN handling" / cpal wasapi internals / wgpu 30 surface API | :105-107, 238-253, 141-142 | UNVERIFIED | Tarball re-verification not run for tao/cpal/wgpu (download budget); marked open. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Title: "Platform Record: Windows (WebView2 / Media Foundation / WASAPI / wgpu-DX12)" — inventory of the Windows platform stack | W08-A06 platform-windows audit: orphan crate, "no Cargo.toml in the workspace lists platform-windows"; workspace Cargo.lock (this session, rg): zero tauri/wry/tao/winit/wgpu/cpal/windows entries; apps/desktop/Cargo.toml: "NO tauri/wry import (the honest deferral)". | Both are true in their own frame: the research is real, but NOTHING it describes is in the shipped release lane. The record is an ADR input, not an implementation inventory — the matrix's platform columns (`implemented` on windows for container rows) are backed by zero Windows execution anywhere (baseline parity-report: windows 0/252). |

## 4. Fabrication / inflation indicators
None found — the opposite: this is the best-evidenced doc audited so far in my chunk.
Every checkable fact checked out (15+ exact matches, incl. a byte-exact MSDN quote and
5/5 wry source claims in the published tarball). The [VERIFY] discipline is real: 12 open
items are enumerated (:308-321) instead of hidden. The only soft spots: (a) the store
price (USD 0.99) is asserted from a rendered page I cannot re-parse the same way;
(b) "40.6M downloads" for wry (:140) not re-verified; (c) version drift since
2026-10-19 is unflagged in a doc dated "verified" — a freshness gap, not a lie.

## 5. Reachability (repo side — mandatory)
NOT reachable: this doc feeds ADR-001/003/005; the code it would gate (crates/platform-windows,
1010 LOC: audio/capability/dialog/events/gpu + windows dir) is an ORPHAN (W08-A06: sole
caller its own test suite; no workspace Cargo.toml depends on it). The tauri/wry closure
exists ONLY in apps/desktop-tauri — workspace-EXCLUDED (own Cargo.lock: tauri 2.12.5,
wry 0.57.2, tao 0.37.2, webview2-com 0.39.1, windows 0.62.2 — verified this session) and
its build is INFRA-BLOCKED in this sandbox (gdk-sys; baseline cargo-build-desktop-tauri.txt)
— so no Windows capability claim anywhere in the repo is executed. Every Windows platform
cell in feature-matrix.toml is REAL-UNPROVEN at best.

Commands run (this agent): crates.io API fetches ×13 (wry, tauri, tauri-runtime-wry, tao,
winit, raw-window-handle, webview2-com, windows, windows-sys, windows-core,
windows-webview, wgpu, cpal); displaycatalog.mp.microsoft.com product lookups ×2 (3 live
ids + dead old id); learn.microsoft.com fetches ×2 (MFCreateVideoDecoder → 404;
ICoreWebView2Controller2 alpha quote → byte-exact); wry-0.57.0.crate download + tar
extract + rg source verification; rg over workspace Cargo.lock, apps/desktop/Cargo.toml,
apps/desktop-tauri/Cargo.{toml,lock}, crates/renderer-wgpu/Cargo.toml; ls docs/adr,
docs/dependencies/records.
