# AUDIT: docs/gates/logs/phase7-exit-clippy.log (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A08 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines | 20 (5 header + 13 Checking + Finished + exit marker) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — Phase-7 gate evidence log, cited by docs/gates/phase7-exit.md:45 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | member-list parse of both logs (r1=96, r2=13 Checking lines); 13/13 membership verified against root Cargo.toml:41; worktree existence check -> ABSENT; baseline cargo-clippy.txt cross-check; phase7-exit.md:45 quote check |

## 1. One-paragraph verdict
A well-formed warm-cache clippy confirmation: command, worktree, date and exit code are all recorded in the log itself (CLIPPY_EXIT=0 at :20), the 21.80s "Finished" matches the citing doc's "r2 in 21.8 s warm" exactly, and all 13 "Checking" entries are genuine workspace members forming a patch-closure-consistent set with r1's full 96-line battery. It is a historical worktree artifact (w24f, gone today) and it re-lints only 13 of ~36 members by itself — the full-battery proof lives in the r1 companion, which this log honestly points to.

## 2. What actually works (verified)
— verified by parsing both logs: r1 = 96 `Checking` lines (full graph incl. externals), r2 = 13 `Checking` lines, and every r2 name (input, media-probe, transcode, lumen-cli, lumen-web, first-vertical, desktop, lumen-mem-audit, lumen-sweep, streaming, codecs, demux, playback-engine) is a real workspace member per root Cargo.toml:41 `members = ["crates/*", "apps/*", "tools/xtask", "tests/integration/first-vertical", "tools/mem-audit", "tools/sweep"]`.
— verified closure consistency: the 13-member set = a plausible touched-set {input, media-probe, transcode, streaming, codecs, demux, playback-engine} + its dependents {lumen-cli, lumen-web, desktop, first-vertical, lumen-mem-audit, lumen-sweep} — the shape cargo produces for a warm re-check after a patch touching those crates.
— verified citations: phase7-exit.md:45 `**PASS** (exit 0; r2 in 21.8 s warm, r1 cold 1 m 14 s)` — log :19 `Finished ... in 21.80s`, :20 `CLIPPY_EXIT=0`; r1 log :250 `in 1m 14s`, :251 `CLIPPY_EXIT=0`. Both timings and both exit codes reproduce EXACTLY.
— verified current-tree corroboration: audit/runs/baseline/cargo-clippy.txt:118 `Finished \`dev\` profile [unoptimized] target(s) in 59.06s` + AGENT-SPEC §13 baseline fact "clippy -D warnings: CLEAN".

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/phase7-exit-clippy.log:6-18
Code: `    Checking input v0.1.0 (/home/z/my-project/wt/w24f/crates/input)` ... (13 Checking lines total)
What is wrong: r2 alone re-lints 13 of ~36 workspace members — as standalone evidence it would under-prove "workspace clippy clean"; it is only sufficient because the header (:5) explicitly pairs it with the r1 full battery ("body preserved below in phase7-exit-clippy-r1.log", audited REAL by W17-A04). The pairing is honest, so this is a scope note, not fraud.
Confidence: HIGH — Verification: ran (member parse, 13 vs 96).

F-002 [S4]
Where: docs/gates/logs/phase7-exit-clippy.log:2
Code: `# Worktree: /home/z/my-project/wt/w24f (branch w24/f @ a4e35e7)`
What is wrong: the capture worktree no longer exists anywhere on this machine (`/home/z/my-project/wt` ABSENT; `/tmp/my-project/wt` has no w24f) — the evidence is era-locked to 2026-11-10T17:11:12Z and cannot be re-run or re-anchored; current-tree clippy cleanliness rests on the orchestrator baseline instead (cited above).
Confidence: HIGH — Verification: ran (ls of both wt roots).

## 4. Test honesty
Not a test file. The log embeds its own exit marker (CLIPPY_EXIT=0) — better than sibling logs that leave the exit code to the citing doc (the S4 pattern in W17-A05's log audits).

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "r2 in 21.8 s warm" | phase7-exit.md:45 | REAL | log :19 `in 21.80s` |
| "exit 0" | phase7-exit.md:45 | REAL | log :20 `CLIPPY_EXIT=0` |
| "r1 cold 1 m 14 s" | phase7-exit.md:45 | REAL | r1 log :250 `in 1m 14s` |
| (implied) workspace-wide clean | this log alone | SHALLOW-as-standalone | 13/36 members re-linted (F-001); r1 carries the full battery |

## 6. Dead / duplicated / unreachable code
N/A (data artifact).

## 7. Dependencies & flags
N/A — captured output of `cargo clippy --workspace --all-targets -- -D warnings` (:4).

## 8. Open questions for the next agent
- None. No fabrication indicators: header, command, timings and exit code are internally consistent and mutually consistent with r1 and the citing doc.
