# AUDIT: ui/scripts/dev-shots.mjs
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 115 / 1 (top-level script: arg parse, playwright resolve, capture loop) |
| File verdict | REAL-UNPROVEN (tooling; not executable in this sandbox) |
| Reachable from shipped entry point? | N/A — developer tool (wave-26 visual harness screenshots, contract §4.7); not part of any shipped bundle, not wired into CI (rg over .github/workflows: only ui.yml runs l10n/vitest/playwright suites, not this script) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read + `ls ui/node_modules/.bin` (absent -> cannot execute); arg/exit-code path analysis |

## 1. One-paragraph verdict
A real, honestly-instrumented screenshot helper: playwright capture of named
routes against a running dev server, with an honest quality bar — any console
error or pageerror FAILS that shot (written to a log) and the script exits 1.
It seeds the onboarding-complete gate (same shape as the e2e suite) and can
pre-seed theme/accent settings overrides via the localStorage persistence key.
Nothing fake; it is un-runnable here only because ui/node_modules is absent
and npm installs are banned.

## 2. What actually works (verified)
— Code read: exit codes are honest (2 usage errors L60/65/70, 1 on any console
error or navigation failure L101-111, 0 only when all shots clean).
— Playwright resolution handles both install locations with a clear failure
message (L42-45). — Console-error capture wired BEFORE navigation (L80-81).
Execution UNVERIFIED: no playwright binary in this sandbox (ui/node_modules
absent; npm budget rule).

## 3. Findings
F-001 [S4]
Where: ui/scripts/dev-shots.mjs:72, 78
Code:
```
const [w, h] = opt.size.split("x").map(Number);
```
What is wrong: no validation — `--size 1440` yields h = NaN and
`browser.newPage({ viewport: { height: NaN } })` (L78) sits OUTSIDE the
per-pair try (L96), so a malformed size crashes the whole run with a stack
instead of a usage error.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: ui/scripts/dev-shots.mjs:27-32
Code:
```
// Resolve playwright from the ui workspace regardless of where this copy
// lives (agents run it from /home/z/my-project/scripts/, the durable copy
// lives in ui/scripts/).
```
What is wrong: documents a DUPLICATED copy of this script (a sandbox
scripts/ twin); the two copies can drift. The fallback path
"../rust-vlc/ui" (L32) encodes a sandbox layout assumption.
Confidence: MEDIUM — Verification: read; the sibling copy's existence
UNVERIFIED (out of my file list).

## 4. Per-function table
Script is a single top-level flow (parse L47-73, seed L82-95, capture L96-112,
exit L115). No functions to tabulate; >100 LOC but linear tool code — compact
audit per spec §12 allowance is borderline; sections above cover every branch
(7 options, 3 error exits, 2 capture outcomes).

## 5. Test honesty
No tests (dev tool). Its own quality bar (console errors fail) is the honest
part — screenshots cannot silently pass with a broken page.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "logs console errors to stdout (quality bar: zero)" | dev-shots.mjs:20-21 | REAL | L101-104 fail path |
| "seeds `lumen.onboarding.v1` ... same shape the e2e a11y suite seeds" | dev-shots.mjs:18-19 | UNVERIFIED | e2e suite seeding not inspected (out of scope) |

## 7. Dead / duplicated / unreachable code
The documented duplicate copy (F-002). Nothing else.

## 8. Dependencies & flags
node:fs/path/module + playwright (dev dependency, resolved dynamically). No
feature gates. Requires an already-running dev server on --port (does not
start one — caller's job).

## 9. Open questions for the next agent
- Does the e2e a11y suite really seed the identical onboarding gate shape
  (claim, L18-19)? If the shapes drift, gated-route screenshots could render
  differently from e2e.
