# AUDIT: scripts/release/make-appimage.sh
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / functions | 103 / 1 main + die() |
| File verdict | REAL (guards verified; execution evidence = worklog 7.w22g session, not re-runnable here — no desktop binary without cargo) |
| Reachable from shipped entry point? | NO — legacy wave-22 packager, now UNWIRED: release.yml linux lane runs scripts/packaging/build-appimage.sh (a reimplementation), not this file. Only docs/release-checklist.md:104 still cites it (stale) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `sh -n scripts/release/make-appimage.sh` -> OK; `ls assets/brand/` -> No such directory; grep release.yml pins -> found (L804-823); AppRun shape `sh -n` -> OK |

## 1. One-paragraph verdict
A disciplined POSIX-sh packager: `set -eu`, ten required-env guards (`:?`), tool/path/binary existence checks, mktemp staging with a trap, build-time bin-name substitution in the generated AppRun, pinned-runtime invocation (`--runtime-file`, no rolling download), and a non-empty output assertion. Its pinning claims cross-check against release.yml (appimagetool 1.9.1 + type2 runtime 20251108, SHA-256 verified in the workflow), and the worklog records a real build+execute session (exit 0). Two S4 nits: a bare-name `Exec=` in the desktop entry and a hardcoded Categories line that ignores the product env var.

## 2. What actually works (verified)
— `sh -n scripts/release/make-appimage.sh` -> exit 0 (syntax clean, POSIX).
— Header claims verified against release.yml:804-823: pinned tool URL `.../appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage`, runtime URL tag `20251108`, `sha256sum -c` verification — exactly the "URL + SHA-256 pinned IN THE WORKFLOW" the script's header promises.
— Honest placeholder-icon posture: `ls assets/brand/` -> directory does not exist; the comment at L9-11 says as much ("assets/brand/*.png do not exist yet").
— Prior execution (not re-runnable here): worklog.md:644 "REAL AppImage built via make-appimage.sh with the pinned tools AND EXECUTED (APPIMAGE_EXTRACT_AND_RUN -> bundled dummy binary ran, exit 0)".

## 3. Findings
F-001 [S4] — Desktop entry `Exec=` is a bare binary name.
Where: scripts/release/make-appimage.sh:77-86
Code:
```
[Desktop Entry]
Type=Application
Name=${PRODUCT_NAME}
Comment=${PRODUCT_DESCRIPTION}
Exec=${BIN_DESKTOP}
Icon=${PRODUCT_LOWER}
```
What is wrong: `Exec=lumen-desktop` requires the bundled binary to be resolvable on PATH for a desktop-launcher invocation; the AppImage runtime installs desktop entries whose Exec must point at the AppImage (or use an AppRun-relative convention). The AppRun inside is correct (absolute `$HERE/usr/bin/...`), but the .desktop line can break menu launching while the AppImage itself runs fine. Confidence: MEDIUM (desktop-file integration semantics; not executable here) — Verification: read.
F-002 [S4] — Hardcoded Categories duplicates a product-driven value.
Where: scripts/release/make-appimage.sh:85 vs packaging env contract
Code:
```
Categories=AudioVideo;Video;Player;
```
What is wrong: `packaging-env.py` emits `PACKAGING_DESKTOP_CATEGORIES=AudioVideo;Video;Player;` (see docs/packaging/evidence/w26m-t1-bin-tauri-env.md output L69) — the product env var this script could consume — yet the script hardcodes the same string. Today they match; the seam lets product.toml drift from shipped .desktop categories silently. Confidence: HIGH — Verification: read + cross-doc comparison.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| die | 47 | REAL | — | error+exit 2 |
| main | 29-102 | REAL | S4 ×2 | guards, staging, build, verify-output |

## 5. Test honesty
No tests; worklog session evidence (real build, executed binary, exit 0) is the recorded validation; `sh -n` re-verified here.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "appimagetool 1.9.1 ... SHA-256 pinned IN THE WORKFLOW" | make-appimage.sh:13-16 | REAL | release.yml:804-823 (URLs + sha256sum -c) |
| "executed locally on linux x86_64 (worklog 7.w22g)" | make-appimage.sh:26-27 | REAL-UNPROVEN (session log; not re-runnable — no cargo here) | worklog.md:634,644 |
| "deterministic placeholder icon — assets/brand/*.png do not exist yet" | make-appimage.sh:9-11 | REAL | `ls assets/brand/` -> No such file or directory |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Requires python (icon gen via scripts/release/gen-icon.py — exists), install(1), mktemp, appimagetool + runtime paths from the workflow. Env-required-not-defaulted posture (worklog.md:640).

## 9. Open questions for the next agent
- Does the AppImage desktop-integration path (runtime's desktop-file installation) tolerate the bare `Exec=`? Only answerable with a real AppImage run on a desktop env.


## CORRECTION (same day, wiring verified)
I initially wrote "invoked by release.yml linux lane" — WRONG. Verified wiring: release.yml linux lane -> scripts/packaging/verify-linux-artifacts.sh:121 -> scripts/packaging/build-appimage.sh, which REIMPLEMENTS the AppDir/AppRun/appimagetool flow (its header: "scripts/release/make-appimage.sh flow for release purposes") and does NOT call this script. `grep -rn "make-appimage" .github/ scripts/packaging/` -> zero invocations (comment mentions only). This script is superseded legacy: syntactically valid, guards intact, execution evidence from worklog 7.w22g only (pre-supersession). The release-checklist:104 citation of it is stale relative to release.yml's actual drivers. Upgrade F-002 context: doc-vs-workflow drift, recorded here for the release-checklist owner.
