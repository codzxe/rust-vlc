# AUDIT: ui/src/screens/player/tools/video/video.css
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 461 / ~35 class selectors |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: imported by the video tools (tools/video/*, mounted via ToolsDialogHost on /player); classes constructed at VideoCard.tsx:61-62 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python class cross-reference (exit 0); sed/rg quote checks (exit 0) |

## 1. One-paragraph verdict
Presentation-only CSS for the four video tools (Colors/Crop/Essential/HDR): all 35 class tokens live (the two `--colors`/`--crop` modifiers are template-constructed at VideoCard.tsx:62 `video-tool--${tool}`), tokens everywhere, raw px only as frame-measured structural geometry with the SVG source cited per rule. Zero findings — checked every selector group, every raw value, and the cross-file grammar dependencies (dynamics.css card shell, menu.css dropdown rows). Context for the master question (verified in the sibling TSX, not this file): the sliders this sheet styles are LOCAL-STATE-ONLY — "Controls stay fully interactive with LOCAL state, visibly marked staged" (VideoCard.tsx:28-29) with `data-pending-command` markers; no engine stage or catalog command exists for video effects.

## 2. What actually works (verified)
— python cross-reference: 35 classes, 0 unused (33 literal + 2 template-constructed).
— Slider value column hidden per frame: `.video-tool .dynamics-row__value { display: none; }` (L41-43) matches the label-only rows claim and is pinned by harness.test.tsx:106-108 (asserts the rule exists in the served CSS).

## 3. Findings
None. Checked: selector liveness (all 35), token discipline (raw px annotated `frame:`), selector-specificity chains into shared grammar (`.video-subdropdown .lumen-dropdown__*`, `.video-hdr .lumen-dropdown__*` — targets exist in menu.css), focus-visible rings on every interactive control (hex input, crop inputs, link toggle), no `!important`, no duplicated blocks.

## 4. Per-function table (selector groups)
| Selector group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.video-tool .dynamics-row__value` hide | 41-43 | REAL | - | frame-true; test-pinned |
| `.video-row(__label/__spacer)` line rows | 45-69 | REAL | - | 32px band, 15px pitch |
| `.video-hex(__input/__swatch)` | 71-110 | REAL | - | 116×32 rx6, 22×22 rx2 swatch |
| `.video-sep--1..5` cadence | 129-162 | REAL | - | per-gap separators |
| `.video-subdropdown` (+ dropdown reskin) | 178-196, 376-388 | REAL | - | shared Dropdown grammar |
| `.video-crop-row/__input/__label/__value/__control/__unit/__link` | 199-334 | REAL | - | link toggle states incl. aria-pressed |
| `.video-hdr` dropdown + option check | 327-421 | REAL | - | 44px rows per menu.css grammar |

## 5. Test honesty
harness.test.tsx:106-108 asserts the `display:none` rule exists in the compiled CSS text — would fail if the rule were deleted (real pin, unusual and good). Frame-measurement claims UNVERIFIED (no renderer).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Semantic tokens only ... raw px below is structural geometry pinned by the design exports" | L31-32 | TRUE | read: every raw px carries a `frame:` comment |
| "the shared Dropdown grammar since w26.i16 (menu.css 44px rows ...)" | L25-26 | TRUE | menu.css owns the row grammar; overrides only skin trigger/list |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Depends on dynamics.css card grammar and menu.css dropdown classes (both verified present). No flags.

## 9. Open questions for the next agent
- The staged-note look this sheet styles marks controls awaiting `video.set-*` commands — who owns that catalog gap (same family as audio.set-volume/EQ refusals)?
