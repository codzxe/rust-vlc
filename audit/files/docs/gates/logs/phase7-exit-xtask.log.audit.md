# AUDIT: docs/gates/logs/phase7-exit-xtask.log (DATA AUDIT — log; chunks part1 L1-800 + part2 L801-959 = one file)
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Date | 2026-11-24 |
| Lines | 959 |
| File verdict | REAL (authentic xtask battery output captured in worktree w24f) — STALE vs HEAD |
| Reachable from shipped entry point? | N/A (log data; cited by docs/gates/phase7-exit.md:51-57 and docs/reviews/w24-s-chain-findings.md:240, w24-f-gate-findings.md:190) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | awk/sed/rg/diff — all exit 0 except diff (exit 1, expected) |

## 1. One-paragraph verdict
Genuine captured run of 7 xtask gates (check-architecture, codegen --check, schema-gen
--check, fixtures --check, lint-literals, check-identity, sbom --check), all
XTASK_EXIT=0. Internal arithmetic is exact: the 452 vs 453 "allowlisted" counts match
my line counts, and the two allowlist blocks differ by exactly ONE line
(`window_config.rs:427` counted as hex-color only in lint-literals, identity+hex-color
in check-identity). But this log was produced in worktree `/home/z/my-project/wt/w24f`
(branch w24/f @ a4e35e7, dated 2026-11-10), NOT the main tree: on HEAD, the
orchestrator baseline re-runs 3 of these gates RED (lint-literals 1 violation,
check-identity 1 violation, sbom DRIFT, arch edges FAIL). Any doc citing this log as
current gate evidence is citing stale data.

## 2. What actually works (verified)
- `awk '/^## Cmd:/'` -> 7 commands; `awk '/^XTASK_EXIT/'` -> 7 lines all `=0`. Consistent.
- `awk 'NR>=38 && NR<=489 && /^  allowed:/' | wc -l` -> **452**; `awk 'NR>=496 && NR<=948 && /^  allowed:/' | wc -l` -> **453**. Matches log's own "452 allowlisted" (L490) and "453 allowlisted" (L949).
- `sed -n '38,489p' | sort` vs `sed -n '496,948p' | sort` -> diff = exactly 1 line: `apps/desktop/src/window_config.rs:427: identity — test fixture TOML mirroring product.toml [brand] — parse-target data` present only in check-identity. Explains 453−452.
- Spot-checked cited lines against current tree: session.rs:2015 -> `.join("lumen-codecs-hw-vaapi-probe-tests")` (scratch-dir identity, matches log L187's claim); window_config.rs:427 -> `// lumen-allow-literal: test fixture TOML mirroring product.toml [brand] — parse-target data` (reason string byte-identical to log L141).

## 3. Findings
F-001 [S2]
Where: docs/gates/logs/phase7-exit-xtask.log:1-4 vs audit/runs/baseline/full_gate.txt:59-75
Code (log):
```
# Worktree: /home/z/my-project/wt/w24f (branch w24/f @ a4e35e7)
# Date: 2026-11-10T16:56:35Z
```
Code (baseline):
```
lint-literals: FAIL — 1 violation(s):
  apps/cli/src/host.rs:1799: identity — identity(identity.name)
```
What is wrong: the log records all-7-green in a w24/f worktree, but on HEAD the same
gates are red (lint-literals FAIL, check-identity FAIL, sbom DRIFT, arch edges FAIL —
baseline full_gate.txt:59-75, 42-51). The log is cited by phase7-exit.md:51-57 as the
exit-gate evidence. Why it matters: the phase-7 exit "PASS" claims rest on data from a
different tree/commit; on HEAD four of the audited facts (edge count 97, 0 violations,
121900 B sbom, 157 components) no longer hold (baseline: 158 components, subject
Lumen 0.1.3 vs log's 0.1.0-dev). What correct looks like: re-run the battery on HEAD
and store the current log, or stamp the doc with the commit that the evidence covers.
Confidence: HIGH — Verification: read + baseline citation.

F-002 [S3]
Where: docs/gates/logs/phase7-exit-xtask.log:490,949
Code:
```
lint-literals: PASS — 0 violations, 452 allowlisted
check-identity: PASS — 0 violations, 453 allowlisted
```
What is wrong: the tool prints the allowlist SIZE as its headline result; the two
passes differ by one entry (window_config.rs:427 identity), i.e. "0 violations" is
achieved by whitelisting 452/453 sites, not by removing literals. Why it matters:
readers can cite "0 violations" as a hygiene win when 453 exemptions are the actual
state. What correct looks like: print exempt/total ratio alongside violations.
Confidence: HIGH — Verification: read (counts re-derived by awk).
Note: 8 of the allowlisted entries say "identity wiring pending (see help.rs
[PROPOSAL])" (L81, L104 etc.) — the tool itself records that product identity is
still hardcoded in CLI prose.

F-003 [S4]
Where: docs/gates/logs/phase7-exit-xtask.log:186-188
Code:
```
  allowed: crates/codecs/src/video/h264/hw/vaapi/device.rs:144: identity — identity — cfg(test) scratch dir prefix — test-only fixture, not a shipped surface
```
What is wrong: duplicated classifier word "identity — identity —" (three entries:
device.rs:144, session.rs:2015, session.rs:2041). Cosmetic tool-output defect
indicating the reason strings are hand-maintained data, not generated.
Confidence: HIGH — Verification: read.

## 4. Data integrity checks
- 452/453 allowlisted: verified by awk count (see §2). CONSISTENT.
- "18602 string-literal sites classified" (L37, L495): identical in both blocks. CONSISTENT (re-derivation would need a build — UNVERIFIED, cargo-build-budget).
- "558 files scanned" (L36, L494) both blocks. CONSISTENT.
- sbom: "components: 157 (118 registry, 39 workspace/path)" -> 118+39=157. ARITHMETIC OK. Baseline HEAD: 158 (119+39) — drift by exactly 1 component since the log.
- architecture: "97 internal edges, 46 dev edges" (L11) vs "143 internal edges (normal + dev)" (L12): 97+46=143. ARITHMETIC OK.
- Provenance: worktree `/home/z/my-project/wt/w24f` — cannot re-execute xtask battery (cargo-build-budget on 2-core sandbox; also worktree may not exist — UNVERIFIED: `ls /home/z/my-project/wt` not attempted, tree is read-only evidence only).
- Line-number drift spot-check: log L433 claims hls/live.rs:1466 is "url — test-double URL"; HEAD live.rs:1466 is `"lumen-backpressure-{}-{}s.wav"` (scratch filename). Line numbers in the log refer to the w24f tree, not HEAD — expected for a captured log, but it makes the allowlist non-machine-checkable against HEAD without a re-run.

## 5. Consumers (who cites this data)
- docs/gates/phase7-exit.md:51-57 — rows 8-14 all cite this log as PASS evidence (STALE per F-001; that doc belongs to another agent's domain).
- docs/reviews/w24-s-chain-findings.md:240 and w24-f-gate-findings.md:190 — claim "all verbatim in phase7-exit-xtask.log"; verbatim-presence is TRUE, but those reviews do not disclose the worktree provenance.
