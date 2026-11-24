# AUDIT: ui/src/screens/settings/areas/subtitles.css
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 174 / 0 (stylesheet, 10 selectors) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by areas/subtitles.tsx (routed at /settings/subtitles; classes referenced there + settings.css) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 selector cross-check (10/10 consumed); `rg -n "disabled\|staged\|featureGate" areas/subtitles.tsx` |

## 1. One-paragraph verdict
Real, frame-measured stylesheet; all 10 selectors consumed. The hostile catch
is css:59-67: it overrides `:disabled { opacity: 1 }` for every control class in
the area — by design, because the ENTIRE subtitles settings area is staged:
subtitles.tsx renders every row (toggles, dropdowns, text fields, segmented)
`disabled` with `data-staged` markers ("disabled means never fired", tsv:167,
204, 229). This sheet is the component of that staging that makes dead controls
render at full strength; honesty is delegated to tooltips/ⓘ notes.

## 2. What actually works (verified)
— python3 cross-check: 10/10 `.settings-area-subtitles*` selectors referenced
(subtitles.tsx + settings.css). — Slider grammar pins (track 12px in a 24px
band, thumb 24px, 9 ticks at 10%-90%) are coherent geometry (css:120-163).
— The color-swatch custom property (--settings-area-subtitles-swatch) is set
from row data per css:93-95 comment — consumed by subtitles.tsx.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/areas/subtitles.css:59-67
Code:
```
/* w26.gfix-B (g13-P2-3): staged controls render FULL-STRENGTH — the frames
 * show live-looking controls; the honesty lives in the disabled state
 * (inert, never fired) + the ⓘ notes, not in dimming. */
.settings-area-subtitles .lumen-switch:disabled,
```
What is wrong: the whole Subtitles settings area is a staged, fully-inert
surface (subtitles.tsx: every row disabled, `data-staged="subtitles-settings-seed"`),
and this rule paints those dead controls at opacity 1 — indistinguishable from
live controls except via tooltip/ⓘ. The stated rationale is fidelity to a
Figma frame that shows live-looking controls.
Why it matters: a user cannot tell from appearance that NOTHING in this screen
persists or applies; the disabled semantics are visually erased. This is the
Wave-11 "settings UI wired to nothing" pattern with the dimming removed.
What correct looks like: either wire the rows to the config settings commands
(config.set-setting exists in the catalog) or keep a visible inert treatment.
Confidence: HIGH — Verification: read + rg (all row components disabled in
subtitles.tsx).

## 4. Per-function table
N/A (stylesheet). 10 selectors + shared-grammar overrides; all consumed (§2).

## 5. Test honesty
subtitles.test.tsx exists (area file's suite); asserts staged markers and
disabled state per its own comments — validates the HONESTY layer, not
functionality (there is none). Not run (no node_modules; and see my
VisualizerDisplay audit F-002 — the tree currently cannot run vitest at all).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the honesty lives in the disabled state (inert, never fired) + the ⓘ notes" | subtitles.css:60-61 | REAL | subtitles.tsx:167/204/229 all `disabled` |
| "swatch filled with the row's VALUE (set via the ... custom property from the row data)" | subtitles.css:93-95 | REAL | custom property consumed by subtitles.tsx (cross-check) |

## 7. Dead / duplicated / unreachable code
None in this file. The sheet styles a wholly staged area (F-001).

## 8. Dependencies & flags
Semantic tokens + the one custom property; no raw colors except documented
frame provenance. No imports, no feature gates.

## 9. Open questions for the next agent
- `config.set-setting` exists in the command catalog — is there an engine-side
  reason the subtitles area cannot persist through it like other areas do?
  Which areas ARE wired (SettingsProvider/useSettings) vs staged? (Wave-11
  cross-reference: localStorage mock persistence claim.)
