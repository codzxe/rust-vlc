# AUDIT: ui/src/screens/player/player.css
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 606 / ~43 class selectors |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: imported by PlayerScreen.tsx:136 (`import "./player.css"`) -> /player route |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python class cross-reference (exit 0); rg/sed quote checks (exit 0) |

## 1. One-paragraph verdict
Clean, token-driven layout CSS for the player screen: every color is a `var(--…)` token except two light-theme `#2b2b2b`/`#eaeaea` overrides, each guarded by a stylelint-disable comment citing the frame and the "picture-frame rule". All 43 classes cross-reference to live TSX (the two `--left/--right` modifiers come from the template literal `lumen-player__panel--${side}` at PlayerScreen.tsx:1532). One documented layering violation: a cross-sheet selector into appshell.css classes lives here because the lane contract forbade editing appshell.css — working, but admitted debt. The `:has()`-based rules (L603) depend on webview support that I could not verify in this sandbox.

## 2. What actually works (verified)
— python cross-reference: 43 classes, 0 truly unused (41 literal + 2 template-constructed).
— `.app-shell__rail` target exists (appshell.css:195); `.app-shell__sidebar`/`__frame` exist (appshell.css:160-174).
— `data-sidebar` attribute consumed at L603 matches PlayerScreen.tsx:950 `data-sidebar={sidebarVisible ? "open" : "closed"}`.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/player.css:597-606
Code:
```
 * lane's file (k05 / Lead own it — contract §4.2 forbids this lane from
 * editing it). Handoff logged in docs/waves/w26/logs/w26.k06.md: adopt this
 * rule into appshell.css keyed on the shell's own data hook, delete it
 * here. Until then it stays — working, provenance-commented, the only
 * cross-sheet reach in this file. */
.app-shell__frame:has(> .app-shell__sidebar[hidden]):has(.lumen-player[data-sidebar="closed"])
  > .app-shell__rail {
```
What is wrong: a player-lane stylesheet reaches into appshell DOM (`app-shell__frame/rail/sidebar`), coupling two lanes' CSS; the file itself flags it as debt awaiting the appshell lane. Also stacks two `:has()` pseudo-classes — webview support UNVERIFIED (no browser in sandbox; if unsupported the Player-6 rail-hidden state silently fails). Confidence: HIGH (debt) / UNVERIFIED (runtime `:has()` support) — Verification: read + rg.

## 4. Per-function table (selector groups)
| Selector group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-player` root + `--strip` neutralizer | 34-60 | REAL | - | strip host reuses class; neutralization verified against portal use (PlayerScreen.tsx:1198) |
| chrome `[data-chrome-part]` fade + reduced-motion | 62-84 | REAL | - | instant-switch under prefers-reduced-motion as documented |
| dock sidebar + head/meta/create/rows/search/modes/grid | 86-368 | REAL | - | all matched to PlaylistsDock JSX |
| `.lumen-player__body/__main/__topbar/__title` | 370-413 | REAL | - | topbar only for custom layouts (PlayerScreen.tsx:988-997) |
| `.lumen-player__sr-title`/`__sr-note` | 415-427, 571-582 | REAL | - | sr-only clip pattern, correct |
| `.lumen-player__canvas` + light override + `__media` | 429-461 | REAL | - | picture-frame hex override justified; media = MediaSurface video |
| `.lumen-player__now` identity + light pins | 463-514 | REAL | - | contrast reasoning documented (1.3:1 fix) |
| `.lumen-player__controls` band | 516-533 | REAL | - | 44px structural |
| `.lumen-player__panel(--left/--right/[data-emphasized])` | 535-569 | REAL | - | template-constructed modifiers |
| `.app-shell__frame:has(...)` rail rule | 584-606 | REAL-UNPROVEN | S4 | F-001 |

## 5. Test honesty
No CSS-specific tests; layout classes exercised via PlayerScreen.test.tsx (baseline ui-test.txt 1705 passed). Frame-measurement claims (377px→283px etc.) UNVERIFIED — no renderer.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (§3.2); raw px appears solely for structural dimensions" | L31-32 | TRUE | rg: hex only at L460, L509, both stylelint-disabled w/ reason |
| "the only cross-sheet reach in this file" | L601-602 | TRUE | rg: only L603-606 targets app-shell classes |

## 7. Dead / duplicated / unreachable code
None dead. The `:has()` rules are conditionally dead if the webview lacks support (UNVERIFIED).

## 8. Dependencies & flags
Tokens only; no flags. Cross-file dependency on appshell.css class names (F-001).

## 9. Open questions for the next agent
- appshell lane: was the L603 rule ever adopted into appshell.css as the handoff log promised? (rg appshell.css for `data-sidebar` — not present as of this audit.)
