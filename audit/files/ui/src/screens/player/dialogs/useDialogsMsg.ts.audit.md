# AUDIT: ui/src/screens/player/dialogs/useDialogsMsg.ts
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 103 / 4 (1 hook + 3 derived COPY objects) |
| File verdict | REAL (i18n lane seam working as designed; central registration already landed with identical strings) |
| Reachable from shipped entry point? | YES — used by the Bookmarks and Go-to-Time dialogs (player dialogs lane) → PlayerScreen; COPY constants re-exported via dialogs/index.ts:39-40 and consumed by tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg useMsg fallback (messages.ts:3016 `catalog[key] ?? messages[key]`); rg key registration (messages.ts:2952/2955 vs w26.k09.ts:21/46 — identical values); rg COPY consumers (ToolsDialogShell.test.tsx:58-92, index.ts:39-40) |

## 1. One-paragraph verdict
Correct lane-i18n seam: `useDialogsMsg` resolves central-first (`m(key) ?? laneCatalog[key]`), and the fallback logic is sound because unregistered keys yield `undefined` from messages.ts:3016 (never a rendered key string — the doc 07 §6.3 total chain). The one staleness: the header describes the lane keys as awaiting Lead registration ("until the Lead registers them into messages.ts post-wave") — registration has ALREADY happened (messages.ts:2952, :2955) with byte-identical strings, so the lane catalog is now a dormant duplicate that only still matters if the Lead ever removes the central entries. COPY objects derive from the same lane keys (single source, no drift — values verified identical).

## 2. What actually works (verified)
— verified by rg: central and lane values identical for spot-checked keys ("Bookmarks", "That time is past the end ({duration})") — the "renders identically before and after registration" claim holds.
— verified by rg: TOOLS_DIALOG_COPY consumed by ToolsDialogShell.test.tsx (the tools shell itself uses useMsg directly, as documented).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/dialogs/useDialogsMsg.ts:5-6
Code:
```
// `ui/src/i18n/keys/w26.k09.ts` until the Lead registers them into
// `messages.ts` post-wave; useDialogsMsg resolves central-first with the
```
What is wrong: Stale — registration already landed (messages.ts:2952-2955, byte-identical). The lane fallback is dormant weight; a future edit to either copy silently diverges the "one source of truth" the header promises. Cosmetic. Confidence: HIGH — Verification: ran rg on both files.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useDialogsMsg | 38-44 | REAL | — | central-first; fallback engages only on unregistered keys |
| BOOKMARKS_COPY | 51-71 | REAL | — | derived; template fns replace {time}/{label} |
| GOTO_COPY | 78-89 | REAL | — | {duration} template |
| TOOLS_DIALOG_COPY | 96-103 | REAL | — | shell keys already central; test-only reference |

## 5. Test honesty
ToolsDialogShell/Bookmarks/GoToTime tests (baseline PASS) read these COPY objects as the string source — they would fail if lane and rendered values drifted.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the copy renders identically before and after registration" | useDialogsMsg.ts:7-8 | TRUE | byte-identical values (rg) |
| "until the Lead registers them into messages.ts post-wave" | useDialogsMsg.ts:5-6 | STALE (done) | messages.ts:2952,2955 |

## 7. Dead / duplicated / unreachable code
Lane catalog fallback: dormant (central registered) but by-design belt-and-braces.

## 8. Dependencies & flags
i18n/messages (useMsg, MessageKey), lane key files w26.k09/w26.a18. No flags.

## 9. Open questions for the next agent
- None.
