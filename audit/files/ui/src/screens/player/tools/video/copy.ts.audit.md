# AUDIT: ui/src/screens/player/tools/video/copy.ts
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A05) |
| Date | 2026-11-24 |
| Lines / functions | 30 / 1 (useVideoMsg) + 1 const (FALLBACKS) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — useVideoMsg → ColorsTool.tsx:39,200 / HdrTool / CropTool / VideoCard / VideoSliderRow / shared/SliderRow → ToolsDialog.tsx:39 imports the four tools → PlayerScreen.tsx:1203 ToolsDialogHost |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg (consumers, useMsg impl, mounts) -> hits cited; python3 key-set/value-diff (w26.c4 vs w26.a18 vs messages.ts) -> overlap ∅, 31+3 registered, 0 value drift; sed quotes verified |

## 1. One-paragraph verdict
A correct seam-first label resolver: central catalog (`useMsg`) first, then the lane's pre-registration key files, then the raw key — the fallback chain is total (no path yields `undefined` textually, worst case the key string itself). Every factual claim in the header verifies: the two lane files share zero keys, and — the critical honesty check — all 34 lane keys are already registered in messages.ts with byte-identical values, so "copy renders identically before and after registration" is true. The only blemish is that the entire fallback layer is presently dormant (nothing routes past the central catalog).

## 2. What actually works (verified)
— verified by python3 parse: w26.c4.ts = 31 keys, w26.a18.ts = 3 keys, intersection = ∅ (the "no key is defined twice" claim), all 34 present in messages.ts, **value drift = 0**.
— verified by baseline ui-test.txt:77,123,145 — ColorsTool (16), HdrTool (6), CropTool (9) tests pass, all rendering through this resolver.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/video/copy.ts:17-27
Code: (verbatim)
```
const FALLBACKS: Readonly<Record<string, string>> = {
  ...dialogsLaneKeys,
  ...laneKeys,
};
```
What is wrong: dormant layer — since all 34 lane keys are centrally registered with identical values, `m(key as MessageKey)` always wins and `FALLBACKS[key]` is never reached for any existing key; the merged object plus two key-file imports are dead weight in the shipped bundle today. Why it matters: two i18n sources must be kept value-synced forever (drift would silently change nothing now, but re-introduce divergence the moment a key is unregistered); the `Record<string, string>` typing also erases the key union so a typo'd key falls to the `?? key` raw-string path instead of failing compilation. What correct looks like: `Readonly<Record<VideoLabelKey, string>>` typing, or deleting the fallback once registration is complete. Confidence: HIGH — Verification: read + python3 diff.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useVideoMsg | 23-30 | REAL | S4 | 3-level fallback, memoized on `[m]`; total chain verified |
| FALLBACKS (const) | 17-20 | REAL (dormant) | S4 | merge verified disjoint; currently unreachable in effect |

## 5. Test honesty
No tests in this file; the resolver is exercised by every video-tool test (baseline lines above) — if the chain broke (returned undefined), text assertions in ColorsTool/HdrTool/CropTool tests would fail.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no key is defined twice — the toggle labels belong to w26.a18, the row/staged copy to this lane" | copy.ts:15-16 | REAL | python3 intersection of the two key sets = ∅ |
| "Copy renders identically before and after the Lead registers the keys" | copy.ts:4-6 | REAL | python3 value diff lane vs messages.ts = 0 drift over all 34 keys |
| "catalog → lane pre-registration → key" chain | copy.ts:22 | REAL | useMsg (messages.ts:3011-3018) returns `catalog[key] ?? messages[key]`; copy.ts:27 adds `?? FALLBACKS[key] ?? key` |

## 7. Dead / duplicated / unreachable code
FALLBACKS is live code, dormant data (see F-001). No duplication: the a18 toggle-label reuse avoids redefining keys.

## 8. Dependencies & flags
react (useCallback), i18n/messages (useMsg, MessageKey), i18n/keys/w26.a18 + w26.c4, ./types (VideoLabelKey). No feature gates.

## 9. Open questions for the next agent
None for this file. (Related: w26.c4's [VERIFY engine lane] marker on gradient-mode options — flagged in the c4 key file, engine-side truth unconfirmed.)
