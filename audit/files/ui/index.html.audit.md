# AUDIT: ui/index.html
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 12 / 0 |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — THE entry document (vite root; main.tsx mounted at #root) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg documentElement/lang setters (none); rg document.title handling (App.tsx route titles, w26.k19) |

## 1. One-paragraph verdict
Minimal, correct vite entry: charset, viewport, title, #root, module script.
document.title is dynamically managed (App.tsx route titles, verified by rg —
w26.k19), so the static `<title>Lumen</title>` is only the pre-hydration
flash. One a11y gap: `lang="en"` is never updated by the locale system.

## 2. What actually works (verified)
- #root mount point exists (main.tsx throws if missing — verified in the
  main.tsx read).
- No favicon, no preloads, no analytics — a clean shell.

## 3. Findings
F-001 [S4]
Where: index.html:2
Code:
```
<html lang="en">
```
What is wrong: `document.documentElement.lang` is never written by any code
(rg over src: only dataset.theme/accent in tests) — with ?locale=pseudo or
any future locale the page announces "en" to assistive tech forever. Today
only en-US ships, so impact is dormant. What correct looks like:
setActiveLocale also sets documentElement.lang.
Confidence: HIGH — Verification: rg.

## 4. Per-function table (<100 LOC — document)
N/A (static document).

## 5. Test honesty
N/A.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| (implicit) title is app-managed | — | REAL | App.tsx:210-216 route-title wiring (rg) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Consumed by vite config (root). No gates.

## 9. Open questions for the next agent
- The Rust catalog's window.title.idle ("{product_name}") is an ENGINE-side
  title contract; the UI manages its own titles — which one owns the desktop
  window title at runtime is a shell question (apps/desktop-tauri lane).
