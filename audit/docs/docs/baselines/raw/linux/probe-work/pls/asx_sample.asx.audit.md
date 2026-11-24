# CLAIM AUDIT: docs/baselines/raw/linux/probe-work/pls/asx_sample.asx
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 1 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 1 / 0 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A 7-line ASX 3.0 playlist probe fixture (XML, well-formed, one entry
with a title + absolute ref href). No claims embedded; its only defect
is the out-of-repo href target.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | well-formed ASX 3.0 sample with one entry/ref | whole file | REAL | parses as XML (structure: asx/title/entry/ref+title); version="3.0" attribute present |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None — fixture, not evidence.

Findings: F-001 [S4] href targets /home/z/my-project/tmp/probe/media/
wav_pcm_py.wav — outside the repo; fixture is not self-contained.
Commands: read/file inspection (exit 0).
