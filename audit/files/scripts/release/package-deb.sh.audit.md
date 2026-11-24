# AUDIT: scripts/release/package-deb.sh
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Date | 2026-11-24 |
| Lines / functions | 126 / 1 (die) — linear build flow |
| File verdict | REAL (genuine dpkg-deb packer, dry-run validated per worklog 7.w22g) — SUPERSEDED/legacy: contradicts the current payload contract |
| Reachable from shipped entry point? | NO (no workflow calls it; current release.yml uses scripts/packaging/build-deb.sh via verify-linux-artifacts.sh; only docs/RELEASE.md:109 cites it) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg (callers) -> 0 workflow hits; worklog.md:634-660 read; bash -n equivalent not needed (POSIX read line-by-line); quotes sed-verified |

## 1. One-paragraph verdict
A real, defensively-written POSIX deb packer from the 7.w22g release lane: `set -eu`,
12 named `: ${VAR:?}` guards, mktemp staging with trap cleanup, umask-normalized
0644 modes, non-empty output check — and the worklog records a REAL dry-run (a
4924-byte .deb built and `dpkg-deb --info/--contents + -x` round-tripped on this
host, dummy binaries). But it is the OLD lane: it stages `BIN_DESKTOP` (the exit-86
stub), points the desktop entry `Exec=` at the stub, declares NO webkit Depends, and
hardcodes docs/categories/Section — all decisions that now live in
packaging/packaging.toml and that the current payload contract (w30.8) REVERSED. A
deb built by this script today would be refused by verify-linux-artifacts.sh's
stub-absence assertion. Its only live reference is docs/RELEASE.md:109, which itself
names the superseded script set.

## 2. What actually works (verified)
- worklog.md:634-660 (task 7.w22g): "REAL .deb built via package-deb.sh on this host (dpkg-deb --info/--contents + -x roundtrip verified: control fields config-driven, usr/bin + applications + hicolor icon layout, root ownership)" — the header's "dry-run tested on Linux (worklog 7.w22g)" claim is REAL.
- Mode hygiene: L87-89 chmod 0644 after `cp` with the umask rationale; install -m 0755 for binaries — correct.
- All control fields are env-driven (PRODUCT_* from product-env.py) — no identity literals in the script.
- Icon: gen-icon.py placeholder (clean-room; assets/brand does not exist — `ls assets/` fails — so the "do not exist yet" comment is still true and corroborates the baseline config-validator "4 missing brand assets" FAILURE).

## 3. Findings
F-001 [S3]
Where: scripts/release/package-deb.sh:46,70,85,98,107-118
Code:
```
: "${BIN_DESKTOP:?package-deb.sh: BIN_DESKTOP not set}"
...
install -m 0755 "$desktop_bin" "$root/usr/bin/$BIN_DESKTOP"
...
Exec=${BIN_DESKTOP}
```
What is wrong: this script ships the exit-86 `lumen-desktop` stub as a user payload,
points the desktop entry at it, and declares no runtime Depends — the exact payload
shape the w30.8/w30.9 era REVERSED (verify-linux-artifacts.sh:210-211 dies if the
stub is in a payload; the current entry Exec=BIN_TAURI; the current deb Depends
carries libgtk-3-0 + libwebkit2gtk-4.1-0). Why it matters: any consumer following
docs/RELEASE.md:109 (which still names this script) would build a deb that fails
the current release verification contract and ships the confusing stub to users.
What correct looks like: retire the script (delete or reduce to a pointer to
scripts/packaging/build-deb.sh) and fix RELEASE.md's Linux section. Confidence:
HIGH — Verification: read + rg (0 workflow callers; contradiction cross-checked
against verify-linux-artifacts.sh and gen-desktop-entry.sh:43).
F-002 [S4]
Where: scripts/release/package-deb.sh:71,86,101,110
Code:
```
for f in "$cli_bin" "$desktop_bin" LICENSE README.md; do
...
Categories=AudioVideo;Video;Player;
...
Section: video
```
What is wrong: packaging decisions (shipped docs list, desktop categories, deb
Section) are hardcoded here while packaging/packaging.toml declares itself the
"Single source of truth for PACKAGING decisions" — a duplicated-decision surface
that can silently drift (and HAS: the current config's docs list is
["LICENSE","README.md"] — same values today, but only by luck of no drift yet).
Confidence: HIGH — read.

## 4. Per-function table
| Stage | Lines | Verdict | Notes |
|---|---|---|---|
| env guards | 38-48 | REAL | 12 named `:?` guards incl. PRODUCT_ACCENT_COLOR (icon) |
| tool/arch checks | 57-67 | REAL | dpkg-deb presence; amd64/arm64 map with typed die; python resolution for icon |
| input checks | 69-73 | REAL | missing-input die names the build command |
| staging | 75-89 | REAL | mktemp + trap; umask-normalized 0644 |
| desktop entry | 93-102 | REAL (then) / contract-violating (now) | Exec=BIN_DESKTOP (F-001); categories hardcoded (F-002) |
| control | 107-118 | REAL (then) | config-driven fields; Description prose claims "Both shipped binaries" — the stub era's truth |
| build + check | 120-126 | REAL | dist dir, non-empty output check, arch echo |

## 5. Test honesty
The dry-run evidence (worklog 7.w22g) is real and was validated with dpkg-deb
round-trip — for the PAYLOAD OF ITS ERA. Nothing current re-validates it; the
current lane's proof lives in the 8.E3 evidence chain (linux-lane-20261110.log).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "dry-run tested on Linux (worklog 7.w22g)" | L34 | REAL | worklog.md:634-660 records the real deb build + roundtrip |
| "Every metadata field is config-driven (via scripts/release/product-env.py)" | L13-14 | REAL | all PRODUCT_* env keys asserted at L38-44 |
| "No maintainer scripts ... ships only files" | L23-26 | REAL | no postinst/prerm in staging |
| docs/RELEASE.md:109 "Linux: ... `.deb` by `package-deb.sh`" | docs/RELEASE.md | STALE | current release.yml calls scripts/packaging/build-deb.sh (via verify-linux-artifacts.sh:118) — RELEASE.md describes the superseded 7.w22g lane (flagged for RELEASE.md's owner) |

## 7. Dead / duplicated / unreachable code
The entire script is effectively legacy: 0 workflow callers, 1 stale doc reference,
superseded by scripts/packaging/build-deb.sh. Its sibling legacy set
(make-app.sh, make-dmg.sh, make-appimage.sh, package-windows.sh) shares the same
status (RELEASE.md:99-112 names them; release.yml does not call them).

## 8. Dependencies & flags
dpkg-deb, install/cp/chmod, gen-icon.py (exists at scripts/release/gen-icon.py),
product-env.py environment. No config-file read (predates packaging.toml).

## 9. Open questions for the next agent
- docs/RELEASE.md owner: the release-lane section (L99-112) names the entire
  superseded scripts/release/* set — reconcile with release.yml's actual
  scripts/packaging/* calls, or the next person packaging by the README will
  use the stub-shipping lane.
- Is there a retention reason for scripts/release/package-*.sh (history?
  reference?) — if none, they are dead weight contradicting current policy.
