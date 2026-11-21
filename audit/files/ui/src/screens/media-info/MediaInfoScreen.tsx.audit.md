# AUDIT: ui/src/screens/media-info/MediaInfoScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 29 / 1 (MediaInfoScreen) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (web UI): App.tsx:37 lazy route → /media-info (route table :81,:108; window-title map :224). Screen test suite ran green in baseline vitest (row 23). Data path: MediaInfoProvider → `media.get-info` command through the app transport (main.tsx:56 selectAppTransport: TauriTransport in the desktop shell / DemoTransport in the browser dev shell) — desktop shell cannot compile at HEAD (W14), so the real-engine path is ceiling-blocked; the browser dev shell renders the disclosed demo engine. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | Read of MediaInfoScreen.tsx + MediaInfoScreen.test.tsx (151 lines) + MediaInfoProvider.tsx header + demo-transport.ts header; rg route/msgid/CSS claims (App.tsx:37,81,108,224; messages.ts:125,1553; media-info.css:160 `width: min(680px, 100%)`); rg MediaInfoPanel consumers (PlayerScreen.tsx:99 — the "also mounted" claim is real); rg documentElement in the screen (none — claim verified); fixtures.ts import census (test-only). |

## 1. One-paragraph verdict
A 29-line thin screen that does exactly what its header claims and nothing else: one h1 through useMsg ("screen.media-info" → "Media information"), the shared MediaInfoPanel under a per-route MediaInfoProvider with `showTitle={false}` (single-heading discipline, reason commented), a token-only CSS column (media-info.css:160 carries the promised `min(680px, 100%)`), and no documentElement mutation. The data honesty lives one layer down and is verified real: MediaInfoProvider seeds from the session-scoped `media.get-info` command and treats `null` as "nothing reported, never a fabricated MediaInfo" — fixtures.ts is imported ONLY by the two test files (census). The test suite is exemplary: fixture transport for composition, a not-found transport for the honest empty state (asserting the r14 Card grammar + the "Open media" CTA that fires the real shell-wide OPEN_MEDIA_EVENT), and a DemoTransport regression test that pins the µs-wire duration contract with the exact old-code failure named ("renders 0:00" pre-k17). No defects found in this file; what was checked is enumerated below.

## 2. What actually works (verified)
- Route + heading: /media-info lazy route (App.tsx:37); h1 msgid exists in union+catalog (messages.ts:125, 1553); test :45 asserts heading level 1 + accessible name.
- Shared-panel claim: PlayerScreen.tsx:99 imports MediaInfoPanel + MediaInfoProvider — the "also mounted in the player shell's media-info region" claim (:2-3) is a census fact.
- Single heading: showTitle={false} with the reason commented (:23-24); the route's h1 is the page heading.
- Column grammar claim: media-info.css:160 `width: min(680px, 100%)` — the "min(680px, 100%) working-screen column" header claim is in the sheet.
- "never touches documentElement while mounting" (:8-9): no documentElement reference in the screen (rg) — contrast with theme screens that legitimately set data-* on <html>.
- Honest data path: MediaInfoProvider (read) — media.get-info + media.info-ready events; null = honest empty; notExposed vs unreachable are distinct verdicts ("a dead transport is a different verdict from feature-not-exposed"); fixtures.ts consumed only by tests (census).
- Empty-state CTA seam: test :79-106 proves the "Open media" button dispatches the shell-wide OPEN_MEDIA_EVENT — a real path to the populated body, not a dead button.
- µs-wire regression test (:109-150): DemoTransport session.open for the bunny clip → 635,000,000 µs → "Duration10:35" rendered — with the comment naming the pre-fix failure mode; this is the W-family unit-confusion class actually pinned by a test.
- Transport honesty chain: demo-transport.ts header discloses the browser-dev-shell scope ("no bytes are decoded, no clocks are wall-locked, no audio leaves the speakers"; unsupported commands answer typed feature-not-exposed) — the screen's dev-shell data is labeled at the seam, not presented as an engine.

## 3. Findings
None. Checklist coverage for the zero-finding claim:
- Fakeness: no stubs, no constants-as-data, no ignored props; the screen is composition only (all 29 lines read).
- Correctness: no logic to get wrong — heading/provider/panel; provider error states handled downstream (MediaInfoPanel's file, tested there).
- Performance: one provider per route mount; no duplicate probing concern (route and player region are exclusive — routes replace, never stack).
- Tests: all 4 would fail on regression (heading/sections, empty state + Card grammar, CTA event, DemoTransport duration) — none tautological; the FixtureTransport is a transport mock, the subject under test is the screen's composition + honest-state wiring.
- Hygiene: i18n via useMsg (no literals); tokens via CSS (no inline styles); no dead code (panel/provider shared, screen sole owner of the route body).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MediaInfoScreen | 16-29 | REAL | — | Thin composition; showTitle rationale commented; data-screen hook for tests/CSS |
## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "renders the screen heading and the fixture-driven sections" :43 | YES | Heading role+level+name, wire-driven section content ("Matroska (MKV)"), data-screen/data-mi-section hooks |
| "renders the honest empty state when nothing is open" :53 | YES | not-found error → data-state=empty + "No media open" + r14 Card grammar asserted |
| "empty state: Open media fires the shell-wide open-media event" :79 | YES | Event listener proves the dispatch — the CTA is a real seam, not decoration |
| DemoTransport duration test :109 | YES | End-to-end through the demo engine's exact wire shape; duration assertion fails on any µs/ms regression (the pre-k17 code rendered "0:00") |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "/media-info route body: the MediaInfoPanel under a page heading … content is the shared panel (also mounted in the player shell's media-info region)" | :1-3 | REAL | App.tsx:37; PlayerScreen.tsx:99 census |
| "a top-anchored working-screen column (min(680px, 100%), Settings content-column grammar)" | :5-6 | REAL | media-info.css:160 |
| "the screen paints tokens only and never touches documentElement while mounting" | :8-9 | REAL | rg — no documentElement reference |
| Provider: "`info: null` means no media info reported … never a fabricated MediaInfo" | MediaInfoProvider.tsx:4-7 | REAL | Provider read; empty-state test asserts the honest rendering |
## 7. Dead / duplicated / unreachable code
- None in this file. fixtures.ts is test-scoped (census); the panel/provider are shared (PlayerScreen + this screen).
## 8. Dependencies & flags
MediaInfoPanel + MediaInfoProvider (siblings, own test files), useMsg, media-info.css. Transport chain: main.tsx:56 selectAppTransport → TauriTransport (desktop; unbuildable at HEAD, W14) / DemoTransport (browser dev shell, scope disclosed at demo-transport.ts:1-20). No feature flags. W5/W13's engine-info hard-coding pin does NOT touch this screen: media-info renders command results, not hard-coded output.
## 9. Open questions for the next agent
- MediaInfoPanel.tsx + MediaInfoProvider.tsx (not assigned to me): the unknown-field AC and the notExposed/unreachable note rendering live there — the provider's effect lifecycle (request id increment, cancelled flag) deserves its own per-function audit.
- demo-transport.ts (not assigned to me): the "living app" simulation is the browser shell's whole engine — its honesty envelope (feature-not-exposed per command) is the load-bearing claim for every screen like this one in the dev shell.
