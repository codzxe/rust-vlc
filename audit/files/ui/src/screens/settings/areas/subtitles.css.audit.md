# AUDIT: ui/src/screens/settings/areas/subtitles.css
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 174 / 15 selector rules |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web app build): subtitles.tsx (the Subtitles settings area) imports this sheet and applies all 15 classes (`rg` cross-check: every selector's class appears in subtitles.tsx, incl. the `--settings-area-subtitles-swatch` custom property set from row data at subtitles.tsx:290). Baseline #22 build OK, #23 vitest 1705 passed incl. subtitles.test.tsx (13 cases). No GUI binary hosts it (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `Read` subtitles.css 1-174 (complete); `rg -o "settings-area-subtitles[a-z-]*" subtitles.tsx | sort -u` (10 class stems — all sheet selectors used, zero dead rules); `rg -n "settings-area-subtitles-swatch" subtitles.tsx` (the swatch custom property is fed real row data); `rg -c -- "--radius-hairline:" ../themes/generated/tokens.css` (exists — no dangling var); `rg -n "describe|it(" subtitles.test.tsx` (13 behavioral cases incl. "never touches the transport (no engine seam exists for these rows)" — the area this sheet styles is fully staged); `sed -n` quote verification. |

## 1. One-paragraph verdict
A tight, area-scoped sheet that pins only what the frame measures beyond the shared settings grammar: 320×36 compact triggers, equal-cell segmented control, the 320×32 color well with a data-driven swatch, and a carefully-constructed opacity slider band (12px track in a 24px hit band, 8×24 thumb with 6px overhang, nine 2×6 degree ticks, 0/50/100 scale). Every color is a token or token mix, every raw px carries a `frame:` citation, the swatch value comes from real row data, and the class inventory is fully consumed. The one judgment-call finding: the staged-controls override renders disabled rows at FULL opacity (62-67) — a documented design decision ("the honesty lives in the disabled state + the ⓘ notes, not in dimming") that removes the standard visual cue of inertness from every staged row in the area.

## 2. What actually works (verified)
— Verified by `rg`: all 15 selector rules target classes used by subtitles.tsx; the `--settings-area-subtitles-swatch` property is set from row values (subtitles.tsx:290), so the swatch really renders the row's color.
— Verified by token check: all referenced tokens exist (`--radius-control`, `--radius-inset`, `--radius-hairline`, `--color-surface`, `--color-surface-muted`, `--color-accent-tinted`, `--color-content-secondary/tertiary/bright`, `--font-size-section/body/caption`) — no dangling var().
— Verified by read: the opacity slider's geometry is internally consistent — 24px band, track centered (`calc(50% - 6px)`), thumb centered (`calc(50% - 12px)`), tick marks centered on the same centerline (`calc(50% - 3px)`); the tick overlay is `pointer-events: none` so the slider keeps the full hit band.
— Verified by baseline #23: subtitles.test.tsx runs green — 14 frame rows, 9 degree ticks, ⓘ placement, staged markers, "never touches the transport".

## 3. Findings

### F-001 [S4]
Where: ui/src/screens/settings/areas/subtitles.css:59-67
Code:
```css
/* w26.gfix-B (g13-P2-3): staged controls render FULL-STRENGTH — the frames
 * show live-looking controls; the honesty lives in the disabled state
 * (inert, never fired) + the ⓘ notes, not in dimming. */
.settings-area-subtitles .lumen-switch:disabled,
.settings-area-subtitles .lumen-dropdown__trigger:disabled,
```
What is wrong: the override forces `opacity: 1` on every staged (disabled) control in the area, removing the dimming that is the conventional visual signal of inertness. A staged switch at full strength is visually indistinguishable from a live one until the click does nothing; the disclosure is delegated to the ⓘ notes and the (absent) hover feedback. This contradicts the sibling areas' own framing — effects.tsx's header promises controls that are "visible, inert, never a dead control that LOOKS alive", while this rule makes disabled controls look alive by design (with a documented rationale: the Figma frames paint them live).
Why it matters: consistency and perceivability: within one settings screen family, some staged areas dim (the shared Button disabled treatment) and this one does not; users' only cue is interaction failure. Not a WCAG violation (the controls are correctly disabled for AT), hence S4.
What correct looks like: a consistent staged presentation across areas (either dim everywhere with the frame-fidelity exception documented per-area, or a shared "staged" affordance like the ⓘ-adjacent marker on the control itself), or at minimum a hover cursor change on staged rows.
Confidence: HIGH — Verification: read (rule + both rationales traced; effects.tsx's contradicting promise read in my own audit of that file).

## 4. Per-function (selector) table
| Selector | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-dropdown` (area-scoped) | 32-34 | REAL | — | 320px column fill. |
| `.lumen-dropdown--compact …__trigger` | 38-40 | REAL | — | Frame 36px (vs shared 40) — deviation documented. |
| `.lumen-segmented` (+segment) | 50-57 | REAL | — | Equal cells (flex-basis 0) + ≈15px labels; shared-primitive deviation scoped here. |
| staged-controls full-strength override | 62-67 | REAL | F-001 | Documented design decision; see finding. |
| `__color` well | 73-85 | REAL | — | 320×32; 17px/5px insets per frame arithmetic. |
| `__color-hex` | 87-91 | REAL | — | Tabular hex glyphs; secondary ink. |
| `__color-swatch` | 96-104 | REAL | — | 22×22 rx2, data-driven fill via the row's custom property, 1.5px bright@21% inner stroke. |
| `__opacity` / `__opacity-track` | 108-118 | REAL | — | Column + relative band. |
| slider track/fill/thumb pins | 123-144 | REAL | — | A2 grammar scoped: 12px track centered, flat tinted fill, 8×24 thumb with 6px overhang — geometry arithmetic consistent. |
| `__opacity-ticks` / `-tick` | 149-163 | REAL | — | Nine 2×6 marks, pointer-events none, centered via translateX(-50%). |
| `__opacity-scale` | 167-174 | REAL | — | 0/50/100 space-between labels, tabular. |

## 5. Test honesty (subtitles.test.tsx, 13 cases, ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| sections/rows/values suites (42-139) | YES | Frame-order and value oracles over the DOM (14 rows, 9 ticks, ⓘ placement). |
| staged honesty suites (174-215) | YES | "marks every row staged with its pending schema key — no live bindings", "never touches the transport", "binds no schema setting" — the area this sheet styles is fully inert, and the tests pin that. |
| CSS itself | — | No visual regression in this suite (visual configs elsewhere). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only (contract §4.4); raw px below are frame-measured structural dimensions" (14-15) | this file | REAL | Zero raw colors; raw px only on geometry with `frame:` comments. |
| "the ⓘ is the shared SettingInfoDot button … the local twin retired" (23-27) | this file | REAL | No local info-glyph rules remain in this sheet (verified by read). |
| "staged controls render FULL-STRENGTH … the honesty lives in the disabled state + the ⓘ notes" (59-61) | this file | REAL (as a description) | The rule does exactly that — and is F-001's subject. |

## 7. Dead / duplicated / unreachable code
- No dead selectors (all used). No duplication found (the A2 slider pins are scoped per the documented equalizer.css/dynamics.css precedent — same grammar, different mounts).
- Unreachable as a shipped binary (W14) — global UI condition.

## 8. Dependencies & flags
- Depends on the shared dropdown/segmented/switch/field/slider primitives (pinned, not redefined) and 12 tokens (all exist). The one custom property (`--settings-area-subtitles-swatch`) is area-local and fed by the TSX. No flags.

## 9. Open questions for the next agent
- subtitles.tsx owner: the whole area is staged ("no live bindings" test) — the same SHALLOW-with-honesty verdict as my effects.tsx audit applies there; whoever audits it should cross-ref F-001 here (the full-strength rule is this sheet's contribution to that staging story).
