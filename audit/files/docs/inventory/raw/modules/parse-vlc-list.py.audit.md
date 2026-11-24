# AUDIT: docs/inventory/raw/modules/parse-vlc-list.py
| Field | Value |
|---|---|
| Auditor | W18-A04 |
| Date | 2026-11-24 |
| Lines / functions | 171 / 4 code units (module-level parse loop, bucket, mod_json, doc assembly) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — docs provenance tool; consumed by modules-linux.json -> modules-summary.md -> matrix (citation chain only, no build involvement) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `python3 docs/inventory/raw/modules/parse-vlc-list.py docs/baselines/raw/linux/06-vlc-list-verbose.txt docs/baselines/raw/linux/05-vlc-list.txt audit/runs/W18-A04.modules-linux.json` (exit 0); `diff` regenerated vs committed (exit 0, IDENTICAL); python3 stats checks (exit 0); sed quote verify (exit 0) |

## 1. One-paragraph verdict
REAL and verified end-to-end: the script takes its output path as a CLI
argument (no hard-coded repo write, unlike parse-options.py), so I ran it
untouched and read-only — it regenerated docs/inventory/modules-linux.json
**byte-identical** (diff exit 0) from dumps 05/06. Its census checks are not
prose: three real assertions enforce terse-vs-verbose 1:1 agreement
(count + name + description, in order), and the rollup sums to 502 exactly.
One factual defect: the hard-coded counting_rule prose says "lua: 8" where
the same JSON's data shows lua = 11 registrations (8 is lua's
distinct-capability count) — mixed semantics in one example sentence, and
modules-summary.md:33 states the correct 11.

## 2. What actually works (verified)
— Ran the script as-is with out path under audit/ -> exit 0; stdout:
`total 502 | rollup: {'demuxers': 57, 'decoders': 35, 'access': 57, 'filters': 110, 'subtitle': 32, 'playlist': 21, 'services-discovery': 26, 'other': 164} | sum 502`.
— `diff audit/runs/W18-A04.modules-linux.json docs/inventory/modules-linux.json`
-> exit 0, byte-identical: the committed census is regenerable from the raw
dumps by this exact script (provenance proven).
— python3 on regenerated JSON: sum(by_capability counts) = 502; capability
"(none)" = 4 entries named export/bridge/playlist/core (docstring claim
verified); avcodec 5 / stats 6 registrations (prose claim verified);
lua = 11 registrations (prose claim of 8 CONTRADICTED — see F-001).

## 3. Findings
F-001 [S3]
Where: docs/inventory/raw/modules/parse-vlc-list.py:143-146 (propagated into
docs/inventory/modules-linux.json counting_rule field)
Code:
```
    ('counting_rule', 'One entry per module-capability registration printed by --list-verbose '
        '(the 502 header lines; every header carries at most one capability line). A single plugin '
        'binary can register several entries in different capabilities (avcodec: 5, lua: 8, stats: 6). '
```
What is wrong: "lua: 8" is false as an entry count — the data in the SAME
JSON shows lua = 11 registrations; 8 is lua's distinct-capability count
(verified: 11 regs across 8 capabilities). Two of the three example numbers
are registration counts, one is a capability count — mixed semantics in one
sentence. modules-summary.md:33 has the correct list: "avcodec: 5, lua: 11,
stats: 6, ddummy: 6, playlist: 15". Why it matters: a reader auditing
counting integrity from the JSON's own rule text computes the wrong number
(8 vs 11) and may conclude the census is inconsistent. What correct looks
like: "(avcodec: 5, lua: 11 across 8 capabilities, stats: 6)".
Confidence: HIGH — Verification: ran + counted.

F-002 [S4]
Where: docs/inventory/raw/modules/parse-vlc-list.py:16
Code:
```
verbose, terse, out = sys.argv[1], sys.argv[2], sys.argv[3]
```
What is wrong: no argv validation; wrong invocation -> IndexError traceback,
not a usage message. Scratch-tool-grade UX in a committed provenance script.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: docs/inventory/raw/modules/parse-vlc-list.py:24-25
Code:
```
    if line.startswith('   s '):
        cur['shortcuts'].append(line[4:].strip())
```
What is wrong: an 's' line before any module header (cur is None) raises
TypeError, not a located error like the other branches. Pinned input never
does this (ran clean). Contrast: the `else: raise SystemExit(f'line {ln}:
unexpected shape {line!r}')` guard at :37-38 is strict and locates errors
properly — that part is good. Confidence: HIGH — Verification: read + ran.

F-004 [S4]
Where: docs/inventory/raw/modules/parse-vlc-list.py:114-116
Code:
```
for b in ('demuxers', 'decoders', 'access', 'filters', 'subtitle',
          'playlist', 'services-discovery', 'other'):
    seven[b] = OrderedDict([('count', len(rollup[b])), ('capabilities', cap_of(b))])
```
What is wrong: "seven_category_rollup" iterates EIGHT buckets (7 + other);
the name is a lie of one. Consistent with summary's "Seven-bucket total 338
+ other 164 = 502" (57+35+57+110+32+21+26=338 ✓), but the JSON field name
says seven while holding eight. Confidence: HIGH — Verification: ran.

## 4. Per-function table
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module-level parse loop + terse cross-check | 16-55 | REAL | — | ran; strict shape guard (:37-38 SystemExit); asserts :52-55 enforce 1:1 terse/verbose (name+description) — real enforcement, exit 0 |
| bucket() | 58-85 | REAL | — | deterministic rollup rules; ran; 8 buckets sum 502 |
| mod_json() | 95-102 | REAL | — | emits name/shortcuts/description/score/dump_line; dump_line claim in counting_rule verified spot-wise |
| doc assembly + stats | 87-93, 113-171 | REAL | S3 carries F-001 prose | byte-identical output reproduced |

## 5. Test honesty
No tests. Verification = regeneration diff (stronger): script + dumps 05/06 ->
committed JSON byte-identical, and the script's internal asserts all passed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Run: python3 parse-vlc-list.py <verbose-dump> <terse-dump> <out.json>" | docstring:11 | REAL | ran exactly this shape, exit 0 |
| "Four base/parent entries … (export, bridge, playlist, core) … capability "(none)"" | docstring:9-10 | REAL | regenerated JSON: (none) count=4, names match exactly |
| "Sum(by_capability.count) == total == 502, cross-checked 1:1 … against the 502 module lines of dump 05" | counting_rule:147-149 | REAL | re-ran: sum=502; asserts passed; dump 05 = 511 lines = 502 modules + 9 comment lines |
| "this census is regenerable with it" (parser field) | modules-linux.json parser field | REAL | byte-identical regeneration performed |
| "lua: 8" registrations example | counting_rule:145 | FALSE as written | lua = 11 registrations (8 = distinct capabilities); modules-summary.md:33 says 11 |

## 7. Dead / duplicated / unreachable code
No dead code found (unlike parse-options.py). The whole file is unreachable
from the shipped build by design (provenance tooling).

## 8. Dependencies & flags
stdlib only (re, sys, json, collections). No feature flags, no network, no
repo writes — output path is a CLI argument (the design parse-options.py
lacks).

## 9. Open questions for the next agent
- Who consumes the "lua: 8" number downstream? modules-summary.md:33 already
  corrects it to 11 — check whether any other doc copied the wrong 8.
- The 12-item caps in stats (top_reg[:12], top_sc[:12]) are display
  truncations — fine, but nothing asserts the JSON's stats blocks stay in sync
  with by_capability.
