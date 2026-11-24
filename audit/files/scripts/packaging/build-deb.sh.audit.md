# AUDIT: scripts/packaging/build-deb.sh
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 107 / 2 (main flow, die) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — release.yml linux jobs via scripts/packaging/verify-linux-artifacts.sh (the same script the local evidence log ran); also directly usable on a Debian-family host |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## 1. One-paragraph verdict
A clean, real dpkg-deb lane: `set -eu`, 8 fail-loud `:?` env guards, an arch map with a refuse arm, the ONE shared FHS payload stager (no duplicated file lists), `--root-owner-group` packing (ownership independent of the build user — correct for CI), and a `-s` output assert. Executed-for-real evidence exists (docs/packaging/evidence/linux-lane-20261110.log shows genuine dpkg-deb output). ONE REAL DEFECT: the DEBIAN/control Description claims THREE shipped binaries ("the engine-driven CLI, the desktop composition-root binary and the tauri admission shell") while the payload stager ships TWO — the w30.8 decision removed the exit-86 stub from every user payload, but this control text still names it. The .deb's own metadata overclaims its contents — the exact payload-vs-claims class the release audit hunts.

## 2. What actually works (verified)
— Read in full; `bash -n` posture documented; fail-loud guards verified by reading (and the whole lane ran in the local proof — the evidence log's `dpkg-deb: building package 'lumen' ... (arch=amd64, section=video)` line matches this script's echo shape).
— Stager cross-check: stage-linux-payload.sh stages exactly `$BIN_CLI` + `$BIN_TAURI` (install lines read; stub-dropped w30.8 per its own header) — the deb payload is the honest two-bin set.
— Output path: `<slug>-<version>-<arch>-linux.deb` == the artifact-kind the release job expects (`linux.deb`) — consistent with release.yml's kind glob.
- Depends rows (libgtk-3-0, libwebkit2gtk-4.1-0): correct webkit2gtk-4.1 ABI names; script-local by documented decision (w26.m.t2, flagged to Lead — read).

## 3. Findings

F-001 [S3]
Where: scripts/packaging/build-deb.sh:95-98
Code:
```
Description: ${PRODUCT_DESCRIPTION}
 The shipped binaries (the engine-driven CLI, the desktop
 composition-root binary and the tauri admission shell) install under
 /${PACKAGING_LINUX_BINDIR}; this package ships files only, no
 maintainer scripts.
```
What is wrong: the control Description names THREE binaries — including "the desktop composition-root binary" (the exit-86 `lumen-desktop` stub) — while stage-linux-payload.sh ships exactly TWO (BIN_CLI + BIN_TAURI; the stub was dropped from user payloads in w30.8 by explicit decision). The package's own metadata contradicts its contents. Why it matters: this is the shipped-payload-vs-claims class: every user who inspects the .deb (`dpkg -I`) reads a claim about a binary that is not in the package — the very confusion the w30.8 change was made to end, surviving in metadata. What correct looks like: "the engine-driven CLI and the tauri admission shell". Confidence: HIGH — Verification: read (stager + build-deb both read; stub absence verified in stage-linux-payload.sh header + install lines). Severity S3 (one metadata line; no behavior effect) but it violates the repo's own payload-honesty policy — flag prominently.

F-002 [S4]
Where: scripts/packaging/build-deb.sh:94 — `Description: ${PRODUCT_DESCRIPTION}`
What is wrong (latent): if PRODUCT_DESCRIPTION ever contains a colon/comma Debian-control pitfall or multiple lines, the unescaped heredoc line would emit a malformed control file. Today's value ("A clean-room, Rust-native media player.") is clean. Confidence: MEDIUM — Verification: read + current config value checked (single line, no commas).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main flow | 34-107 | REAL | S3 | guards, arch map (refuse arm), stager call, control write (F-001), dpkg-deb pack, -s assert |
| die | 50 | REAL | - | exit 1 with script tag |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| verify-linux-artifacts.sh deb header asserts (the lane's own gate) | PARTIAL | the lane asserts Section/Architecture/Version fields against config (per its docs) — but unless it asserts the Description text, F-001 passes every gate. Not my file; handoff: check whether the verify script pins the Description line (likely not). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "EXECUTED for real in the local 8.E3 proof" | L32-33 | REAL | evidence log contains real dpkg-deb output with matching echo line (spot-checked) |
| "Depends: libgtk-3-0 + libwebkit2gtk-4.1-0 (script-local ...)" | L18-20, L93 | REAL | rows present; rationale + Lead flag documented in-band |
| payload = "stage-linux-payload.sh (the ONE shared FHS tree)" | L23 | REAL | stager read; single source; two bins + docs + entry + icon |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
dpkg-deb (host tool, presence-checked); stage-linux-payload.sh + gen-desktop-entry.sh siblings. No config literals except the documented Depends rows.

## 9. Open questions for the next agent
1. verify-linux-artifacts.sh: does its deb header assertion cover Description (would it catch F-001)?
2. build-rpm.sh: same F-001 class — does the rpm spec's %description also name the removed stub?
