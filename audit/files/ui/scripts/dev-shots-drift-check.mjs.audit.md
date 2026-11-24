# AUDIT: ui/scripts/dev-shots-drift-check.mjs
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 77 / 1 script (sha256 helper + main flow) |
| File verdict | REAL — executed clean; a process-tooling script committed into the product repo |
| Reachable from shipped entry point? | NO — dev tooling (npm run dev-shots:check), never bundled into ui/dist |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `node scripts/dev-shots-drift-check.mjs` -> `OK dev-shots:check — repo copy canonical (sha256 9beeb263a80e…); no agent-facing outer copy ... — nothing to drift.` exit 0; ls of the outer path (absent) |

## 1. One-paragraph verdict
Works exactly as documented — I ran it: exit 0 with the honest "no outer copy in this checkout" branch. The logic is sound (content-hash comparison, env override first, canonical repo copy as truth, loud FAIL + fix instructions on divergence, correct exit codes). The only audit-worthy observation is scope: this is wave-process tooling (it exists to keep 260 agent-facing sandboxes from forking the screenshot tool, per its own header) committed inside the PRODUCT repo's ui/scripts — a CI/dev-only script that references a sibling-directory sandbox layout (`<repo>/../scripts`) no product consumer has. Harmless, documented, but it is process scaffolding living in product territory.

## 2. What actually works (verified)
— verified by execution: `node scripts/dev-shots-drift-check.mjs` -> exit 0, output quoted above (the pass-through branch when no outer copy exists, exactly the documented CI/fresh-clone behavior).
— verified by read: exit 1 + stderr message on hash divergence (L68-76); env override resolution ordered before the sandbox-sibling default (L31-34); no unquoted-variable hazards (node, not shell).

## 3. Findings
F-001 [S4]
Where: ui/scripts/dev-shots-drift-check.mjs:5-13
Code:
```
// Two durable copies of the wave-26 agent screenshot tool exist BY DESIGN:
//   * ui/scripts/dev-shots.mjs        — the repo-committed CANONICAL copy
//   * <sandbox>/scripts/dev-shots.mjs — the agent-facing copy CONTRACT §4.7
//     points 260 agents at (outside the repo; /home/z/my-project/scripts/
```
What is wrong: agent-sandbox process tooling committed into the product repo (and the comment hard-codes an absolute sandbox path, `/home/z/my-project/scripts/`, inside a repo file — machine-specific path in committed source). No product impact (script never ships in ui/dist); hygiene debt. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| outerCandidates resolution | 31-34 | REAL | — | env override first |
| existence probe loop | 42-50 | REAL | — | try/catch per candidate |
| sha256 helper | 36-37 | REAL | — | |
| main flow (pass/diverge branches) | 52-76 | REAL | — | executed pass branch; FAIL branch correct by read |

## 5. Test honesty
No tests; the script IS a check. It would fail loudly on real divergence (message includes both hashes + copy command) — self-evident honesty.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "This check fails loudly the moment the two diverge (content hash comparison...)" (L10-13) | header | REAL | read L60-76 |
| "When no outer copy exists (CI, fresh clones), the check PASSES with a notice" (L16-18) | header | REAL | executed — exactly that branch, exit 0 |
| "Wired as npm run dev-shots:check" (L20-21) | header | REAL-UNPROVEN | package.json wiring not re-checked this run (plausible; UNVERIFIED) |

## 7. Dead / duplicated / unreachable code
None in-file. (The script's REASON FOR EXISTING is a documented duplication of dev-shots.mjs across sandboxes — by design, guarded.)

## 8. Dependencies & flags
node:crypto/fs/path/url only. Env override: DEV_SHOTS_OUTER_COPY (read, never printed). Not part of any bundle.

## 9. Open questions for the next agent
- Should wave-process scripts (this + its sibling dev-shots.mjs) move to a tools/ lane outside ui/scripts, given the absolute sandbox path in the comment?
