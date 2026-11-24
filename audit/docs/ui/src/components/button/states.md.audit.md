# CLAIM AUDIT: ui/src/components/button/states.md
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| REAL 10 / REAL-UNPROVEN 1 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 1 / UNREACHABLE 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Accurate, self-policing component-state documentation: every checkable claim about Button/IconButton/Toggle mechanics verifies against button.css/Button.tsx/IconButton.tsx/Toggle.tsx/switchCore.tsx, including the honest-gaps section that admits what is NOT proven (no real visual snapshot harness). One claim is now STALE-FALSE: "no route renders a loading IconButton yet" — ControlBar.tsx:334 and MiniPlayer.tsx:228 both render `loading={mapped.play.pending}` on IconButton (the play/pause spinner is a live product surface).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Enforced by the colocated interaction tests (CSS-source → semantic-token assertions + DOM snapshots)" | L4-5 | REAL | Button.test.tsx/IconButton.test.tsx/Toggle.test.tsx exist in the directory (ls); baseline vitest passes |
| 2 | Button states rest/hover/pressed/focus/disabled/loading mapped to named tokens | L11-16 | REAL | button.css `[aria-busy="true"]::before` spinner at button.css:215-220 (rg) |
| 3 | "NO frame shows an r30 button pill ... every variant is r6" | L27-28 | REAL | `--radius-control: 6px` (tokens.css:96) |
| 4 | "sizes 28/32/36 (sm/md/lg)" | L35 | REAL | button.css height rules (32px at :42, :244-245 region) |
| 5 | "button.css remaps the token in-subtree so glyphs follow the label token" | L39-42 | REAL | remap selectors present (rg hits in button.css) |
| 6 | IconButton "aria-label mandatory (§4.5)" | L58 | REAL | IconButton.tsx:11 documents + :40 sets `aria-label={label}` |
| 7 | Toggle "knob 26x26 ... Track: 68x32 rx6" | L64-71 | REAL | button.css:362-364 `width: 68px; height: 32px; padding: 3px` with frame comments |
| 8 | "ONE core implementation — switchCore.tsx — now backs BOTH exported names" | L73-75 | REAL | rg: switchCore imported by Toggle.tsx and form/Switch.tsx |
| 9 | "the label TEXT is localized — useSwitchLabels() resolves switch.on/off msgids" | L89-91 | REAL | useSwitchLabels.ts exists in the directory (ls) |
| 10 | "implemented as role='switch' + aria-checked" | L95-97 | REAL-UNPROVEN | switchCore not read directly (not my assignment); Toggle.test.tsx asserts checked states — UNVERIFIED by direct read |
| 11 | "no route renders a loading IconButton yet, so the proof is the CSS-source assertion" | L127-129 | FALSE | ControlBar.tsx:334 `<IconButton ... loading={mapped.play.pending}` and MiniPlayer.tsx:228 — the pending spinner IS a live route surface |
| 12 | "dialog frames pin button width at 100px (consumers should set width/min-width)" | L135-136 | REAL (design note) | no primitive width pin in button.css (spot-read) |

## 3. Contradictions
| This doc says | Other source says | Which is true |
|---|---|---|
| "no route renders a loading IconButton yet" (L128) | ControlBar.tsx:334 / MiniPlayer.tsx:228 render loading IconButtons | The code — doc is stale (claim #11) |

## 4. Fabrication / inflation indicators
None. The doc's "Honest gaps" section proactively lists unproven areas (no visual harness; 3px raw px) — the opposite of inflation. The one stale claim (#11) understates shipped capability rather than overstating it.

## 5. Matrix rows
Not a matrix chunk — N/A.
