# AUDIT: apps/desktop-tauri/icons/icon.png (binary)
| Field | Value |
|---|---|
| Auditor | W14-A09 |
| Date | 2026-11-24 |
| Size / sha256 | 1819 bytes / 7574265e9f4e6766dd5fcd9f11f83ad1228eb0c6afbbc17534f772928a890595 |
| Type | PNG image data, 512 x 512, 8-bit/color RGBA, non-interlaced |
| File verdict | REAL (as a functional placeholder; NOT a brand asset — it is a solid color square) |
| Reachable from shipped entry point? | YES — tauri.conf.json:30 `bundle.icon` lists `icons/icon.png`; tauri's `generate_context!` requires a default icon at compile time (worklog.md:1665-1666: "generate_context! needed the default icon — / apps/desktop-tauri/icons/icon.png now committed"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | file (exit 0); sha256sum; python3/PIL decode; `python3 scripts/release/gen-icon.py <audit-scratch>/regen-icon.png '#4f8cff'` (exit 0) + cmp → BYTE-IDENTICAL; rg for citations; awk product.toml brand block. |

## 1. One-paragraph verdict
A valid 512x512 RGBA PNG that contains exactly one color: #4f8cff at full opacity, all 262,144 pixels — a placeholder square, not an icon. I regenerated it with the repo's own `scripts/release/gen-icon.py '#4f8cff'` and the output is byte-identical (sha256 match, `cmp` exit 0), which proves provenance (generated in-repo, clean-room, no borrowed artwork), determinism, and that there is no hidden content. This same generator+color feeds the Linux packaging lanes (build-appimage.sh:122, package-deb.sh:104, make-appimage.sh:88, stage-linux-payload.sh:97), so the desktop icon is pixel-identical to the deb/AppImage icon — every shipped surface of Lumen currently brands itself as a flat blue square. Disclosed in docs/RELEASE.md known-limit 5; still what ships.

## 2. What actually works (verified)
— `file`: "PNG image data, 512 x 512, 8-bit/color RGBA, non-interlaced".
— PIL decode: 512x512 RGBA; unique colors: 1 — (79, 140, 255, 255) at 100.00%; per-channel extrema all constant (79,79)/(140,140)/(255,255); alpha extrema (255,255). 79,140,255 == #4f8cff == product.toml:95 `accent_color = "#4f8cff"` (the provisional accent, product.toml:93 marks it "[VERIFY] against design package token extraction").
— Byte-identity: `python3 scripts/release/gen-icon.py audit/runs/W14-A09.scratch/regen-icon.png '#4f8cff'` → `gen-icon: wrote ... 512x512 RGBA, fill #4f8cff, 1819 bytes`; sha256 7574265e... == committed; `cmp` exit 0 (PNG-BYTE-IDENTICAL).
— Consumed: tauri.conf.json:29-32 bundle.icon; satisfies tauri 2.12.5 `generate_context!` default-icon requirement and the Linux bundle fallback chain (stage-linux-payload.sh:27-29 "The icon prefers the real brand asset / (product.toml [brand] icon_png_512 ...) and falls back to the deterministic placeholder / (scripts/release/gen-icon.py, accent-colored) until assets/brand / exists").

## 3. Findings
F-001 [S3]
Where: apps/desktop-tauri/icons/icon.png (whole file)
What is wrong: the app's png icon — used by Linux bundles and by tauri wherever a raster is needed — is one flat color, no glyph, no alpha shaping. User-visible packaging placeholder, disclosed (docs/RELEASE.md:166-170, :226-227 "the Linux/MSI packages ship the deterministic accent-color icon"). Root cause is the Wave-1 config-validator FAILURE: product.toml:99 `icon_png_512 = "assets/brand/icon-512.png"` does not exist. Confidence: HIGH — Verification: ran (PIL + regen byte-compare).
F-002 [S4]
Where: apps/desktop-tauri/icons/icon.png + tests/rebrand/ (absence)
What is wrong: rebrand rewrites accent_color and tauri.conf.json but has no icon step — `rg icon` over tests/rebrand/ returns zero matches. Post-rebrand, this file stays #4f8cff while the rest of the product re-colors. Confidence: HIGH — Verification: rg (no matches).
F-003 [S4]
Where: apps/desktop-tauri/icons/ (directory)
What is wrong: no `icon.icns` exists here — tauri.conf.json bundle.icon lists only the png+ico pair. macOS bundling (bundle.targets "all") has no committed icns; whether tauri-bundler 2.12.5 auto-converts the png or fails on a mac host is UNVERIFIED (no macOS host in sandbox; .github workflows contain no gen-icon/gen-ico/icon references at all). Note tauri-bundler CAN generate icns from png in recent versions — needs a live mac run to confirm. Confidence: MEDIUM — Verification: rg .github (no matches) + Glob **/*.icns (only test fixtures).

## 4. Cited as evidence by docs/matrix?
No. docs/feature-matrix.toml has no row citing the desktop icons (only `menu-tray` at :3186, `evidence = ""`, unrelated). worklog.md:1665-1666 documents the commit; W14-A10 build.rs audit (audit/files/apps/desktop-tauri/build.rs.audit.md:60) states "defaults are correct for icons (icons/icon.png + icon.ico both exist)" — existence TRUE, but neither that audit nor any doc claims these are real artwork.

## 5. Wave-1 cross-reference (product.toml brand assets)
NOT the referenced files. product.toml:97-101 declares `assets/brand/icon.ico|icon.icns|icon-512.png|logo.svg`; config-validator (baseline xtask-config-validator.txt:11-14) reports all four missing — `assets/` does not exist at the repo root. The four names exist only as test fixtures with different bytes (tests/rebrand/assets/icon-512.png sha 74657efc...; product-config fixture icon-512.png is not even a PNG — `file` says "data"). This file (7574265e...) matches none of them. It is the tauri-compile-requirement icon, deliberately generated as the clean-room stand-in until assets/brand lands (gen-icon.py:3-8).
