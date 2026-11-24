# AUDIT: ui/src/screens/settings/settings.css
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 859 / ~48 top-level selectors + 3 media blocks |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by SettingsScreen.tsx (rg "settings.css" -> src/screens/settings/SettingsScreen.tsx) -> app shell |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | rg class-extraction + consumer scan (48 classes, 2 dead); sed quote verification; rg data-staged consumers |

## 1. One-paragraph verdict
Hand-written, heavily frame-annotated stylesheet for the settings window;
semantic tokens everywhere, raw px only with `frame:` provenance comments. No
stubbing possible in CSS, but two classes are dead (with a comment that lies
about their consumer) and the staged-inert policy deliberately paints
unwired/disabled controls at full opacity — directly relevant to the master
question (settings rows that look live but do nothing).

## 2. What actually works (verified)
- rg consumer scan of all 48 class selectors against non-test TSX: 46/48 referenced; dead ones listed in F-001.
- The two deletion claims in comments check out: `src/screens/settings/areas/advanced.tsx` does NOT exist (comment lines 849-851), and `.settings-field__head`/`.settings-field__hint` are absent from the file (comment lines 683-685, 516-518) — the stated prunes really happened.
- `data-staged`, `settings-area-staged-note`, `settings-screen__empty`, `settings-screen__results`, `data-mount-focus` all have live TSX emitters (areas/audio.tsx, library.tsx, subtitles.tsx, SettingsScreen.tsx, effects.tsx).

## 3. Findings
F-001 [S3]
Where: settings.css:687-716 (with header claim at 15-16)
Code:
```
/* About panel rows (updates-about) — term/value pairs. The <dl> wrapper
 * (w26.d2 a11y restructure, updates-about.tsx) needs only the UA reset. */
.settings-about__rows {
  margin: 0;
}

.settings-about__row {
```
What is wrong: all 6 rules for `.settings-about__rows`/`.settings-about__row`
(+ `__row dt`, `__row dd`, `__row + __row`) have ZERO TSX consumers —
`rg "settings-about__row" src scripts` matches only settings.css itself, and
updates-about.tsx contains no `<dl>`/`<dt>`/`<dd>` at all (it renders
`settings-field` + `settings-about__value` rows). The comment asserts a
consumer that does not exist. The file header (line 15-16) also claims areas
screens "reuse ... settings-about__row classes" — false today.
Why it matters: dead CSS plus a stale comment that misdirects the next
maintainer. What correct looks like: delete the block and the header claim.
Confidence: HIGH — Verification: ran rg over src+scripts.

F-002 [S3]
Where: settings.css:564-567 and 600-604
Code:
```
.settings-action:disabled {
  opacity: 1; /* staged-inert: frame-full ink, never dimmed (Remotes pattern) */
  cursor: not-allowed; /* honest refusal (gfix-E disabled-thumb precedent) */
}
```
What is wrong: unwired settings buttons (comment names "path-row Browse,
privacy Clear, extensions toolbar") render at FULL visual strength when
disabled — the only visible liveness signal is the cursor, which keyboard and
touch users never see. The comment frames this as honesty-by-disabled-state,
but the shipped visual is "live button". This is the repo's own policy on
dressing dead controls as live ones.
Why it matters: for the master question, this is the mechanism that hides
dead buttons from reviewers and users. What correct looks like: at minimum a
visible inert affordance (not-allowed is not discoverable).
Confidence: HIGH — Verification: read + rg data-staged emitters.

F-003 [S4]
Where: settings.css:385-395
Code:
```
.settings-sections[data-settings-area="global"] .settings-section {
  margin-bottom: 9px; /* live: sections end y587.5, bar y611.5 (see above) */
}
```
What is wrong: nothing broken, but the comment block 362-384 admits the
global-area apply-bar sits 33.5px off the frame and defers a row-pitch pass
("do NOT 'fix' it by inflating this margin") — an acknowledged, unreconciled
design drift. Confidence: HIGH — Verification: read.

## 4. Per-function table (>100 LOC; CSS blocks as units)
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| stage/window/topbar | 31-201 | REAL | tokens + frame px, focus rings present |
| body/sidebar/nav | 203-319 | REAL | data-mount-focus ring suppression documented (312-319) |
| content/sections + area-scoped gaps | 321-414 | REAL | measured overrides; see F-003 |
| rows/field grid | 416-518 | REAL | 320px control column; dead-rule deletions verified |
| action buttons | 520-576 | REAL | see F-002 staged-inert |
| about/staged | 578-604 | REAL | see F-001 |
| badges/reset | 606-665 | REAL | |
| advanced disclosure | 718-760 | REAL | |
| floating bars (restart notice, applybar) | 762-802 | REAL | sticky applybar |
| dirty dialog | 804-828 | REAL | |
| responsive + reduced-motion | 830-859 | REAL | advanced.tsx deletion claim verified |

## 5. Test honesty
No tests in this file. Comment claims "Contracts kept (SettingsScreen.test.tsx
+ areas tests)" — SettingsScreen.test.tsx exists; contract specifics
UNVERIFIED here (belongs to the SettingsScreen audit owner).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "areas/* screens reuse ... settings-about__row classes" | settings.css:15-16 | FALSE | no TSX emits the class (rg) |
| "<dl> wrapper (w26.d2 a11y restructure, updates-about.tsx)" | settings.css:688-689 | FALSE | updates-about.tsx has no dl/dt/dd (rg) |
| ".settings-field__head rule is DELETED" | settings.css:683-685 | REAL | selector absent from file |
| "areas/advanced.tsx component was removed by k13" | settings.css:849-851 | REAL | file absent (ls) |
| staged rows "render FULL-STRENGTH while inert" | settings.css:596-604 | REAL | rules live + data-staged emitted by 3 areas |

## 7. Dead / duplicated / unreachable code
`.settings-about__rows`, `.settings-about__row`, `.settings-about__row +
.settings-about__row`, `.settings-about__row dt`, `.settings-about__row dd`
(687-716) — dead, see F-001.

## 8. Dependencies & flags
Consumes design tokens (--color-*, --space-*, --motion-*, --radius-*). No
feature flags. Loaded via SettingsScreen.tsx import.

## 9. Open questions for the next agent
- Who owns the "staged" capability seam? The CSS says Browse/Clear/extensions
  toolbar buttons are intentionally inert; the areas audit should count how
  many settings actions are dead-by-policy vs actually wired.
