# AUDIT: ui/src/components/appshell/appshell.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 554 / 44 selector blocks |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — imported by AppShell.tsx which wraps every non-onboarding route (App.tsx:179, GatedShell). Same W14 caveat as all UI files: the tauri host binary cannot compile at HEAD (build-desktop-tauri.log.txt exit 101), so no shipped desktop binary exists; browser/dev + vitest runs are the only proven executions (ui-vitest.log.txt 1705 passed). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg -o 'app-shell__[a-z-]+' AppShell.tsx | sort -u (43 classes rendered); rg over src --glob '*.css' for sidebar-toggle/sidebar-back/banner-cluster--end/winctl (0 rules — dead hooks); sed -n '55,80p' theme/lumen.css (duplicate body reset); sed -n '640,700p' AppShell.tsx (active-state mechanism); token existence check (all consumed vars exist incl. --radius-card). |

## 1. One-paragraph verdict
Real, frame-measured shell stylesheet; every styled class is rendered by AppShell.tsx and all 20+ consumed tokens exist in the generated token set. The reverse is not true: five class hooks rendered by AppShell.tsx (`app-shell__rail-link--active`, `app-shell__sidebar-toggle`, `app-shell__sidebar-back`, `app-shell__banner-cluster--end`, base `app-shell__winctl`) have zero CSS rules anywhere — one is a test-only hook, the rest are naming residue. The file also carries a duplicate global `body { margin: 0 }` that theme/lumen.css:69 now owns (documented as a known duplicate). No correctness defects found in the geometry, the chrome-fade visibility trick, or the reduced-motion handling.

## 2. What actually works (verified)
— rg class cross-check: all 44 selector blocks' classes rendered by AppShell.tsx (43-class list above). Token audit: every `var(--…)` consumed exists in src/tokens/generated/tokens.css (incl. --radius-card, --color-surface-chrome). Baseline `npm test` 1705 passed incl. AppShell suites (ui-vitest.log.txt); `npm run build` OK (ui-build.log.txt).

## 3. Findings
### F-001 [S4]
Where: ui/src/components/appshell/appshell.css:30-32
Code:
```
body {
  margin: 0; /* shell owns the full viewport (see note above) */
}
```
What is wrong: duplicate global reset. theme/lumen.css:66-70 now owns `body { margin: 0 }` for EVERY route (main.tsx:32 imports lumen.css globally); the comment there says "appshell.css carries a duplicate until its owner drops it" — the drop never happened.
Why it matters: two owners of one global rule invites drift (e.g. one adds `overflow: hidden` for the desktop frameless window, the other doesn't); harmless today only because the values are identical.
What correct looks like: delete the body block from appshell.css; the base sheet owns the body.
Confidence: HIGH — read both files.

### F-002 [S4]
Where: ui/src/components/appshell/appshell.css:214-217 (comment) vs AppShell.tsx:661
Code:
```
/* Every rail stop is a ≥44px hit (W26 contract; IconWrapper lg inside).
 * The ACTIVE stop is the orange accent at glyph level — the frames carry
```
```
                    ["app-shell__rail-link", isActive ? "app-shell__rail-link--active" : ""]
```
What is wrong: `.app-shell__rail-link--active` is rendered on the active NavLink but has no rule in this stylesheet or any other (rg across all CSS: 0 hits). The accent the comment promises is actually delivered by `variant={isActive ? "strong" : "default"}` + `IconWrapper active` (AppShell.tsx:667-671) through the icon system. The class exists only so AppShell.test.tsx:246 can assert it.
Why it matters: a class whose sole consumer is a test assertion — production DOM carries a hook that styles nothing; the comment's "at glyph level" is the only hint the CSS does not do it.
What correct looks like: either style the active stop here (e.g. `.app-shell__rail-link--active { color: var(--color-accent-icon) }`) or stop rendering the class and assert on the Icon variant.
Confidence: HIGH — rg evidence; read AppShell.tsx:655-674.

### F-003 [S4]
Where: ui/src/components/appshell/appshell.css (absence) vs AppShell.tsx:548,565,580,589,598,707
Code:
```
            className="app-shell__sidebar-toggle"
```
What is wrong: five more rendered class hooks with no CSS anywhere: `app-shell__sidebar-toggle` (tsx:548), `app-shell__banner-cluster--end` (tsx:565), `app-shell__winctl` base (tsx:580/589/598 — only `--close` variant at css:153 and `--os` at css:145 have rules), `app-shell__sidebar-back` (tsx:707). Same dead-hook pattern as F-002 minus even the test consumer (rg: no test asserts these).
Why it matters: dead naming residue in production DOM; misleads future contributors into styling hooks that don't exist.
What correct looks like: delete the classes or add their intended rules.
Confidence: HIGH — rg across src/e2e for each name.

## 4. Per-function table (selector blocks)
| Selector block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `body` | 30-32 | REAL | S4 (F-001) | duplicate of lumen.css:69 |
| `.app-shell` | 34-39 | REAL | — | 100vh column |
| `.app-shell__banner` | 46-57 | REAL | — | 30px chrome strip |
| `.app-shell__banner-cluster` | 59-64 | REAL | — | |
| `.app-shell__appdot` | 68-75 | REAL | — | accent dot |
| `.app-shell__tick` | 79-84 | REAL | — | |
| `.app-shell__crumb` / `__crumb-label` | 87-100 | REAL | — | |
| `.app-shell__wordmark` | 104-114 | REAL | — | absolute centering, pointer-events:none correct |
| `.app-shell__demo-badge` | 116-129 | REAL | — | contrast fix documented (3.2:1→secondary tier) |
| `.app-shell__window` | 133-137 | REAL | — | |
| `.app-shell__window--os` | 145-148 | REAL | — | separator only |
| `.app-shell__winctl--close.lumen-icon-button:hover` | 153-156 | REAL | — | doubled selector for specificity, documented |
| `.app-shell__frame` | 160-164 | REAL | — | |
| `.app-shell__strip` / `:empty` / `> .lumen-player__controls` | 174-187 | REAL | — | cross-component portal seam (stripContext) |
| `.app-shell__rail` | 195-202 | REAL | — | 52px, spec band 48-56 |
| `.app-shell__rail-list` | 204-212 | REAL | — | |
| `.app-shell__rail-link` (+hover) | 218-231 | REAL | S4 (F-002) | focus floor from a11y/focus.css:28-31 |
| `.app-shell__rail-foot` / `__rail-add` | 238-251 | REAL | — | 44px hit fix documented (scrollHeight 902>900) |
| `.app-shell__sidebar` / `[hidden]` | 257-274 | REAL | — | 283px = 377×0.75 pin |
| `.app-shell__sidebar-head` | 277-284 | REAL | — | |
| `.app-shell__sidebar-title` | 286-296 | REAL | — | |
| `.app-shell__sidebar-search` / `-input` (+placeholder/focus) | 299-323 | REAL | — | |
| `.app-shell__sidebar-meta` | 327-335 | REAL | — | |
| `.app-shell__sidebar-count` / `__sidebar-modes` | 337-347 | REAL | — | |
| `.app-shell__sidebar-modes .lumen-icon-button[aria-pressed=true]` | 352-354 | REAL | — | token remap trick |
| `.app-shell__create` / `__create-label` | 362-373 | REAL | — | honest comment: engine has no create-playlist command, local store is the flow |
| `.app-shell__sidebar-body` / `-empty` / `-list` / `-scroll` | 376-392, 431-434 | REAL | — | |
| `.app-shell__plrow` (+hover/focus) / `__plrow-label` | 398-429 | REAL | — | |
| `.app-shell__plgrid` / `__plcard` (+hover) / `-thumb` / `-label` | 437-478 | REAL | — | |
| `.app-shell__main` / `> .lumen-player` | 482-495 | REAL | — | |
| `@media reduced-motion` ×3 | 497-501, 536-553 | REAL | — | hidden-state rule gets its own kill — correct specificity reasoning |
| chrome-fade `html[data-lumen-chrome='hidden']` + base transitions | 519-534 | REAL | — | visibility 0s/160ms delay trick is correct for tab-order removal |

## 5. Test honesty
No tests in this file. AppShell.test.tsx:246 asserts the dead active class (F-002) — the assertion proves the class name is attached, not that anything is visible; the real visual assertion would need the Icon variant. Colocated suites belong to other agents.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The ACTIVE stop is the orange accent at glyph level" | appshell.css:214-217 | REAL (via Icon system, not this CSS) | AppShell.tsx:667-671 variant/IconWrapper active |
| "no sheet in the global chain zeroes body margin… the reset lives here" | appshell.css:25-28 | FALSE (stale) | lumen.css:66-70 now owns it; this is the duplicate |
| "rail + sidebar adopt .lumen-pane" | appshell.css:16-19 | REAL-UNPROVEN | Panel component mounts class; surface sheet audited by another agent |
| sidebar 283px = frame 377×0.75 pin | appshell.css:262-267 | REAL-UNPROVEN | frame measurement claim; no way to re-measure Figma here |

## 7. Dead / duplicated / unreachable code
Dead hooks: F-002/F-003 (5 class names). Duplicate: body reset (F-001).

## 8. Dependencies & flags
Pure CSS. Consumes only existing generated tokens (verified). Depends on the icon system (IconWrapper/Icon) for the active rail accent and on a11y/focus.css for the rail-link focus floor.

## 9. Open questions for the next agent
- onboarding.css:38-39 `min-height: calc(100vh - 16px)` still compensates a body margin that lumen.css:69 removed globally — 16px short hero? (A10 owns onboarding.css part1; flagged here because appshell.css's reset note is the sibling evidence.)
- Is `app-shell__winctl` (in-app demo window controls, tsx:580-598) actually wired to anything in the demo shell, or are they decorative? (AppShell part2 = A04.)
