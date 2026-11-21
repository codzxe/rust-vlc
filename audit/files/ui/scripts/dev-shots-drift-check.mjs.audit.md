# AUDIT: ui/scripts/dev-shots-drift-check.mjs
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-11-21 |
| Lines / functions | 76 / 3 (main flow, sha256 helper, candidate resolution) |
| File verdict | REAL (code) — a drift guard that nothing invokes; passes vacuously in this checkout |
| Reachable from shipped entry point? | N/A (dev tooling). Reachable only as `npm run dev-shots:check` (ui/package.json:30) — invoked by NO workflow, NO gate script, and NOT in the orchestrator's baseline check set (baseline row 24 runs tokens/settings/hotkeys/l10n only). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg `dev-shots:check` repo-wide → only package.json:30, the script itself, and the w26.k20 log (no .github/ hit); rg the check family in .github/ + scripts/ → only l10n:check is CI-wired (ui.yml:116-118); `node scripts/dev-shots-drift-check.mjs` (read-only execution) → "OK … no agent-facing outer copy in this checkout — nothing to drift", exit 0; ls /home/z/my-project/scripts/ → no dev-shots.mjs (outer copy absent) |

## 1. One-paragraph verdict
The script does what it says — hashes the repo copy of dev-shots.mjs, compares against the sandbox's agent-facing outer copy, exits 1 with a refresh instruction on divergence — and its candidate resolution (env override → sandbox sibling → vacuous pass with notice) is correct. The problem is deployment, not code: nothing runs it. It is wired as an npm script that no workflow, no gate, and no baseline lane invokes (verified: the entire `:check` family except `l10n:check` is absent from .github/workflows and scripts/), and in this checkout the outer copy it exists to guard does not exist, so even a manual run takes the vacuous-pass branch. A drift guard that nothing invokes, guarding a copy that is not there, is documentation shaped like a check.

## 2. What actually works (verified)
— verified by execution (`node scripts/dev-shots-drift-check.mjs`, read-only) → exit 0, correct notice output, repo hash printed: the happy path runs.
— verified by reading l.31-50 → candidate resolution is robust: env override filtered for non-strings/empty, missing candidates caught and skipped, first existing candidate wins.
— verified by reading l.60-76 → divergence path prints both paths + both hashes + the exact refresh command and exits 1 — the diagnostic would be actionable (if it could ever fire in a checkout that has both copies).
— verified: ui/package.json:30 wires `dev-shots:check` exactly as the header claims (l.20-21).

## 3. Findings
### F-001 [S3]
Where: ui/scripts/dev-shots-drift-check.mjs:20-21 (claim) vs repo-wide invocation search
Code:
```
// Wired as `npm run dev-shots:check` (the tokens:check/settings:check/
// hotkeys:check/l10n:check family).
```
What is wrong: The wiring is a definition, not an invocation. rg across .github/workflows/ and scripts/ finds ZERO callers of `dev-shots:check`; the orchestrator's baseline (row 24) ran tokens/settings/hotkeys/l10n — not this one. The same search shows the "family" itself is mostly unwired: only `l10n:check` runs in CI (ui.yml:116-118); `tokens:check`, `settings:check`, `hotkeys:check` are as caller-less as this script. So the committed generated artifacts (tokens.css copies, settings screens, hotkey data, dev-shots) can drift with fully green CI — the one-time baseline PASS is the only enforcement that ever happened.
Why it matters: The script's whole purpose ("fails loudly the moment the two diverge", l.11-12) is conditional on a human remembering to run it. The repo's own drift history (dev-shots was FORKED once already — that is why this script exists per w26.k20/j20 P2-6) shows the failure mode is real, and the mitigation is not actually deployed.
What correct looks like: add the `:check` family to ui.yml's check job (one line each) or to scripts/full_gate.sh.
Confidence: HIGH — Verification: repo-wide rg (workflows + scripts + package.json); baseline INDEX row 24 read.
(Filed S3: an unenforced guard is a quality/process defect with concrete drift history behind it.)

### F-002 [S4]
Where: ui/scripts/dev-shots-drift-check.mjs:52-58
Code:
```
if (outer === null) {
  console.log(
    `OK dev-shots:check — repo copy canonical (sha256 ${repoHash.slice(0, 12)}…); ` +
      "no agent-facing outer copy in this checkout (CI/fresh clone) — nothing to drift.",
  );
  process.exit(0);
}
```
What is wrong: In the current wave sandbox the outer copy does not exist (/home/z/my-project/scripts/ contains baseline.sh, citecheck.py, … but no dev-shots.mjs — verified by ls), so the only branch ever exercised here is the vacuous pass. The FAIL branch (the entire reason for the script) has never been observed to run — there is no test for the script, no fixture with two divergent copies, and no recorded execution of the divergence path.
Why it matters: An untested failure path in a tool whose only job is that failure path. If the comparison logic had a bug (wrong candidate order, hash of the wrong file), nothing would ever notice.
What correct looks like: a tiny vitest case (or a fixtures dir with two divergent copies + an env override) asserting exit 1 and the diagnostic.
Confidence: HIGH — Verification: executed the script in this checkout; ls of the outer location.

### F-003 [S4]
Where: ui/scripts/dev-shots-drift-check.mjs:36-39
Code:
```
const sha256 = (path) =>
  createHash("sha256").update(readFileSync(path)).digest("hex");

const repoHash = sha256(repoCopy);
```
What is wrong: The repo copy's read is unguarded — a missing ui/scripts/dev-shots.mjs (the file this guard protects) crashes with a raw ENOENT stack trace and node's default exit, instead of the script's diagnostic style. The candidate loop guards the OUTER reads (l.42-49) but not the canonical one.
Why it matters: The one input the script assumes inviolable gets the least graceful failure; a confused contributor sees a stack, not an instruction.
What correct looks like: try/catch around repoCopy with a "canonical copy missing" FAIL diagnostic and exit 1.
Confidence: HIGH — Verification: read; node's behavior on uncaught exceptions is definitional.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| path setup (`uiRoot`/`repoCopy`) | 28-29 | REAL | — | resolves against the script's own location — layout-independent |
| `outerCandidates` | 31-34 | REAL | — | env override first, non-string/empty filtered |
| `sha256` | 36-37 | REAL | S4 | F-003 (no error handling; also EISDIR if pointed at a dir via env) |
| outer resolution loop | 41-50 | REAL | — | missing candidates skipped correctly |
| vacuous-pass branch | 52-58 | REAL | S4 | F-002 (the only branch exercised in this checkout) |
| identical-pass branch | 60-66 | REAL | — | correct output; unexercised here (no outer copy) |
| FAIL branch | 68-76 | REAL | — | actionable diagnostic + exit 1; never observed running; untested |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (no test exists for this script) | NO | zero coverage: no vitest case, no fixture pair, no CI invocation — the script's three branches are validated only by the one manual execution documented in §2 |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "This check fails loudly the moment the two diverge" | l.11-12 | REAL (code) / PAPER-ONLY (deployment) | FAIL branch correct; nothing invokes the check (F-001) |
| "Two durable copies … exist BY DESIGN" (repo + sandbox outer) | l.5-9 | REAL (per w26.k20 log) / FALSE in this checkout | outer copy absent here (ls) — the design's sandbox layout is not this workspace |
| "Wired as `npm run dev-shots:check`" | l.20-21 | REAL (script exists) / MISLEADING (family framing) | package.json:30 yes; but the "family" is CI-absent except l10n:check (F-001) |
| "the repo copy is the source of truth — refresh the outer copy from it" | l.12-13 | REAL | FAIL branch prints exactly that instruction |

## 7. Dead / duplicated / unreachable code
The script itself is not dead code (it is a tool), but it is UNREACHED by any automation — the deployment-level equivalent. Its subject, dev-shots.mjs, was audited in an earlier wave (audit/files/ui/scripts/dev-shots.mjs.audit.md exists).

## 8. Dependencies & flags
node:crypto, node:fs, node:path, node:url — zero external deps (consistent with the gen-tokens.mjs zero-dep pattern). Env flag: DEV_SHOTS_OUTER_COPY. No feature flags.

## 9. Open questions for the next agent
- Who owns ui.yml / full_gate.sh: the `:check` family gap (tokens/settings/hotkeys/dev-shots absent, l10n present) is a one-PR fix — flag to the orchestrator as a wave-level process finding.
- Does the wave sandbox the CONTRACT §4.7 agents use still exist with the outer copy? If the sandbox layout changed, this guard's premise (and the w26 dual-copy design) is obsolete and the script can be retired with dev-shots.mjs's single-copying.
