# AUDIT: scripts/release/make-dmg.sh
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / functions | 54 / main + die() |
| File verdict | REAL — WIRED in the live macOS release lane (unlike its wave-22 siblings make-appimage/package-windows); macOS-only, unverifiable in this Linux sandbox |
| Reachable from shipped entry point? | NO (packaging script); invocation chain: release.yml:552/694 -> scripts/packaging/build-dmg.sh:93 -> this script (verified) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sh -n scripts/release/make-dmg.sh` -> OK; `sed -n '85,98p' scripts/packaging/build-dmg.sh` -> invocation confirmed |

## 1. One-paragraph verdict
A tight 54-line macOS packager: required-env guards, loud `hdiutil` absence death (correct for Linux dry-runs), bundle existence check, mktemp staging with trap, symlink-based drag-install layout, native `hdiutil create -format UDZO`, and a non-empty output assertion. Unlike make-appimage.sh and package-windows.sh, this script IS still wired into the live release pipeline (build-dmg.sh calls it directly). It can only have been executed on macOS runners — the w22g Linux session could not have run it (it dies at the hdiutil check), so its only execution evidence is the release lanes, which I cannot see from the sandbox.

## 2. What actually works (verified)
— `sh -n` -> exit 0.
— Wiring chain verified: `.github/workflows/release.yml` L552+L694 (`scripts/packaging/build-dmg.sh`) -> build-dmg.sh:93 (`APP_DIR="$APP_DIR" scripts/release/make-dmg.sh`).
— Guard inventory: 5 `:?` env vars (L22-26), hdiutil presence (L35-36), bundle presence (L38-39), `[ -s "$out" ]` (L52).

## 3. Findings
F-001 [S4] — Symlink-following assumption is unvalidated by the script itself; die() exit code (2) inconsistent with package-windows.sh (1).
Where: scripts/release/make-dmg.sh:46-48, 33
Code:
```
# Symlink (not copy): hdiutil -srcfolder follows it and the image keeps
# the standard "<volume>/<Name>.app" drag-install layout.
ln -s "$(cd "$APP_DIR" && pwd)/${PRODUCT_NAME}.app" "$stage/${PRODUCT_NAME}.app"
```
What is wrong: (a) The comment's premise (hdiutil follows the symlink into real content) is macOS behavior this Linux sandbox cannot execute — if wrong, the .dmg would contain a dangling symlink while still passing the `[ -s ]` size check; only a mount-test would catch it, and no mount-test exists in the script or (per docs found) the driver. (b) die exits 2 here vs 1 in package-windows.sh — inconsistent contract across the release family for CI triage. Confidence: HIGH (the facts), MEDIUM (the hdiutil behavior) — Verification: read; hdiutil behavior UNVERIFIED (macOS-only).
Note: worklog.md:644's w22g session lists AppImage execution and make-app.sh field-asserts but NOT a make-dmg.sh run — consistent with it being macOS-only.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| die | 33 | REAL | — | exit 2 |
| main | 35-53 | REAL | — | guards/staging/pack all present |

## 5. Test honesty
N/A. The release lane's post-pack checks (sign-macos/notarize) consume the dmg — those lanes' green claims (worklog rounds) are the only execution evidence; not directly verifiable here.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "macos: .dmg (hdiutil/ditto) | `scripts/release/make-app.sh` + `make-dmg.sh`" | docs/release-checklist.md:102 | REAL (wiring) | build-dmg.sh:87,93 call chain |
| "No third-party tooling, no downloads, no create-dmg dependency" | make-dmg.sh:5-6 | REAL | only hdiutil used (L51) |
| "dies loudly when hdiutil is absent (e.g. Linux dry-runs)" | make-dmg.sh:17 | REAL | L35-36 die |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
hdiutil (macOS); env from the product bridge; APP_DIR default DIST_DIR.

## 9. Open questions for the next agent
- Has any macOS lane actually produced and MOUNT-verified a dmg (symlink-following question)? Check the release run records in worklog for a mounted-dmg verification step.
