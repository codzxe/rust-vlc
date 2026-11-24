# CLAIM AUDIT: docs/baselines/raw/linux/09-upstream-vlc-dir-listing.txt
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 4 / 0 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Genuine curl capture of videolan.org's 3.0.23 release directory. Its FINDINGS section
(purpose-built claim) is fully supported by the verbatim listing above it: platforms =
macosx/os2/win32/win64/winarm64, no Linux dir; tarball + 4 sidecars, no .sha512. Honest
correction of a task-card assumption (no .sha512) is recorded — anti-inflation behavior.

## 2. Claim ledger
| # | Claim (verbatim) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "confirm get.videolan.org publishes NO Linux binaries" | L3 | REAL | listing lines show only macosx/ os2/ win32/ win64/ winarm64/ dirs — no linux entry (cat verbatim) |
| 2 | "source tarball vlc-3.0.23.tar.xz (26,486,988 bytes...)" | tail FINDINGS | REAL | listing line: vlc-3.0.23.tar.xz 26486988 |
| 3 | "NO .sha512 sidecar (contradicts task-card assumption)" | tail | REAL | sidecars listed: .asc .md5 .sha1 .sha256 only |
| 4 | "Same content ... at https://download.videolan.org ... (verified, HTTP 200)" | L25 | REAL-UNPROVEN | cannot re-curl from sandbox; internal-consistent claim about a capture session |

## 3-5. Contradictions / fabrication / matrix rows
None (the one contradiction found — task card vs reality — is honestly documented) / none / N/A.
Commands: cat (whole file, 27 lines).
