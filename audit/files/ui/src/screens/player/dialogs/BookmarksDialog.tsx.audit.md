# AUDIT: ui/src/screens/player/dialogs/BookmarksDialog.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 303 / 5 (component + onAdd/onDelete/onJump + requestId) |
| File verdict | REAL-UNPROVEN (real client wiring, correct units; feature dead in every real host) |
| Reachable from shipped entry point? | YES: PlayerScreen.tsx:796-801 Ctrl+B (DIALOG_KEYMAP) -> PlayerDialogs L1246 `<BookmarksDialog open ...>` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | rg bookmark arms in apps/cli + demo-transport (exit 0); sed quote checks (exit 0) |

## 1. One-paragraph verdict
The dialog is genuinely wired to the command transport — `media.get-bookmarks` on mount, `media.add-bookmark` with the engine-reported position crossed seconds→µs (×1e6, L133), `media.remove-bookmark` per row, and jump via the shared `actions.seek` funnel (µs→seconds ÷1e6, L196) — with honest inline errors, label-preserving failure handling, and a natural-sorted list. But every bookmark command is REFUSED by the only real host: `rg -c "Bookmark" apps/cli/src/dispatch.rs` → 0 matches (unserved arm -> typed `unsupported-for-host`), host.rs: 0 matches; the desktop host adopts the CLI host; the ONLY implementation anywhere is the browser demo's fake transport (demo-transport.ts:607/611/635). In the shipped desktop shell the dialog can only ever show "Load failed"; add/delete error; the jump button — the one action backed by a served command (`playback.seek`) — can never appear because the list can never load. A demo-only feature wearing production clothes, honestly labelled at runtime.

## 2. What actually works (verified)
— Wiring + units: L133 `const timeUs = Math.round(state.position * US_PER_SECOND);` and L196 `actions.seek(bookmark.time / US_PER_SECOND);` — crossings match the schema's µs Timestamp contract both directions.
— Error handling: load failure replaces the empty copy (L237-242) so the UI never claims "no bookmarks" about a list it never saw; failed add keeps the typed label (L157-159); transport `.catch` on all three sends.
— Demo transport implements the three commands (demo-transport.ts:607,611,635) — the feature is fully functional in the browser dev shell (that is the problem, F-001).
— Baseline: dialogs tests pass (ui-test.txt includes src/screens/player/dialogs/).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/player/dialogs/BookmarksDialog.tsx:93-94, 135-143, 171-176 (commands) vs apps/cli/src/dispatch.rs (absent arms)
Code:
```
    void client
      .send({ cmd: "media.get-bookmarks", requestId: requestId() })
```
```
        cmd: "media.add-bookmark",
```
What is wrong: none of `media.get-bookmarks` / `media.add-bookmark` / `media.remove-bookmark` has a handler in the CLI host (`rg -c "Bookmark" apps/cli/src/dispatch.rs` → no matches; route()'s `other => Err(unserved(other))` arm, dispatch.rs:155, answers them; owner_of names no media-bookmark owner), the desktop host adopts the CLI host, and web/control-server refuse more still. Only `ui/src/client/demo-transport.ts` implements them (L607/611/635). Why it matters: Ctrl+B in the shipped desktop app opens a bookmark manager that can never load, create, or delete a bookmark — the feature is fake-in-production, real-in-demo, the exact mock-as-real inversion the audit hunts. The honest inline error line keeps this at S2, not S0 (the UI never pretends success). What correct looks like: serve the three commands in the CLI host against the session's BookmarkList (the engine-side model the demo mirrors — demo-transport.ts:745-748 cites "the engine-side BookmarkList.clear() semantics"), or disable the dialog's actions until a host serves them. Confidence: HIGH — Verification: read + rg (dispatch.rs, host.rs, demo-transport.ts).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| BookmarksDialog (mount fetch) | 91-120 | REAL | S2 | cancellation-guarded; F-001 host refusal |
| sortedBookmarks memo | 124-127 | REAL | - | natural sort, copy not the engine array |
| onAdd | 129-167 | REAL | S2 | µs crossing correct; label kept on failure; F-001 |
| onDelete | 169-192 | REAL | S2 | response list is the truth; F-001 |
| onJump | 194-197 | REAL | - | playback.seek IS served (dispatch.rs:120) — but no row can ever exist in a real host |
| render | 199-303 | REAL | - | Dialog primitive, focus order trick, a11y labels w/ time+label |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| dialogs tests (baseline pass) | YES for dialog logic | they run against DemoTransport (which serves the commands), so unit crossings, error lines, label retention are pinned — but NO test covers the real-host refusal path (the shipped experience) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "FLOW (all through the client transport, wire `media.*-bookmark`)" | L4-5 | TRUE | read |
| "the response's updated list is the truth, never a local guess" | L13-14 | TRUE | L183-185 |
| (implicit) the flow works | whole header | FALSE in every real host | F-001: all three commands refused by CLI/desktop hosts |

## 7. Dead / duplicated / unreachable code
None internal. The jump/delete row rendering is unreachable in a real host (list can't load) — dead-by-host, not dead-by-code.

## 8. Dependencies & flags
No gates. Depends on Dialog/TextField/Button/IconButton primitives, player context (seek), dialogs i18n seam.

## 9. Open questions for the next agent
- Same pattern audit for GotoTimeDialog: `media.get-current-time` IS served (dispatch.rs:134) but what does its seek prefill read? If it uses served commands only, goto-time may be the one dialog that works in production.
