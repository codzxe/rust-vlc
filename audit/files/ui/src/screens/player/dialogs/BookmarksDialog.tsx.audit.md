# AUDIT: ui/src/screens/player/dialogs/BookmarksDialog.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 303 / 5 (BookmarksDialog, onAdd, onDelete, onJump, requestId closure + helpers) |
| File verdict | SHALLOW in production (real dialog, real wire protocol, real demo backend — but NO shipped host implements `media.*-bookmark`; the engine's BookmarkList is wired to nothing) |
| Reachable from shipped entry point? | Dialog itself: NO at HEAD (player route is UI; host crate unbuildable, W14-A01 E0425). Wire commands: catalog-declared (crates/commands/src/catalog/command.rs:386-400) but host-refused — see F-001. Works only against the browser DemoTransport (demo-transport.ts:610-640 implements all three commands). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `rg -l "BookmarkList" apps crates` → ONLY crates/playback-engine/src/bookmarks.rs (no host constructs it); `rg -n "bookmarks" apps/desktop/src/host.rs` → nothing; `sed -n '283,300p' apps/desktop/src/dispatch.rs` → unserved catalog ids answer a typed `unsupported-for-host` refusal (tested); `rg -n "add-bookmark" ui/src/client/demo-transport.ts` → full demo implementation at :610+; test census: BookmarksDialog.test.tsx 13 tests (focus, µs unit contract, error paths); sed quote verification. |

## 1. One-paragraph verdict
As UI code this dialog is excellent: the µs↔seconds wire contract is centralized and regression-tested, add failures never destroy the typed label, load failures replace the misleading empty copy, delete adopts the server's list (never a local guess), jump rides the shared seek funnel, and natural-sort ordering is pinned. But the feature it implements does not exist in any shipped build: `media.get/add/remove-bookmark` are declared in the command catalog and fully implemented in the browser demo transport, while the engine side is a pure-state `BookmarkList` that NO host constructs (repo-wide: only its own file references it). Against the real desktop host every dialog action would receive the typed `unsupported-for-host` refusal (dispatch.rs:292+), so the user gets "load failed" — honest, but the bookmarks feature is a demo-only surface. This is the Wave-8 `audio.set-eq` pattern in its most complete form: UI + catalog + demo backend + orphaned engine half, minus production wiring.

## 2. What actually works (verified)
— verified by BookmarksDialog.test.tsx (baseline #23, 1705-pass set): 13 mutation-sensitive tests — µs unit round-trip (90 s ↔ 90 000 000, :191), failed-add label retention (:310), failed-load error vs empty copy (:329), natural order (:280), escape/focus trap (:357).
— verified by reading demo-transport.ts:610-640: all three commands work in the demo lane (add with µs sign validation, remove with id filter).
— verified by reading dispatch.rs:287-291: an unserved catalog id answers a typed refusal with the request id echoed — the dialog's error paths would fire honestly, no panic/silence.

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/player/dialogs/BookmarksDialog.tsx:4-14 (FLOW header) + crates/playback-engine/src/bookmarks.rs:4-9 + apps/desktop/src/dispatch.rs:287-291
Code (BookmarksDialog.tsx:4-8):
```
// FLOW (all through the client transport, wire `media.*-bookmark`):
//   * mount (per-open): `media.get-bookmarks` → the list;
//   * create: `media.add-bookmark` with the ENGINE-REPORTED position
//     (µs on the wire — schema $defs/Timestamp "Microseconds since media
```
and (crates/playback-engine/src/bookmarks.rs:6-9):
```
//! the wire commands above are
//! carried by the [`commands`] catalog, and this module is the testable
//! list semantics a host (or a future in-engine session) drives them with.
```
What is wrong: The FLOW header describes create/jump/delete "through the client transport" as if a backend exists. It does not: `rg -l "BookmarkList" apps crates` → only bookmarks.rs itself. Neither the CLI host, the desktop shell, nor the tauri host serves `media.*-bookmark`; the shell's dispatch answers unserved ids with the typed `unsupported-for-host` refusal (dispatch.rs:287-291, pinned by its own test). The only working backend is the browser DemoTransport (demo-transport.ts:610-640). The engine module's own doc admits it ("a host … drives them with" — none does).
Why it matters: The Ctrl+B bookmarks dialog is a real, reachable, user-facing VLC-parity surface that cannot load, create, or delete a single bookmark in any shipped build. Every action lands on the inline error line. The feature works only in the browser demo, which is not a shipped entry point.
What correct looks like: wire `BookmarkList` into the desktop shell's dispatch (the module was built for exactly that), or disclose the seam status in the dialog header the way other staged surfaces do.
Confidence: HIGH — Verification: repo-wide BookmarkList census; dispatch.rs read; demo-transport read; catalog command.rs:386-400 read.

### F-002 [S4]
Where: ui/src/screens/player/dialogs/BookmarksDialog.tsx:82-86
Code:
```
  const nextRequestId = useRef(0);
  const requestId = (): number => {
    nextRequestId.current += 1;
    return nextRequestId.current;
  };
```
What is wrong: Request ids are minted per-dialog-mount from 0 in JavaScript. This is the exact pattern W14-A05/A01 recorded against the shell's §4.3 bookkeeping (apps/desktop/src/shell.rs: the host's DeliveryLog only knows ids minted by its own `next_request_id`, so every UI-minted id records a false "response for unallocated request" violation into an unbounded Vec). Multiple dialogs (GotoTimeDialog + this one) also mint colliding id sequences; only the echo makes them usable.
Why it matters: Amplifies a known host-side defect (false-violation noise + unbounded growth) and leaves id allocation uncoordinated across UI senders.
What correct looks like: a single shared id allocator at the transport layer (the tauri-transport owns one), or host-side adoption of UI-minted ids.
Confidence: HIGH (pattern match to W14 finding), MEDIUM (this dialog's specific contribution to the noise is inferred, not measured).
Severity: S4 (host-side manifestation already recorded by W14 — cross-ref, not double-counted).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module header | 1-52 | REAL (accurate about the UI half) | — | does not disclose the missing host wiring (F-001) |
| US_PER_SECOND | 61 | REAL | — | unit contract centralized; regression-tested (:191) |
| BookmarksDialog | 67-198 | REAL | S2 | F-001 production dead-end; error paths honest |
| mount fetch effect | 91-120 | REAL | — | cancelled-guard; eslint-disable for m documented ("module-level constants") — verified: lane catalogs are module consts |
| sortedBookmarks | 124-127 | REAL | — | copy + compareBookmarks (naturalSort.ts, asset-group lane) |
| onAdd | 129-167 | REAL | — | engine-reported position (never clock guess); label cleared only on success; "accepted" outcome adds no fake row |
| onDelete | 169-192 | REAL | — | adopts response list (server truth) |
| onJump | 194-197 | REAL | — | µs→s ÷1e6, shared actions.seek funnel; dialog stays open (VLC parity) |
| render (form/list/rows/header) | 199-303 | REAL | — | focus-order body-first trick; aria-labels with time+label; role="alert" error; empty <p> while loading is harmless noise |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| :149-234 fetch/add/jump/delete flows | YES | transport spy assertions on exact wire payloads (µs) and state updates |
| :236-257 jump absolute seek | YES | asserts the seam call, not the wire |
| :310-356 error paths | YES | failed add keeps label; failed load ≠ empty copy; failed delete keeps row — all three honesty rules pinned |
| :191 µs round-trip | YES | the unit-contract regression (j09 P1) — would catch a s/ms/µs confusion instantly |
| baseline #23 | — | executed green |
| Test gap | — | no test exercises the `unsupported-for-host` refusal (the PRODUCTION outcome) — the suite proves the demo lane, not the product lane |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "FLOW (all through the client transport, wire media.*-bookmark)" | BookmarksDialog.tsx:4 | REAL as protocol / SHALLOW as product | F-001: no host serves the commands |
| "the response's updated list is the truth, never a local guess" | :13-14 | REAL | onDelete adopts result.completed.bookmarks (183-186) |
| "a failed add never destroys the user's typed label" | :32-34 | REAL | :159 clears only on success; test :310 |
| keymaps `bookmarks-dialog` Ctrl+B ui_action "bookmarks" | :16-17 | REAL | PlayerScreen keymap handler (part2 :796) opens the dialog |
| feature-matrix bookmarks row | docs/feature-matrix.toml (W16 lane) | not re-audited here | out of scope |

## 7. Dead / duplicated / unreachable code
No dead code in-file. The feature's engine half (crates/playback-engine/src/bookmarks.rs, 205 lines + tests) is orphaned production code — no caller outside its own file (F-001). Dialog unreachable from shipped binaries (W14-A01 host E0425).

## 8. Dependencies & flags
Depends on generated/commands (Bookmark type), client transport, player controls context, naturalSort, useDialogsMsg lane keys, Dialog/TextField/Button/IconButton primitives. No feature flags. typecheck green (baseline #22).

## 9. Open questions for the next agent
- Host lane: is wiring `BookmarkList` into apps/desktop/src/dispatch.rs scheduled (issue #32 continuation)? The engine module is complete and tested for exactly that.
- Whoever audits demo-transport.ts: confirm the demo bookmark store's id semantics match BookmarkList's (1-based, never reused) — drift would make the demo lane lie about the product lane.
