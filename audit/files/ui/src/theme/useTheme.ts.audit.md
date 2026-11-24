# AUDIT: ui/src/theme/useTheme.ts
| Field | Value |
|---|---|
| Auditor | W15-R2 (credited: A05) |
| Date | 2026-11-23 |
| Lines / functions | 16 / 1 (`useTheme`) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — main.tsx mounts ThemeProvider; App.tsx:297 `<Route path="/dev/tokens" element={<TokenSandbox />} />` -> TokenSandbox.tsx:13,30 calls useTheme() |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg useTheme consumers (exit 0); sed context.ts:20-32; rg toThrow in theme tests (exit 0, 1 unrelated hit) |

## 1. One-paragraph verdict
Correct 8-line consumer hook: reads ThemeContext (default null, context.ts:31),
throws a descriptive Error outside the provider, returns the value otherwise.
Consumers are real (TokenSandbox.tsx:30 on the /dev/tokens route;
ThemeProvider.test.tsx:13). The module-separation rationale in the header
(issue #36, react-refresh/only-export-components) matches the actual file
layout: context in context.ts, provider in ThemeProvider.tsx, hook here.
One hygiene finding: the throw path has no test.

## 2. What actually works (verified)
— verified by `rg -n "useTheme" src` -> 2 consumer files + provider re-export
  comment at ThemeProvider.tsx:127.
— verified by `sed -n '20,32p' src/theme/context.ts` -> context default is
  `null`, so the `ctx === null` guard is the correct outside-provider check.
— Baseline ui-test.txt: 136 files / 1705 tests pass, which compiles this hook
  and exercises it via ThemeProvider.test.tsx.

## 3. Findings
F-001 [S4]
Where: ui/src/theme/useTheme.ts:12-14
Code: (verbatim quote)
```
  if (ctx === null) {
    throw new Error("useTheme must be used inside <ThemeProvider>");
  }
```
What is wrong: No test asserts this throw. rg for toThrow in theme tests finds
only the unrelated accent validation (ThemeProvider.test.tsx:123
`}).toThrow(/unknown accent "magenta"/)`). Why it matters: a mis-mounted
consumer would surface as an uncaught render error instead of a pinned
contract. What correct looks like: one renderHook test asserting the error
outside the provider. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `useTheme` | 8-15 | REAL | — | Null-guard + return; correct against null-default context |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ThemeProvider.test.tsx:37 (destructures via useTheme) | YES | Would crash/undefined if the hook stopped reading the context |
| Throw-path guard | NO TEST — see F-001 | |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Throws outside ThemeProvider" | useTheme.ts:7 | REAL | Lines 11-13 implement exactly that |
| "reads the context from ./context.ts" (module split, issue #36) | useTheme.ts:2-4 | REAL | import at line 6; ThemeProvider.tsx:127 comment confirms layout |

## 7. Dead / duplicated / unreachable code
None. Hook has exactly two production/test consumer files, both live.

## 8. Dependencies & flags
React (useContext) + ./context. No feature gates.

## 9. Open questions for the next agent
None.
