# AUDIT: scripts/quality/report.py
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Lines / functions | 305 / 7 (read, count_lines, astgrep_counts, cargo_metadata, crate_loc, main, plus module consts) |
| File verdict | SHALLOW (runs and produces output, but two of its counting paths are wrong by construction; one output section is dead code) |
| Reachable from shipped entry point? | YES from CI: .github/workflows/audit.yml:446 (`99_run_all.sh report`) and the static/quality tiers (99_run_all.sh:95). Not part of lumen binaries (CI tooling). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | sandboxed copy run under audit/scratch (exit 0, outputs produced) with synthetic reports — empirical proof of F-001/F-002/F-003; python3 re.findall experiments; rg on generators (13_test_hygiene.sh, test_hygiene.py) |

## 1. One-paragraph verdict
The aggregator executes (I ran a sandboxed copy with synthetic evidence files:
exit 0, BASELINE.md + FINDINGS.md produced, test-hygiene counts rendered
correctly from the `counts` dict) but two counting paths are provably wrong and
one branch is provably dead. (1) `count_lines` with pattern `r"^"` inflates
every `^`-based metric by one phantom line and reports an EMPTY evidence file
as 1 — my 3-line typos.txt printed `typos: 4`, my 2-line machete.txt printed
`3`, my 0-byte orphans.txt printed `1`. (2) The FINDINGS auto-counts reader
looks for top-level keys `assertion_free_suspect` / `ignore_without_reason` /
`should_panic_without_expected`, but the real generator (test_hygiene.py:155)
emits `{"counts": ..., "findings": [...]}` — those keys never exist, so the
FINDINGS table renders with zero test-hygiene rows, confirmed by execution.
(3) When cargo is absent, the Workspace/LOC sections silently vanish with no
`n/a` marker, contradicting the file's own header rule. The rest of the
aggregation (astgrep json counts, mutation-score arithmetic, jscpd stats)
reads the documented formats and the mutation-score denominator excludes
unviable as it claims.

## 2. What actually works (verified)
- Sandboxed run (copy at audit/scratch/W17-A08/quality/report.py, synthetic reports): `python3 report.py` -> exit 0, `wrote .../BASELINE.md`, `updated .../FINDINGS.md (auto counts)`.
- BASELINE test-quality section rendered from `counts`: `| assertion-free (candidate fake) | 7 |`, `| ignored tests | 51 |` — correct values from my synthetic json (assertion_free=7, ignore=51): the BASELINE reader keys match the generator.
- clippy `n/a` behavior when evidence missing: `| clippy L0 warnings | n/a |` — the "missing evidence is n/a" rule works for the static-analysis table.
- astgrep-summary branch, mutation `score = caught/(caught+missed+timeout)` (unviable excluded, as documented at :196-198), jscpd statistics block — all read the documented key paths; not exercised (no synthetic inputs) — REAL-UNPROVEN individually.

## 3. Findings
F-001 [S3]
Where: scripts/quality/report.py:225-235 with count_lines:27-31
Code:
```
    for label, path, pat in (
        ("typos", "typos.txt", r"^"),
```
Code (count_lines):
```
    return len(re.findall(pattern, txt, re.M))
```
What is wrong: pattern `r"^"` matches at every line start INCLUDING the
position after a trailing newline and in an empty string (verified:
`re.findall(r"^", "a\nb\n", re.M)` -> 3 for 2 lines; `""` -> 1). Empirically:
3-line typos.txt -> `typos: 4`; 2-line machete.txt -> `3`; **0-byte
orphans.txt -> `orphan modules: 1`**. Every run of every `^` metric (typos,
machete, orphans) is inflated by 1, and a zero-finding run reports 1 — the
header's own contract ("`n/a` means the tool did not run - never that it found
zero", line 95-96) is violated by its own counting. Why it matters: BASELINE.md
is the repo's "measured state" document; systematic off-by-one metrics and
false 1s on clean runs are wrong numbers presented as evidence. What correct
looks like: `txt.count("\n") + (0 if txt.endswith("\n") or not txt else 1)` or
`len([l for l in txt.splitlines() if l.strip()])`. Confidence: HIGH —
Verification: ran (sandboxed) + python3 experiment.

F-002 [S3]
Where: scripts/quality/report.py:278-286
Code:
```
            tj = json.loads(th_json)
            for k in ("assertion_free_suspect", "ignore_without_reason", "should_panic_without_expected"):
                if k in tj:
                    lines.append("| tests: %s | %d | `reports/test-hygiene.json` |" % (k, len(tj[k])))
```
What is wrong: the real test-hygiene.json shape is
`out = {"counts": dict(counts), "findings": rows}` (scripts/quality/test_hygiene.py:155
— the committed generator). None of `assertion_free_suspect` /
`ignore_without_reason` / `should_panic_without_expected` exist at top level,
so `if k in tj` never fires — dead code confirmed by execution (FINDINGS.md
rendered with an empty table despite counts assertion_free=7,
ignore_no_reason=3, should_panic_no_expected=4 in the input). Also `len(tj[k])`
implies list values, but the actual per-item lists live under `findings` keyed
by `category` (test_hygiene.py:146 uses category "assertion_free", "ignore_without_reason"...).
Why it matters: FINDINGS.md — the document meant to surface fake-test
suspects — silently never carries them; the tool's second output is hollow for
this entire data source. What correct looks like: derive from
`tj["counts"]["assertion_free"]` or group `tj["findings"]` by category.
Confidence: HIGH — Verification: ran + read generator.

F-003 [S4]
Where: scripts/quality/report.py:99-126
Code:
```
    if meta:
        pkgs = meta["packages"]
```
What is wrong: when cargo_metadata() returns None (cargo absent — my sandbox
proved this path), the `## Workspace` and `## Size and test density` sections
are omitted entirely with no `n/a` marker, violating the header's own rule
("Missing evidence is reported as NOT RUN, never guessed" / "`n/a` means the
tool did not run"). The BASELINE consumer cannot distinguish "cargo did not
run" from "no crates". Confidence: HIGH — Verification: ran (sections absent
in sandbox output).

F-004 [S4]
Where: scripts/quality/report.py:293-297
Code:
```
    if marker in body:
        body = body.split(marker)[0] + marker + "\n" + "\n".join(lines) + "\n"
```
What is wrong: everything after `<!-- AUTO-COUNTS -->` is discarded on every
run; any hand-written content placed after the marker is silently destroyed.
The append-else-branch preserves prior content, so the intent is
append-only — but the rewrite path deletes post-marker text. What correct looks
like: append after marker, or document that the marker must be last.
Confidence: HIGH — Verification: read.

F-005 [S4]
Where: scripts/quality/report.py:79
Code:
```
                tests += len(re.findall(r"#\[(?:tokio::)?test\]", src))
```
What is wrong: attribute forms with arguments (`#[tokio::test(flavor = "multi_thread")]`, `#[test_case(...)]`, `#[rstest]`) are not counted — LOC/test-density table undercounts tests for parameterized/async suites. Confidence: HIGH — Verification: read (regex).

F-006 [S4]
Where: scripts/quality/report.py:44-47
Code:
```
            hits = json.load(open(os.path.join(d, fn), encoding="utf-8"))
            rows[rid] = len(hits) if isinstance(hits, list) else 0
```
What is wrong: a non-list JSON payload (malformed evidence) is recorded as 0
— indistinguishable from "ran clean", against the file's own n/a-vs-zero
doctrine; `except Exception: rows[rid] = None` handles parse errors correctly,
but shape errors should also be None. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| read | 20-24 | REAL | — | utf-8-sig + errors=replace — tolerant read; missing -> None |
| count_lines | 27-31 | REAL-FLAWED | S3 | F-001 zero-width `^` pathology; correct for literal patterns |
| astgrep_counts | 34-48 | REAL | S4 | F-006 dict->0 |
| cargo_metadata | 51-58 | REAL | — | --no-deps --locked; failure -> None (used by F-003 path) |
| crate_loc | 61-82 | REAL-FLAWED | S4 | F-005 attr undercount; `src.count("\n")+1` line approx; walk excludes target/node_modules/.git |
| main | 85-301 | REAL-FLAWED | S3 | F-002 dead FINDINGS branch; F-003 silent omission; F-004 marker rewrite; everything else (static-analysis table, mutation table, jscpd, NOT RUN) works |

## 5. Test honesty
No tests anywhere for this script (no unit tests, no CI self-test — 99_run_all.sh
just runs it and tails output; audit.yml:446 runs `report` tier which invokes it
and never asserts its exit code beyond bash defaults). Mutation reasoning:
deleting the `if k in tj` branch would change nothing observable (already dead);
deleting the typos loop would remove a bullet — nothing fails. The script's own
output is the only check, and nobody diffs it. UNVERIFIED: whether audit.yml
gates on BASELINE/FINDINGS content (no assertion found in audit.yml).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every number written here is derived from a file that exists in docs/audit/reports/" | docstring:4-5 | FALSE (numbers derived are also WRONG by +1; empty file -> 1) | sandboxed run outputs |
| "Missing evidence is reported as NOT RUN, never guessed" | docstring:5-6 | SHALLOW | n/a works in the static table; missing cargo makes whole sections vanish silently (F-003); empty evidence file guessed as 1 (F-001) |
| "`n/a` means the tool did not run - never that it found zero" | BASELINE header :95-96 | SHALLOW | same: empty typos.txt -> "1" |
| "BASELINE : docs/audit/BASELINE.md / FINDINGS : docs/audit/FINDINGS.md" | 99_run_all.sh:97-98 | REAL | write paths :266,289-298 (writes verified in sandbox) |

## 7. Dead / duplicated / unreachable code
- Lines 282-284 (the three-key FINDINGS loop): dead against the real generator
  schema — never fires. Proven by execution.
- `summ` astgrep-summary.json branch (144-149): depends on
  astgrep-summary.json being written by 03_astgrep.sh/06; unverified whether
  any producer writes `counts.production` — UNVERIFIED (out of my file list;
  note for the scripts/quality/* owner).

## 8. Dependencies & flags
stdlib only (json/os/re/subprocess/sys/collections/datetime). Runs `cargo
metadata --no-deps --locked` (cheap, read-only). No CLI args; fixed paths
ROOT/docs/audit. Writes exactly two repo files (BASELINE.md, FINDINGS.md) —
this is why I executed a COPY under audit/scratch, not the original.

## 9. Open questions for the next agent
- Does any doc cite BASELINE.md numbers (LOC totals, test counts) as evidence
  for claims? If yes, F-001's +1 and F-005's undercount propagate into those
  claims — check consumers of docs/audit/BASELINE.md.
- scripts/quality/test_hygiene.py (not in my list) shares the category names
  ("ignore_without_reason" etc.) that report.py's dead branch wanted — the two
  files disagree on schema; whoever fixes one should fix both.
