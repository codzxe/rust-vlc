# CLAIM AUDIT: ui/src/components/slider/states.md
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Claims extracted | 10 |
| REAL 8 / REAL-UNPROVEN 2 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNREACHABLE 0 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Rare case: a component doc that matches its implementation claim-for-claim.
Every behavioral contract row I could check against code verifies: the
keyboard table (arrows ±step, Shift/PageUp/Down ×10, Home/End, live-then-commit
order) is implemented verbatim in interaction.ts:147-183; the pending-seek
truth table matches Seekbar.tsx:31-54 and slider.css's data-pending rules; the
token-gap section is honest about two gaps open "after 10 waves".

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "pointer drag — `onChangeLive` continuously; pointer release — `onCommit` once" | :12-13 | REAL | interaction.ts onPointerMove/Up; props Slider.tsx:34,38 |
| 2 | "`pointercancel` — nothing (abort)" | :14 | REAL | interaction.ts:141-144 endDrag(event, false) |
| 3 | "ArrowRight/Up ... `onChangeLive` → `onCommit` (that order)" | :15 | REAL | interaction.ts:180-181 emitLive(clamped); onCommit?.(clamped) |
| 4 | "Shift+Arrow, PageUp/PageDown — idem, coarse step (×10)" | :16 | REAL | interaction.ts:154-160 (step*10) |
| 5 | "after commit it renders only what the caller passes back — never an optimistic jump" | :19-20 | REAL | Slider renders props only; dragValue cleared on release |
| 6 | "pending (`pending` prop): knob locked at requested target ... fill stays at engine position" | :62 | REAL | Seekbar.tsx:33 pending?: number; slider.css:340-342 data-pending thumb accent-strong |
| 7 | "Buffer ranges: engine-reported ... never invents buffer data" | :67-70 | REAL | Seekbar.tsx:118-131 renders caller-passed ranges; ControlBar feeds engine state |
| 8 | "BOTH gaps are still open after 10 waves" (token gaps) | :74 | REAL | rg tokens.css: no --color-eq-fill-* / --gradient-volume-level (my slider.css audit) |
| 9 | "Enforced by the colocated tests" | :5-6 | REAL-UNPROVEN | tests exist (Slider.test.tsx:10, EQSlider.test.tsx:7 read the CSS); pass status rests on baseline, not re-run |
| 10 | "L8 flush contract ... pinned by pixel-math tests" | :26-27 | REAL-UNPROVEN | formulas match slider.css:104,205,409; "pixel-math" assertions not individually re-executed |

## 3. Contradictions
None found.

## 4. Fabrication / inflation indicators
None. The doc even documents its own drift history (w26.k03 corrections at
:32-33) and labels the buffer token mapping [PROPOSAL] (:68).

## 5. Matrix rows
Not a matrix chunk — n/a.

## 6. Findings
None. (Checked: interaction contract vs interaction.ts; truth table vs
Seekbar.tsx + slider.css; token gaps vs tokens.css; anatomy table vs
slider.css geometry — all consistent.)
