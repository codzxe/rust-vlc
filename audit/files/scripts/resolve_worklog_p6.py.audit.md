# AUDIT: scripts/resolve_worklog_p6.py
| Field | Value |
|---|---|
| Auditor | W17-A10 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Lines / functions | 55 / 2 nested (entries, task_id) + module main flow |
| File verdict | REAL (core resolution logic executed and correct; 2 real correctness defects at conflict-style edges) |
| Reachable from shipped entry point? | NO — repo maintenance tool (worklog merge resolution), not shipped code; recorded real use in the merge trains (worklog.md:19 "Kept 4 referenced in worklog narrative"; docs/worklog/archive-2026-10.md:10493/:10654 "worklog conflicts via scripts/resolve_worklog_p6.py") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | patched sandbox copy (git-add stubbed; repo untouched) over 7 synthetic conflict cases — outputs below; rg callers; sed quote verifications |

## 1. One-paragraph verdict
The happy path is genuinely correct and I executed it: a synthetic ours/theirs
conflict resolved to exactly ours(A) → theirs(C) with the duplicated Task ID B
deduped, markers eliminated, separators reconstructed, exit 0. The no-conflict
path is a clean no-op. But the script only understands git's DEFAULT conflict
style: under merge.conflictStyle=diff3 (a stock git config) the `|||||||`
ancestor section lands inside the "ours" capture — I proved ancestor text leaks
into the resolved file AND, combined with the Task-ID dedup, the entire theirs
side is dropped (content loss, not just noise). The Task-ID dedup itself drops
content-DIVERGENT entries with the same ID (the docstring's "keep BOTH sides"
is contradicted by the code), tid-less exact duplicates are kept twice, and
the newline-collapse regex rewrites spacing everywhere in the file, not just
at join seams. For a script that rewrites the project's primary log file,
diff3 handling and content-aware dedup are not optional polish.

## 2. What actually works (verified)
All via a patched copy (subprocess git-add stubbed) run on synthetic
worklog.md files under audit/scratch (repo untouched; real script never
executed because it shells `git add` — git is prohibited in this audit):
- Case 1 (canonical conflict, ours A+B / theirs B+dup+C): exit 0; output
  "resolved 1 hunk(s); marker-free: True"; kept IDs [A, B, C] — B deduped
  (its body count = 1); ours-then-theirs order preserved.
- Case 2 (no markers): "no conflict markers found — nothing to do", exit 0,
  file unchanged.
- Case 5/6b (unusual marker suffixes incl. trailing-space): resolved
  correctly to "a\n---\nb\n".

## 3. Findings
F-001 [S3]
Where: scripts/resolve_worklog_p6.py:17
Code:
```
pattern = re.compile(r"<<<<<<< HEAD\n(.*?)=======\n(.*?)>>>>>>> [^\n]*\n", re.S)
```
What is wrong: the pattern assumes git's default 'merge' conflict style. Under merge.conflictStyle=diff3 the hunk is `<<<<<<< HEAD ... ||||||| ancestors ... ======= ... >>>>>>> ref`, so group(1) ("ours") captures the ancestor section too. Empirical (case 4, diff3 input): "ancestor line OLD" LEAKED into the resolved worklog.md, and the theirs body was dropped entirely (see F-002). Why it matters: the script's whole job is producing a clean worklog; diff3 users get ancestor garbage baked into the log with exit 0 and "marker-free: True". What correct looks like: also match and discard `\|\|\|\|\|\|\|` sections, or refuse to run when diff3 markers are present. Confidence: HIGH — Verification: ran it (synthetic diff3 hunk).
F-002 [S3]
Where: scripts/resolve_worklog_p6.py:37-45
Code:
```
    for entry in entries(ours) + entries(theirs):
        tid = task_id(entry)
        if tid:
            if tid in seen:
                continue
            seen.add(tid)
        kept.append(entry)
```
What is wrong: dedup is by Task ID ALONE — when both sides carry the same Task ID with DIFFERENT bodies (a real merge case: an agent amended its entry on a branch), theirs is silently discarded. Empirical (case 4): "theirs line" absent from output while "ours line" present, both under Task ID: A. The docstring's "append-only protocol, keep BOTH sides" (:2) is contradicted by this code; only its second clause ("dedups exact-duplicate Task IDs", :6-7) is implemented, and even that is misnamed — the code does not check for "exact-duplicate" content, just ID equality. What correct looks like: dedup only on ID AND body equality, or merge/flag divergent bodies for human review. Confidence: HIGH — Verification: ran it.
F-003 [S4]
Where: scripts/resolve_worklog_p6.py:40-45
Code:
```
        tid = task_id(entry)
        if tid:
            ...
```
What is wrong: entries WITHOUT a "Task ID:" line are never deduped — an exact-duplicate tid-less entry on both sides is kept twice (empirical case 3: count = 2). Blast radius: separator-split fragments (see F-004 in spirit) and any future Task-ID-less entries. Confidence: HIGH — Verification: ran it.
F-004 [S4]
Where: scripts/resolve_worklog_p6.py:51-52
Code:
```
# Collapse >2 consecutive newlines introduced by the join.
resolved = re.sub(r"\n{3,}", "\n\n", resolved)
```
What is wrong: the comment says "introduced by the join" but the regex is GLOBAL — pre-existing 3+ newline runs anywhere in worklog.md (outside any conflict) are silently rewritten. Empirical (case 7): "a\n\n\n\nkeep spacing" became "a\n\nkeep spacing" and "tail\n\n\n\nend" became "tail\n\nend" — neither near a conflict. Why it matters: it mutates the log's untouched content as a side effect. What correct looks like: apply the collapse only to the segments the join built (out.append content), or per-hunk. Confidence: HIGH — Verification: ran it.
F-005 [S4]
Where: scripts/resolve_worklog_p6.py:53-54
Code:
```
open(PATH, "w", encoding="utf-8").write(resolved)
subprocess.run(["git", "add", PATH], check=True)
```
What is wrong: worklog.md is overwritten BEFORE `git add` runs with check=True — if git fails (not a repo, lock, hooks), the script dies with a traceback AFTER the file was rewritten: modified-but-unstaged half-done state, no backup of the pre-resolution content (recovery only via git's own copy of the pre-write blob). Confidence: MEDIUM — Verification: read (git path not executed — sandbox forbids git).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| entries() | 29-31 | REAL | — | splits on "\n---\n"; matches the file's dominant structure (worklog.md: 115 `^---$` separators) |
| task_id() | 33-35 | REAL | — | first "Task ID:" per entry; None for tid-less fragments |
| dedup loop | 37-45 | REAL, lossy | S3 | F-002/F-003 |
| hunk pattern | 17 | REAL, style-blind | S3 | F-001 |
| join + collapse | 46-52 | REAL, overreaching | S4 | F-004 |
| write + git add | 53-54 | REAL | S4 | F-005 |

## 5. Test honesty
No tests in the tree for this script (rg: none). Mutation reasoning: my case
1 would catch a broken join/dedup; nothing in the repo would catch F-001
(diff3) or F-002 (divergent bodies) — they only fire on real merge conflicts
in non-default configurations.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Phase-5 proven recipe ... keep BOTH sides ... dedups exact-duplicate Task IDs" | resolve_worklog_p6.py:2-7 | HALF-FALSE | "keep BOTH sides" is contradicted by the Task-ID dedup (F-002); "exact-duplicate" is not what the code checks |
| "worklog conflicts via scripts/resolve_worklog_p6.py" (merge trains) | docs/worklog/archive-2026-10.md:10493, :10654 | TRUE-historical | recorded use in the phase-3 era merge train |

## 7. Dead / duplicated / unreachable code
Not dead (referenced + used in merge trains). Sibling scripts
resolve_w1_session_flow.py / resolve_w2_scheduler.py /
resolve_worklog_append.py survive for the same reason (worklog.md:19).

## 8. Dependencies & flags
stdlib only (re, subprocess, sys). Hardcodes PATH="worklog.md" relative to
CWD — must run from repo root (usage line says so). Shells out to `git add`.

## 9. Open questions for the next agent
- Did the phase-3-era merge trains run with default conflict style? If any
  maintainer used diff3, ancestor garbage may already sit in worklog.md
  history (spot-check old entries for `|||||||` remnants).
- resolve_worklog_append.py (the other kept resolver) shares the
  separator-split family — check it for the same F-001/F-002 classes.
