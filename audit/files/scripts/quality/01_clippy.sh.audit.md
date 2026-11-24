# AUDIT: scripts/quality/01_clippy.sh
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / functions | 67 / run_level + main |
| File verdict | REAL sweep/reporter — but its exit code is structurally always 0, defeating the runner's reporter contract |
| Reachable from shipped entry point? | NO — quality tool; run via scripts/quality/99_run_all.sh `static` tier (J reporter), itself driven by .github/workflows/audit.yml:80 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | bug reproduction of `grep -c ... || echo 0` (executed, output below); read of common.sh (mkdir -p $OUT, not_run) |

## 1. One-paragraph verdict
A genuine 4-level clippy sweep (L0 `-D warnings`, L1 pedantic, L2 nursery, L3 restriction lints on 7 hot crates) with correct `PIPESTATUS[0]` capture after `tee`, honest per-level warning/error counts, and an honestly-recorded platform-linux/macos exclusion. But it is a reporter whose exit code is ALWAYS 0 — the `J` wrapper in 99_run_all.sh exists to record reporter exit codes, and this one can never be non-zero even when the L0 strict level fails — so the runner's own recording plumbing is dead for this tool. A smaller display bug (executed): the `grep -c ... || echo 0` fallback double-prints "0" into the count variable.

## 2. What actually works (verified)
— Read: run_level captures `rc=${PIPESTATUS[0]}` correctly (L38-39) and prints per-level exit/warning/error counts (L43).
— Honest exclusion: L11 `not_run "clippy/workspace-all-targets" "platform-linux+platform-macos excluded: ..."` — the not-run ledger in common.sh (L26-31) appends timestamped rows; "NEVER invent a result" posture.
— common.sh:11 does `mkdir -p "$OUT"` — the clippy log writes are not lost on a fresh tree.
— L3 histogram greps (`#[warn(lint)]`) give a real per-lint distribution.

## 3. Findings
F-001 [S3] — The script can never exit non-zero; reporter rc-recording is dead for this tool.
Where: scripts/quality/01_clippy.sh:34-67 vs scripts/quality/99_run_all.sh:19-21
Code (99_run_all.sh):
```
J()  { bash "$HERE/$1" "${@:2}"; local rc=$?
       [ "$rc" -ne 0 ] && FAILED+=("$1 rc=$rc"); return 0; }
```
and the tail of 01_clippy.sh:
```
say ""
say "logs: $OUT/clippy-*.log"
```
What is wrong: `run_level` ends with `say`/`head` pipelines (exit 0); nothing propagates `${PIPESTATUS[0]}` or the per-level failure to the script's own exit status. Even a failing L0 (`-D warnings` — the level that corresponds to the merge gate) leaves the sweep exiting 0, so `FAILED+=` in the J wrapper never fires and the "non-zero reporters (recorded...)" section of the tier report stays silent. The data is in the logs; the plumbing that would surface it in the tier summary is inert for this script. Confidence: HIGH — Verification: read (control-flow trace of every path).
F-002 [S4] — `grep -c ... || echo 0` writes TWO zeros when the count is 0 (executed).
Where: scripts/quality/01_clippy.sh:41-42
Code:
```
  warns=$(grep -c '^warning: ' "$logf" 2>/dev/null || echo 0)
  errors=$(grep -c '^error' "$logf" 2>/dev/null || echo 0)
```
Executed reproduction:
```
$ v=$(grep -c '^warning: ' /dev/null 2>/dev/null || echo 0); printf 'A[%s]B\n' "$v"
A[0
0]B
```
What is wrong: `grep -c` already prints `0` before exiting 1, so `|| echo 0` appends a second zero; the `say "level=... warnings=..."` line splits in two and the summary line is corrupted for any clean level (L0 on a clean tree prints garbage). Confidence: HIGH — Verification: ran (reproduced above).
F-003 [S4] — L3 loop's `cargo metadata` guard is loop-invariant (checked per-iteration, never varies) — cosmetic; the `break` on failure is correct but the check belongs outside the loop. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run_level | 34-46 | REAL | — | PIPESTATUS technique correct; rc not propagated (F-001) |
| main (levels + L3) | 48-67 | REAL | S3/S4 | F-001..F-003 |

## 5. Test honesty
N/A (tool). The clippy merge gate itself lives in rust-clippy.yml / ci.yml (separate lanes) — this script is the evidence sweep, not the gate.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "T01 - strict clippy sweep: L0 baseline, L1 pedantic, L2 nursery, L3 targeted" | 01_clippy.sh:2 | REAL (structure) | levels read at L48-50, L54 |
| "its exit code is recorded but does not fail the tier" (J contract) | 99_run_all.sh:19 | DEFEATED by this script | F-001 |
| platform-linux/macos test targets do not compile off-Linux (F-001 reference) | 01_clippy.sh:10-11 | REAL-UNPROVEN | not re-compiled here (no cargo); consistent with audit.yml comment re F-001 platform gating |

## 7. Dead / duplicated / unreachable code
None dead; L3 restriction-lint block is per-crate by design (workspace noise rationale documented at L52).

## 8. Dependencies & flags
cargo clippy (present in CI; absent in this sandbox — no execution here); tee/grep/sed/sort/uniq; common.sh.

## 9. Open questions for the next agent
- Should the L0 level's failure propagate (exit 1) so the J contract records it? The tier is informational, but the audit.yml scoreboard depends on recorded reporter failures.
