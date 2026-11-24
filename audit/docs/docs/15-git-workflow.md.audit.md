# CLAIM AUDIT: docs/15-git-workflow.md (598 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 11 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 REAL / 0 / 0 / 0 / 2 PAPER-ONLY ([PROPOSAL]s not built) / 0 / 3 UNVERIFIED (git history claims) |
| Internal contradictions found | 0 (all staleness is self-timestamped) |

## 1. Blunt verdict
This is governance law for git flow, written 2026-10-19 as "Draft v1" and explicitly
time-stamped ("checked against the repo (2026-10-19)"). Every factual claim I can check
without git is TRUE at its date and mostly still true: design/figma is exactly 75 MB on
disk (105 SVGs today vs 102 claimed — grew by 3), the 1.A2 salvage precedent exists
verbatim in the archived worklog, .gitignore covers the exact listed patterns plus the
Q7 follow-up (tool-results/ now at :89), and §7.3's commit-generated-artifacts policy is
live (themes/generated/tokens.css + crates/theme-tokens/src/generated/tokens.rs +
schemas/generated/{settings-docs.md,settings-help.txt}). The stale parts are all honest
time-capsule drift: "no .github/ yet" (now 11 workflows), ".gitignore 61 lines" (now 121),
"docs/01–10" tree (now far more), the four-commit repo state, and "worklog line 109" for
1.A2 (the worklog was archived; the entry is at archive-2026-10.md:107). Nothing is
fabricated; the commit-list hashes and "green run on main" are UNVERIFIED because git is
out of scope for this audit. One policy observation: five design SVGs individually exceed
5 MB (Player-4.svg = 6.4M) — the doc's own >5 MB trigger is scoped to "binary" assets and
SVGs are XML text, so no self-violation, but the spirit of the budget rule is being
stressed.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Current state, checked against the repo (2026-10-19): branch `main` only, four commits ... 9850301/3139b58/33f86fa/f6d2ae5" | :22-29 | UNVERIFIED (git) | Agents may not run git; hashes recorded only here. Time-stamped claim; current repo state obviously has later history (worklog.md:2298 lines records wave-3+ merges). |
| 2 | "design/figma (102 SVGs, 75 MB on disk; ~77 MB package per worklog 1.0)" | :34-35, :382-383 | REAL (size exact; count grew) | `du -sh design/figma` -> 75M; `find design/figma -name '*.svg' | wc -l` -> 105 (3 added since 2026-10-19). |
| 3 | ".gitignore ... 61 lines, verified ... covers Rust, Node/UI, WASM, secrets, agent scratch (worktrees/, *.pid, agent-ctx/), packaging outputs" | :419-423 | REAL list; line count stale | Current .gitignore = 121 lines; worktrees/ (:66), *.pid (:67), agent-ctx/ (:73) all present as claimed; tool-results/ (:89) — the §7.4(3) recommendation was executed. |
| 4 | "main has no CI yet (no `.github/` exists)" | :201-203 | STALE (was true 2026-10-19) | `ls .github/workflows/` -> 11 live workflows since 2026-10-22 (W3-10, see ci.yml header). Claim was explicitly dated; cutover §3.2 step 3 (commit ci.yml) has happened. |
| 5 | "confirm one green run on `main`" (cutover step 3) | :216-218 | UNVERIFIED | No CI run logs exist in-repo; agents cannot run git/GitHub. |
| 6 | 1.A2 precedent: "an agent timed out before reporting; the Lead ... committed it, and appended the worklog entry ... on its behalf" (worklog line 109) | :197-199 | REAL (line drifted) | docs/worklog/archive-2026-10.md:107-108 verbatim: "Task ID: 1.A2 / Agent: general-purpose (timed out before reporting; Lead verified output and appended this entry on its behalf)". Original worklog.md was archived, shifting the line number. |
| 7 | §7.3 "commit generated files (token CSS ... token Rust constants, schema-generated settings docs)" + freshness check | :405-413 | REAL | themes/generated/tokens.css + crates/theme-tokens/src/generated/tokens.rs exist; schemas/generated/{settings-docs.md,settings-help.txt,settings.d.ts} exist; tokens.rs:11-14 documents the committed outputs; xtask has tokens.rs + schema_gen.rs (`--check` support). Doc's example path `ui/tokens/generated` inaccurate — actual `themes/generated/` — but the doc marks the policy [PROPOSAL] and the substance landed. |
| 8 | Branch protection: "1 approval — approvals restricted to the Lead via CODEOWNERS" | :492-493 | PAPER-ONLY | `.github/CODEOWNERS` does not exist (ls). [PROPOSAL]-marked. |
| 9 | "any single binary asset > 5 MB ... triggers an LFS/external-store review" | :389-391 | Not triggered (definition-scoped) | Five SVGs > 5 MB exist (Player-4.svg 6.4M; Player - Light.svg, Player-1/3.svg, sidebar.svg). Rule text says "binary asset"; SVGs are XML text — no self-violation, but 75 MB in-repo cost is real. |
| 10 | "The current repo already conforms in shape (`docs(wave-a): ...`, `chore: bootstrap ...`)" | :154-155 | UNVERIFIED (git) | Commit subjects not inspectable without git; recorded in :26-28 block. |
| 11 | §10.2 recovery: "Recovered-By: Lead after agent timeout" trailer, 1.A2-style salvage | :538-545 | REAL precedent | Corroborated by docs/worklog/archive-2026-10.md:278 "1 (1.A2) timed out post-write and was salvaged by Lead verification". |

## 3. Contradictions
None. Cross-checks run: doc-15 gitignore claims vs .gitignore:66-89; doc-15 vs doc-10 §4.4
required-checks list (§9.1 copies it verbatim and correctly marks not-yet-existing checks
"until a check exists, it cannot be required" :475-476 — consistent with doc-10's
PAPER-ONLY findings); design package size/count vs disk. The doc never claims the
implementation-phase machinery (merge queue, CODEOWNERS, squash trailers, archive refs)
is live — all [PROPOSAL].

## 4. Fabrication / inflation indicators
None. The doc's strongest honesty device is §5.1 item 10 (:318-320): "feature-matrix.toml
status changed only by tools/parity-report from CI evidence, never by hand ... a
hand-edited status rejects the merge" — the repo's actual practice (agents hand-edit
evidence/notes fields in backfills, statuses moved by Lead PRs) diverges from this rule,
but the doc describes the TARGET model and marks itself "not yet in force" (:37-38).

## 5. Not verified
- All git-history claims (#1, #10, #5): no git in sandbox by rule; not fabricated-flagged,
  just UNVERIFIED.
- Tracked repo size vs the 250 MB trigger: du -sh . = 1000M includes .git (304M) and
  untracked build outputs; tracked-content size cannot be computed without git.
