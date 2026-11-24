# CLAIM AUDIT: docs/inventory/options-macos.json
| Field | Value |
|---|---|
| Auditor | W18-A07 |
| Date | 2026-11-24 |
| Claims extracted | 9 (self-describing honesty claims + 4 expected-difference claims + cross-file/live citations) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | REAL 7 / UNVERIFIED 2 (auhal/qtcapture page *contents*, qt-family presence hypothesis) / STUB-BUT-HONEST 1 (the inventory itself: options=[] by design) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This file is the anti-fabrication pattern done right: the macOS options inventory is
EMPTY (`"options": []`, `"totals": null`, `"captured": null`), it says so in a shouting
warning ("NO OPTIONS, COUNTS, OR DEFAULTS IN THIS FILE ARE MEASURED … Nothing here may be
cited as observed behavior"), and every citation it does make survives live re-verification:
all three videolan wiki URLs return HTTP 200 today, the live Preferences page really
contains the "Location of preferences" section and the
`Library/Preferences/org.videolan.vlc/vlcrc` path, and the Linux figures it quotes
(1476 options under 187 module sections, 442 core-program) reconcile exactly against
options-linux.json. The consequence for the matrix is brutal but honest: the macOS
platform column of feature-matrix.toml (which says `implemented` on macos for every
container and subtitle row in my chunks) rests on an inventory base of zero measured
macOS facts — macos statuses are unproven by construction, and this file is the receipt.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "NO OPTIONS, COUNTS, OR DEFAULTS IN THIS FILE ARE MEASURED. The options array is EMPTY by design" | options-macos.json:13 | TRUE | `python3 json.load` → `options: 0, totals: None, status: blocked-on-hardware`. No fabricated data present — verified by parse. |
| 2 | "status": "blocked-on-hardware" + runbook reference "docs/baselines/hardware-template.md §4 (help-flag probes, --longhelp / --advanced --longhelp / --full-help dumps, --reset-config vlcrc walk…)" | :8-10 | REAL (with one drift) | `hardware-template.md:108` → "## 4. macOS reference-Mac run (closes macOS [VERIFY-blocked-on-hardware])" exists; §4.1 step 5 covers module+options dumps (`--full-help` etc.) — but the `--longhelp`/`--advanced --longhelp`/`--reset-config vlcrc-walk` specifics named in blocked_reason are not verbatim in §4.1's table: citation drift, minor. |
| 3 | "Dump filenames once captured: 12..16 numbering pattern of docs/baselines/raw/macos/" | :11 | REAL | `ls docs/baselines/raw/macos/` → `00…04` exist today (the macOS capture set so far); the Linux sibling `raw/linux/` has exactly `12-options-longhelp.txt … 16-options-vlcrc.txt` — the numbering pattern cited is real. |
| 4 | "expected path ~/Library/Preferences/org.videolan.vlc/vlcrc" + source "wiki.videolan.org/Preferences/ (HTTP 200 … section 'Location of preferences')" | :35-40 | REAL (live) | This session: `curl -sL --compressed …/Preferences/` → HTTP 200; page contains `Location of preferences` and `Library/Preferences/org.videolan.vlc/vlcrc` (rg on /tmp/prefs.html). |
| 5 | "module pages verified live 2026-10-19: …/auhal/, /qtcapture/ (HTTP 200 each)" | :50-51 | REAL (live) | This session: both URLs → HTTP 200 (curl ×2). Page *contents* (module option families) not re-verified: UNVERIFIED beyond liveness. |
| 6 | "(observed on Linux: 1476/1476 records live under 187 module sections, only 442 under 'core program')" | :48 | REAL (cross-checked) | options-linux.json: `totals.options_parsed_full_help: 1476`; `:72-73 module_sections_in_advanced_longhelp: 187, module_sections_in_full_help: 187`; my python3 count: core-program options = 442. All three figures exact. |
| 7 | "schema": "lumen-phase0/options-inventory/1.0" | :2 | REAL | Identical schema string in options-linux.json (`:schema` read) — the family is coherent. |
| 8 | "source_baseline: … docs/baselines/vlc-3.0.23.md platforms.macos" (pin artifacts + hashes) | :4-5 | REAL | vlc-3.0.23.md:24 → "All 3 dmgs downloaded … MATCH 9/9 — SHA-256+SHA-1+MD5 … GPG `.asc`: Good signature ×3 … blocked-on-hardware". The pin record exists with hashes. |
| 9 | "A default macOS install includes the Qt interface … qt module option family is expected present … [VERIFY count on hardware]" (expected difference #3) | :58-61 | UNVERIFIED (hypothesis) | Explicitly marked a hypothesis needing the hardware dump; internally labeled "Hypothesis from the Linux observation" for #4 as well — honest. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| (none internally) | — | — |
| Doc: macos options inventory = zero measured facts (blocked-on-hardware) | feature-matrix.toml rows 001-013/057-065: `macos = "implemented"` on 20 rows | Both true simultaneously, and that IS the contradiction at the program level: the matrix asserts platform statuses for a platform whose inventory base is a declared null. `implemented` is not `verified` (no evidence required), but the parity story advertises macos columns whose ground truth is unmeasurable in-sandbox and unmeasured on hardware to date. |

## 4. Fabrication / inflation indicators
None. The file's whole design is anti-fabrication: empty arrays, null totals, explicit
warning, [VERIFY-blocked-on-hardware] markers on every forward-looking claim. The two
expected-difference entries that could have tempted invention (Qt family, module-scoped
macOS-only families) are written as hypotheses with capture instructions instead of numbers.
One cosmetic defect: blocked_reason's runbook description slightly outruns the runbook's
own step table (see claim #2).

## 5. Reachability (repo side)
Not applicable — pure data doc; feeds matrix R8 (inventory completeness) and doc 02 §3.3.
The macos evidence pipeline it documents terminates at hardware that has never been
available (all baseline macOS runs are `blocked-on-hardware`), so every macos cell in the
matrix is REAL-UNPROVEN at best — this file is the proof of that limitation, not a defect.

Commands run (this agent): python3 json.load (parse + field dump + option/module counts);
ls docs/baselines, docs/baselines/raw/{linux,macos}, docs/inventory; rg over
options-linux.json / options-summary.md / hardware-template.md / vlc-3.0.23.md;
curl ×4 (Preferences/ + auhal/ + qtcapture/ + compressed re-fetch) with rg path/section
verification on the fetched page.
