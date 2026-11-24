# AUDIT: scripts/packaging/build-dmg.sh
| Field | Value |
|---|---|
| Auditor | W17-A09 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Lines / functions | 100 / 1 (die) |
| File verdict | REAL (guard layer executed by me; macOS-only steps CI-executed per worklog/release evidence) |
| Reachable from shipped entry point? | YES — .github/workflows/release.yml:552 (macos-arm64 job) and :694 (macos-x64 job) both run `scripts/packaging/build-dmg.sh` with packaging-env.py-provided env |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | sh -n (0); bash -n (0); env -i guard runs x5 (exit 2/1/1/1/2 — outputs below); rg caller/deps/config greps; sed/od quote verifications |

## 1. One-paragraph verdict
This is a genuine, tight lane orchestrator, not a fake: every guard is real and
I executed them (missing-env → exit 2 with the actionable message; wrong OS →
exit 1 "the macOS lane"; bad arch → exit 1; universal=true → exit 1 refusing
the unimplemented lipo build; missing BIN_TAURI → exit 2), syntax is clean
under both sh -n and bash -n, both callee scripts exist and are executable,
the universal refusal is genuinely config-driven (packaging.toml [macos]
universal = false → packaging-env.py:312 exports it → the script dies on
anything but "false"), and the dmg name contract at :96 byte-matches
make-dmg.sh's own output line. The full lane (codesign/notarize/hdiutil) needs
macOS — honestly fenced in the header as CI-only — and its execution is
recorded in-tree (worklog.md:1997-2003: the RC1 runs produced macOS
dmg/zip artifact sets, run 43466814633). Two S4 hygiene notes only.

## 2. What actually works (verified)
- `sh -n scripts/packaging/build-dmg.sh` -> 0; `bash -n` -> 0.
- Guard executions (real runs, outputs pasted):
  - `env -i sh scripts/packaging/build-dmg.sh` -> stderr "build-dmg.sh: 51: PRODUCT_NAME: build-dmg.sh: PRODUCT_NAME not set (run packaging-env.py first)", exit 2.
  - OS_ID=linux variant -> "build-dmg.sh: error: build-dmg.sh is the macOS lane (OS_ID=linux)", exit 1.
  - ARCH_ID=armv7 -> "unexpected macOS ARCH_ID=armv7", exit 1.
  - PACKAGING_MACOS_UNIVERSAL=true -> "packaging.toml [macos] universal = true, but the universal2 (lipo) build is NOT implemented in v1 — flip the config only together with the implementing card", exit 1.
  - BIN_TAURI unset -> "build-dmg.sh: 59: BIN_TAURI: BIN_TAURI not set", exit 2.
- Wiring: release.yml:500-552 + :653-694 call it; dependencies
  scripts/release/make-app.sh, scripts/release/make-dmg.sh,
  scripts/packaging/sign-macos.sh all exist (executable, rg-verified).
- Payload contract: BIN_TAURI required (:59) and make-app.sh:57/:77 installs
  it as Contents/MacOS/$BIN_TAURI (CFBundleExecutable) — the w30.8
  stub-dropped posture, consistent with release.yml:8-18.

## 3. Findings
F-001 [S4]
Where: scripts/packaging/build-dmg.sh:84
Code: `mkdir -p "$DIST_DIR" "$APP_DIR"`
What is wrong: no stale-artifact hygiene — a partially-written ${PRODUCT_NAME}.app from an aborted prior run stays in APP_DIR (mkdir -p only appends); make-app.sh installs over the members it knows, so unknown stale members would survive into the dmg. Why it matters: CI scratch dirs are usually fresh, so blast radius is small, but local re-runs can inherit ghosts. What correct looks like: rm -rf of the specific .app before staging (or a version-stamped APP_DIR). Confidence: MEDIUM — Verification: read (make-app.sh not executed — needs built binaries).
F-002 [S4]
Where: scripts/packaging/build-dmg.sh:96-97
Code: `dmg="$DIST_DIR/${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}.dmg"`
What is wrong: the artifact-naming convention is duplicated between this orchestrator and make-dmg.sh's own `out=` line — the two can drift apart on a rename. Why it matters: today drift fails LOUDLY (the -s check dies "expected dmg not found after make-dmg.sh"), so it is self-detecting, but the duplication is the rot vector. What correct looks like: make-dmg.sh echoes its output path and the orchestrator consumes it (or a shared name function). Confidence: HIGH — Verification: read both files side by side.

## 4. Per-section table (100 LOC script; single function die())
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header/pipeline docs | 1-48 | accurate | — | pipeline order, universal decision, env contract, honest local-proof boundary all verified against callees |
| set -eu + 6 ${VAR:?} guards | 49-62 | REAL | — | all six executed (guards 1/5 above) |
| defaults + ROOT cd | 63-68 | REAL | — | TARGET_DIR/DIST_DIR/APP_DIR defaults; cd ROOT makes relative defaults safe |
| die() + OS/ARCH guards | 70-77 | REAL | — | executed (guards 2/3) |
| universal refusal | 79-82 | REAL | — | executed (guard 4); config-driven, matches packaging.toml:96 |
| 4-step lane | 84-98 | REAL (macOS-only) | S4 x2 | make-app → sign-macos --app → make-dmg → sign-macos --dmg; all callees exist; findings F-001/F-002 |
| completion echo | 100 | fine | — | — |

## 5. Test honesty
No tests in file (shell lane script). The repo's coverage of it is CI
execution (release.yml macOS jobs) plus the gate.sh packaging gate
(bash -n/structure checks — rg: gate.sh has no dmg execution; it checks
structure only). UNVERIFIED: the actual codesign/notarize path (needs macOS
+ credentials; the honest-unsigned branch is the one the RC1 runs exercised
per worklog:2002 "UNSIGNED sidecars (honest labeled-unsigned)").

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "macOS lane (build-dmg.sh — bundle, codesign, dmg, notarize, staple)" | docs/packaging/README.md:70 | TRUE | the 4-step pipeline :86-98 matches exactly |
| "macOS dmg ... `scripts/packaging/build-dmg.sh`; sign-macos.sh [blocked: R-35]" | docs/release-checklist.md:133 | TRUE | both scripts exist; R-35 unsigned path verified in sign-macos.sh:203-207 |
| "staging steps are pure file ops; codesign/notarytool/hdiutil are CI-only" | docs/packaging/README.md:70 | TRUE | make-dmg.sh:35-36 dies off-macOS; header fence :45-47 |

## 7. Dead / duplicated / unreachable code
None dead. Duplication: the artifact naming (F-002). The script is
w24/c-lane code reached only via release.yml macOS jobs — reachable.

## 8. Dependencies & flags
Consumes: make-app.sh, make-dmg.sh (scripts/release), sign-macos.sh
(--app/--dmg modes at :60-64). Env: PRODUCT_NAME/LOWER/VERSION, BIN_TAURI,
OS_ID, ARCH_ID, PACKAGING_MACOS_UNIVERSAL (exported by packaging-env.py:312
`put("MACOS_UNIVERSAL", str(macos["universal"]).lower())`), TARGET_DIR,
DIST_DIR, APP_DIR. No feature-flag gating.

## 9. Open questions for the next agent
- Did the RC1 macOS jobs run the SIGNED or UNSIGNED arm? worklog says
  unsigned sidecars shipped; a signed run has never been evidenced anywhere
  (R-35 still open) — sign-macos.sh's real codesign path remains
  REAL-UNPROVEN tree-wide.
- F-001's stale-.app ghost: worth a one-line rm -rf before make-app.sh.
