# AUDIT: scripts/corpus_supervisor.sh
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines / functions | 22 / 1 (single supervisor loop) |
| File verdict | REAL-UNPROVEN (works only on its original host; dead here) |
| Reachable from shipped entry point? | NO — one-shot infra supervisor; only reference is the worklog narrative (docs/worklog/archive-2026-10.md:1816) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read (full); ls tools/corpus (gen_corpus.py present); rg TOTAL marker; ls of the hardcoded absolute paths (absent) |

## 1. One-paragraph verdict
A resume-loop supervisor for the corpus generator: relaunches
gen_corpus.py up to 40 times until the `[gen] TOTAL` completion line lands
in the log, tolerating the sandbox reaper's kills. The success-marker
contract is real (gen_corpus.py:1047 prints `[gen] TOTAL ...` — the grep
pattern matches). Its defects: three hardcoded absolute paths that only
exist on the original build host (repo at /home/z/my-project, corpus and
logs OUTSIDE the repo), making the script a machine-specific fossil that
cannot run on any other checkout, and a Traceback-grep that sticks (once
any traceback is in the appended log, the rc!=0 backoff stops firing).

## 2. What actually works (verified)
— Success-marker consistency: `grep -q "^\[gen\] TOTAL"` matches
tools/corpus/gen_corpus.py:1047 `print(f"[gen] TOTAL {len(self.files)} files, ")`
— the contract between supervisor and generator is real.
— Bounded retry (40), per-attempt rc and timestamp logging, explicit
give-up + exit 1.
— The resume-safety premise (plan.json + incremental manifest) is
corroborated by the worklog archive:1816 narrative.

## 3. Findings
F-001 [S3]
Where: scripts/corpus_supervisor.sh:5-7
Code:
```
OUT=/home/z/my-project/corpus
LOG=/home/z/my-project/logs/corpus-gen.log
cd /home/z/my-project
```
What is wrong: hardcoded host-absolute paths — the repo is expected at
/home/z/my-project with the corpus and its log written OUTSIDE the
repository tree; on this tree (/home/z/my-project/audit-workspace) and on
any other clone, the script is unrunnable as-is (ls: /home/z/my-project/
corpus and /home/z/my-project/logs do not exist). A committed script with
a single-host address baked in is environment-coupled dead weight.
What correct looks like: `ROOT="$(cd "$(dirname "$0")/.." && pwd)"`-style
resolution (the pattern the quality scripts use) and corpus/logs under
gitignored repo paths.
Confidence: HIGH — Verification: read + ls.
F-002 [S4]
Where: scripts/corpus_supervisor.sh:17
Code:
```
  if [ $rc -ne 0 ] && ! grep -q "Traceback" "$LOG"; then
    sleep 2
    fi
```
What is wrong: the Traceback check scans the WHOLE appended log — after
any historical traceback exists in it, every later nonzero-exit attempt
skips the 2s backoff (the condition inverts permanently). Also a rc==0
exit without TOTAL (partial completion) relaunches with no pause at all.
Cosmetic pacing defects only.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| supervisor loop | 8-22 | REAL-UNPROVEN | S3 | marker contract real; host-absolute paths (F-001); traceback-grep stickiness (F-002) |

## 5. Test honesty
N/A (one-shot infra). Mutation reasoning: the loop provably terminates
(40 cap) and detects success via a real marker; the failure modes are
pacing, not correctness.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the generator is resume-safe (plan.json + incremental manifest), so relaunch it until the TOTAL line lands" | :2-4 | REAL | worklog archive:1816 documents the design; gen_corpus.py:1047 emits the marker |

## 7. Dead / duplicated / unreachable code
Unreachable on this host entirely (paths absent). Kept as history; the
supervisor's narrative value is the reaper-tolerance design.

## 8. Dependencies & flags
python3 -u tools/corpus/gen_corpus.py --count 400 --budget-mb 1700; seq,
grep, date. The generator and its CLI exist in-tree (tools/corpus/).

## 9. Open questions
Was the corpus actually completed under this supervisor on the original
host, and is any supervisor log preserved as evidence? The corpus
(/home/z/my-project/corpus) is not in this tree and no corpus-gen log is
committed — the 500-file corpus-v4 story rests on the worklog narrative.
