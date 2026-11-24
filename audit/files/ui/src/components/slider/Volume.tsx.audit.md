# AUDIT: ui/src/components/slider/Volume.tsx (compact, 57 LOC — spec §13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 57 / 2 (VolumeSlider, formatPercent) |
| File verdict | REAL (thin, correctly-wired slider wrapper) |
| Reachable from shipped entry point? | YES — ControlBar.tsx:477-482 `<VolumeSlider ... onCommit={actions.setVolume}>` -> context.tsx:389-390 `dispatch({ args: { volume: value }, cmd: "audio.set-volume", ... })` -> client transport -> lumen_dispatch (full chain verified by rg) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumer chain (ControlBar:49,477-482; context.tsx:93-94,389-390); read of full file |

## 1. One-paragraph verdict
The volume slider is fully wired to the engine: drag is visual-only
(onChangeLive), commit on release/keyboard dispatches the REAL
`audio.set-volume` command (range 0..1 matching the doc 05 command layer),
and the context layer updates state only on engine completion (:93
comment) — the never-optimistic posture, consistent with state-mapping.
The wrapper itself is honest plumbing: percent formatting for a11y text,
caller-supplied aria-label, the w26.a04 className-merge fix documented
in-code. No findings.

## 2. What actually works (verified)
— Wire chain (above, three rg hops, all real code).
— Slider.test coverage via the slider suite (vitest baseline); the
  wrapper's className merge is pinned by the documented a04 regression
  (consumer class no longer clobbers the grammar class).

## 3. Findings
None. Checked: value clamping (delegated to Slider min=0/max=1/step=0.01),
literal leakage (none — formatPercent is computed, labels are caller-side),
ref forwarding (correct, displayName set), rest-spread ordering
({ ...sliderProps } {...rest} — rest can still override sliderProps keys
other than className, e.g. a caller-supplied min would silently win; no
current caller does).

## 4. Per-function table (compact)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| formatPercent | 25 | REAL | rounded percent |
| VolumeSlider | 27-55 | REAL | commit seam verified to lumen_dispatch |

## 5. Test honesty
Slider primitive's suite (colocated in components/slider/) covers the
drag/commit model; this wrapper is exercised via ControlBar tests
(:616 cites the real engine host). Baseline green.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Engine-reported volume, 0…1 (doc 05 command layer range)" | :16 | REAL | generated commands audio.set-volume args.volume: number; context dispatch :390 |
| "the level fill resolves to the frame-measured green→amber→red ... when present, else the accent" | :10-12 | REAL-UNPROVEN | CSS token fallback in slider.css (not re-measured) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Slider primitive only. No flags.

## 9. Open questions for the next agent
- Slider.tsx auditor: does the primitive clamp programmatic `value` (a
  stale engine event >1 or <0) before rendering width percent? An
  out-of-range value would overflow the fill.
