# AUDIT: scripts/resolve_w2_scheduler.py
| Field | Value |
|---|---|
| Auditor | W17-A08 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines / functions | 37 / 2 (flat splice script + idx helper) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — one-shot historical merge utility; its TARGET (crates/video-engine/src/scheduler.rs) is live: video-engine is a dep of apps/cli/Cargo.toml:63, apps/desktop/Cargo.toml:43, renderer-wgpu/Cargo.toml:29 |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | scratch rerun on today's file -> exit 0, 9 live lines silently deleted (diff pasted); synthetic conflicted-layout run -> exit 0, 9 markers dropped, union preserved; mismatched-layout run -> exit 1, file left WITH markers; sed/hex quote verification |

## 1. One-paragraph verdict
A one-shot splice with nine hard-coded 1-based line indices that worked exactly once (its historical use is recorded at archive-2026-10.md:7429) and has been a loaded weapon ever since: run it today on the clean, live scheduler.rs and it silently deletes nine real code lines — including `SchedulerSync {`, a closing brace, and a test's `PresentationScheduler::new(...)` — while printing `resolved: 1702 lines` and exiting 0. The :36 assert cannot catch this because the clean file contains no markers to leave behind. There is zero marker validation before the destructive write.

## 2. What actually works (verified)
— verified by synthetic reconstruction (markers placed at the exact 1-based positions the script hardcodes: 489/492/498, 536/540/547, 1061/1205/1343): `python3 rw2s_hist2.py` -> `resolved: 1351 lines`, exit 0; all 9 marker lines dropped; both sides preserved in HEAD-then-theirs order for all 3 regions (label check: R1-HEAD-A,R1-HEAD-B,R1-THEIRS-A,... R3-THEIRS-B). The union arithmetic is correct for its one historical input.
— verified historical use: archive-2026-10.md:7429 `scheduler.rs union-resolved with W2-04 — one missing shared-tail brace, scripts/resolve_w2_scheduler.py` (the script did run in W2-02's merge).
— verified by mismatch-layout run: markers at wrong positions -> exit 1 via :36 assert — the assert does fire when markers survive, but only AFTER the file was already rewritten (see F-002).

## 3. Findings
F-001 [S2]
Where: scripts/resolve_w2_scheduler.py:10-11, 17-32, 36-37
Code: `# 0-based marker indices from rg (1-based 489, 492, 498 / 536, 540, 547 /
# 1061, 1205, 1343)` / `out += lines[:idx(489)]` ... `print("resolved:", len(joined.splitlines()), "lines")`
What is wrong: the splice is anchored to absolute line numbers of a file state that no longer exists, with no validation that those lines are conflict markers. Executed against TODAY's clean 1711-line scheduler.rs (scratch copy): exit 0, message `resolved: 1702 lines`, and `diff` shows exactly these 9 live lines deleted:
```
489d488  <         SchedulerSync {
492d490  <             last_anchor_epoch: None,
498d495  <             drops_in_window: 0,
536d532  <             self.changes_in_window = 0;
540d535  < }
547d541  <             .field("cfg", &self.cfg)
1061d1054 <     }
1205d1197 <         let mut sched = PresentationScheduler::new(cfg(), clock.clock());
1343d1334 <             0,
```
Why it matters: brace imbalance + deleted struct/test literals = compile-breaking corruption of source reachable from the shipped lumen-cli/desktop entry points, delivered with a success exit code and a success message. The :36 assert is structurally incapable of detecting this (a clean input leaves no markers). What correct looks like: read the marker lines and abort unless they match `<<<<<<<`/`=======`/`>>>>>>>`, or delete the script now that its one merge is done.
Confidence: HIGH — Verification: ran (corruption diff pasted).

F-002 [S3]
Where: scripts/resolve_w2_scheduler.py:34-36
Code: `p.write_text("".join(out))
joined = "".join(out)
assert "<<<<<<<" not in joined and ">>>>>>>" not in joined`
What is wrong: destructive write precedes the sanity check (same defect class as resolve_worklog_append.py and resolve_w1_session_flow.py). Empirical: on a mismatched layout the script rewrote the target file still containing 6 conflict markers, THEN raised AssertionError (exit 1) — the corrupt result is on disk. Bare `assert` also vanishes under `python -O`.
Confidence: HIGH — Verification: ran (mismatch-layout probe).

F-003 [S4]
Where: scripts/resolve_w2_scheduler.py:7
Code: `p = Path("crates/video-engine/src/scheduler.rs")`
What is wrong: CWD-relative path — correct only when invoked from the repo root; from anywhere else it raises FileNotFoundError. No argument for the target path.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: scripts/resolve_w2_scheduler.py:2-3 vs archive-2026-10.md:7429
Code: `"""Union-resolve the W2-04 x W2-02 scheduler.rs merge: both sides'
doc lines and test blocks are kept."""` / archive: `scheduler.rs union-resolved with W2-04 — one missing shared-tail brace, scripts/resolve_w2_scheduler.py`
What is wrong: even on its one historical run the union splice was insufficient — a shared-tail brace was missing afterward and had to be fixed by hand; the docstring presents "Union-resolve ... both sides kept" as a complete resolution when the historical record says it produced non-compiling output that needed a manual patch. Kept-forever retention of a proven one-shot with known rerun-corruption semantics is the hygiene debt.
Confidence: HIGH — Verification: read (archive citation sed-checkable at :7429).

## 4. Per-function table (37 loc — brief)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| idx helper | 12-13 | REAL | — | trivial 1-based→0-based |
| splice assembly | 15-32 | REAL (one historical input) | F-001 | hardcoded indices, no marker check |
| write+assert+report | 34-37 | DEFECTIVE order | F-002 | write-before-validate |

## 5. Test honesty
No tests. Execution evidence = my three scratch runs (historical-layout PASS, today-rerun silent corruption, mismatch-layout corrupt-then-crash) plus the archived worklog record of its real use.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Union-resolve the W2-04 x W2-02 scheduler.rs merge" | :2 | REAL (mechanism) | synthetic-layout run: union preserved, exit 0 |
| "both sides' doc lines and test blocks are kept" | :3 | REAL | label check R1/R2/R3 HEAD-then-theirs |
| (implicit) splice yields a compiling file | — | FALSE-at-the-time | archive-2026-10.md:7429: "one missing shared-tail brace" fixed by hand |

## 7. Dead / duplicated / unreachable code
Not wired anywhere; referenced only by archive-2026-10.md:7429 (historical use) and worklog.md:19 (keep-decision). Dead by purpose; retained in shipped scripts/.

## 8. Dependencies & flags
stdlib pathlib only. No flags. Mutates live source when (re)run.

## 9. Open questions for the next agent
- The two other kept resolvers (resolve_w1_session_flow.py audited W17-A06, resolve_worklog_p6.py never audited — R1 died) share this defect family; should the whole keep-list be deleted rather than curated?
