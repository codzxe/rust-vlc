# CLAIM AUDIT: docs/baselines/raw/macos/03-dmg-downloads.txt
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 4 / 2 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Th honest evidence log it claims to be: three DMG downloads with sizes cross-checked
against the directory listing, `file` output, and UDIF "koly" trailer dumps. The
honesty marker: the task card's "macos/" URL 404s and the log says so, records the
correction to "macosx/", and states zero retries were needed. Nothing inflated.

## 2. Claim ledger
| # | Claim (verbatim, trimmed) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "arm64 ... Final size: 51,273,389 bytes ... exit=0" | mid | REAL | verbatim in file with curl meter line |
| 2 | "intel64 ... 57,544,217 ... exit=0" | mid | REAL | verbatim |
| 3 | "universal ... 89,124,160 ... exit=0" | mid | REAL | verbatim |
| 4 | "SIZE CROSS-CHECK vs directory listings ... -> match" (x3) | lower | REAL-UNPROVEN | listing (dump 01) not in my scope; internal arithmetic is consistent |
| 5 | "each dmg's last 512 bytes begin with the magic 'koly'" | tail | REAL-UNPROVEN | od dumps are in the file verbatim; DMGs are gitignored tmp/ artifacts — cannot re-verify in sandbox |
| 6 | "task card's probe URL used path segment 'macos/' which 404s ... corrected" | header | REAL | verbatim admission; anti-fabrication behavior |

## 3-4. Contradictions / fabrication indicators
None.
## 5. Matrix rows
N/A (evidence baseline for the macOS pinning wave). Commands: cat (43 lines).
