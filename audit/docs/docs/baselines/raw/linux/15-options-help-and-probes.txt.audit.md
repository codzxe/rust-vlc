# CLAIM AUDIT: docs/baselines/raw/linux/15-options-help-and-probes.txt
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 5 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 1 / 4 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A raw VLC-3.0.23 help-flag probe record. Internally exact: the embedded
`--help` dump is verifiably 407 lines as claimed, the footers match VLC's
real help grammar, and the negative probes (help-verbose empty,
--advanced redundant) are honestly recorded as command outcomes with exit
codes. The only thing I cannot prove is that VLC 3.0.23 itself emitted it
(no vlc binary in this sandbox) — REAL-UNPROVEN as a record.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "--help (basic usage + core non-advanced options, 407 lines)" | :8 | REAL | `awk NR>=11&&NR<=417 | wc -l` → 407 (dump body between command and `# exit=0` at 418) |
| 2 | "--help-verbose … NOT a dump flag on 3.0.23" | :420-426 | REAL-UNPROVEN | no vlc binary in sandbox; consistent with VLC docs (help-verbose is a modifier) |
| 3 | "--advanced --full-help vs --full-help → 0 lines, byte-identical" | :428-431 | REAL-UNPROVEN | cannot re-run; no contradiction in record |
| 4 | "--full-help hides 4 advanced-only modules in this build" | :434-435 | REAL-UNPROVEN | footer text quoted matches VLC's real "Note: N modules were not displayed" grammar |
| 5 | "Binary: tmp/pinning/extract/usr/bin/vlc from debs … (not system-installed)" | :5 | REAL-UNPROVEN | path not in repo; binary absent from sandbox |

## 3. Contradictions
None. Footer strings at 415-417 ("Note: add --advanced … use '-H'.") are
genuine VLC help-footer text, consistent with claim 1/4.

## 4. Fabrication / inflation indicators
None — includes a NEGATIVE result (probe 2) and byte-diff null result
(probe 3); fabricators rarely record failures.

Commands: awk/wc line count (exit 0), sed/grep structure scan. VLC run:
UNVERIFIED (no binary).
