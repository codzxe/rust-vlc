# AUDIT: ui/src/screens/discovery/discovery.css
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 120 / 11 selector groups |
| File verdict | REAL (composed-from-grammar stylesheet; all classes live; one stale "pending token" note whose token has already landed) |
| Reachable from shipped entry point? | YES — imported by DiscoveryScreen.tsx → /discovery route → App. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | class sweep (11/11 discovery__* used in DiscoveryScreen.tsx); var sweep (12/12 resolve); rg --color-surface-panel-raised in tokens.css (2 hits — EXISTS) |

## 1. One-paragraph verdict
Solid composed stylesheet for the one route with no direct frame (the derivation from playlist/thumbnail/sidebar grammars is documented per selector). Hostile-text safety on OS volume ids (`max-width: 40%` + `overflow-wrap: anywhere`) is a deliberate, cited guard. Only nit: the light-mode comment says the panel-step token is "pending (theme lane i19)" — `--color-surface-panel-raised` already exists in tokens.css (2 definitions), so the rule now resolves to the real token and its fallback is vestigial; the comment misdates the tree.

## 2. What actually works (verified)
— sweep: 11/11 classes referenced in DiscoveryScreen.tsx; 12/12 vars resolve.
— rg: token exists (tokens.css, dark+light).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/discovery/discovery.css:118-119
Code:
```
:root[data-theme="light"] .discovery__panel {
  background-color: var(--color-surface-panel-raised, var(--color-surface-raised));
}
```
What is wrong: The comment above it (L116-117) describes `--color-surface-panel-raised` as "the pending panel-step token (theme lane i19)" — it has landed (tokens.css defines it twice, dark+light). The fallback chain and the comment are stale bookkeeping; behavior is correct either way. Confidence: HIGH — Verification: ran rg on tokens.css.

## 4. Per-function table (file >100 LOC)
| Selector group | Lines | Verdict | Notes |
|---|---|---|---|
| .discovery .lumen-section-header__title | 18-20 | REAL | documented 16→15px drift pin |
| .discovery shell | 22-30 | REAL | min(680px,100%) column |
| __head/__title/__subtitle | 34-52 | REAL | |
| __section/__count/__count-num | 56-70 | REAL | |
| __panel (+--prose) | 76-84 | REAL | |
| __volume-list/__volume-id | 88-102 | REAL | hostile-text guard (V7) |
| __network-status | 106-110 | REAL | |
| light-mode panel step | 118-120 | REAL | S4 stale comment (F-001) |

## 5. Test honesty
DiscoveryScreen suites in baseline (PASS) cover the screen; CSS is visual-harness territory.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the pending panel-step token (theme lane i19) wins the day it lands" | discovery.css:116-117 | STALE — it landed | tokens.css defines --color-surface-panel-raised |

## 7. Dead / duplicated / unreachable code
The `var(--color-surface-raised)` fallback is unreachable now that the token exists (harmless).

## 8. Dependencies & flags
Card/ListRow/SectionHeader primitives' grammars, token set. No flags.

## 9. Open questions for the next agent
- None.
