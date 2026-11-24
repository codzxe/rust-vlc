# AUDIT: ui/src/components/button/button.css
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 512 / 4 component skins (Button 36-235, IconButton 239-344, Switch core 346-497, motion-off 499-512) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (Button.tsx:3 + IconButton.tsx:3 `import "./button.css"`; Button/IconButton used across all screens) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg class/variant usage; rg hex scan (comments only); rg aria-busy/loading production usage |

## 1. One-paragraph verdict
Complete, live button/IconButton/switch stylesheet. Every selector family has
production consumers (variant classes are built dynamically in Button.tsx:56-58
as `lumen-button--${variant}` — verified variant="tinted" in OpenMediaDialog.tsx:288,
variant="ghost" in PlaylistScreen.tsx:463, size="sm" widely). The busy/spinner
rules (aria-busy) are exercised by real screens (ControlBar.tsx:334,
NetworkStreamScreen.tsx:302 set loading). Zero raw hex in rules — all 17 hex
strings are in comments documenting frame measurements; 82 var() token uses.
This is what states.css's header claims and fails to be.

## 2. What actually works (verified)
— rg "aria-busy" production: StageOverlays.tsx:122, PlaylistBrowse.tsx:203,
ControlBar.tsx:391, MiniPlayer.tsx:265 — busy CSS is wired to real states.
— rg hex scan: every `#xxxxxx` occurrence (lines 6,11,14,16,24,55,154-155,
286-287,292,354,420-422,429,478-479) is inside a comment; rules are token-only.
— Switch dual grammar (.lumen-toggle/.lumen-switch) both consumed:
button/Toggle.tsx + form/Switch.tsx + button/switchCore.tsx.

## 3. Findings
None. Checked and how: (a) dead selectors — none, all four skins + variants +
sizes + states have TSX consumers (dynamic class construction verified in
Button.tsx:56-58); (b) token purity — hex scan comments-only; (c) a11y —
focus-visible rings, disabled inertness (pointer-events:none), reduced-motion
blocks for both spinners and the switch (487-512); (d) specificity traps —
the primary/tinted glyph remaps restate at equal specificity with documented
cascade reasoning (112-119, 146-150, 300-306); (e) contrast fixes documented
with measured ratios (152-170, 284-298, 477-485).

## 4. Per-function (selector block) table
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| .lumen-button base + states | 36-88 | REAL | - | icon token remap technique documented |
| primary/tinted/ghost variants | 90-189 | REAL | - | variant consumers verified |
| sizes + disabled + busy | 191-235 | REAL | - | spinner keyframes |
| .lumen-icon-button | 239-344 | REAL | - | busy glyph-swap via visibility (332-334) |
| switch core (dual grammar) | 346-497 | REAL | - | one skin, two class sets — divergence-proofed |
| reduced-motion blocks | 487-512 | REAL | - | covers all animated rules |

## 5. Test honesty
Toggle.test.tsx reads button.css as text (line 9) and pins skin strings —
presence-only tests; would not catch visual regressions. Noted for the Toggle
owner; no tests live in this file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (§3.2). Raw px appears solely for structural dimensions" | button.css:32-34 | REAL | hex scan: comments only; raw px limited to heights/28/48 chips/toggle geometry |
| "Pending cue (w26.k01 j01-12): the doc's promised spinner" | button.css:314-318 | REAL | aria-busy set by ControlBar/NetworkStream/Onboarding loading props |

## 7. Dead / duplicated / unreachable code
None found. The dual `.lumen-toggle*`/`.lumen-switch*` selector lists are
deliberate duplication with a documented Lead ruling (346-355) to prevent
skin drift — acceptable.

## 8. Dependencies & flags
tokens.css (--radius-control, --motion-duration-fast, --opacity-emphasis,
--opacity-disabled, color/surface ramp). icon.css token --color-stroke-icon.
No flags.

## 9. Open questions for the next agent
None.
