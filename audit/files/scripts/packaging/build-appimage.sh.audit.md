# AUDIT: scripts/packaging/build-appimage.sh
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines / functions | 137 / 1 (die) + main flow |
| File verdict | REAL (executed for real — the build half has committed evidence; the w30.8 payload change's launch evidence is newer than the committed log) |
| Reachable from shipped entry point? | N/A — release/packaging infra; invoked by release.yml Linux jobs (release.yml:720-812) and locally proven by docs/packaging/evidence/linux-lane-20261110.log |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | sh -n OK; ls -l gen-icon.py (no exec bit — comment TRUE); rg evidence logs (AppImage built + launch test exit 86); rg release.yml pinning block |

## 1. One-paragraph verdict
A real, defensive POSIX AppImage builder: `set -eu`, 12 `:?` env guards,
die-on-everything, trap cleanup of the staging dir, an explicit usr/bin
AppDir-convention assert tied to packaging.toml (so a config change away from
usr/bin fails loudly instead of silently mismatching the deb/rpm lanes), a
generated AppRun with correct build-time/runtime variable escaping, a shared
desktop-entry writer, and a real-brand-icon-with-documented-fallback path.
The build half is PROVEN by committed evidence
(docs/packaging/evidence/linux-lane-20261110.log:56 `build-appimage.sh: wrote
/tmp/w24c-dist/lumen-0.1.0-dev-x86_64-linux.AppImage`). The pinning story
checks out: release.yml:804-812 downloads appimagetool 1.9.1 release assets
(SHA-256-verified step name) + type2 runtime dated tag 20251108 with
--runtime-file — the evidence log's scary "continuous build (git version
8c8c91f)" self-description is the 1.9.1 tag commit per release.yml:807-809,
not a rolling download. One real evidence gap: the committed launch evidence
predates the w30.8 payload flip — the log's launch test shows the OLD exit-86
stub ("AppImage skeleton: exit 86 (as documented)", log line 73), so the
CURRENT AppRun-executes-BIN_TAURI behavior has NO launch evidence in-tree.

## 2. What actually works (verified)
- `sh -n scripts/packaging/build-appimage.sh` -> clean (ran, exit 0).
- AppRun heredoc escaping: `HERE=\$(cd "\$(dirname "\$0")" && pwd)` + `exec "\$HERE/$appdir_bin/${BIN_TAURI}" "\$@"` — read; build-time substitution only for path/name; runtime vars escaped. Correct.
- Evidence of real execution: linux-lane-20261110.log lines 7-56 (appimagetool creates a 4.0 filesystem, marks executable, script reports `wrote ...AppImage`).
- gen-icon.py exec-bit comment (lines 113-116) TRUE today: `-rw-rw-r--` for scripts/release/gen-icon.py (mode-only fix STILL not landed) — the through-the-interpreter invocation is load-bearing.
- Pinning: release.yml:804-812 — appimagetool 1.9.1 release asset + type2 runtime 20251108, no rolling runtime (the `--runtime-file` requirement at line 59 of the script enforces it).
- Runtime-dep boundary honestly documented (lines 23-32): no bundled webkitgtk; the deb/rpm declare the same deps.

## 3. Findings
F-001 [S3]
Where: scripts/packaging/build-appimage.sh:16-21 vs docs/packaging/evidence/linux-lane-20261110.log:72-73
Code:
```
# Payload binaries (w26.m.t2, updated w30.8): the AppDir carries
# ONE executable — BIN_TAURI (the ADR-004 tauri admission shell, the
# REAL GUI). The exit-86 lumen-desktop stub rode this image through
```
What is wrong: the script's cited evidence log post-dates only the STUB era —
its launch test records `AppImage skeleton: exit 86 (as documented)`, i.e. the
v0.1.0-dev AppImage launching the old stub. After the w30.8 flip (AppRun
execs BIN_TAURI directly) there is NO in-tree launch evidence for this
artifact: tauri launches are PENDING-CI (w26m-t5) and the RC1 "Linux verify+
launch harness green" claim is not re-verifiable from the tree. The header's
"EXECUTED for real in the local 8.E3 proof" is true for the BUILD half only.
Confidence: HIGH — Verification: read both; the launch half UNVERIFIED.
F-002 [S4]
Where: scripts/packaging/build-appimage.sh:117-123
Code:
```
if [ ! -f "$icon_src" ]; then
    PY=$(command -v python3 || command -v python) ||
        die "neither python3 nor python found on PATH (icon generation needs the stdlib)"
```
What is wrong: duplicated PY-probe idiom (same as common.sh/sign-macos.sh —
3rd copy in the tree); the gen-icon.py exec-bit workaround comment says
"flagged to the Lead for a mode-only fix" — still unfixed (mode 644 today).
Confidence: HIGH — Verification: ls -l.
F-003 [S4]
Where: scripts/packaging/build-appimage.sh:101-107
Code:
```
cat >"$appdir/AppRun" <<EOF
#!/bin/sh
```
What is wrong: nothing functional — the generated AppRun is minimal
(env-untouched). Note only: no `--` handling or arg forwarding beyond "$@"
(already forwarded). No GUI quirks handled (e.g. GDK_BACKEND) — acceptable
for the documented boundary. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory; file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| die | 66 | REAL | — | stderr + exit 1 |
| main flow | 47-137 | REAL (build half evidenced; launch half F-001) | S3 | guards -> convention assert -> stage -> AppRun -> entry -> icon -> tool -> size assert |

## 5. Test honesty
The script fails loud on every wrong precondition (12 `:?`, `-x`/`-f`/`-s`
asserts, usr/bin convention check). Mutation reasoning: a payload rename
would fail at line 86 (missing tauri binary) with the exact build hint; a
bindir config drift dies at line 79 naming the requirement. The size assert
(line 136) catches appimagetool writing an empty file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "EXECUTED for real in the local 8.E3 proof ... and in the release workflow's Linux jobs" | :45-46 | TRUE (build half) / UNVERIFIED (release half) | evidence log line 56; release.yml wiring read |
| "appimagetool ... the PINNED ... 1.9.1 + pinned type2 runtime" | :2,34-39 | TRUE | release.yml:804-812 pin block + README.md:86-100 |
| "the wave-22 commit carries it without the exec bit" (gen-icon.py) | :113-116 | TRUE | ls: -rw-rw-r-- |
| "Since w30.8 ... AppRun execs BIN_TAURI directly" | :16-21 | TRUE-code / UNPROVEN-launch | AppRun heredoc line 105 execs $BIN_TAURI; no post-w30.8 launch evidence (F-001) |

## 7. Dead / duplicated / unreachable code
None dead. PY-probe duplication (F-002).

## 8. Dependencies & flags
APPIMAGETOOL + RUNTIME_FILE required env (pinning enforced by requirement);
gen-desktop-entry.sh (shared writer, exists); gen-icon.py via interpreter;
python3/python; mktemp; install; appimagetool's APPIMAGE_EXTRACT_AND_RUN=1
(no FUSE — matches the sandbox/runner reality).

## 9. Open questions for the next agent
- Post-w30.8 AppImage launch evidence: when tauri-launch.yml gets its first
  green run, does the AppImage lane get one too? Track against
  docs/packaging/evidence (w26m-t4-release-tauri.md may own this).
