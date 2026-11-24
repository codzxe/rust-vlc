# AUDIT: docs/inventory/raw/options/parse-options.py
| Field | Value |
|---|---|
| Auditor | W18-A04 |
| Date | 2026-11-24 |
| Lines / functions | 484 / 4 (parse_dump, parse_vlcrc, main, nested close) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — docs provenance tool, not in build (no scripts/CI references; consumed only by docs/inventory/*.md + feature-matrix.toml) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 5 |
| Commands run | sed (quote verify, exit 0); patched-copy run `python3 audit/runs/W18-A04.parse-options-verify.py` (exit 0); `diff` regenerated vs committed JSON (exit 0, IDENTICAL); awk independent counts (exit 0) |

## 1. One-paragraph verdict
This script is REAL and its provenance chain holds: I ran a byte-equivalent copy
(only the output path and repo-root bootstrap redirected; everything else
untouched) and it regenerated docs/inventory/options-linux.json **byte-identical**
(diff exit 0) from the four raw dumps in docs/baselines/raw/linux/. Every
headline number the script and its manifest claim (615/1476/1476 options,
124/187/187 module sections, leftovers 3/2/1, vlcrc 1439/1433/6, default
agreement 252/252) reproduced exactly, and independent awk recounts of the raw
dumps agree. The defects found are hygiene: one dead compiled regex, dead
append-then-filter logic, inconsistent field semantics, unguarded marker split,
and hard-coded prose stats that can silently rot.

## 2. What actually works (verified)
— `python3 audit/runs/W18-A04.parse-options-verify.py` -> exit 0; stdout:
`d12: 615 d13: 1476 d14: 1476`, `module sections: 124 187 187`, `leftovers: 3 2 1`,
`vlcrc records: 1433`, `collisions: 6`.
— `diff audit/runs/W18-A04.options-linux.json docs/inventory/options-linux.json`
-> exit 0, no output: the committed inventory is exactly regenerable from the
raw dumps by this script (the core provenance claim).
— awk recount of raw dumps: dump12 total=615, dump13 total=1476, dump14
total=1476 (matches "parsed == raw option-line counts" claim); vlcrc walk:
1439 name=value lines, 1433 unique, 6 dups (same 6 names as help-dump collisions).
— NOTE: the script itself MUTATES the repo (writes
docs/inventory/options-linux.json), so per instructions I did NOT run it as-is;
I ran the two-line patched copy under audit/runs/ writing only under audit/.

## 3. Findings
F-001 [S3]
Where: docs/inventory/raw/options/parse-options.py:26-31
Code:
```
FLAGS_RE = re.compile(
    r"^(--(?P<name>[a-z0-9_-]+))"                        # primary long flag
    r"(?:,\s*(?P<neg>--no-(?P<negname>[a-z0-9_-]+)))?"   # optional --no- form
    r"(?:\s+(?P<arg>\{.*\}|<[^>]*>))?"                   # optional {choices} or <type>
    r"\s*$"
)
```
What is wrong: FLAGS_RE is defined and never referenced anywhere else in the
file (rg "FLAGS_RE" -> only line 26). The real flag grammar is the separate
inline regexes at lines 104/129/131. Dead code whose drift (charset `[a-z0-9_-]`
vs the actually-used `[a-zA-Z0-9_+-]`) already proves it is not maintained.
Why it matters: a reader auditing "the grammar" may audit the dead regex.
What correct looks like: delete it. Confidence: HIGH — Verification: read + rg.

F-002 [S4]
Where: docs/inventory/raw/options/parse-options.py:78-84 (and 51)
Code:
```
        if line.strip() == "":
            # whitespace-only line: a column-33 help slot with empty text
            if cur is not None:
                cur["_help"].append("")
                continue
            prev_blank = False
            continue
```
What is wrong: the appended "" is always discarded by
`" ".join(x for x in cur["_help"] if x)` at line 51, so the append is dead; the
branch's only real effect is "a whitespace-only line does not close the current
option" (33-space chromecast slots — 6 such lines exist in dumps 13/14, awk
count). The comment claims a mechanism (empty help slot) that has zero output
effect. Why it matters: misleading comment on subtle logic. Confidence: MEDIUM —
Verification: read + ran. Severity LOW because behavior is correct.

F-003 [S4]
Where: docs/inventory/raw/options/parse-options.py:157-159
Code:
```
            cur["option_name"] = name
            cur["short_form"] = short
            cur["negation_form"] = neg
```
What is wrong: short_form is stored bare ("f", no dash) while negation_form is
stored with leading dashes ("--no-x"). Consumers must know two conventions.
Confidence: HIGH — Verification: read + regenerated JSON inspected.

F-004 [S4]
Where: docs/inventory/raw/options/parse-options.py:411-412
Code:
```
            "6 flat-namespace collisions; the SAME 6 names also appear on multiple lines in "
            "the vlcrc walk (cross-extraction agreement between --full-help and vlcrc)"),
```
What is wrong: hard-coded count "6" in prose. Verified TRUE today (collisions =
6; vlcrc dups = same 6 names) but nothing in the script checks the prose against
the computed `collisions` — if the dump ever changes, the JSON silently carries
a false annotation. Confidence: HIGH — Verification: ran.

F-005 [S4]
Where: docs/inventory/raw/options/parse-options.py:232
Code:
```
    body = text.split(marker, 1)[1]
```
What is wrong: unguarded IndexError if the vlcrc marker line is absent from
dump 16. Pinned input contains both markers (verified), so unreachable today;
robustness debt only. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: docs/inventory/raw/options/parse-options.py:458
Code:
```
    with open("docs/inventory/options-linux.json", "w", encoding="utf-8") as f:
```
What is wrong: every run mutates a committed repo file with no --check/dry-run
mode, so nobody can re-verify provenance without dirtying the tree (I had to
patch a copy). Deterministic output (verified byte-identical) limits the damage
to inconvenience. Confidence: HIGH — Verification: ran (patched copy).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse_dump | 34-224 | REAL | — | State machine over help dump; ran end-to-end on 3 dumps; leftovers 3/2/1 all footer notes (verified in output) |
| close (nested) | 48-55 | REAL | — | Joins _help/_long; filters empty slots (see F-002) |
| parse_vlcrc | 227-263 | REAL | — | Ran; 1439 lines/1433 unique/6 dups match manifest; later duplicate name silently overwrites earlier record in `recs` (last-wins) — matches INI last-wins convention, acceptable |
| main | 266-481 | REAL | — | Aggregation + JSON emit; assertion at 279 enforces nesting; regenerates committed JSON byte-identically |

## 5. Test honesty
No tests in this file. Verification is the regeneration-diff (section 2), which
is stronger than any unit test here: it pins script+dumps -> committed output.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Run from anywhere: python3 docs/inventory/raw/options/parse-options.py" | docstring:3 | REAL | parents[4]+os.chdir logic; my copy ran from audit/ path context |
| "1476 option records ... from dump 14" | worklog/archive-2026-10.md:503 | REAL | run: d14=1476; awk raw count 1476 |
| "types string 519 / integer 420 / bool 267 / enum 162 / float 108" | worklog/archive-2026-10.md:503 | REAL | run stdout: identical |
| "5 enforced checks" / "checks_enforced" list | raw/options/manifest.json:54-60 | PARTLY FALSE | only 1 of 5 is enforced in code (the nesting assert, line 279); "parsed == raw option-line counts", "leftovers footer-only", vlcrc counts, default-agreement are recorded stats — I verified all 4 independently TRUE (awk + run), so the numbers are honest but "enforced" overstates the script |
| "byte-identical to --advanced --full-help (diff = 0 lines)" | script line 334-335 citing raw 15 | REAL-UNPROVEN | raw 15:428-429 records the probe claim ("-> 0 lines, byte-identical"); the diff output itself was not captured as a file and vlc cannot be re-run here |

## 7. Dead / duplicated / unreachable code
FLAGS_RE (F-001); dead `cur["_help"].append("")` (F-002); local re-import
`from collections import Counter as _C` at line 261 shadows the module-level
`Counter` import at line 16 (harmless duplication). File itself is unreachable
from the shipped build by design (provenance tooling).

## 8. Dependencies & flags
stdlib only (json, os, re, pathlib, collections). No feature flags. No network.
Reads docs/baselines/raw/linux/12..16; writes docs/inventory/options-linux.json.

## 9. Open questions for the next agent
- manifest.json's "5 enforced checks" wording (see §6) — whoever audits
  raw/options/manifest.json should demand the assert count be stated precisely.
- 37 full-help names absent from vlcrc ("fh_only=37") are annotated in the JSON
  with categories (a)/(b)/(c); category (c) remainder is flagged [VERIFY] there —
  still open on the repo's own books.
