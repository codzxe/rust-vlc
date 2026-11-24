# AUDIT: ui/src/client/transport.ts
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 89 / 1 interface + 1 class |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: the CommandTransport seam every transport implements (TauriTransport/DemoTransport/MockTransport); re-exported types consumed by context.tsx, PlaylistProvider, screens |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg transport consumers + refusal-code vocabulary cross-check (exit 0); baseline ui-test.txt (transport tests pass) |

## 1. One-paragraph verdict
The single-import-site seam over the generated wire types plus an honest MockTransport: only `system.list-commands` is simulated (empty listing); every other command answers the typed `feature-not-exposed` envelope — no invented data. The cross-cutting catch (root of PlayerScreen F-002): the error-code VOCABULARY splits along transport lines — this mock (and the demo transport) refuse with `feature-not-exposed`, while the real desktop host refuses with `unsupported-for-host` (apps/cli/src/dispatch.rs ErrorCode::UnsupportedForHost). PlaylistProvider.tsx:203 gates its not-exposed state on the MOCK's code only, so the honest-degradation logic was written against the fake hosts' dialect and never fires in production.

## 2. What actually works (verified)
— Seam design: single import site for src/generated (L3-4 claim holds: rg shows generated/commands imports otherwise only via this file's re-exports and direct generated imports in a few screens — see F-001 note).
— MockTransport honesty: default arm returns the typed error (L77-85); `sent` getter for test assertions.
— Baseline: transport.test.ts passes (ui-test.txt).

## 3. Findings
F-001 [S3]
Where: ui/src/client/transport.ts:80 (vocabulary) vs ui/src/screens/playlist/PlaylistProvider.tsx:203 (consumer)
Code:
```
            code: "feature-not-exposed",
```
What is wrong: the refusal-code dialect mismatch is anchored here: the UI's own test/scaffold transports speak `feature-not-exposed`, the shipped desktop host speaks `unsupported-for-host`. Consumers that branch on refusal codes (PlaylistProvider's notExposed) were validated against the mock dialect and are dead in production. The transport seam itself is fine; the ecosystem contract is split. What correct looks like: one shared refusal-code predicate (e.g. isCapabilityRefusal(code)) covering both codes, used by every consumer. Confidence: HIGH — Verification: read + rg across dispatch.rs / demo-transport.ts / PlaylistProvider.tsx.

## 4. Per-function table
N/A (<100 LOC): interface + class reviewed in full (send switch, subscribe set, sent getter).

## 5. Test honesty
transport.test.ts (baseline pass) pins the mock's refusal shape and the listing simulation — it cannot catch the production-dialect mismatch (F-001), because the mock IS the dialect it tests.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this file is the ONLY module that reaches into src/generated" | L3-4 | PARTLY FALSE | rg: screens (PlayerScreen, ConvertScreen, BookmarksDialog, config-settings) import types from ../../generated/commands directly — the single-import-site rule is aspirational, not enforced |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Generated wire types. No gates.

## 9. Open questions for the next agent
- tauri-transport.ts: does it translate host error codes at all, or pass `unsupported-for-host` through verbatim? (Determines whether the fix belongs in the transport or the consumers.)
