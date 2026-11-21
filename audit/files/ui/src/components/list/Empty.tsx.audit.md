# AUDIT: ui/src/components/list/Empty.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 28 / 1 (ListEmpty) + props interface |
| File verdict | REAL (presentational primitive, 3 shipped consumers, test-pinned including its css contract) |
| Reachable from shipped entry point? | YES (UI tree): PlaylistScreen.tsx, PlaylistPanel.tsx, DiscoveryScreen.tsx, MediaInfoPanel.tsx render it (rg). W14 ceiling (no GUI binary) applies. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -ln "ListEmpty" ui/src` → 4 screen consumers + keys file; `sed -n '454,486p' list.css` → .lumen-empty rules exist, body colored `--color-content-secondary` with the w26.d2 a11y-sweep comment; `sed -n '522,550p' List.test.tsx` → 3 tests incl. the css-contract assertion pinning secondary title + secondary body; read of the whole file. |

## 1. One-paragraph verdict
A correct 28-line primitive: `Omit<…, "title">` cleanly repurposes the HTML title attribute as a ReactNode heading, optional body/action, quiet-by-design (test pins the absence of img/svg), safe spread order (className destructured, no explicit a11y attributes to clobber). One defect: both doc comments still describe the body line as "tertiary" text, but list.css stepped the body up to `--color-content-secondary` in the w26.d2 axe contrast fix — and the test file pins the new secondary value while this file's comments pin the old story.

## 2. What actually works (verified)
— verified by List.test.tsx:522-550 (baseline #23): title/body/action render and are visible, no img/svg artwork, body/action absent when omitted, and the css-contract test asserts `.lumen-empty__title` and `.lumen-empty__body` both contain `var(--color-content-secondary)`. All three FAIL on the regressions they name.
— verified by `rg`: 4 shipped screens consume it (playlist empty state, discovery, media-info).
— verified by read: rest-spread cannot shadow anything meaningful (title/body/action/className/children all destructured).

## 3. Findings
### F-001 [S4]
Where: ui/src/components/list/Empty.tsx:6,13 (+ ui/src/components/list/list.css:474-478)
Code:
```
 * ListEmpty — the quiet empty-list state (W25 spec §2: "empty space is fine
 * — a quiet rail"): a secondary-tone title, an optional tertiary body line
```
(:13)
```
  /** Optional explanatory line (tertiary text). */
  body?: ReactNode;
```
What is wrong: The body has not been tertiary since the w26.d2 a11y sweep: list.css:474-478 says tertiary "failed the axe color-contrast audit on every consumer (3.2:1 dark / 2.45:1 light)" and colors the body `var(--color-content-secondary)` — same tier as the title, calm kept by size instead. List.test.tsx:546-549 pins the secondary value. Both of this file's comments predate the fix.
Why it matters: The component's own documentation describes a contrast contract that the css deliberately broke (for good reason) — a reader trusting "tertiary" reintroduces the axe failure.
What correct looks like: "an optional body line (secondary ink, caption size — stepped up from tertiary in the w26.d2 contrast fix)".
Confidence: HIGH — Verification: css read; test read; comments read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ListEmptyProps | 10-17 | REAL | S4 | F-001 stale "tertiary" doc; Omit<"title"> repurposing is clean |
| ListEmpty | 19-28 | REAL | — | null-guarded optional slots; children passthrough; trim() on className composition |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| renders title/body/action without artwork | YES | visibility assertions + img/svg absence |
| body and action are optional | YES | negative class queries |
| CSS: secondary title + secondary body | YES | literal css-source assertion — this is the test that contradicts the component comment (F-001) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a secondary-tone title, an optional tertiary body line" | :5-6 | STALE | css body is secondary (list.css:478, test :548) |
| "at most one action" | :7 | REAL (by convention) | single `action` slot; no enforcement needed for a slot prop |
| "No artwork, no illustration" | :7-8 | REAL | test asserts zero img/svg |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
React types + list.css only. No engine dependency, no flags.

## 9. Open questions for the next agent
- None.
