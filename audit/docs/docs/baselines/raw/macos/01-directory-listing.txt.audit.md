# CLAIM AUDIT: docs/baselines/raw/macos/01-directory-listing.txt (62 lines — raw capture)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 2 (curl capture authenticity; sizes match pin record) |
| REAL / ... / FALSE | 2 REAL / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Genuine raw curl capture of the official macOS directory listing (verbatim nginx HTML
embedded), with command + exit code + byte count recorded. The captured arm64 dmg size
(51,273,389 B) matches the master pin record's claimed size byte-exactly — the two files
independently corroborate each other. No claims beyond captured data.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "curl ... exit=0, 2154 bytes received" + verbatim listing | :7-10 | REAL | File content is the capture itself; HTML title "Index of /videolan/vlc/3.0.23/macosx/". |
| 2 | Captured file sizes used as pin evidence | listing rows | REAL | "vlc-3.0.23-arm64.dmg ... 51273389" == vlc-3.0.23.md:24 "arm64 51,273,389 B". |

## 3-4. Contradictions / fabrication indicators
None; none.
