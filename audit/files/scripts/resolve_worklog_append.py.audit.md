# AUDIT: scripts/resolve_worklog_append.py
| Field | Value |
|---|---|
| Auditor | W17-A10 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines / functions | 39 / 1 (flat script; 3 sanity checks) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — operator merge utility; zero references in .github/ or docs/; sole in-tree reference is worklog.md:19 (the keep-decision itself) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | real invocation -> FileNotFoundError (pasted below); scratch-copy single-hunk run -> exit 0, correct resolution; scratch-copy multi-hunk run -> exit 1, file left WITH conflict markers; python3 ast.parse -> OK; hex dump of :25 -> byte-exact `text[mid` |

## 1. One-paragraph verdict
The single-conflict append-append resolution logic is real and verified by execution, but the script is welded to one machine layout (hardcoded absolute path one directory ABOVE this repo root — it crashes FileNotFoundError in this checkout), and its sanity checks run AFTER the destructive write: on a multi-hunk conflict it rewrites the worklog into a half-resolved state that still contains conflict markers and only then crashes. It has no recorded use anywhere in the tree.

## 2. What actually works (verified)
— verified by running the REAL script: `python3 scripts/resolve_worklog_append.py wX` ->
```
FileNotFoundError: [Errno 2] No such file or directory: '/home/z/my-project/worklog.md'
```
(the read fails before any write — safe probe; proves the shipped path is dead in this checkout).
— verified by scratch-copy run (PATH patched to audit/scratch/W17-R2/wl_single.md), single-hunk conflict:
output `resolved: HEAD entries kept [H1] then branch entries [B1]`, exit 0; resulting file is `PRE / Task ID: A1 / Task ID: H1 / (blank) / Task ID: B1 / POST` — exactly the documented resolution (keep HEAD, append branch).
— verified by `python3 -c "import ast; ast.parse(...)"` -> OK; `bash`-level mangling check: hex dump of line 25 = `6272616e...746578745b6d6964...` = `branch_content = text[mid + len(mid_marker):end]` (display layer strips `[m` sequences; file bytes are intact).

## 3. Findings
F-001 [S3]
Where: scripts/resolve_worklog_append.py:10
Code: `PATH = "/home/z/my-project/worklog.md"`
What is wrong: hardcoded absolute path OUTSIDE the repo. In this checkout the worklog lives at /home/z/my-project/audit-workspace/worklog.md; the hardcoded target does not exist, so the script can never succeed here (empirical FileNotFoundError above). Any clone at any other location = same crash. On the original dev layout it mutates a file above the repo root. Why it matters: a shipped utility that only works on one machine's directory layout is dead code for everyone else, and it writes outside repo scope. What correct looks like: relative path from the script's own location, or a path argument.
Confidence: HIGH — Verification: ran (FileNotFoundError pasted).

F-002 [S3]
Where: scripts/resolve_worklog_append.py:28-31 vs :33-35
Code: `resolved = pre + head.rstrip("\n") + "\n\n" + branch_content.strip("\n") + "\n" + post

with open(PATH, "w", encoding="utf-8") as f:
    f.write(resolved)

# sanity checks
assert "<<<<<<<" not in resolved and ">>>>>>>" not in resolved`
What is wrong: the destructive write happens BEFORE the sanity checks, and the resolver handles exactly ONE conflict hunk (three `text.index` calls at :19-21 anchor only the first block; the remainder is spliced verbatim into `post`). Empirical multi-hunk run (scratch copy): exit 1 via the :34 assert, but the file on disk was already rewritten and still contains `<<<<<<< HEAD` / `=======` / `>>>>>>> wQ` at its lines 6/8/10. Why it matters: a realistic merge (branch edited an old worklog entry AND appended) produces multiple hunks; the script leaves the worklog in a half-resolved state with raw conflict markers committed-to-disk, then crashes. What correct looks like: validate `resolved` before opening for write, and loop over all hunks.
Confidence: HIGH — Verification: ran (multi-hunk probe output pasted in log).

F-003 [S4]
Where: scripts/resolve_worklog_append.py:34-35
Code: `assert "<<<<<<<" not in resolved and ">>>>>>>" not in resolved
assert "\n=======" not in resolved`
What is wrong: runtime sanity implemented as bare asserts — under `python -O` both vanish and the only guard against writing a bad file is gone. Not debug-only code, so assert is the wrong tool.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: scripts/resolve_worklog_append.py:9, 19-21
Code: `branch = sys.argv[1]` / `start = text.index(start_marker)`
What is wrong: no argument handling (omitted arg -> IndexError traceback; the docstring's Usage line is never printed by the script) and no marker validation (invoked on a clean file -> ValueError traceback from `index`). Crashes instead of messages for the two most likely operator mistakes.
Confidence: HIGH — Verification: read (argv[1] traceback case not separately executed — LOW-cost but skipped; the FileNotFoundError and assert paths were executed).

F-005 [S4]
Where: scripts/resolve_worklog_append.py:2-5 vs worklog.md:19
Code: `"""Generic worklog.md append-append merge-conflict resolver.` / worklog.md:19 `... Kept 4 referenced in worklog narrative: resolve_w1_session_flow.py, resolve_w2_scheduler.py, resolve_worklog_append.py, resolve_worklog_p6.py.`
What is wrong: the retention rationale is circular for this script — its ONLY "narrative reference" is the keep-decision line itself; no entry anywhere records this resolver actually being used on a real conflict. Family precedent: resolve_w1_session_flow.py was audited as "kept only as anti-pattern record" (W17-A06) with the same no-marker-validation splice defect class.
Confidence: HIGH — Verification: rg over repo (worklog.md:19 + manifest/assignments only).

## 4. Per-function table (39 loc — brief)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| arg/path setup | 9-10 | BROKEN-in-this-tree | F-001 | hardcoded out-of-repo path |
| marker parse | 15-26 | REAL (single hunk) | F-002 | first-hunk-only anchoring |
| resolve+write | 28-31 | REAL / order-defect | F-002 | write precedes checks |
| sanity+report | 33-39 | REAL | F-003 | bare asserts; Task-ID report verified by execution |

## 5. Test honesty
No tests. Execution evidence = my scratch runs (single-hunk PASS, multi-hunk corrupt-then-crash, real-path FileNotFoundError). No committed evidence of any historical run.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Generic worklog.md append-append merge-conflict resolver" | :2 | SHALLOW | single-hunk-only, single-machine path (F-001/F-002) |
| "Usage: resolve_worklog_append.py <branch-name>" | :5 | TRUE | executed with arg in scratch run |
| "Resolution: keep HEAD content in full, then append the branch (theirs) content." | :4 | REAL | single-hunk run output byte-inspected: H1 kept, B1 appended |

## 7. Dead / duplicated / unreachable code
Not wired to any CI workflow or shipped entry point; retention justified only by worklog.md:19. Dead in this checkout (F-001).

## 8. Dependencies & flags
stdlib only (sys). No flags. Writes outside repo scope when the target path exists (dev layout).

## 9. Open questions for the next agent
- Was this resolver ever run on the original dev machine (pre-sandbox layout)? No in-tree record exists (UNVERIFIED).
- Should the 4-script "keep" list from worklog.md:19 be re-reviewed for deletion now that 2 of 4 audited members carry the same defect class?
