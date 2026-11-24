# AUDIT: scripts/quality/19_concurrency.sh
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Date | 2026-10-30 |
| Lines / functions | 31 / 3 blocks (model census, surface census, catalogue) |
| File verdict | SHALLOW — a census script wearing a model-checking title; honest PARTIAL note only when zero models exist |
| Reachable from shipped entry point? | YES (CI): audit.yml static tier (every PR) → 99_run_all.sh:38 `J 19_concurrency.sh` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `bash -n` (exit 0); rg replication of both censuses (exit 0): loom usages = 0, shuttle usages = 0, concurrency surface = 99 non-test crate files. Script NOT executed directly — common.sh writes to docs/audit/reports (outside audit/; forbidden), so censuses were re-run with rg instead. |

## 1. One-paragraph verdict
"T19 - concurrency model checking (loom / shuttle)" performs no model checking of any kind: it greps the tree for loom/shuttle usage (both ZERO today — no concurrency model exists anywhere in this repo), greps for concurrency primitives to list files where a model would be needed (99 non-test files in crates/), and prints a five-line aspirational catalogue of invariants that no tool ever checks. The script is honest about this only in the zero-usages branch ("this task is PARTIAL"), so the defect is the name and the tier-level impression: every PR runs "19. concurrency" green while the actual model-checking program — the thing the title promises — is a grep count plus a wish list.

## 2. What actually works (verified)
- `bash -n scripts/quality/19_concurrency.sh` → exit 0.
- 19a census replicated with rg (exact regexes): `loom::|#\[cfg\(loom\)\]` → **0 usages**; `shuttle::` → **0 usages** — the script's PARTIAL note branch (:11-14) is the live branch on today's tree.
- 19b surface census replicated: 99 non-test files in crates/ match `\b(Arc|Mutex|RwLock|AtomicU?[0-9]+|Condvar|mpsc|crossbeam)\b` — the "where a model is needed" list is real and substantial.

## 3. Findings
F-001 [S3]
Where: scripts/quality/19_concurrency.sh:2, 22-30
Code:
```
# T19 - concurrency model checking (loom / shuttle).
...
 target                  invariant
 media-clock             time never decreases; one epoch per tick
 packet-queue            bounded capacity; no lost/duplicated item
 playback-engine         no illegal state transition under concurrent commands
 audio-output            shutdown while playing: no deadlock, no UAF
 renderer-wgpu           frame submission ordering
```
What is wrong: the title promises model checking; the body runs zero loom/shuttle executions and the 19c "model catalogue" is a static heredoc of invariants that no code checks — a name that lies about the deliverable. Why it matters: the static tier runs on every PR and prints a green "19" section; a reader of the audit log has no signal that the five catalogue invariants (deadlock-free audio-output shutdown, packet-queue no-loss, etc. — the highest-risk classes in a media player) are checked by nothing at all. What correct looks like: rename the block to "concurrency surface census" or land one loom model and wire `cargo +nightly loom test` for it. Confidence: HIGH — Verification: read + rg census (loom=0, shuttle=0). The honesty note at :12-13 ("writing a loom model is a source change, which this program does not do") is present and true, which keeps this at S3 rather than a false claim.

F-002 [S4]
Where: scripts/quality/19_concurrency.sh:7-8
Code:
```
n_loom=$(grep -rn --include=*.rs -E 'loom::|#\[cfg\(loom\)\]' crates apps tools 2>/dev/null | wc -l)
```
What is wrong: `grep -rn` over the tree (repo standard is rg — slower, and stderr discarded so a grep failure reads as "0 usages" → the PARTIAL branch is also the failure branch). Confidence: HIGH — Verification: read.

F-003 [S4]
Where: scripts/quality/19_concurrency.sh:17
Code:
```
grep -rn --include=*.rs -E '\b(Arc|Mutex|RwLock|AtomicU?[0-9]+|Condvar|mpsc|crossbeam)\b' crates 2>/dev/null \
```
What is wrong: the surface census scans only `crates`, while 19a scans `crates apps tools` — apps/ and tools/ concurrency primitives (CLI, sweep, mem-audit tools) are invisible to the census. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: scripts/quality/19_concurrency.sh:17-18
Code:
```
  | grep -v '/tests/'
```
What is wrong: excludes tests whose path contains `/tests/` but not `tests/` at repo root level or `mod tests` inline blocks — the census is approximate (acceptable for a census; noted so the 99-file figure is not over-trusted). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| 19a model census | 6-14 | REAL (census) | — | loom=0, shuttle=0 today; PARTIAL note honest |
| 19b surface census | 16-20 | REAL (census) | S4 | 99 files; crates-only scope |
| 19c catalogue | 22-30 | STUB (as checking) | S3 | aspirational heredoc; nothing checks these invariants |

## 5. Test honesty
No self-tests. Mutation reasoning: deleting 19c entirely changes no output a consumer could rely on; deleting 19b leaves the tier green.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "T19 - concurrency model checking (loom / shuttle)" | :2 | **FALSE as a description of behavior** | zero loom/shuttle runs in the script body; rg-verified zero loom/shuttle usages in the tree — nothing to check and nothing checking |
| "NOTE: no concurrency model exists yet -> this task is PARTIAL" | :12-13 | **REAL** | rg censuses: 0/0 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
grep/awk/sort/uniq only — no loom, no shuttle, no cargo. No feature flags.

## 9. Open questions for next agent
- Does any doc/worklog claim "concurrency model checked (T19)" for waves that touched packet-queue or playback-engine? That claim would be false — flag it to the doc owner.
- 99 files with concurrency primitives and zero models: the 6.H1/19 gap is the single largest untested-risk surface found in this agent's files.
