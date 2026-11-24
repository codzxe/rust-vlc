# CLAIM AUDIT: docs/inventory/options-windows.json

| Field | Value |
|---|---|
| Auditor | W18-A08 |
| Date | 2026-11-23 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 8 REAL / 4 REAL-UNPROVEN (wiki-cited Windows expectations) / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict

This file is an honest empty shell and says so at the top of its lungs: every
Windows-specific fact is gated behind `[VERIFY-blocked-on-hardware]`, the
options array is literally empty, totals/parse_coverage/reconciliation/leftovers
are all `null`. The only measured facts it recites are Linux facts, and I
verified all three against `docs/inventory/options-linux.json` — 1476 option
records, 187 module sections, exactly 442 options under module `core`. Zero
fabrication: nothing in this file pretends to be observed Windows behavior.

## 2. Claim ledger

| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | `"status": "blocked-on-hardware"` | L7 | REAL | file read; no Windows capture claimed |
| 2 | "NO OPTIONS, COUNTS, OR DEFAULTS IN THIS FILE ARE MEASURED." | L9 | REAL | L27 `"options": []`; L23-26 `"totals": null, "parse_coverage": null, "reconciliation": null, "leftovers": null` |
| 3 | "The options array is EMPTY by design — option lists are NOT fabricated" | L9 | REAL | `python3 json.load`: `len(options) == 0` |
| 4 | "1476/1476 records live under 187 module sections" (Linux, cited in expected_differences #4) | L53 | REAL | options-linux.json: `options_parsed_full_help: 1476`; `module_sections_in_full_help: 187` |
| 5 | "only 442 under 'core program'" | L53 | REAL | options-linux.json: Counter over `module` field → `('core', 442)` |
| 6 | "vlcrc path %APPDATA%\vlc\vlcrc" (expected, wiki-cited) | L31-35 | REAL-UNPROVEN | public-doc citation; cannot verify in sandbox |
| 7 | "Windows users have to use the --option-name=\"value\" syntax" | L38-41 | REAL-UNPROVEN | public-doc citation |
| 8 | "dvd:// takes a drive letter ... (vlc dvd:///D:/)" | L45-48 | REAL-UNPROVEN | public-doc citation |
| 9 | "Windows-only module option families: dshow, mmdevice, directx_aout, waveout, direct3d/glwin32/wingdi" | L52-55 | REAL-UNPROVEN | module names cross-consistent with `modules-windows.json` expected lists (that file: status blocked-on-hardware, no counts) |
| 10 | "A default Windows install includes the Qt interface ... large qt option family expected" | L59-61 | REAL-UNPROVEN | hypothesis, explicitly flagged `[VERIFY count on hardware]` |
| 11 | "core program section ... 442 options ... expected substantially invariant" | L66-68 | REAL (the 442) / REAL-UNPROVEN (the invariance hypothesis, self-flagged) | 442 verified; hypothesis labeled "not a citation" |
| 12 | `dump_files.*` all `[VERIFY-blocked-on-hardware]` | L12-18 | REAL | no dump files exist for windows in docs/baselines/raw/ (only linux/) |

## 3. Contradictions

| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| none found | — | the file cites Linux facts that reproduce exactly; Windows claims are all marked unverified |

## 4. Fabrication / inflation indicators

None. The file's design (empty arrays, null totals, per-field
[VERIFY-blocked-on-hardware] markers, warning line) is the anti-fabrication
pattern working as intended. The Linux numbers it borrows are accurate
(1476/187/442 all re-derived by command).

## 5. Commands run (read-only)

- `python3 -c "import json; d=json.load(open('docs/inventory/options-windows.json')); print(len(d['options']), d['status'])"`
  → `0 blocked-on-hardware`
- `python3` Counter over `docs/inventory/options-linux.json` options' `module`
  field → `('core', 442)`; totals dict → `options_parsed_full_help: 1476`,
  `module_sections_in_full_help: 187`.
- `ls docs/baselines/raw/` → windows dump files absent (linux only), matching
  the doc's dump_files placeholders.
