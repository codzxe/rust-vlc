# 15 — Git Workflow & Branch Governance

| Field | Value |
|---|---|
| **Status** | Draft v1 — Wave C, Task 1.C15 (planning wave, Lead commits) |
| **Authority** | MASTER-PROMPT (MP) §13 (CI), §15 (multi-agent protocol), §16 (phase gates), §17 (reporting/done); AGENTS.md §3 (hard rules), §4 (worklog), §5 (task IDs), §6 (timeouts) |
| **Consumes** | doc 10 §4 (CI pipeline, required checks, merge queue, evidence store), doc 10 §8 (regression protocol), doc 13 (wave program & task cards — written in parallel Wave C, refs are to its planned structure), doc 14 (risk register, parallel) |
| **Consumed by** | doc 13 (every card's "branch/merge rules" field), all implementation agents, the Lead's merge protocol, CI branch-protection config |
| **Parallel docs** | docs 11–14 are being written concurrently (Wave C); where I cite doc 13's card fields (Task-ID, Owned-files, Depends-on) these follow the AGENTS §2 assignment contract and may shift slightly when doc 13 lands |

## 0. Scope, sources, and current repository state (verified)

This document is the version-control law for a program that will eventually run
150+ agents against one monorepo. It defines: the branching model, commit and
trailer conventions, the planning-phase exception (what we do today), the
implementation-phase cutover, worktree parallelism, the Lead merge gate, tags
and phase gates, artifact/large-file policy, traceability, CI branch
protection, and recovery procedures. It traces every rule to MP §, AGENTS §,
or a sibling doc; choices that go beyond the brief are marked `[PROPOSAL]`;
unverified facts are marked `[VERIFY]` (AGENTS §3.4).

**Current state, checked against the repo (2026-10-19):** branch `main` only,
four commits, no remote branches, no `.github/` yet:

```
9850301 docs(wave-b): plugin architecture, config system, UI/figma fidelity, web/wasm, testing/CI
3139b58 docs(wave-a): architecture, phase0 audit plan, engine plan, codec roadmap, command layer
33f86fa chore: bootstrap repository — governance, design package, master prompt
f6d2ae5 Initial commit
```

All by `Lead Agent` (initial environment commit by `Z User`). Subjects
already follow Conventional-Commits shape; the bootstrap body carries a
`Task ID: 1.0` trailer. The tree holds `docs/01`–`10`, governance files,
`design/figma` (102 SVGs, 75 MB on disk; ~77 MB package per worklog 1.0) and
untracked `tool-results/` scratch (§7.4). We are in the planning-wave
exception (§3); §1 onward is the target model for implementation phases, not
yet in force.

**Operating principles.** (P1) `main` is always green — merge-blocking CI per
MP §13 and doc 10 §4.4. (P2) Agents never merge to `main`; the Lead (or a CI
bot acting on the Lead's recorded review) merges — AGENTS §3.8. (P3) One task
card = one merge. (P4) Every artifact of work is join-keyed by Task ID: card →
branch → commits → worklog → CI evidence → feature-matrix transition. (P5)
History is disposable below the card; evidence and tags are not.

## 1. Branching model

### 1.1 Trunk-based, Lead-gated

- **`main`** is the only long-lived integration branch. It is protected
  (§9.3), receives no direct pushes, and every commit on it passed the full CI
  tier (doc 10 §4.4) plus the Lead review gate (§5). Phase-gate tags (§6) are
  cut from `main` only.
- **`agent/<task-id>`** — short-lived implementation branches, one per task
  card from doc 13's wave program. Task IDs use the AGENTS §5 implementation
  scheme `<phase>-<subsystem>-<n>` (e.g. `3-clock-2`), which is
  slash-free and branch-safe. Lifetime budget: **≤ 7 calendar days** from first
  push to merge or mandatory rebase + status review `[PROPOSAL]`.
- **`phase/<n>-integration`** (e.g. `phase/5-integration`) — created only when
  a phase's waves integrate over multiple days and cards need a stable staging
  point newer than the last `main` state. Cards still pass the full gate onto
  it; the Lead merges `phase/<n>-integration` → `main` as a fast-forward (or a
  single `chore(phase-n): sync` merge commit if a record is wanted)
  `[PROPOSAL]`. The branch is deleted immediately after `phase-<n>-gate` is
  cut (§6). If a phase integrates inside a day or two, skip it entirely.
- **`release/vX.Y`** — cut from `main` at Phase 8 (and later minors); scope
  freezes: only release engineering, packaging fixes and documentation land
  there. Versioning: `0.<phase>.<n>` during development (e.g. `v0.3.0` when
  Phase 3 closes) and `1.0.0` at Phase 8 completion, when the MP §17
  done-criteria hold (language audit, config-driven rebrand test, UI within
  Figma tolerance, sandboxed plugins, web+desktop playing through one command
  layer, merge-blocking CI, honest parity report) `[PROPOSAL: 0.x scheme]`.
- **`hotfix/<task-id>`** — branched from `release/vX.Y` for a verified defect
  in a shipped build. A hotfix must carry a regression test (doc 10 §8.1), is
  merged back to **both** its release branch and `main` (cherry-pick the
  squash commit; trailers preserved), and produces `vX.Y.Z`. Hotfixes are
  exceptional: anything found pre-release is a normal card on `main`.

### 1.2 Why NOT gitflow

Gitflow (long-lived `develop` + release branches cut from it) is rejected:

1. **Agent parallelism is breadth, not depth.** 150+ agents produce many small
cards (hours-to-days) — the exact shape trunk-based development with short
branches serves. A `develop` sink accumulates integration risk invisibly
   and delays breakage detection.
2. **`main` must be green and anchors the gates.** MP §13 makes CI
   merge-blocking on three OS; MP §16 makes phase gates pass "with recorded
   evidence". Both need one authoritative, always-buildable lineage; with
   gitflow, "done" would mean "done on `develop`" and evidence would fork
   between two trunks.
3. **Small tasks need cheap merges, not release ceremony.** Per-batch release
   branches and version bumps on `develop` add Lead review load without adding
   safety the merge queue doesn't already give (doc 10 §4.4).
4. **The Lead gate is a single choke point by design** (MP §15). One
   integration trunk keeps the review queue and the merge queue aligned.

### 1.3 Branch taxonomy

| Branch | Created by | Lifetime | Merged by | Deleted when |
|---|---|---|---|---|
| `main` | — | forever | — | never |
| `agent/<task-id>` | agent (or Lead provisioning) | ≤ 7 days `[PROPOSAL]` | Lead / CI bot on Lead review | after archive (§1.4) |
| `phase/<n>-integration` | Lead | days–weeks | Lead | after `phase-<n>-gate` |
| `release/vX.Y` | Lead at release | until EOL | Lead | never (archived) |
| `hotfix/<task-id>` | Lead | days | Lead | after back-merge |
| `archive/<task-id>` | CI bot on merge | end of phase `[PROPOSAL]` | — | per retention Q2 |

### 1.4 Merge authority and the squash policy `[PROPOSAL]`

Rules: agents never merge to `main` (P2; AGENTS §3.8 "merge to `main` is
always Lead-only"); **one merge per task card** — if a card's diff is too big
for one reviewable merge, the card is split in doc 13, not the merge.

**Squash-merge policy** `[PROPOSAL]`: each card lands on `main` as exactly one
squash commit. The squash message is assembled as: first line
`<type>(<scope>): <subject>` from the branch's best commit (Lead may rewrite),
body = card summary + evidence, then the **mandatory trailer block**:

```
Task ID: 3-clock-2
Refs: docs/13-agent-wave-program.md#card-3-clock-2
Evidence: CI run <id> (merge-group head)
Card-Status: implemented
```

The `Task ID:` trailer **must survive the squash** — the Lead (or merge bot)
is responsible for verifying it is present post-rewrite; a squash commit
without it is a gate failure. Tradeoff: squash erases intra-card granularity (bad for `git bisect` below
card level) in exchange for one revertable unit per card, a card↔commit 1:1
mapping, and a linear `main` (doc 10 §4.4). Mitigation: before deleting the
merged branch, push it to `archive/<task-id>` (retained until the phase's
gate tag; Q2), so intra-card history stays bisectable on demand.

## 2. Commit conventions

### 2.1 Message format (binding for all branches, all phases)

```
<type>(<scope>): <subject ≤72 chars, imperative, lowercase>

<body — required for non-trivial changes, wrapped ~80>

Task ID: <id>
Refs: <card anchor>
Evidence: <commands + results, or CI run id>
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci`,
`chore`, `revert`, `wip` (agent branches only). Scope = crate, workspace area,
or doc (`demux`, `ui-tokens`, `config`, `docs`, `release`). The subject must
not claim verification: "verified" appears only with evidence (AGENTS §3.4).
The current repo already conforms in shape (`docs(wave-a): …`,
`chore: bootstrap …`); the trailer becomes mandatory at cutover (§3.2).

### 2.2 Body requirements for non-trivial changes

The body states **what** changed, **why** (rationale; ADR link if a fixed
decision per MP §2 is touched), and **evidence** — exact commands and actual
results (MP §13 honesty rule; doc 10 §2.4). Trivial commits (typo, format)
may omit the body but never the trailer.

### 2.3 Worklog parity

Every card-closing merge must contain, **in the same merge or an immediate
follow-up `docs(worklog): append <task-id>` commit**, the agent's `worklog.md`
section in the AGENTS §4 format; a card-closing merge without it fails the
Lead gate (§5.1). During planning, parity is realized as: agent writes files,
Lead commits, and the worklog entry exists in the same wave commit (§3.1).

### 2.4 Commit signing `[PROPOSAL: defer]`

**Proposed: defer signing until Phase 8, then sign only annotated tags** (and
optionally the Lead's merge commits). Tradeoff: cryptographic provenance would strengthen the "who merged what"
audit trail (MP §15), but agents are ephemeral and cannot hold long-lived
keys; the equivalent guarantee comes from Lead-only merges, Task-ID trailers,
CI evidence bound to commits, and branch protection. Managing 150 keypairs or
a shared agent key adds failure modes (lost keys, history surgery) without
adding information the evidence chain lacks. Revisit at the first
`release/vX.Y`; SSH signing on the forge (doc 10 §4.1) is the low-friction
path.

## 3. The planning-phase exception (current state) and the Phase 1 cutover

### 3.1 Current protocol (in force today)

Per AGENTS §3.8 ("during Phase 1 planning, agents write files but do NOT run
`git commit` — the Lead reviews and commits"):

1. Each Wave A/B/C agent writes only its assigned `docs/<nn>-*.md`, appends
   its worklog section, and reports; **no agent runs `git commit`**.
2. The Lead reviews the wave's files and lands them as **one wave-attribution
   batch commit** — evidence: `3139b58 docs(wave-a): …` (5 docs) and
   `9850301 docs(wave-b): …` (5 docs) — with the card list and Task IDs in
   the body; per-agent detail lives in `worklog.md`.
3. Exceptions are logged: the 1.A2 precedent (worklog line 109) — an agent
   timed out before reporting; the Lead verified the file, committed it, and
   appended the worklog entry *on its behalf*, explicitly marked. Formalized
   in §10.2.
4. `main` has **no CI yet** (no `.github/` exists); "green" means the Lead's
   review. Acceptable only because planning waves touch docs only; the moment
   code lands, §9 applies.

### 3.2 Cutover to implementation-phase branching (exact steps)

Executed by the Lead at Phase 1 start, in order, each step a commit on
`main` under the still-active planning protocol:

1. **Close planning.** Land final Wave C docs (11–15) and the
   `docs/ROADMAP.md` consolidation as the last batch-planning commits.
2. **Phase 0 gate check.** Verify audit deliverables per doc 02's exit gate;
   if it passes with recorded evidence, cut `phase-0-gate` (§6 format); if
   not, record the gap and proceed — gate tags are never cut speculatively
   (MP §16).
3. **CI skeleton.** Commit `.github/workflows/ci.yml` with doc 10 §4.2's
   minimum viable tier (`lint` hub, `build (linux+wasm)`, `test (linux)`) and
   the `xtask` gate entry point (doc 10 §2.2); confirm one green run on `main`.
4. **Branch protection.** Enable §9.3 on `main` with exactly the checks that
   exist; grow the required list as jobs land (doc 10 §4.4) — never remove a
   check from "required" to unblock a merge `[PROPOSAL]`. Enable the merge
   queue (all-checks mode).
5. **Grant branch rights.** Per AGENTS §3.8, announce in `worklog.md` + doc 13
   that implementation agents may commit to `agent/<task-id>` branches
   (never `main`).
6. **Provision first cards.** Per Phase 1 card: `worktrees/<task-id>` +
   `agent/<task-id>` (§4.1); agents commit with trailers; the full gate runs;
   the first Lead squash-merge (§1.4) proves the pipeline end-to-end.
7. **Sunset the exception.** From here on the Lead makes no batch-commits of
   agent-authored content outside documented recovery (§10.2) and release
   engineering; planning practice survives only for chartered docs-only
   waves.

## 4. Worktrees for agent parallelism

### 4.1 Layout

MP §15: implementation agents work "on separate crates, branches or
worktrees"; `.gitignore` already excludes `worktrees/`. Canonical layout:

```
git worktree add worktrees/<task-id> -b agent/<task-id> main
```

- Every concurrently-running implementation agent gets **its own worktree
  directory** `worktrees/<task-id>` off the repo root, branch `agent/<task-id>`
  based on the **same `main` SHA** (the wave-kickoff commit named in the wave
  card), so merge order is deterministic against a fixed base.
- The main working tree is **reserved for the Lead**: integration, shared-file
  edits (AGENTS §1), hotfixes, tag operations. Agents never work in the main
  tree during implementation phases.
- Each worktree gets its own `target/` and `node_modules/` (working-tree-
  relative, gitignored), so concurrent `cargo` runs don't contend on locks;
  cost is disk (N × build size). Optional shared `sccache` to bound it
  `[PROPOSAL]`.

### 4.2 Merge order and conflict prevention

- **Merge order = dependency order** from the wave card's `Depends-on` field
  (doc 13). The Lead merges cards topologically; independent cards (disjoint
  owned files) may merge in any order — the merge queue serializes them
  safely (§9.2).
- **Conflict prevention is file-ownership, not git mechanics**: agents touch
  only their card's owned files (AGENTS §3.7 scope discipline); shared
  interfaces and root config are Lead-edited (AGENTS §1); a card that must
  touch shared files declares them and is serialized by `Depends-on`. With
  owned-files discipline, conflicts are rare and usually a card-boundary
  violation, not a merge problem.

### 4.3 When plain branches suffice vs worktrees

- **Plain `agent/<task-id>` branch** (no worktree) suffices when the agent
  works asynchronously against the shared remote on a disjoint crate and
  never needs a concurrent local checkout.
- **Worktree is required** when multiple agents run concurrently on the same
  machine and repo (this program's default, and the reason `worktrees/` is
  gitignored), or when an agent needs to build/test while the Lead is
  integrating in the main tree.
- **Neither** during planning waves (§3.1): doc-writing agents share the main
  tree under the Lead-commit protocol.

### 4.4 Cleanup protocol

After a card's merge is green on `main` and the archive ref exists (§1.4):
`git worktree remove worktrees/<task-id>`, then delete the branch. After a
**squash**-merge the branch is not an ancestor of `main`, so `git branch -d`
refuses; the Lead authorizes `git branch -D` only after verifying CI green +
the archive ref. Stale worktrees (> 14 days with no card activity) are pruned
by the Lead after a worklog note `[PROPOSAL: 14-day rule]`; `git worktree
prune` runs on every cleanup; `git worktree list` is the inventory checked
before recovery (§10.2). Never `git worktree remove --force` a dirty tree
without first diffing it — that is potentially lost agent work.

## 5. Merges, reviews, and the Lead gate

### 5.1 Review checklist (every merge, no exceptions)

The Lead (or a CI bot applying the Lead's recorded review) verifies before
enqueuing a merge:

1. **Scope adherence** — the diff touches only the card's owned files
   (AGENTS §2, §3.7); out-of-scope hunks reject the merge.
2. **Tests present and green** — new/changed behavior is covered; the full CI
   tier passes on the merge-group head (doc 10 §4.4; MP §13).
3. **No test weakening** — no test deleted, skipped, or loosened to pass; the
   regression-guard check (doc 10 §8.2) must be green.
4. **No unsafe code** — `#![forbid(unsafe_code)]` intact in application/core
   crates; any unsafe addition requires a prior ADR (MP §9; AGENTS §3.3).
5. **No forbidden dependencies** — `Cargo.toml`/lockfile diffs only from the
   allowlist; cargo-deny/vet/SBOM green (MP §9; doc 10 §4.6).
6. **Lints clean** — fmt + clippy `-D warnings` per the workspace lint block
   (doc 10 §3).
7. **Docs updated** — behavior changes update their owning doc section and any
   ADR in the same merge.
8. **Worklog appended** — the agent's section exists (AGENTS §4; §2.3 here).
9. **Commit hygiene** — Conventional Commit subjects and `Task ID:` trailers
   on every branch commit (§2.1).
10. **Matrix left to the tool** — `feature-matrix.toml` status changed only
    by `tools/parity-report` from CI evidence, never by hand (MP §15; doc 10
    §7.3); a hand-edited status rejects the merge.

### 5.2 Rebase vs merge policy

- Agent branches are **rebased onto `main` before merge** — linear history is
  required on `main` (doc 10 §4.4). The merge queue runs the full tier on the
  queue head, so the final validation is always against current `main`.
- **Never merge red.** A failing branch rebases, fixes, and re-enters the
  queue; there is no "merge and fix forward" path to `main`.
- On `main`: squash commits per card (§1.4); the only non-squash commits are
  the Lead's direct governance/CI/release commits (each still trailer-tagged
  where a card exists) and, if used, the `phase/<n>-integration` sync commits.

### 5.3 Fast-turnaround / staleness rule

Reviews and merges target **≤ 24 h** from "ready" state to merge
`[PROPOSAL]`. A branch is stale when `main` moved underneath it by **> 10
commits**, by any commit touching the card's dependency closure, or when the
merge-group run fails: the owner rebases immediately. At the 7-day age limit
(§1.1) the Lead re-scopes, reassigns, or closes the card as `blocked` with a
reason (MP §17). Stale agent branches are the main conflict source in
trunk-based programs; cheap frequent rebases beat heroic ones.

### 5.4 Conflict ownership

**The later card rebases.** "Later" = higher position in the wave's
dependency-ordered merge list (doc 13). If two cards genuinely overlap, the
Lead either serializes them (merge A; rebase B onto it) or re-splits file
ownership explicitly in doc 13 — overlaps are a card-authoring defect, and
the fix is recorded there, not improvised in the merge.

## 6. Tags and phase gates

- **Gate tags** `phase-0-gate` … `phase-8-gate` (annotated) are cut **only
  when the phase gate passes with recorded evidence** (MP §16: "A phase is
  done only when its gate passes with recorded evidence on each claimed
  platform"). The Lead cuts them; the tag message is the evidence summary:
  gate criteria met, pinned CI run URLs (doc 10 §4.5 pinning),
  `docs/evidence/INDEX.md` entries, a feature-matrix snapshot, and the
  sign-off line. Example shape:

```
tag phase-3-gate
Reliable engine gate: clocks/queues/generations/seek/A-V sync/recovery
Evidence: CI runs #<id-linux> #<id-windows> #<id-macos> (full tier, pinned);
docs/evidence/INDEX.md entries E-…; matrix snapshot docs/evidence/phase-3-matrix.toml
Sign-off: Lead, <date>
```

- **Version tags** `vX.Y.Z`: `0.<phase>.<n>` during development, `v1.0.0` at
  Phase 8 completion per the MP §17 done-criteria (§1.1). Cut on
  `release/vX.Y` (or `main` for pre-release minors).
- **Immutability**: published tags never move and are never deleted. A bad
  tag gets a `phase-N-gate-RETRACTED` companion note and a corrected
  successor; moving a tag is an ADR-class emergency (§10.3). §9.3 enforces
  this mechanically.
- Signed tags once §2.4's deferral lapses `[PROPOSAL]`.

## 7. Artifacts and large files

### 7.1 Design exports — keep in repo `[PROPOSAL]`

`design/figma` is 102 SVGs, 75 MB on disk (~77 MB original package, worklog
1.0). **Recommendation: keep in-repo, no LFS.** (a) SVG is XML text —
diffable and reviewable, which matters because the exports are the pinned
visual source of truth (MP §7) and re-exports are tracked risk (doc 14);
(b) the tree is read-mostly — it changes only on deliberate re-exports, so
LFS's per-fetch bandwidth buys little; (c) LFS adds CI authentication, quota,
and shallow-clone complexity for 150+ agents and every runner. Cost accepted:
~75 MB on every full clone. Budget rule: any **single binary asset > 5 MB** or
**repo growth past 250 MB** triggers an LFS/external-store review
`[PROPOSAL: thresholds]`. Measure the CI clone-time delta once runners exist
`[VERIFY]`; shallow clone (`fetch-depth: 1`) already minimizes it.

### 7.2 Fixture corpus

Follows doc 10 §5: small, legally distributable fixtures in-repo within that
doc's size budget (§5.5), with the licensing ledger; large generated corpora
(malformed/truncated/discontinuous cases) live outside git — content-addressed
store with recorded hashes fetched by CI and `xtask` `[PROPOSAL: mechanics
owned by doc 10 §5.3]`. The fixture corpus, not design, is the realistic
first LFS candidate (§11 Q5).

### 7.3 Generated artifacts — commit them, with a freshness check `[PROPOSAL]`

**Decision: commit generated files** (token CSS/`ui/tokens/generated`, token
Rust constants, schema-generated settings docs) and enforce freshness in CI.
Justification: committed output makes token/schema changes reviewable as the
real diff the UI/build consumes, keeps builds reproducible without running
the generator in every context, and gives `git bisect` accurate states. The
staleness risk that motivates CI-only generation is removed by a
**freshness check**: `cargo xtask tokens --check` (and `schema-gen --check`)
regenerate and fail if the committed files differ (deterministic generation
is a hard requirement of the token pipeline, doc 07 / MP §7). CI-only
generation was rejected: unreviewable effective diffs, worse local runs, and
drift between generated and committed states.

### 7.4 `.gitignore` discipline

The current file (61 lines, verified) already covers: Rust (`target/`,
`*.rs.bk`), Node/UI (`node_modules/`, `dist/`, `ui/coverage/`), WASM
(`**/pkg/`), secrets (`.env*`, `*.pem`, `*.key`, `secrets/`), logs/temp, OS,
editors, agent scratch (`worktrees/`, `*.pid`, `skills/`, `upload/`,
`download/`, `agent-ctx/`), and packaging outputs (`*.dmg`, `*.msi`, …).
Rules going forward: (1) any new build-output directory gets its `.gitignore`
entry **in the same merge** that introduces it; (2) CI runs a clean-worktree
check (`git status --porcelain` empty after build) `[PROPOSAL]`; (3) add
`tool-results/` (untracked scratch, §0) at the next Lead commit `[Q7]`;
(4) generated-but-tracked files (§7.3; `docs/evidence/INDEX.md`, doc 10
§4.5) are the only deliberate exceptions — freshness-checked, never
hand-edited.

## 8. History integrity and traceability

### 8.1 The traceability chain

Every feature is traceable end-to-end, joined by Task ID (P4):

**card** (doc 13: Task-ID, Owned-files, Depends-on, acceptance) →
**branch/worktree** `agent/<task-id>` (§4) → **commits** with `Task ID:`
trailers (§2.1), squash-preserved (§1.4) → **worklog section** (AGENTS §4) →
**CI evidence run** (doc 10 §4.5: artifacts + evidence-index entries bound to
the merge-group head) → **feature-matrix transition** by `tools/parity-report`
(doc 10 §7.3) — the only authority allowed to set `verified` (MP §15: "Agents
never mark their own work verified"). A status transition without this chain
is invalid (§5.1 item 10).

### 8.2 `git bisect` hygiene

`main` is always green (P1) and squash granularity means **one commit per
card** — ideal bisect boundaries. Intra-card investigation uses the
`archive/<task-id>` refs (§1.4). On agent branches, intermediate commits
should be "green-ish" (at minimum `cargo check`-able); the full gate is not
run per-commit on branches (cost), acceptable because branches are disposable
and only the squash result is permanent. Knowingly red intermediate states on
`main`-bound branches carry a `wip` prefix (§2.1).

### 8.3 Changelog generation `[PROPOSAL: git-cliff]`

**Proposal: `git-cliff`**, driven by the Conventional Commits history, with
the `Task ID` trailer surfaced for per-card attribution. Generated
`CHANGELOG.md` is committed by the Lead at release points only (0.x phase
releases, 1.0), not per-merge — per-merge changelogs would duplicate the
worklog. Alternatives (hand-written changelog, `conventional-changelog`)
were rejected for maintenance cost / Node toolchain drag respectively.

## 9. CI integration

### 9.1 Required status checks before merge

Exactly the doc 10 §4.4 list — all must pass on the merge-group head:
`lint`, `ui`, `build (linux|windows|macos|wasm)`, `test (linux|windows|macos)`,
`contracts`, `plugin-conformance`, `audits`, `config-validate`,
`matrix-validate`, `visual` ×3 OS, `packaging+launch` ×3 OS, `fuzz-smoke`,
`regression-guard`. The fast-PR tier (doc 10 §4.3) gives authors signal but
gates nothing. Until a check exists, it cannot be required (§3.2 step 4);
after it is required, it is never quietly un-required.

### 9.2 Merge queue usage

GitHub merge queue, **all-checks mode**, per doc 10 §4.4: a queued merge
re-runs the full tier against the evolving `main` head; a failing batch
member is ejected and the rest re-queued `[PROPOSAL]`. Flow: agent pushes
`agent/<task-id>` to origin → Lead review (§5.1) → Lead (or bot acting on
the Lead's recorded approval) opens the PR and enqueues it → queue validates
→ squash-merge lands on `main` with the trailer block (§1.4) → archive ref
pushed → branch/worktree cleaned (§4.4). Agents never hold queue rights.

### 9.3 Branch protection rules (concrete GitHub settings `[PROPOSAL]`)

On `main` (and mirrored to `phase/*`, `release/*`):

1. Require a pull request before merging; **1 approval** — approvals
   restricted to the Lead via CODEOWNERS (`* @lead`) `[PROPOSAL]`.
2. Require review from Code Owners; dismiss stale approvals on new pushes.
3. Required status checks = §9.1, **strict** (branch must be up to date).
4. Require merge queue (all checks).
5. Require linear history.
6. Block force pushes; restrict deletions.
7. Restrict pushes that bypass PRs to the Lead only; zero routine admin
   bypass — any bypass is a §10.3 recovery event with a mandatory worklog
   entry within 24 h.
8. Require signed commits: **off** during 0.x (§2.4) `[PROPOSAL]`.
9. Tag protection rules: `v*` and `phase-*-gate` — maintainer-only creation,
   no deletion or update (§6) `[PROPOSAL]`.

Generic-forge equivalents (self-hosted Gitea/GitLab): protected branch with
allowlist = {Lead}, required pipelines = §9.1, linear history enforced,
merge via queue or serialized Lead merges — see Q4.

### 9.4 Force-push policy

**Never** on `main`, `phase/*`, `release/*`, or any protected tag. **Allowed
on the agent's own `agent/<task-id>` before review starts** (keeping branch
history clean is encouraged); once review has begun, a force-push requires
the Lead to re-request review (approvals auto-dismiss, §9.3 item 2).
Mechanics: `git push --force-with-lease` only, and never force-push a branch
that is checked out in another worktree (§4.4).

## 10. Recovery procedures

### 10.1 Reverted merge

If a landed card must come out: `git revert <squash-sha>` (a single, clean
revert — the squash policy's payoff), message `revert: <original subject>`
with trailers `Task ID: <same id>` and `Reverts: <sha>`. Then: the card is
**reopened** in doc 13 (status `in_progress`, or `blocked` with reason per
MP §17); the Lead appends a worklog entry documenting the revert and its
cause; `tools/parity-report` rolls back any matrix statuses whose evidence
referenced the reverted builds, and the evidence-index entries are marked
superseded (doc 10 §7). A re-land of the same work is a **new card and new
branch** from current `main` — never a revert-of-the-revert on a stale base.

### 10.2 Lost agent work (the 1.A2 precedent)

Planning-phase precedent (worklog 1.A2): an agent timed out before reporting;
the Lead inspected the file, confirmed completeness, committed it, and
appended the worklog entry marked "Lead verified output and appended this
entry on its behalf". Implementation-phase procedure: (1) `git worktree list`
+ `git status` + `git stash list` in `worktrees/<task-id>`; (2) salvageable
state is committed by the Lead to the agent's branch with the normal trailers
plus `Recovered-By: Lead after agent timeout (worklog <id>)`; (3) the card
stays `in_progress` with a continuation card in doc 13 (AGENTS §6: partial
work is reported, never silently truncated); (4) if nothing is salvageable,
the card is reissued and the event logged in worklog + doc 14. Never salvage
silently.

### 10.3 Corrupted branch / force-push accident

- **Agent branch**: recreate from `main` (or from `archive/<task-id>` /
  origin) — branches are disposable by design (P5); no ceremony.
- **`main`**: protection should make this impossible (§9.3); if it happens:
  freeze merges; recover the last good SHA from reflog, runner clones, or the
  newest tag; **prefer revert-forward** for merely-wrong commits. True history
  restoration (`git push --force <good>:main`) is an ADR-class emergency:
  Lead-declared, worklog-logged, followed by an all-agents resync
  (`git fetch && git reset --hard origin/main`) and a doc 14 entry.
  `phase/*` and `release/*`: same procedure.

### 10.4 Secret committed

Assume compromise (MP §12 posture). Order: **(1) rotate the secret first**;
(2) purge history — `git filter-repo` (or BFG) across all branches and tags,
coordinated force-push, tag recreation, re-clone by every agent and CI;
(3) prevention gate: secret scanning (`gitleaks`) in the lint hub
`[PROPOSAL]`; (4) worklog + risk-register entry. The existing `.gitignore`
already blocks `.env*`, `secrets/`, `*.pem`, `*.key` (§7.4); the residual
risk is hardcoded secrets in source, which the scanner covers. Evidence
referencing pre-purge history is re-pinned (doc 10 §4.5).

## 11. Open questions for the Lead

1. **Squash vs rebase-merge** (§1.4): squash gives card-granular `main` and
   single-commit reverts; rebase-merge preserves intra-card history without
   archive refs. Confirm squash + archive refs.
2. **Archive retention** (§1.4): until the phase gate tag, until 1.0, or
   forever? Storage grows with wave concurrency.
3. **Signing** (§2.4): confirm deferral + Phase 8 tag-signing; signed
   phase-gate tags earlier?
4. **Forge** (§9.3): doc 10 §4.1 proposes GitHub Actions; if self-hosted is
   possible, merge-queue equivalents need re-specification.
5. **LFS cutover** (§7.1–7.2): confirm 5 MB/250 MB thresholds; measure CI
   clone-time of the 75 MB design tree once runners exist `[VERIFY]`.
6. **Queue throughput** (§9.2): only-Lead approvals may queue deeply at high
   wave concurrency (doc 13) — batch-size tuning or an approved bot?
7. **`tool-results/`** (§7.4): add to `.gitignore` at the next Lead commit?
8. **`phase/<n>-integration` gating** (§1.1): full merge-queue gate onto the
   integration branch, or Lead-verified only (queue for `main` alone)?
9. **Hotfix ownership**: which Phase 8 card owns the release/hotfix runbook
   (channels, signing, packaging) §1.1 assumes?
10. **Changelog** (§8.3): confirm `git-cliff`; `CHANGELOG.md` at 0.x releases
    or only 1.0?

---

*End of doc 15. Traceability: this document implements MP §13/§15/§16/§17 and
AGENTS §3–§6 for version control; it consumes doc 10 §4/§7/§8 and doc 13's
card model. Changes to this doc are Lead-gated like any other.*

