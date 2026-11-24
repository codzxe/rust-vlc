# DATA AUDIT: docs/baselines/raw/linux/06-vlc-list-verbose.txt
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | 1,288 lines; 502 module header lines + capability/shortcut sub-lines; footer `# exit=0` (:1288) |
| File verdict | REAL |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | awk censuses (2 methods); python3 json cross-check; rg consumers |

## 1. What it is / provenance
Raw dump of `vlc --list-verbose` from the pinned VLC 3.0.23 binary (same env as dump 05,
2026-10-19 12:01 UTC). Header self-describes format: module name lines, 's' shortcut lines,
'c' capability (score) lines, "Module count: 502".

## 2. Data verification (executed)
- Module census: awk counts 502 module header lines (two independent awk filters agree:
  502 = 502) — EXACTLY the header's claimed "Module count: 502".
- Footer "# exit=0" at :1288 — run success recorded in-artifact.
- Derived product cross-check: docs/inventory/modules-linux.json meta declares
  `"total": 502` with counting rule "One entry per module-capability registration printed
  by --list-verbose (the 502 header lines)" and "Every module object carries dump_line =
  line number of its header in dump 06"; in-tree parser
  docs/inventory/raw/modules/parse-vlc-list.py is committed (census regenerable).
- Consumer tables: docs/inventory/modules-summary.md states Total module registrations
  (Linux, measured) 502, sum of per-capability counts 502 = total. All three layers
  (raw dump, json census, summary tables) agree on 502.
- Tail of dump (verbatim, sed-verified): `tdummy                 Dummy font renderer` /
  `   c text renderer (1)` / `freetype               Freetype2 font renderer` /
  `  core                   core program` — consistent with the census rule that base
  modules (core) print without capability lines.

## 3. Claim ledger
| # | Claim (verbatim) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Module count: 502" | :6 | REAL | awk census = 502 (exit 0) |
| 2 | "# exit=0" | :1288 | REAL-UNPROVEN | in-artifact footer; vendor run not repeatable offline |
| 3 | "'c' lines = capability (score)" | :3-4 | REAL | spot-checked (e.g. `c video converter (250)`, `c text renderer (100)`) |

## 4. Findings
None. Checked and how: module count (awk), format consistency (sed spot-quotes), footer
exit code (rg), derived-product agreement (python3 json meta), consumers (rg).

## 5. Consumers
docs/inventory/modules-summary.md, docs/inventory/modules-linux.json (per-entry dump_line
back-references), docs/worklog/archive-2026-10.md. Prior W19-A08 audit (old tree) concurs:
"REAL, module count 502 reproduced".
