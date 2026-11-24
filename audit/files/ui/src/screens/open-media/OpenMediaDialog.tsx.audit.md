# AUDIT: ui/src/screens/open-media/OpenMediaDialog.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 326 / 6 (OpenMediaDialog, SourceRow, parseSourceInput, sourceIcon, sourceLocation, openSource/onPathSubmit/clearAndClose handlers) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | YES — AppShell owns the `lumen:open-media` CustomEvent listener and mounts this dialog (header L4-9 contract; AppShell.tsx mounts, verified by event name grep below) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | rg for OPEN_MEDIA_EVENT listener; read of open-path.ts funnel; sed quote verification |

## 1. One-paragraph verdict
A real, honest open-media surface: every activation (row click, Enter, pasted path, footer Open) goes through the ONE funnel `openPathRequest` → `session.open`/`playlist.load` over the ClientProvider transport, with request-supersede guards and demo clips correctly gated to the demo transport in the browser shell only. One real defect: the response is never inspected — an engine error result (file not found, no decoder) still closes the dialog and navigates to the player exactly like a success; only a transport-level rejection is caught, and it too routes onward.

## 2. What actually works (verified)
— verified by code path: openSource (L150-172) → `client.send(openPathRequest(source, requestId))` → open-path.ts:66-70 (`cmd: "playlist.load"` / `cmd: "session.open"`) → transport. Demo gating L126-131: `!isDesktopShell() && isDemoTransport(client)`. parseSourceInput (L76-81): scheme → url, else file; blank → null (honest invalid).
— NOT executed live (no engine session in sandbox) — REAL-UNPROVEN at runtime.

## 3. Findings
F-001 [S2]
Where: ui/src/screens/open-media/OpenMediaDialog.tsx:150-172
Code:
```
    void client
      .send(openPathRequest(source, requestId))
      .catch(() => {
        // Transport unreachable — the player screens surface their own
        // honest typed errors; the dialog still closes and routes.
      })
      .finally(() => {
        // A newer activation superseded this one — it owns the close/route.
        if (requestId !== nextRequestId.current) return;
        setOpening(false);
```
What is wrong: the resolved CommandResponse is discarded — `response.result` may be the typed error envelope (`{error: {code: "file-not-found" | "no-decoder" | ...}}`) and the flow still runs the success path: dialog closes, input clears, `navigate(openPathRoute(source))`. Compare OnboardingScreen.tsx:660-664 which DOES check `"error" in response.result` for the same funnel. A user pasting a bogus path gets shuttled to the player with no dialog-level feedback and must find the error elsewhere (the player's error surface — which presupposes the session/open event stream surfaces it; UNVERIFIED that it does for open failures). Why it matters: false-success UX on the most common failure (bad path). What correct looks like: inspect result, keep the dialog open with the field's error copy on failure (the field already has an error slot). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| sourceLocation/sourceIcon | 58-73 | REAL | - | data helpers, exported for contract test |
| parseSourceInput | 76-81 | REAL | - | scheme/file split; blank → null |
| OpenMediaDialog | 83-307 | REAL-UNPROVEN | S2 | funnel + guards; error swallowing F-001 |
| openSource/onPathSubmit/clearAndClose | 126-190 | REAL | - | single-flight + supersede guards |
| SourceRow | 309-326 | REAL | - | plain activation button, full location rendering |

## 5. Test honesty
OpenMediaDialog tests exist in the lane (not in my list); the funnel behavior is asserted per OnboardingScreen's sibling tests. UNVERIFIED whether any test covers the error-result case (F-001 suggests not).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every open goes through the ONE funnel (client/open-path.ts) — media files session.open, playlist files playlist.load" | comment L128-130 | REAL | L152 + open-path.ts:60-70 |
| "the player screens surface their own honest typed errors" | comment L154-155 | UNVERIFIED | player error surface not audited here; F-001 shows the dialog itself surfaces nothing |
| "no folder source exists in any transport, so none is shown" | header L18-19 | REAL | SourceSpec kind is "file"|"url" only (transport.ts re-exports) |

## 7. Dead / duplicated / unreachable code
None found.

## 8. Dependencies & flags
Dialog/TextField/Button/Icon primitives, demo-transport (clips + recent), open-path funnel. Runtime gates: isDesktopShell(), isDemoTransport(). No env vars.

## 9. Open questions for the next agent
- Does the player surface an error UI when session.open returns an error result (vs. only transport throw)? If not, F-001 upgrades to S1 (silent total failure of the open flow).
