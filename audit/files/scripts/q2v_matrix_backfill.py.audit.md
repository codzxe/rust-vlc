# AUDIT: scripts/q2v_matrix_backfill.py
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 290 / 8 (esc, status_line, load, check, apply, explain, dispatch; plus the T transition table, 38 entries) |
| File verdict | REAL (historical snapshot tool; drift detector behaves correctly today: --check exits 1 with 10 documented drifts) |
| Reachable from shipped entry point? | NO — not a shipped artifact; a wave-5 audit reproducer. Nothing invokes it in CI/gates (by design: "the Lead / R2-V gate can re-run it"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |

## 1. One-paragraph verdict
A working, fail-loud snapshot/transition tool for the wave-5 feature-matrix backfill. I EXECUTED `--check` (read-only): exit 1 with 10 drift problems, all of which are genuine history (VP9 was later admitted via rusty_vp9 + 2026-11-10 compat-matrix triage downgraded linux to partial; matroska's seam closed; aac/alac flipped partial on linux) — i.e., the drift detector detects real drift, and the matrix rows' own notes say "SUPERSEDED by the 6.M-vp9 admission". The `--apply` path refuses rows that are neither baseline nor target (correct guard against double-applying onto evolved state). No fraud: this tool does not fabricate matrix status; it encodes transitions with citations and refuses unexpected states.

## 2. What actually works (verified)
— `python3 scripts/q2v_matrix_backfill.py` -> exit 1, 10 drift lines printed (executed; output in run log) — the check mode runs, parses the real 252-row docs/feature-matrix.toml, and R3/R4/R5/R6 checks execute (the "blocked without blocked_reason (R5)" hit on the current vp9 row is a true positive of the vocabulary rule against today's row).
— `--explain` -> prints the 38-transition table (executed; BrokenPipe crash only from my `| head` — see F-003).
— `grep -c '^\[\[feature\]\]' docs/feature-matrix.toml` -> 252 rows (matches the SPEC's "252 matrix rows" figure).
— REFUSE path read: L238-241 refuses a row whose current status is neither old nor new — prevents clobbering an evolved matrix.

## 3. Findings

F-001 [S4]
Where: scripts/q2v_matrix_backfill.py:263-264
Code:
```
        if t.get("blocked_reason") and not rows[t["id"]].get("blocked_reason"):
            bm = re.search(r'(^notes = "[^"]*"\n)', src, re.M)
```
What is wrong: `bm` is computed and never used — dead code (the actual insertion uses `blk` at L266-269). Confusing leftover from an earlier edit. Confidence: HIGH — Verification: read.

F-002 [S4]
Where: scripts/q2v_matrix_backfill.py:255-261
Code:
```
            nm = re.search(r'(^notes = ")([^"]*)(")', src[m2.start():], re.M | re.S)
```
What is wrong (subtle, apply-path only): `m2.start()` is an offset into the PRE-replacement `src`; `src[m2.start():]` re-slices the POST-replacement string. It happens to remain correct because group(1) (the row header) is re-emitted verbatim at the same offset, but this is offset-staleness by luck, not by construction; also `re.S` on `[^"]*` lets the notes span newlines into the next row if the row lacked a notes line. --apply is now effectively frozen (REFUSE on any evolved row), so the risk is archival. Confidence: MEDIUM — Verification: read/reasoned; not executed (apply would mutate a READ-ONLY file — prohibited).

F-003 [S4]
Where: scripts/q2v_matrix_backfill.py:284-285
Code:
```
        print(f"  {t['id']:34s} {o:19s} -> {n:19s}  {t.get('evidence', t.get('blocked_reason', ''))[:100]}")
```
What is wrong: no SIGPIPE/BrokenPipeError handling — `--explain | head` dies with a traceback (observed in my own run: BrokenPipeError at line 284). Cosmetic for a manual tool. Confidence: HIGH — Verification: ran (crash observed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| esc | 181-182 | REAL | - | backslash+quote escaping for TOML strings |
| status_line | 185-187 | REAL | - | formats `status = { ... }` for the 4 platforms |
| load | 190-192 | REAL | - | tomllib load of the 3421-line matrix (executed OK) |
| check | 195-226 | REAL | - | EXECUTED: drift detected (10), R3/R4/R5/R6 enforced, exit 1 |
| apply | 229-275 | REAL-UNPROVEN | - | REFUSE guards sound (read); never executed (would write matrix) |
| explain | 278-285 | REAL | S4 | EXECUTED; no broken-pipe handling (F-003) |
| dispatch | 288-290 | REAL | - | unknown mode silently maps to check — mild footgun (typo `--chek` runs check, arguably fine) |
| T table | 54-176 | REAL | - | 38 transitions with per-row evidence citations; cross-checked 3 rows vs matrix history (superseded notes present) |

## 5. Test honesty
No in-file tests; the tool's own --check IS the test. Mutation reasoning: flipping one matrix status by hand would be caught by check (verified behavior — today's 10 drifts were all caught); removing an evidence string from an implemented row trips R4 (code path L207-208).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "CHECKS the file against the expected post-backfill state (drift detector — exit 1 on any mismatch)" | docstring L9-11 | REAL | executed: exit 1, 10 mismatches |
| "can APPLY them idempotently" | docstring L9 | REAL-UNPROVEN | idempotence guard L236-237 read; apply not executed (read-only repo) |
| "The parity tool ... is ABSENT, so the documented-honest path ... is this committed reproducer" | docstring L4-7 | REAL | consistent with baseline: parity-report 0/252 verified |
| "every transition's evidence is a landed per-format record ... or ... Task ID — stated honestly" | docstring L13-15 | REAL | 2 rows (subtitle-dvb-subs, subtitle-vobsub) carry "NO docs/formats record yet — audit gap G-1" in-band |

## 7. Dead / duplicated / unreachable code
Dead `bm` (F-001). The whole script is automation-unreachable (manual re-run tool) — documented as such.

## 8. Dependencies & flags
stdlib only (re, sys, tomllib). No network, no cargo. Runs < 1 s.

## 9. Open questions for the next agent
1. docs/gates/phase5-records-audit.md (the context doc) — does it record the 10-drift outcome as expected supersession? (phase4-exit.md is in my queue, not phase5.)
2. Who re-runs this after each wave? If nobody, the "drift detector" fired once (now) and stays red forever — a permanently-red check trains people to ignore red.
