# AUDIT: ui/src/screens/open-media/open-media.css
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 221 / ~20 selectors + 1 media block |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — OpenMediaDialog.tsx (mounts the open-media dialog from the app shell / menus) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | class-extraction + TSX consumer scan (0 dead selectors) |

## 1. One-paragraph verdict
Inner anatomy of the Open Media dialog; token-disciplined, zero dead
selectors. Two honesty-relevant choices are visible in the CSS itself: source
rows render the FULL source string with `overflow-wrap: anywhere` (comment:
"ellipses hid the one piece of data that distinguishes entries"), and rows
are "plain activation buttons ... no aria-pressed state, no focus-as-selection"
(138-142). No findings.

## 2. What actually works (verified)
- 0/20 dead selectors (consumer scan vs OpenMediaDialog.tsx).
- Full-bleed divider uses negative margins matching the body's 20px gutters
  (110-112) — arithmetic consistent with 80.
- Reduced-motion wrapped in `no-preference` guard (217-221) — correct
  direction (transition only when motion allowed).

## 3. Findings
None. Checked: dead selectors (none), token discipline (raw px carries
frame: comments), duplication (none), media handling (correct), z-index
ownership (moved to shared primitive per comment — not this file's concern).

## 4. Per-function table (>100 LOC; CSS blocks as units)
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| surface column | 21-36 | REAL | 560px scale, pinned header/footer |
| header | 40-70 | REAL | 50px strip, quiet title class |
| body + path form | 74-112 | REAL | form-association submit documented |
| pick sections + rows | 116-199 | REAL | full source strings, no hidden data |
| footer | 204-215 | REAL | 100x32 actions |
| reduced-motion | 217-221 | REAL | |

## 5. Test honesty
No tests in this file; OpenMediaDialog tests belong to that component's
audit owner.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the footer Open is this form's submit button (form association)" | open-media.css:86-90 | UNVERIFIED | CSS comment; the form attribute lives in OpenMediaDialog.tsx (not my file) |
| "Rows are plain activation buttons ... no focus-as-selection" | open-media.css:140-142 | REAL | no aria-pressed/focus styles in this sheet |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Design tokens + surface primitive classes. No gates, no backend surface.

## 9. Open questions for the next agent
- OpenMediaDialog.tsx auditor: verify the form-association claim (footer
  Open is type=submit of the path form) — the CSS promises it.
