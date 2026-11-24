# AUDIT: apps/desktop-tauri/icons/icon.ico (binary)
| Field | Value |
|---|---|
| Auditor | W14-A09 |
| Date | 2026-11-24 |
| Size / sha256 | 687 bytes / a8a8c582580de1d0580a49d4a611d4db0a26e4493436e833bf7593cacfb1858e |
| Type | MS Windows icon resource — 1 entry, 256x256, PNG-compressed (Vista-style), 32 bpp |
| File verdict | REAL (as a functional placeholder; NOT a brand asset — it is a solid color square) |
| Reachable from shipped entry point? | YES — tauri.conf.json:31 `bundle.icon` lists `icons/icon.ico`; tauri-build embeds it into the Windows resource of `lumen-tauri.exe` (gen-ico.py:4-6: "The tauri-build Windows resource step requires `icons/icon.ico` beside / the crate"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | file (exit 0); sha256sum; python3/PIL decode + ICO header parse; `python3 scripts/release/gen-ico.py <audit-scratch>/regen-icon.ico '#4f8cff'` (exit 0) + cmp → BYTE-IDENTICAL; rg for citations. |

## 1. One-paragraph verdict
A structurally valid, spec-conformant ICO that is also, provably, a placeholder: a single 256x256 PNG-compressed entry whose every pixel is RGB(79,140,255) = #4f8cff, fully opaque — zero artwork, zero transparency, one unique color. I regenerated it with the repo's own `scripts/release/gen-ico.py '#4f8cff'` and got a byte-identical file (sha256 match, `cmp` clean), so provenance, determinism, and the clean-room "no borrowed third-party icon" claim (gen-ico.py:8-10) are all VERIFIED BY EXECUTION. The Windows executable of this product therefore carries a featureless solid blue square as its icon. That is honestly disclosed in docs/RELEASE.md known-limit 5, but it is what ships.

## 2. What actually works (verified)
— `file`: "MS Windows icon resource - 1 icon, 256x256 with PNG image data, 256 x 256, 8-bit/color RGBA, non-interlaced, 32 bits/pixel".
— ICO header parse (python3/struct): reserved=0, type=1 (icon), count=1; ICONDIRENTRY width/height bytes = 0 (spec encoding of 256), planes=1, bpp=32, payload 665 bytes at offset 22; payload is a real PNG (`89504e470d0a1a0a`). PIL decodes it: 256x256 RGBA, 1 unique color (79,140,255,255), alpha extrema (255,255).
— Byte-identity: `python3 scripts/release/gen-ico.py audit/runs/W14-A09.scratch/regen-icon.ico '#4f8cff'` → `gen-ico: wrote ... 687 bytes`; sha256 a8a8c582... == committed file; `cmp` exit 0 (ICO-BYTE-IDENTICAL).
— Referenced and consumed: tauri.conf.json:29-32 `"icon": [ "icons/icon.png", "icons/icon.ico" ]`. Wave-1 cross-ref: `audit/runs/baseline/xtask-config-validator.txt:11` shows the REAL brand ico lives elsewhere — "violation: brand.icon_ico: asset path `assets/brand/icon.ico` does not exist under the root" (product.toml:97 `icon_ico = "assets/brand/icon.ico"`).

## 3. Findings
F-001 [S3]
Where: apps/desktop-tauri/icons/icon.ico (whole file)
What is wrong: the shipped Windows app icon is a single flat color — 1 unique pixel value #4f8cff across all 65,536 pixels (PIL count, verified). It identifies nothing; any user distinguishing Lumen from any other blue-rectangle app does so by window title alone. Disclosed (docs/RELEASE.md:166-170 "**Placeholder icons**: `assets/brand/icon.*` referenced by / `product.toml` do not exist yet..."), so not fraud — but the packaged artifact is a placeholder, and the Wave-1 missing-brand-asset failure is the direct cause. What correct looks like: assets/brand/icon.ico landing and tauri.conf.json/bundle pointing at it (or rebrand wiring regenerating this file). Confidence: HIGH — Verification: ran (PIL decode + byte-identity regen).
F-002 [S4]
Where: apps/desktop-tauri/icons/icon.ico + tests/rebrand/ (absence)
What is wrong: the rebrand plane swaps `brand.accent_color` (product.toml:95) and rewrites apps/desktop-tauri/tauri.conf.json (rebrand_skeleton.rs:402, 452) but contains no step that regenerates or swaps the tauri icons — `rg icon` over tests/rebrand/ returns zero matches. A rebrand to the test fixture accent `#ff5500` (nova.toml:62) would leave the desktop icon permanently #4f8cff. Placeholder drift, no more, since assets/brand does not exist yet. Confidence: HIGH — Verification: rg (no matches).

## 4. Cited as evidence by docs/matrix?
No. The only icon-related feature-matrix row is `menu-tray` (docs/feature-matrix.toml:3186 "Tray icon + menu (Quit et al.)", status not_started, `evidence = ""`) — it cites nothing. W14-A10's build.rs audit (audit/files/apps/desktop-tauri/build.rs.audit.md:60) notes both icon files exist — consistent with this audit.

## 5. Wave-1 cross-reference (product.toml brand assets)
These are NOT the icons product.toml references. product.toml:97-101 points at `assets/brand/icon.ico` / `icon.icns` / `icon-512.png` / `logo.svg` — none exist (config-validator: 4 violations, result FAILURE; `assets/` is absent from the repo root entirely). The only in-tree copies of those four names are test fixtures with different hashes and fake content (tests/rebrand/assets/icon.ico is a 16x16 real ICO, sha d4483b0e...; crates/product-config/tests/fixtures/.../icon.ico is ASCII text) — none match a8a8c582.... apps/desktop-tauri/icons/icon.ico is the tauri-internal icon required by tauri-build, deliberately generated as a stand-in (gen-icon.py:3-6 "product.toml [brand] points at assets/brand/icon.{ico,icns,png-512}, but / those brand assets do not exist yet in this repository").
