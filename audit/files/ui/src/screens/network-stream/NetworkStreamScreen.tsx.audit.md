# AUDIT: ui/src/screens/network-stream/NetworkStreamScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 312 / 4 (isValidNetworkUrl, loadRecentStreams, saveRecentStreams, NetworkStreamScreen) |
| File verdict | REAL — with one swallowed-error path |
| Reachable from shipped entry point? | YES — App route /open-network-stream (ControlBar app-menu entry navigates here, ControlBar.tsx:974) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "it\(" NetworkStreamScreen.test.tsx` (12 tests); `sed -n 1,60p ClientProvider.tsx` (no global error surfacing); full read |

## 1. One-paragraph verdict
Genuinely wired: the Open action sends the real `session.open` command with `{ source: { kind: "url", url } }` (L147-152), double-submit guarded, superseded-request counter checked (j16 P3-9 regression handled), invalid URLs rejected client-side with an announced error, recents persisted honestly in localStorage with corruption-tolerant reads. The one real defect: the `.catch(() => {})` on the send (L153-155) swallows transport-unreachable failures while the justification comment claims "the player surfaces its own honest error" — false for this path. ClientProvider is a bare context (no error bus); the player's error surface only fires for commands dispatched through PlayerControls' own context or engine events, neither of which exists for a failed direct send. Net effect on a dead backend: click Open -> spinner stops -> silent navigation to an idle player.

## 2. What actually works (verified)
— verified by NetworkStreamScreen.test.tsx:123-137 ("submits: session.open with the url source, then routes to /player") and :112-121 (invalid URL sends nothing); 12 tests total; baseline vitest 1705 passed.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/network-stream/NetworkStreamScreen.tsx:153-163
Code:
```
      .catch(() => {
        // Transport unreachable — the player surfaces its own honest error.
      })
      .finally(() => {
```
What is wrong: the catch drops the failure and the finally navigates to /player regardless. The comment's justification is false: ClientProvider.tsx (entire file, 29 lines) is a bare React context with no error channel, and PlayerScreen's lastError only updates on transport-failure of commands dispatched through its OWN dispatch() (context.tsx:336-351) or engine events. A rejected direct `client.send` from this screen surfaces nowhere. Why it matters: on the shipped desktop shell with an unresponsive backend, the primary "Open Network Stream" flow fails silently — a dead-button-shaped UX with no error. What correct looks like: setError with a transport-unavailable message (the dialog already has an announced error row), or route the open through a shared dispatch that owns error surfacing. Confidence: HIGH — Verification: read of this file + ClientProvider.tsx + context.tsx:336-351.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| isValidNetworkUrl | 62-69 | REAL | — | http/https only, URL-parser based, pure, test-pinned |
| loadRecentStreams | 71-81 | REAL | — | corrupt-store tolerant, capped at 6 |
| saveRecentStreams | 83-89 | REAL | — | swallowed quota error disclosed (persistence optional — acceptable) |
| NetworkStreamScreen | 91-312 | REAL | S3 | F-001; openStream double-submit guard, supersede counter, initial-focus fix, role=alert error |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| :47 URL validation | YES | pure function assertions |
| :87 initial focus on URL field | YES | regression for the ringed-✕ defect |
| :112 invalid URL sends nothing | YES | asserts no command |
| :123 session.open + route | YES | asserts sent command payload |
| :139 named open -> local playlist | YES | asserts createPlaylist side effect |
No test covers the send-rejection path (F-001) — the gap that let the false comment survive.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Transport unreachable — the player surfaces its own honest error." (L154) | comment | FALSE | ClientProvider has no error channel; player lastError requires its own dispatch (context.tsx:336-351) |
| "The frame's 'multiple links' promise still awaits an engine catalog command" (L27-29) | comment | REAL | commands.ts CommandRequest has only single-source session.open (verified in generated catalog) |
| "A non-empty Playlist Name ... recorded as a LOCAL saved playlist ... no engine command exists for it" (L29-32) | comment | REAL | createPlaylist is playlistsStore (local); honest disclosure |

## 7. Dead / duplicated / unreachable code
None — all handlers live.

## 8. Dependencies & flags
react-router, localStorage, playlistsStore (local-device data, disclosed). No flags.

## 9. Open questions for the next agent
- Does the ENGINE accept `kind: "url"` sources and actually stream http(s)? That is the streaming crate's audit; the UI contract is real either way. UNVERIFIED.
- Same swallowed-catch pattern exists in ControlBar's recent-openings handler (ControlBar.tsx:937-941) — same false-justification risk; noted there.
