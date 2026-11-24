# CLAIM AUDIT: ui/src/components/menu/states.md
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Claims extracted | 9 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 8 / 1 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An accurate, unusually honest behavior spec: every timing constant, keyboard behavior, and a11y wiring I checked matches the code exactly, including the [PROPOSAL] markers for un-pinned values and a "Known gaps" section that admits real deviations (label color ruling pending, no translucency token). Zero false claims found. (An earlier draft marked claim 9 FALSE based on a display artifact — retracted; see the claim ledger.)

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Hover intent for submenus: open 150 ms, close 100 ms" | states.md:49-50 | REAL | timings.ts:17,20 (150/100); MenuPopup.tsx:293,306 |
| 2 | "typeahead accumulates printable chars for 500 ms ... jumps to the next label prefix match" | states.md:76-78 | REAL | timings.ts:14 (500); Dropdown.tsx:304 |
| 3 | "Tooltip: show 500 ms, hide 150 ms" | states.md:84-85 | REAL | timings.ts:23,26; Tooltip.tsx:34-35 |
| 4 | "Escape/Tab close + restore and are SCOPED to the dropdown (stopPropagation...)" | states.md:70-72 | REAL | Dropdown.tsx:298-299 |
| 5 | "Context-menu coordinates ARE viewport-clamped since w26.f07 — every popup measures itself on mount" | states.md:96-98 | REAL | MenuPopup.tsx:14,75 (useLayoutEffect measure + flip/shift) |
| 6 | "ArrowUp opens with LAST-item focus (w26.k02, j02 J02-9 — APG menubar)" | states.md:55-56 | REAL | MenuBar.tsx:92-99 (setOpenFocus("last")) |
| 7 | "Tab is trapped (wraps) inside an open menu" | states.md:57-58 | REAL-UNPROVEN | MenuPopup.tsx trap not read in full; MenuBar audit L37-42 consistent |
| 8 | "labels are i18n keys resolved through the resolveLabel prop ... never hardcode labels" | states.md:31-34 | REAL | MenuBar default identityResolveLabel (dev), hosts wire useMsg (ControlBar) |
| 9 | (implicit) dialog/menu Tab trap machinery works as specced | states.md (Tooltip/Escape sections) | REAL | Surface.tsx:174 `el.closest("[hidden]")` — intact (fixed-string count 1); the structural filter is sound |

## 3. Contradictions
None internal.

## 4. Fabrication / inflation indicators
None — [PROPOSAL] markers on un-pinned timings, known-gaps section admits deviations. This is what honest docs look like.

## 5. Matrix rows
N/A (not a matrix chunk).
