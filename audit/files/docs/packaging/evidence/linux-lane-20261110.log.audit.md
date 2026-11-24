# AUDIT: docs/packaging/evidence/linux-lane-20261110.log (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines | 99 (read in full) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — run-evidence log |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read (full); cross-diff vs w24c-r3.log; rg build-appimage.sh; rg 'reproduc' docs/ |

## 1. One-paragraph verdict
The earlier same-day (2026-11-10) sibling of w24c-r3.log: identical battery
structure, honest rpm spec-only label, PASS summary. It cross-corroborates
the r3 log in every shared detail (appimagetool git 8c8c91f build 296,
identical squashfs stats, identical 121900-byte SBOMs, same f0b22c1b...
SBOM hash) while the artifact hashes DIFFER between the two runs — proving
the artifacts are timestamp-dependent (non-reproducible), which no doc
claims otherwise. Same SBOM duplication defect as the sibling.

## 2. What actually works (verified)
— Cross-log consistency: SBOM hash f0b22c1b... appears 4x here AND 4x in
w24c-r3.log — two independent runs producing the same bytes (the SBOM is
workspace-level, deterministic).
— The .spec SHA (17bd1068...) is IDENTICAL across both runs — spec text is
deterministic; tar.gz/deb/AppImage hashes differ (3b5eae75/a39dafa8/
c0b86e29 here vs 78f81c0f/0e279eba/9fd200fe in r3) — archives embed
timestamps; no reproducible-build claim exists in docs to contradict.
— AppImage payload design verified against build-appimage.sh: the AppImage
ships ONE executable (the GUI shell) — the 889.45 KB uncompressed / 4-file
squashfs is the stub binary + entry + icon + AppRun, consistent (build-
appimage.sh:82-101 "The GUI shell — the AppImage's one executable").
— CLI smoke fixture path exists (verified in the r3 audit).

## 3. Findings
F-001 [S3]
Where: docs/packaging/evidence/linux-lane-20261110.log:95-98
Code:
```
f0b22c1bc2a8705d5d1f6e330efa201e842fa70eaf84bb626580ff8af6b75ad7  /tmp/w24c-dist/lumen-0.1.0-dev-x86_64-linux.tar.gz.sbom.cdx.json
f0b22c1bc2a8705d5d1f6e330efa201e842fa70eaf84bb626580ff8af6b75ad7  /tmp/w24c-dist/lumen-0.1.0-dev-x86_64-linux.deb.sbom.cdx.json
f0b22c1bc2a8705d5d1f6e330efa201e842fa70eaf84bb626580ff8af6b75ad7  /tmp/w24c-dist/lumen-0.1.0-dev-x86_64-linux.AppImage.sbom.cdx.json
f0b22c1bc2a8705d5d1f6e330efa201e842fa70eaf84bb626580ff8af6b75ad7  /tmp/w24c-dist/lumen-0.1.0-dev-x86_64-linux.spec.sbom.cdx.json
```
What is wrong: identical to the sibling log's finding — all four
"per-artifact" SBOMs byte-identical (121900 bytes each, :79-88), i.e. one
workspace CycloneDX document stamped four times; per-artifact naming
promises differentiation the content does not have.
Confidence: HIGH — Verification: read + cross-log diff.
F-002 [S4]
Where: docs/packaging/evidence/linux-lane-20261110.log:91-94 vs w24c-r3.log:110-113
Code:
```
3b5eae753466e74cf63557415b82ed82a37c430c9463186d63a365c09ed4db4d  /tmp/w24c-dist/lumen-0.1.0-dev-x86_64-linux.tar.gz
```
What is wrong: the same-day reruns produce different artifact hashes
(tar.gz/deb/AppImage) — artifacts are timestamp-dependent; release
checksum manifests are run-specific, so a consumer cannot verify a
released artifact against another lane's manifest. No doc claims
reproducible builds (rg 'reproduc' — only risk-register/plugin-doc
contexts), so no contradiction, but the checksum practice silently
assumes run-local meaning.
Confidence: HIGH — Verification: read + diff of two logs.
F-003 [S4]
Where: docs/packaging/evidence/linux-lane-20261110.log:1
Code:
```
=== 1. build .tar.gz (plain archive lane) ===
```
What is wrong: no header at all — no date/host/branch/exit-code line (the
r3 sibling has one); provenance rides entirely on the filename.
Confidence: HIGH — Verification: read.

## 4. Data summary
Steps 1-9 of verify-linux-artifacts.sh: tar.gz, deb (dpkg-deb), AppImage
(pinned appimagetool, squashfs 4.0 zstd, 4 files/8 inodes/3 dirs), rpm
spec-only (honest label :58-63), deb launch tests (CLI --version, CLI
smoke decode of the wav fixture a_decoded=1, desktop skeleton exit 86),
AppImage extract-and-run (exit 86), tar.gz member check, 4 SBOMs (xtask
sbom, relative target path), SHA256SUMS manifest, PASS summary.

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "verify-linux-artifacts.sh: PASS — ... packaged binaries launch-tested" | :99 | REAL | consistent with r3 sibling + current script reading |
| "rpmbuild absent ... labeled, never faked" | :58-60 | REAL | no rpmbuild output in log |

## 6. Dead / duplicated code
N/A (log).

## 7. Dependencies & flags
appimagetool pinned (git 8c8c91f, build 296, 2025-12-04) — same pin in
both logs; dpkg-deb native.

## 8. Open questions
Why are the two same-day logs (w24c-dist and w24c-dist2) both committed —
which run is the canonical one? Neither manifest's artifact hashes match the
other, so each is only self-consistent.
