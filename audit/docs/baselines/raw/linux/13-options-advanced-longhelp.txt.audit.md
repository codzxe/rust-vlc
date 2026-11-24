# CLAIM AUDIT: docs/baselines/raw/linux/13-options-advanced-longhelp.txt (5 parts, L1-3364)
| Field | Value |
|---|---|
| Auditor | W19-A08 |
| Claims extracted | 6 (provenance header claims + manifest cross-reference) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 6 / 0 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Genuine raw evidence capture, not a claims doc. Pinned-VLC 3.0.23 `--advanced --longhelp`
dump with full provenance (binary path, dump command, date, exit 0). Line count and parsed
record counts reproduce the inventory manifest's claims exactly. Chunk ranges L1-795,
796-1592, 1593-2386, 2387-3186, 3187-3364 all fall inside the real 3364-line file.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "VLC 3.0.23 pin — raw dump 13: options tree with advanced options" | L1 | REAL | L1-10 header: pinned binary path + `vlc -I dummy --advanced --longhelp`; content is VLC options text (L795-800 Kate section, L3186-3190 key-viewpoint) |
| 2 | "3364 lines incl. preamble" (manifest.json claim) | inventory/raw/options/manifest.json | REAL | `wc -l` -> 3364 |
| 3 | "1476 option records, 187 module sections" (manifest) | manifest.json | REAL | ran parser summary: `d13: 1476 ... module sections: 124 187 187` (matches); MODULE_RE count on this file -> 187 |
| 4 | "exit=0" footer | L3363 | REAL | sed -n '3360,3364p': "# exit=0" present |
| 5 | "4 modules were not displayed because they only have advanced options" | L3360 | REAL-UNPROVEN | verbatim in file; upstream text, cannot re-run VLC (sandbox) — consistent with the '-H' note on adjacent lines |

## 3-4. Contradictions / fabrication indicators
None found. The file's own [VERIFY] markers are honest upstream quirks, not repo claims.
## 5. Matrix rows
N/A (evidence file; cited as baseline by docs/inventory/options-summary.md and
options-linux.json — options matrix rows are other agents' scope).
Commands: wc -l; sed -n spot checks; python3 parse-options.py summary run; rg counts.
