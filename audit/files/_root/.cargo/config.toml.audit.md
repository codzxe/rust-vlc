# AUDIT: .cargo/config.toml
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-21 |
| Lines / functions | 15 / 1 config key ([alias].xtask) + 2 provenance comment blocks |
| File verdict | REAL (the alias is real and executed in the baseline; the no-target-dir policy is real and honored; the provenance comment names the wrong villain commit) |
| Reachable from shipped entry point? | N/A (build config, not code). Executed reachability: YES — baseline runs #8–16 all invoke `cargo xtask <cmd>` (feature-audit, parity-report, config-validator, check-architecture, fixtures --check, lint-literals, check-identity, sbom --check, web-budget --check) and produce real command output (INDEX.md lines 18–27). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `git log --oneline --follow -- .cargo/config.toml` (3 commits) · `git show 3116aecd -- .cargo/config.toml` (EMPTY diff — the blamed commit never touched the file) · `git show 3116aecd:.cargo/config.toml` and `3116aecd^:` (both contain the `[build] target-dir = "/home/z/my-project/target"` content) · `git log --all --oneline -S 'target-dir = "/home/z/my-project/target"' -- .cargo/config.toml` (introduced by 62c48b61, removed by 8bd872a7) · `git show 8bd872a7 -- .cargo/config.toml` (the restore diff) · `rg -n 'xtask' .github/workflows/ci.yml` (note 1 at lines 71–81; expanded form at 238/242/278) · `git ls-files \| rg '\.cargo/config'` (no nested cargo configs) |

## 1. One-paragraph verdict
A two-line config with a thirteen-line history lesson, and the config half is impeccable: the `xtask` alias is the canonical runner, executed nine times in the wave-0 baseline with real output, and the deliberate absence of `[build] target-dir` (the Lead-sandbox path that once broke fresh checkouts) is verified — the file contains only the alias. The history lesson gets its archaeology wrong: it blames commit `3116aec` for overwriting the file and hard-coding the sandbox path, but `git show 3116aecd -- .cargo/config.toml` is an empty diff — the overwrite entered the tree via the W5-CLOSE merge `62c48b61` (Oct 23), two days before `3116aecd` (Oct 25), and was removed by `8bd872a7` (Nov 9, the restore commit that wrote this comment). The practical content is true; the provenance attribution is false.

## 2. What actually works (verified)
- `[alias] xtask = "run -p xtask --locked --"` is live and executed: baseline #8–16 (`cargo xtask feature-audit` … `cargo xtask web-budget --check`) all ran through it with real results (INDEX.md:18–27).
- "Deliberately NO [build] target-dir": verified — the file's only section is `[alias]`. The baseline used an env override instead (`CARGO_TARGET_DIR` shared, INDEX.md header), exactly the policy the comment prescribes.
- The ci.yml cross-reference is accurate: `.github/workflows/ci.yml:71–81` ("note 1") documents the fresh-clone alias failure and the expanded `cargo run -p xtask --locked -- <cmd>` workaround, and the workflow's run steps (lines 238, 242, 278) use the expanded form — consistent with a file whose restore postdates the workflow's defensive style.
- No nested `.cargo/config.toml` anywhere in the tree (git ls-files) — the root config governs all subtrees, including the workspace-excluded `apps/desktop-tauri` builds (its own manifest path builds still read this config).

## 3. Findings

### F-001 [S4]
Where: .cargo/config.toml:4-7,9-11
Code:
```
# original content (worklog 1.A3) but was dropped when 3116aec overwrote
# the file; restored so fresh checkouts work without CI's expanded
```
and
```
# Deliberately NO [build] target-dir: 3116aec hard-coded the Lead-sandbox
# path /home/z/my-project/target, which broke fresh checkouts and hosted
```
What is wrong: The provenance attribution is false. `git show 3116aecd -- .cargo/config.toml` produces an EMPTY diff — commit 3116aecd (Oct 25, "gate(5.R2-V): Phase-5 exit") never touched this file. The `[build] target-dir = "/home/z/my-project/target"` content entered the tree via the W5-CLOSE merge `62c48b61` (Oct 23; `git log --all -S 'target-dir = "/home/z/my-project/target"'` names 62c48b61 as introducer and 8bd872a7 as remover, and `3116aecd^:.cargo/config.toml` already contains the broken content). The restore commit `8bd872a7` (Nov 9) wrote this comment blaming a commit that merely inherited the breakage.
Why it matters: This comment is the file's entire raison d'être beyond two lines of alias — a provenance record for future maintainers. It names the wrong commit for both the alias drop and the hard-code, sending any future archaeologist to 3116aecd where they will find nothing. In a project whose whole methodology is evidence chains, a wrong chain in the Lead-owned config is exactly the rot the methodology exists to prevent.
What correct looks like: "…dropped when the W5-CLOSE merge (62c48b61, via a wave-5 branch) overwrote the file with a Lead-sandbox `[build] target-dir`…" — or simply cite 8bd872a7's own diff.
Confidence: HIGH — Verification: read + four git commands (empty diff at 3116aecd; pickaxe introducer/remover; parent content; restore diff).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| comment: alias provenance | 3-7 | SHALLOW (wrong attribution) | S4 | blames 3116aec; empty diff there (F-001) |
| comment: no-target-dir rationale | 9-12 | REAL (wrong attribution) | S4 | policy correct and honored; villain wrong (F-001) |
| `[alias] xtask` | 14-15 | REAL, EXECUTED | — | baseline #8–16 ran through it 9× |

## 5. Test honesty
No tests. The alias's functionality is proven by execution: nine baseline xtask invocations with per-command output captured in INDEX.md (runs #8–16). Note for the xtask auditors: those runs are the only executed evidence of the whole tools/xtask surface; their raw logs are NOT committed (evidence gap E-1 in my log).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "[alias] xtask — canonical entry point" | .cargo/config.toml:3-4 | REAL | baseline #8–16 executed `cargo xtask <cmd>` |
| "dropped when 3116aec overwrote the file" | .cargo/config.toml:5-6 | FALSE | 3116aecd diff for this file is empty; introducer was 62c48b61 (F-001) |
| "3116aec hard-coded the Lead-sandbox path /home/z/my-project/target" | .cargo/config.toml:9-10 | FALSE | same: hard-code present at 3116aecd^, introduced by 62c48b61 |
| "Deliberately NO [build] target-dir … override via CARGO_TARGET_DIR" | .cargo/config.toml:9-12 | REAL | file has no [build]; baseline used env CARGO_TARGET_DIR |
| ci.yml note 1 describes the workaround | .cargo/config.toml:6-7 | REAL | ci.yml:71-81 matches verbatim story |

## 7. Dead / duplicated / unreachable code
None. One key, one purpose, no nested configs overriding it.

## 8. Dependencies & flags
The alias pins `--locked`, so every xtask invocation runs against the committed lockfile — the right posture for a gate runner. No other cargo settings (no [net], [term], [build] overrides) — the minimal file is the correct file.

## 9. Open questions for the next agent
1. Which of the 14 wave-5 branches merged by 62c48b61 actually carried the `.cargo/config.toml` overwrite? (`git log 62c48b61^2 -- .cargo/config.toml` on the merge's second parent would pin it — I stopped at the merge; the branch set is large.)
2. The restore commit 8bd872a7 (Nov 9) is authored by `Z User <z@container>` — the sandbox identity, i.e., written during the audit era, not by the original project author. Provenance-conscious auditors should note which "Lead" commits postdate the project handover.
3. full_gate.sh hard-codes its root to `/home/z/my-project` (baseline INDEX tooling-gaps note — S3 for Wave 17): the same Lead-sandbox-path disease this file cured in Oct/Nov. Check whether other scripts still carry sandbox paths.
