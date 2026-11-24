# AUDIT: scripts/release/make-app.sh
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Date | 2026-10-30 |
| Lines / functions | 123 / 3 (die, xml_escape, main flow) |
| File verdict | REAL (executed end-to-end with hostile inputs) |
| Reachable from shipped entry point? | YES — .github/workflows/release.yml:500,552 → scripts/packaging/build-dmg.sh:87 `APP_DIR="$APP_DIR" scripts/release/make-app.sh` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | execution in audit/ sandbox (exit 0, output quoted below); `rg` reachability traces (exit 0) |

## 1. One-paragraph verdict
A correct, guarded, honestly-documented POSIX bundle builder. I executed it with hostile identity inputs (name containing `& < >`, bundle id containing `"`): it produced the exact documented layout (Contents/{Info.plist, MacOS/<BIN_TAURI>, Resources/{LICENSE, README.md}}), escaped every XML metacharacter correctly, and exited 0. The binary is installed with mode 0755. Guards are real: five `:?` env guards, binary-existence check with actionable message, LICENSE/README existence check. `set -eu` present (unlike the quality-tier scripts).

## 2. What actually works (verified)
- Executed: `env PRODUCT_NAME='Lumen&Co <v>' PRODUCT_VERSION=0.1.0 PRODUCT_VERSION_CODE=42 PRODUCT_BUNDLE_ID='dev.lumen.app"x' BIN_TAURI=lumen-tauri TARGET_DIR=audit/runs/make-app-test/target APP_DIR=audit/runs/make-app-test/dist sh scripts/release/make-app.sh` → exit 0.
- Produced plist escapes verified byte-exact:
```
    <string>Lumen&amp;Co &lt;v&gt;</string>
    <string>dev.lumen.app&quot;x</string>
```
  (make-app.sh:80-83 `xml_escape` sed chain — `&`, `<`, `>`, `"` all handled).
- Layout verified: `ls -R` → Contents/{Info.plist, MacOS/lumen-tauri, Resources/{LICENSE, README.md}}; `ls -l Contents/MacOS/` → `-rwxr-xr-x` (install -m 0755).
- Guard verified by read: make-app.sh:51-57 five `: "${VAR:?…}"` guards; :68 `[ -f "$tauri_bin" ] || die "missing tauri binary: …"`; :69 LICENSE/README check.

## 3. Findings
F-001 [S4]
Where: scripts/release/make-app.sh:88-120
Code:
```
cat >"$app_root/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
```
What is wrong: the generated plist is never linted (`plutil -lint` exists on every macOS runner the script targets). Why it matters: an escaping regression would surface only at Gatekeeper/notarization time, far downstream of the script. Blast radius small — the escaping is currently correct (proved above). What correct looks like: `plutil -lint "$app_root/Contents/Info.plist"` when available. Confidence: HIGH — Verification: read + ran.

F-002 [S4]
Where: scripts/release/make-app.sh:102-105
Code:
```
    <key>CFBundleVersion</key>
    <string>${PRODUCT_VERSION_CODE}</string>
```
What is wrong: this script writes VERSION/VERSION_CODE/BUNDLE_ID into the plist with no format validation of its own (Apple requires CFBundleVersion to be a monotonic integer triplet). Mitigated upstream: scripts/release/product-env.py:158-160 dies if `identity.version_code` is not an int — verified by rg. Defense-in-depth absent in this script if ever invoked outside product-env.py. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| env guards / config | 49-69 | REAL | — | 5 `:?` guards; binary/LICENSE checks; `set -eu` |
| bundle layout | 71-78 | REAL | — | executed: correct dirs, 0755 binary |
| xml_escape | 80-86 | REAL | — | executed with `&<>"` inputs — all escaped |
| Info.plist emission | 88-120 | REAL | S4 | correct output; no lint (F-001) |
| summary echo | 122 | REAL | — | states the one-exe fact correctly |

## 5. Test honesty
No automated test exists for make-app.sh (no scripts test lane covers it in this repo — the only in-repo consumers are build-dmg.sh and manual runs). Mutation reasoning: if xml_escape were gutted, the committed release would still build on Linux and fail only on macOS. My execution above is the only live proof in this audit.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Pure file operations — runs on any POSIX host (layout verifiable on Linux; the release workflow runs it on macOS)" | make-app.sh:43-44 | REAL | Executed on this Linux sandbox, exit 0, layout above. |
| "the bundle is NOT code signed and NOT notarized" | make-app.sh:36-37 | REAL-UNPROVEN | Sign/notarize steps live in sign-macos.sh / build-dmg.sh (outside this file's scope; the comment states a limit, not a capability — no reason to doubt it). |
| "w30.8 re-pointed it [CFBundleExecutable] from the exit-86 lumen-desktop stub" | make-app.sh:9-17 | REAL | :106-107 `<key>CFBundleExecutable</key><string>${BIN_TAURI}</string>` — the tauri bin is the only staged executable (verified layout: exactly one file in MacOS/). |

## 7. Dead / duplicated / unreachable code
None. make-dmg.sh:39 re-checks the bundle exists (`[ -d "$app" ] || die "bundle not found: $app (run make-app.sh first)"`) — redundant-but-harmless cross-script guard.

## 8. Dependencies & flags
POSIX sh (`#!/bin/sh`); needs `install`, `cp`, `mkdir`, `sed`, `cat`, `printf`. Env: PRODUCT_NAME/VERSION/VERSION_CODE/BUNDLE_ID/BIN_TAURI (all required), TARGET_DIR (default `target`), DIST_DIR/APP_DIR (default `dist`). No feature flags. Upstream producer product-env.py validated (see F-002).

## 9. Open questions for the next agent
- scripts/packaging/build-dmg.sh (the actual CI caller, another agent's domain) — confirm it exports all five required vars or the `:?` guards will (correctly) abort the macOS lane.
- Does any committed doc claim the bundle is signed? make-app.sh:36-37 explicitly says the opposite — the honest claim; keep it that way.
