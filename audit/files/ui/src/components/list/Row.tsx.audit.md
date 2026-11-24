# AUDIT: ui/src/components/list/Row.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 91 / 1 (ListRow) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — consumers: PlaylistPanel, PlaylistScreen, MiniPlayer, DiscoveryScreen, settings extensions area; 13 references in List.test.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumers; read |

## 1. One-paragraph verdict
Single well-built presentational row primitive. The kebab's
stopPropagation-before-onClick (line 65) is the one interaction trap and it
is handled. The density classes are constructed dynamically
(`lumen-lrow--${density}`) — this is the file that keeps list.css's two
"dead-looking" selectors alive (verified in the list.css audit). No findings.

## 2. What actually works (verified)
- 5 production consumers + test coverage (13 refs in List.test.tsx).
- `draggable` booleanish handling: `"false"` correctly suppresses the handle (line 56).
- HTML `title` attr conflict resolved via `Omit<...,"title">` — the title prop is the row label, documented.

## 3. Findings
None. Checked: prop spreading (rest excludes consumed keys), a11y (aria-label
on kebab, aria-hidden dots/handle), event containment (stopPropagation),
dynamic class construction, dead code (none — all props used).

## 4. Per-function table (<100 LOC — single function)
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| ListRow | 42-91 | REAL | slots + states all CSS-backed (list.css audit) |

## 5. Test honesty
List.test.tsx exercises the primitive (13 references); density/draggable/
kebab branches covered per the CSS class verification chain.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Roles and aria wiring belong to the consumer" | Row.tsx:38-40 | REAL | no role hard-coded; rest props carry it |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
React types + density.ts + list.css. None.

## 9. Open questions for the next agent
- None.
