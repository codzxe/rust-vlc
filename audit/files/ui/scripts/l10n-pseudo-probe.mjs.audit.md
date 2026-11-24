# AUDIT: ui/scripts/l10n-pseudo-probe.mjs
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 197 / 4 (probeUrl, waitForServer, stopServer, main walk) |
| File verdict | REAL-UNPROVEN (sound logic; not executed — needs vite dev server + chromium, outside sandbox budget) |
| Reachable from shipped entry point? | N/A (dev tooling) — wired as `npm run l10n:pseudo-probe` (package.json:29); output dir gitignored (.gitignore:7-8) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg package.json/.gitignore wiring; rg pseudo.ts bracket transform; read full file |

## 1. One-paragraph verdict
A Playwright clipping/overflow probe that boots a throwaway vite server,
seeds the onboarding gate, and walks 24 routes under the pseudo locale.
The hardening history in the header (port polling, finally-block server
cleanup, guarded process-group kill, first-run seeding, explicit IPv4
binding) describes real defensive engineering, and the code matches the
description. Not executable in this sandbox; three small quality nits
(off-by-one route-count comment, a coverage-gap selector, a misnamed report
field).

## 2. What actually works (verified)
- Wiring: package.json:29 `l10n:pseudo-probe`; output `pseudo-probe/` ignored (.gitignore:7-8) — no stray artifacts in git.
- Server lifecycle: waitForServer polls 250ms/60s cap with die-fast (96-110); stopServer kills the detached group then falls back (113-126); browser close + stopServer in `finally` (192-196) — the leak fixes the header claims are implemented as described.
- Onboarding seed gated per-path so /onboarding renders its real first-run state (146-156) — matches the claimed intent.

## 3. Findings
F-001 [S4]
Where: l10n-pseudo-probe.mjs:50-52
Code:
```
/** The full routable inventory (w26.k20): 12 non-settings renderable routes
 * (dev fixture excluded) + all 13 settings areas — the same truth the
 * visual manifest and the a11y sweep walk. */
```
What is wrong: the array below lists 11 non-settings routes (player,
player/music, miniplayer, playlist, media-info, discovery, convert,
onboarding, keyboard-shortcuts, open-network-stream, no-such-route), not 12.
Either a route is missing from the walk or the comment counts wrong —
"the same truth the visual manifest ... walk" is therefore UNVERIFIED.
Confidence: HIGH — Verification: read + counted.
F-002 [S4]
Where: l10n-pseudo-probe.mjs:175
Code:
```
      for (const el of document.querySelectorAll("main *, [role] *")) {
```
What is wrong: clipped-text detection only inspects elements under `main` or
under any [role] node — copy living in headers/asides/footers outside those
subtrees escapes the probe. Coverage gap, report-only tool, but the gap is
silent. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: l10n-pseudo-probe.mjs:182
Code:
```
      return { pageOverflowX, ellipsizedCount: clipped.length, samples: clipped.slice(0, 4), bracketed: document.body.textContent.includes("[") };
```
What is wrong: the header sells this as "leftover pseudo brackets" (line 12),
but the pseudo transform brackets EVERY string (pseudo.ts:75
`` return `[${core}]`...``), so `bracketed: true` is the EXPECTED state — the
field actually detects "pseudo NOT applied" when false. The name inverts the
meaning. Confidence: HIGH — Verification: read + rg pseudo.ts.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| routes inventory | 53-78 | REAL | S4 | see F-001 |
| probeUrl | 81-93 | REAL | - | 2s timeout, error->false |
| waitForServer | 96-110 | REAL | - | die-fast on server exit |
| stopServer | 113-126 | REAL | - | group kill + fallback, never throws |
| main walk | 128-197 | REAL | S4 | see F-002/F-003; report-only exit semantics (clipping findings never fail the run, documented 38-41) |

## 5. Test honesty
No automated tests for the probe itself (it IS a test tool). Header's five
hardening claims each map to visible code — verified by read (see §2). The
"verified: 200 on IPv4" claim (line 34) is the author's, not mine —
UNVERIFIED.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "walks the full 24-route inventory" | l10n-pseudo-probe.mjs:28-29 | REAL | 24 routes counted in array |
| "12 non-settings renderable routes" | l10n-pseudo-probe.mjs:50 | FALSE | 11 listed (F-001) |
| "clipping findings are REPORT data, not failures" | l10n-pseudo-probe.mjs:38-41 | REAL | exit code only set on load failures (188-191) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@playwright/test chromium (devDependency), node builtins. Dev-only.

## 9. Open questions for the next agent
- Cross-check the route list against the app's actual Router inventory
  (App.tsx) — F-001's off-by-one may hide a missing route (e.g. /dev/tokens
  exclusion wording vs. an 12th real route like /settings/hotkeys).
