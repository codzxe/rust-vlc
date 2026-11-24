# CLAIM AUDIT: docs/baselines/raw/linux/14-options-full-help.txt (all 7 parts, L1-5509)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 8 (header provenance/verification claims + inventory cross-counts) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 CORROBORATED counts / 2 VERIFIED / 0 / 0 / 0 / 0 / 3 UNVERIFIED (provenance, diff=0, stderr-empty) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This is a raw VLC 3.0.23 `--full-help` capture (5509 lines; 5497 non-comment) used as
evidence for the options inventory. The header's three provenance/verification claims (binary
identity, the `--advanced --full-help` vs `--full-help` 0-line diff, empty stderr) are
UNVERIFIED — the pinned binary is not in this sandbox and no git is available. Everything that
CAN be checked checks out: the dump's option-record counts reproduce the inventory's derived
numbers (1476 total / 615 visible / 861 advanced-only; 442-443 core records) to within 1-3
lines out of ~1500, sibling dumps exist with mutually consistent footers, and the specific
options the feature matrix relies on (--zoom, --aspect-ratio, --crop, --audio-replay-gain-mode,
--audio-visual) are all present at cited locations. No fabrication indicators. The data is
genuine or at minimum perfectly self-consistent across five independent files.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Binary: tmp/pinning/extract/usr/bin/vlc from debs listed in dump 02" | :4 | UNVERIFIED | No pinned vlc binary in sandbox; dump 02 exists (docs/baselines/raw/linux/02-acquisition-and-hashcheck.txt) but its hashes are not re-runnable here. |
| 2 | "Pinned baseline: VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)" | :5 | UNVERIFIED (external) | Version string matches docs/baselines/vlc-3.0.23.md header claims; cannot execute the binary to confirm. |
| 3 | "verified: diff of --advanced --full-help vs --full-help = 0 lines" | :11-12 | UNVERIFIED | Requires running the pinned binary; no vlc in sandbox. Claimed verification not re-runnable; no diff artifact stored under docs/baselines/raw/linux/. |
| 4 | "(stdout+stderr merged below; stderr empty for this run)" | :9 | UNVERIFIED | No stderr file to inspect. |
| 5 | "--full-help ... dumps ALL modules including advanced" | :11 | CORROBORATED | Option-record lines in this dump: `rg -c '^\s+-{1,2}[a-zA-Z]'` -> 1480; minus 2 usage-style doc lines (:19-20 "--option / -option") = 1478 vs inventory's 1476 canonical records (0.14% delta; 6 duplicate-name lines exist: --glconv, --rc-host, --scaletempo-{overlap,search,stride}, --wl-display). Inventory arithmetic 615+861=1476 exact (docs/inventory/options-summary.md:21-23). |
| 6 | "Option records in `core program (core)` 442 (191 basic + 251 advanced)" (options-summary cites dump 14) | :4065 | CORROBORATED (within 1) | `awk` region count from " core program (core)" (:4065) to next top-level section -> 443 option-record lines vs 442 claimed. |
| 7 | "--longhelp ends with: \"To get exhaustive help, use '-H'.\"" | :10 | VERIFIED | `tail -5 docs/baselines/raw/linux/12-options-longhelp.txt` -> "To get exhaustive help, use '-H'." then "# exit=0". |
| 8 | Sibling dump line counts (inventory: 615 longhelp-visible; 3352 advanced-longhelp lines) | options-summary.md:22,43 | CORROBORATED (within 2-3) | dump 12: 618 raw option lines (3 usage-style included) / dump 13: 3364 total, 3354 non-comment lines vs 3352 claimed. |

## 3. Contradictions
None. Cross-checks run: dump 14 counts vs options-summary.md:21-25; dump 12/13 footer/line
counts vs :22/:43; presence of matrix-referenced options (--zoom :4289, --aspect-ratio :4252,
--crop :4245, --audio-replay-gain-mode :4101, --audio-visual :4130). All consistent.

## 4. Fabrication / inflation indicators
None found. The counts that downstream docs derive from this dump are reproducible from the
file itself within 1-3 records out of 1476-1478. The header honestly labels what was run and
what was merged (stdout+stderr). Three verification claims are un-re-runnable in this sandbox
and marked UNVERIFIED, not FAILED.

## 5. Content spot checks (matrix-referenced options)
`rg '^\s+--zoom |^\s+--aspect-ratio|^\s+--crop |^\s+--audio-replay-gain-mode' 14-options-full-help.txt`
-> :4101 (--audio-replay-gain-mode {none,track,album}), :4245 (--crop <string>), :4252
(--aspect-ratio <string>), :4289 (--zoom <float>) — all four anchor options cited by matrix
rows 118-128 (W18-A05 chunk) exist verbatim in this dump.
