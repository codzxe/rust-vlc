# AGENTS.md — Multi-Agent Working Protocol

This file is binding for every agent (human or AI) working in this repository.
The authoritative project brief is `docs/MASTER-PROMPT.md`. If anything here
conflicts with it, the MASTER-PROMPT wins.

## 1. Roles

- **Lead agent**: architecture, shared types and interfaces, command and plugin
  contracts, dependency approval, integration order, CI policy, final
  verification, all commits to `main` during Phase 1. Reviews and accepts/rejects
  every assignment.
- **Investigation agents**: produce source-linked reports in `docs/`.
- **Implementation agents**: one per stable subsystem, on separate crates,
  branches or worktrees. Agents never edit the same shared interface or root
  config at the same time.

## 2. Assignment contract

Every assignment states: exact scope, out-of-scope, owned files or crates,
interfaces, acceptance criteria, tests, commands to run, dependency-policy rules,
integration dependencies, and the required final report (files changed, tests run
with actual results, limitations, open issues).

## 3. Hard rules

1. **Clean-room**: no VLC source, comments, assets, logos, shaders, branding.
   Public specifications, observed behavior, independent tests only.
2. **Rust-only engine** (§9 of MASTER-PROMPT). Any dependency you want to add
   must be recorded with language, source, transitive graph, build scripts,
   native links, license, maintenance, provenance — and approved by the Lead
   before use. Never assume "Rust API" means "pure Rust".
3. **No unsafe code** in application or core crates. Platform adapters need an
   approved ADR + documented invariants.
4. **Never fabricate**: no invented test results, no "verified" without
   automated evidence, no invented crate capabilities. Facts you could not
   verify must be marked `[VERIFY]`. Design proposals that go beyond the brief
   must be marked `[PROPOSAL]`.
5. **Never weaken or delete a test to pass it.** Every defect gets a regression
   test.
5a. **Engineering directives are binding** (docs/engineering-directives.md):
   zero-allocation hot paths, bounded memory everywhere, no naive-complexity
   algorithms, dirty-world parsers, no destructive lifecycle shortcuts, no
   synthetic-echo-chamber testing (D-1..D-5); functional reality over ritual,
   no deferring the hardest problem, tribal-knowledge capture, hardware-clock
   realism & binary hot paths, and the four-question pre-submission reality
   check (D-6..D-10); randomized real-media corpus mandate (D-11). Every
   card's final report includes the pre-submission audit checklist (D-0..D-5)
   and answers the D-10 four questions (MEASURE / NOT tested / what breaks /
   who hits it first) — a report without all four answers is not accepted.
6. **Ask, don't invent**: when a requirement is ambiguous, request a
   specification decision from the Lead instead of inventing behavior. Only stop
   and ask for decisions that change an interface, policy, or fixed decision.
7. **Scope discipline**: touch only your assigned files. Shared interfaces and
   root config are edited by the Lead (or by exactly one agent with explicit
   temporary ownership).
8. **Commits**: during Phase 1 planning, agents write files but do NOT run
   `git commit` — the Lead reviews and commits. Later phases may grant branch
   commit rights; merge to `main` is always Lead-only.
9. **Reporting**: your final report to the Lead is ≤200 words: files changed,
   key decisions/proposals, tests + actual results (if any), limitations,
   open questions.

## 4. Worklog protocol (mandatory)

Single shared log: `worklog.md` at the **repository root** (append-only).
Prior entries are archived under `docs/worklog/` (see `docs/worklog/README.md`);
the live log at the repo root remains the one append-only file agents write.

- **Before starting**: read it to understand prior work.
- **After finishing** your Task ID: append a section using exactly this format:

```markdown
---
Task ID: <id, e.g. 2-a>
Agent: <agent name>
Task: <one-line description>

Work Log:
- <concrete step>
- <concrete step>

Stage Summary:
- <key results / decisions / artifacts>
```

Never overwrite or edit another agent's entries. Never create separate log
files (the `docs/worklog/` archive is Lead-maintained bookkeeping, not an
agent log — agents append to the root `worklog.md` only).

## 5. Task ID scheme

`<phase>.<wave><letter>` for planning waves (e.g. `1.A3`), and
`<phase>-<subsystem>-<n>` for implementation tasks (e.g. `3-clock-2`). Wave
program: `docs/13-agent-wave-program.md`.

## 6. Context & timeout discipline

- One document / one crate / one bounded task per agent.
- Read only what your assignment lists (MASTER-PROMPT sections, specific docs).
- If you cannot finish, write what you completed to your file with a
  `## Unfinished` section and report it — do not silently truncate.
