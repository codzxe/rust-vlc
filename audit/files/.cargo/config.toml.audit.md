# AUDIT: .cargo/config.toml
| Field | Value |
|---|---|
| Auditor | W01-A04 |
| Date | 2025-11-23 |
| Lines / functions | 15 / 1 config key ([alias] xtask) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (dev/CI configuration) — but exercised by every gate-lane invocation: scripts/full_gate.sh:29,36,50,57,64,71 call `cargo xtask ...`, which resolved and ran in the baseline (full_gate.txt sections produced output) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read of the file; rg of scripts/full_gate.sh for `cargo xtask` (6 uses); rg of .github/workflows/ci.yml L65-95 (note 1) + all `cargo run -p xtask` steps; worklog archive check (1.A3 alias record, archive-2026-10.md:714) |

## 1. One-paragraph verdict
A correct, minimal, live cargo config whose single alias is the canonical xtask entry point and demonstrably works in this checkout (baseline full_gate ran `cargo xtask check-architecture/fixtures/lint-literals/check-identity/sbom/web-budget` through it). The comment block accurately describes the file's current state: alias present, deliberately no `[build] target-dir` (verified — the file contains only `[alias]`). One provenance imprecision: the comment says the 1.A3 original alias was "restored", but the worklog's 1.A3 record shows the original was `run -p xtask --` without `--locked`; the restored form adds `--locked` (matching ci.yml's expanded workaround) — a real behavior delta (cargo xtask now refuses to run on a stale lockfile) presented as a restoration. Historical claims about commit 3116aec are UNVERIFIED (git history out of budget).

## 2. What actually works (verified)
- Alias resolution: scripts/full_gate.sh:29-71 use `cargo xtask <cmd>`; baseline full_gate.txt shows those six sections executing (e.g. lint-literals output at full_gate.txt:59-63) — the alias resolves and runs the runner in this checkout.
- File content matches its own claims: only `[alias]` present; no `[build]` section ("Deliberately NO [build] target-dir" — TRUE).
- The ci.yml cross-reference exists as described: .github/workflows/ci.yml:71-85 "note 1" documents the alias loss and the `cargo run -p xtask --locked --` expansion used by every xtask step in CI (ci.yml:238,242,278,280,282,444,462).

## 3. Findings
F-001 [S4]
Where: .cargo/config.toml:3-7 vs :14-15
Code:
```
# [alias] xtask — canonical entry point for the workspace task runner
# (tools/xtask): `cargo xtask <cmd>`. This alias was the file's only
# original content (worklog 1.A3) but was dropped when 3116aec overwrote
# the file; restored so fresh checkouts work without CI's expanded
```
and the actual key:
```
[alias]
xtask = "run -p xtask --locked --"
```
What is wrong: the comment frames the current alias as a restoration of the 1.A3 original, but the worklog's 1.A3 record (docs/worklog/archive-2026-10.md:714) shows the original was `xtask = "run -p xtask --"` — no `--locked`. The restored form adds `--locked`, a semantic change (any `cargo xtask` invocation now hard-fails when Cargo.lock is stale relative to manifests) that is nowhere stated as a change; it silently matches ci.yml's expanded workaround instead. Harmless-to-good policy, imprecisely narrated. What correct looks like: one clause — "restored (with the CI lane's `--locked`)".
Confidence: HIGH — Verification: read (worklog record vs current key).

## 4. Per-function table
Not applicable (15-line config). Key checked: `[alias] xtask = "run -p xtask --locked --"` — valid alias syntax (trailing `--` forwards subcommand args), package name matches tools/xtask (`[package] name = "xtask"`), `--locked` consistent with the repo's locked-lane posture (baseline release lane built `--locked` clean).

## 5. Test honesty
No tests (config file). Functional proof is the baseline full_gate run through the alias (see §2).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| alias present so `cargo xtask <cmd>` works on fresh checkouts (L4-7) | comment | REAL | alias at L14-15; baseline full_gate executed it |
| "Deliberately NO [build] target-dir" (L9-12) | comment | REAL | file has no [build] section (read) |
| alias "was the file's only original content (worklog 1.A3)" (L4-5) | comment | REAL | archive-2026-10.md:714 "(only content; …)" |
| "restored" (faithful) (L6-7) | comment | MISLEADING | original lacked `--locked` (archive :714); F-001 |
| 3116aec dropped the alias / hard-coded /home/z/my-project/target (L5, L9-11) | comment | UNVERIFIED | requires git history (banned for agents); corroborated textually by ci.yml:71-85 note 1 |

## 7. Dead / duplicated / unreachable code
None. Single key, live, used by scripts and developers.

## 8. Dependencies & flags
No dependencies. `--locked` in the alias binds xtask invocations to the committed Cargo.lock (see F-001 for the undocumented delta).

## 9. Open questions for the next agent
- ci.yml owner: notes 1 and 2 (ci.yml:71-85) are stale TODAY — the alias IS restored and no target-dir is hard-coded; the "[PROPOSAL] … then these strings can revert to `cargo xtask ...`" revert was never done (all steps still use the expanded form — functionally identical, textually drift-prone). The notes' premises ("DOES NOT RESOLVE in a fresh clone", "hard-codes build.target-dir") are false against the current tree and should be re-dated or removed.
