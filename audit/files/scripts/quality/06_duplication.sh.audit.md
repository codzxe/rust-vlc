# AUDIT: scripts/quality/06_duplication.sh
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Date | 2026-10-30 |
| Lines / functions | 44 / 3 blocks (npx gate, jscpd run, report parser) |
| File verdict | REAL (census reporter — no gate claimed, none faked) |
| Reachable from shipped entry point? | YES (CI): audit.yml static tier (PR) → 99_run_all.sh:33 `J 06_duplication.sh` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `bash -n` (exit 0); `command -v npx` (exit 0, present); parser body executed verbatim against a synthetic jscpd report (exit 0, output below). jscpd itself NOT run — npx package download = network/npm (load discipline). |

## 1. One-paragraph verdict
An honest census: it says "duplication census" and that is exactly what it is — no threshold, no verdict, no enforcement claim. The jscpd invocation is real (rust tokenizer, min-tokens 50, json reporter, target/ ignored, scans crates/apps/tools/tests), and the parser half I executed verbatim against a synthetic report works correctly (statistics extraction + per-file duplicated-token clustering + backslash normalization). Defects are plumbing-level only: the jscpd exit code is discarded, the tool version floats (`jscpd@4`), and the parser's schema assumptions are unverified against jscpd 4's actual output shape.

## 2. What actually works (verified)
- `bash -n scripts/quality/06_duplication.sh` → exit 0.
- Parser body (the exact L23-39 code) executed against a synthetic report:
```
files:       42
lines:       1000
clones:      7
duplication: 4.5%

top files by duplicated tokens:
      88  crates//demux//src//avi//mod.rs
```
  (synthetic names used doubled backslashes; `.replace("\\", "/")` normalization applied — statistics, clones, percentage, and the 15-row cluster table all correct).
- Honest failure path: if jscpd produces no report, `not_run "jscpd" "report not produced"` (:42) is recorded — a tool failure does not masquerade as a census result.

## 3. Findings
F-001 [S4]
Where: scripts/quality/06_duplication.sh:13-19
Code:
```
npx --yes jscpd@4 \
  --format rust \
  --min-tokens 50 \
  --reporters json \
  --output "$OUT/jscpd" \
  --ignore "**/target/**" \
  crates apps tools tests 2>&1 | tail -20
```
What is wrong: the pipeline's status (pipefail → jscpd's rc) is discarded; nothing after it. Why it matters: a jscpd crash whose stderr survives `tail -20` still leaves the script green (the else-branch only fires when the report file is missing, and a partial write is possible). Census has no threshold, so no enforcement is claimed — this is plumbing debt, not a broken gate. Confidence: HIGH — Verification: read.

F-002 [S4]
Where: scripts/quality/06_duplication.sh:13
Code:
```
npx --yes jscpd@4 \
```
What is wrong: floating major version — every run resolves the latest 4.x from the network; results (and the parser's schema assumptions) drift silently across runs; also needs npm network access on every CI invocation. What correct looks like: a pinned version (the repo pins ast-grep 0.51.3 in common.sh:53 — the same discipline is absent here). Confidence: HIGH — Verification: read.

F-003 [S4]
Where: scripts/quality/06_duplication.sh:35-36
Code:
```
    for f in dup.get("firstFile", {}), dup.get("secondFile", {}):
        clusters[f.get("name", "?").replace("\\", "/")] += dup.get("tokens", 0)
```
What is wrong: assumes jscpd 4 emits `firstFile`/`secondFile` as OBJECTS with a `name` key. If the actual schema uses strings (some jscpd versions emit `firstFileName`), the parser raises AttributeError — which, notably, WOULD make the script exit non-zero (last command = the heredoc), i.e. this is the only quality script so far whose parser crash is actually visible to the tier. UNVERIFIED: jscpd@4's real schema needs the tool run. Confidence: MEDIUM — Verification: parser logic executed on synthetic data (works); real schema UNVERIFIED (jscpd not run — network).

F-004 [S4]
Where: scripts/quality/06_duplication.sh:7-10
Code:
```
if ! command -v npx >/dev/null 2>&1; then
  not_run "jscpd" "npx/node not installed"
  exit 0
fi
```
What is wrong: missing-tool → exit 0 (recorded, but green to the tier). Same pattern as T11/T23; acceptable for a J reporter by design, noted for the pattern count. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| npx gate | 7-10 | REAL | S4 | not_run recorded; exit 0 |
| jscpd run | 12-19 | REAL instrument | S4 | rc discarded; floating version |
| report parser | 21-43 | REAL | S4 | executed on synthetic data; real-schema UNVERIFIED |

## 5. Test honesty
No self-tests. Mutation reasoning: if the parser printed only zeros, nothing would notice — the census has no consumer-side assertion.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "T06 - duplication census (jscpd; Rust tokenizer)" | :2 | **REAL** | census by name and by behavior; parser executed; no verdict/threshold asserted anywhere in the script |

## 7. Dead / duplicated / unreachable code
None. `pct` assigned then printed — fine.

## 8. Dependencies & flags
npx/node (jscpd@4, floating), python3 JSON parsing. Scan roots: crates apps tools tests. Ignore: `**/target/**`. min-tokens 50, format rust.

## 9. Open questions for next agent
- Does any committed doc claim a duplication percentage from T06? No committed jscpd report exists (`docs/audit/reports/` absent) — any percentage cited anywhere is unverifiable from the tree.
- Pin jscpd exactly when the tier is made enforceable; and verify the jscpd@4 firstFile/secondFile schema at that run.
