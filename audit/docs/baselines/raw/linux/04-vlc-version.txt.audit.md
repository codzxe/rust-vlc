# CLAIM AUDIT: docs/baselines/raw/linux/04-vlc-version.txt
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 3 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 3 / 0 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Genuine `vlc --version` capture of the pinned binary; the version string is the exact
identifier every downstream file (feature-matrix.toml meta baseline) cites.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)" | L7 | REAL | cat verbatim; identical string in docs/feature-matrix.toml meta.baseline (line 34 area) — cross-doc consistency proven |
| 2 | "exit=0" | L13 | REAL | verbatim footer |
| 3 | Provenance: pinned debs, not system-installed | L4 | REAL | header verbatim; system vlc absent (not re-run — sandbox) |

## 3-5. Contradictions / fabrication / matrix rows
None / none / N/A (evidence baseline). Commands: cat (whole file).
