# AUDIT: ui/scripts/dev-shots.mjs
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 115 / 5 sections (resolve, parse, seed, capture loop, exit) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A — a developer/agent screenshot harness (node script), run by hand; drives the real dev server + real UI routes. Never shipped. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read e2e/a11y.spec.ts:143-160 (seeding-shape claim verified — same `lumen.onboarding.v1` payload + same `lumen.settings.overrides.v1` key); eslint globals cross-check (scripts glob + inline /* global window */); argument-parsing trace. |

## 1. One-paragraph verdict
A workmanlike screenshot tool with a real quality bar (zero console errors or the run fails; pageerror captured; per-shot console logs written; exit codes 2/1/0 used correctly) and verified test-parity: the onboarding/settings seeding is byte-shape-identical to the e2e a11y suite's (checked). Its defects: a committed hard-coded sandbox path as a playwright-resolution fallback ("../rust-vlc/ui" — the audit sandbox layout baked into repo code, the same class as the full_gate.sh /home/z/my-project root already recorded as S3 in the baseline INDEX), unvalidated --size parsing (NaN viewport), and no pre-flight check that the dev server is up (a missing server surfaces as a raw 20s timeout per pair).

## 2. What actually works (verified)
— Seeding parity with e2e/a11y.spec.ts:143-160 confirmed (complete:true + choices{telemetry,updateChecks,onlineSuggestions}; overrides store key identical). Console-error gate: lines 79-81 collect + 101-107 fail. Baseline-adjacent: the visual/e2e lanes that consume screenshots pass in CI (ui.yml:124-136); the script itself is not in the baseline runs.

## 3. Findings
### F-001 [S3]
Where: ui/scripts/dev-shots.mjs:26-33
Code:
```
// Resolve playwright from the ui workspace regardless of where this copy
// lives (agents run it from /home/z/my-project/scripts/, the durable copy
// lives in ui/scripts/).
let chromium;
for (const base of [
  resolve(import.meta.dirname, ".."), // inside ui/scripts/
  resolve(import.meta.dirname, "../rust-vlc/ui"), // sandbox scripts dir
]) {
```
What is wrong: the committed script hard-codes the Lead audit-sandbox layout (`../rust-vlc/ui` relative to a copied location, and the comment names /home/z/my-project/scripts/) — repo code encoding one machine's directory structure. The baseline INDEX already records the same defect class for full_gate.sh ("hard-coded Lead-sandbox path — S3 finding for Wave 17").
Why it matters: the fallback silently rots outside that sandbox; a contributor running a copied script from any other layout gets the opaque "cannot resolve playwright" error; the tool was built for the audit agents, not the product, and lives in the product tree.
What correct looks like: resolve playwright from ui/node_modules via the ui package only, or take a --ui-root flag / env var.
Confidence: HIGH — read.
### F-002 [S4]
Where: ui/scripts/dev-shots.mjs:72
Code:
```
const [w, h] = opt.size.split("x").map(Number);
```
What is wrong: no validation — `--size foo` yields w=NaN,h=undefined → `chromium.newPage({viewport:{width:NaN}})` throws per pair with a playwright-internal error rather than a usage message.
Why it matters: CLI ergonomics; garbage in, cryptic error out.
What correct looks like: validate `Number.isInteger(w) && Number.isInteger(h)` and exit 2 with usage.
Confidence: HIGH.
### F-003 [S4]
Where: ui/scripts/dev-shots.mjs:97
Code:
```
    await page.goto(`http://localhost:${opt.port}${route}`, { waitUntil: "networkidle", timeout: 20000 });
```
What is wrong: no pre-flight check that the dev server is listening — with the server down, every pair burns a 20s timeout and reports "FAILED: net::ERR_CONNECTION_REFUSED" per shot; a 10-route run wastes 200s before telling the user what one fetch would have said instantly.
Why it matters: developer time; the usage block doesn't state the server prerequisite either.
What correct looks like: one `fetch`/socket probe of localhost:port before the loop, exit 2 with "start `npm run dev` first".
Confidence: HIGH — read; not executed (no dev server in sandbox; run prohibited).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| playwright resolution loop | 29-45 | REAL | S3 (F-001) | createRequire trick works; fallback path is the sandbox hardcode |
| argv parse | 47-71 | REAL | S4 (F-002 at 72) | unknown-option + bad-pair exits correct |
| size parse + mkdir | 72-73 | REAL | S4 | |
| capture loop (seed/visit/shoot/gate) | 75-113 | REAL | S4 (F-003) | addInitScript seeding parity verified; console gate real |
| exit | 114-115 | REAL | — | browser.close before exit ✓ |

## 5. Test honesty
No tests for this script (a dev tool). The dev-shots-drift-check.mjs (`npm run dev-shots:check`, package.json) is the committed drift gate — another agent's file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "same shape the e2e a11y suite seeds" | dev-shots.mjs:18-19 | REAL | a11y.spec.ts:143-160 byte-compared |
| "logs console errors to stdout (quality bar: zero)" | dev-shots.mjs:20-21 | REAL | 101-107: any console error → ✗ + failures++ |

## 7. Dead / duplicated / unreachable code
None. (The sandbox fallback is reachable only in one machine's layout — F-001.)

## 8. Dependencies & flags
node:module/fs/path, playwright (resolved dynamically, F-001). No npm deps declared — correct for a script run inside the ui workspace.

## 9. Open questions for the next agent
- scripts/dev-shots-drift-check.mjs owner: does the drift check validate the seeding shape against a11y.spec.ts automatically (so the parity I verified manually cannot rot)?
- Is there a second durable copy at /home/z/my-project/scripts/ in the repo (the comment implies agents copied it OUT of the tree — if a stray copy is committed anywhere, that's an orphan-file finding for the asset wave).
