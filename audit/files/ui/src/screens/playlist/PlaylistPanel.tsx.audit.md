# AUDIT: ui/src/screens/playlist/PlaylistPanel.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 456 / 9 top-level functions + inline handlers (queueRowMenu, PlaylistPanel, PlaylistRow, NavigationSeam, RoutedNavigation, useQueueMenuMsg, openMediaDialog, onRowDrop, closeAdd) |
| File verdict | SHALLOW (exemplary UI code + 27 dispatch-spy tests, but the entire control surface dispatches commands NO shipped backend serves — the only thing that animates it is the DemoTransport simulation) |
| Reachable from shipped entry point? | YES as web-UI source: `main.tsx → App.tsx:203 → PlayerScreen.tsx:1543 <PlaylistPanel />` (imports at :100). NO as a shipped desktop app: apps/desktop-tauri does not compile (W14; baseline #2 exit 101); even if it did, every `playlist.*` command it dispatches is refused by the engine host (see F-001). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `rg -n "playlist" apps/cli/src/dispatch.rs` (:191-193 all `playlist.*` → "the playlist model is a Phase-4 card"; :163-176 typed `UnsupportedForHost` refusal); `rg -n "PlaylistLoad" apps crates --type rust` (served by NOBODY — only request-construction at apps/desktop/src/dispatch.rs:241, catalog defs, policy list); `rg -n "PlaylistChanged" crates/playback-engine/src/events.rs` (:99 "no playlist model in the engine; Phase 4"; :1986 `assert!(!is_engine_emitted(...))`); `sed -n '80,300p' PlaylistProvider.tsx` (dispatch treats ONLY `feature-not-exposed` as not-exposed, :142/:203); `rg -n "playlist" src/client/demo-transport.ts` (:228-238 the demo serves all 11 commands — simulation only); `sed -n '510,540p' components/icon/icons.tsx` (skip-back/forward-15 = 15-second rewind glyphs); queue.ts math re-derivation (dragToMoveIndex correct); baseline #23 (1705 vitest incl. PlaylistPanel.test.tsx 27 it-blocks) |

## 1. One-paragraph verdict
This is the best-engineered "honest UI" file in the wave's sample — null-honest engine state, disabled no-seam menu rows instead of theater, an external-drag guard against `Number("") === 0`, correct post-removal move-index math, and 27 dispatch-spy tests that would catch any wiring regression. And it is all facade against the real backend: not one `playlist.*` command this panel dispatches is served by any shipped host — the CLI host refuses the whole family with `unsupported-for-host` ("the playlist model is a Phase-4 card", apps/cli/src/dispatch.rs:191-193), the playback engine asserts it never emits `playlist.changed` (events.rs:1986), and the provider's honesty escape-hatch checks the WRONG error code (`feature-not-exposed`, PlaylistProvider.tsx:142/203) so the real shell would show live buttons that silently do nothing instead of the promised "not connected" note. The panel's entire lived experience exists only inside DemoTransport — a browser simulation that decodes no bytes.

## 2. What actually works (verified)
— verified by queue.ts re-derivation → `dragToMoveIndex(before, from, length)` post-removal conversion is correct for all three panel uses (drop-reorder :164, play-next :330 — insert-at-currentIndex+1 with shift when source precedes target, play-later :362 — append); edge from=before no-op guarded at :161.
— verified by `rg -n "playlist" src/generated/commands.ts` → every command string the panel dispatches (`playlist.play-item/next/prev/move/remove/clear/set-shuffle/set-repeat/enqueue`) is in the generated wire catalog — the UI speaks the real protocol, no invented commands.
— verified by baseline #23 → PlaylistPanel.test.tsx's 27 tests pass (10k-entry windowing bound, dispatch spy assertions per control, context-menu 8-row pinned order, no-seam rows disabled, empty-payload drag no-op).
— verified by `rg -n "DEMO_CLIPS" enqueueSources.ts` → demo clips are gated `isDesktopShell() ? [] : …` (:25-27) — the fake sources never render in the real shell.

## 3. Findings

### F-001 [S2]
Where: ui/src/screens/playlist/PlaylistPanel.tsx:180-222, 354-368 (every control) vs apps/cli/src/dispatch.rs:191-193 and PlaylistProvider.tsx:203
Code:
```
    if id.starts_with("playlist.") {
        "the playlist model is a Phase-4 card"
```
What is wrong: the panel's complete control surface (prev/next/shuffle/repeat/clear/+popup/play-now/play-next/play-later/remove/drag-reorder) dispatches `playlist.*` commands, and no shipped host serves a single one: the CLI engine host refuses the entire family (apps/cli/src/dispatch.rs:190-193 → `unserved()` at :163-176 returns `ErrorCode::UnsupportedForHost`), apps/desktop only CONSTRUCTS `playlist.load` wire requests (dispatch.rs:241) without serving them, and the engine has no playlist model (crates/playback-engine/src/events.rs:99, with a test at :1986 asserting `playlist.changed` is never engine-emitted). The provider's honesty mechanism — `notExposed` → the "not connected" note (rendered at PlaylistPanel.tsx:243) — only triggers on `error.code === "feature-not-exposed"` (PlaylistProvider.tsx:142, :203); the actual refusal code `unsupported-for-host` (generated/commands.ts:42) falls through: `dispatch` returns null, no state change, no note.
Why it matters: in the real desktop shell every button in this panel would be a live-looking silent no-op — the exact "dead buttons" condition the honesty design (header :11-13 "never a faked known off", :23-25 "never theater") exists to prevent, defeated by an error-code mismatch. Today it is masked twice over (no GUI binary — W14; browser lane runs DemoTransport, which simulates all 11 commands at demo-transport.ts:228-238).
What correct looks like: the provider treats `unsupported-for-host` as not-exposed too (or maps both to one "capability gap" code per doc 05 §5.2), and the matrix/docs stop implying a functioning playlist against the real engine until Phase 4 lands.
Confidence: HIGH — Verification: read of all four files + generated catalog; baseline #1 (workspace builds — the refusal path is compiled-in, not speculative).

### F-002 [S3]
Where: ui/src/screens/playlist/PlaylistPanel.tsx:180-185
Code:
```
          <IconButton label={m("playlist.controls.prev")} size="sm" onClick={() => playlist.prev()}>
            <Icon name="skip-back-15" size={TOOLBAR_ICON_SIZE} />
          </IconButton>
```
What is wrong: the previous/next TRACK buttons use the 15-second jump glyphs — `skip-back-15` is icons.tsx:518 `Icon.svg "name=backward-15-sec"`, a circular arrow wrapping a literal "15" numeral (icons.tsx:520-524 path data), i.e. the universal seek-back-15-seconds icon. The action is `playlist.prev()` (previous track) and the accessible label is "playlist.controls.prev". Sighted users see a 15-second rewind; the glyph promises seek-by-15s, the button skips tracks. These are the only two uses of these icons in src/ (rg).
Why it matters: icon/action contradiction on primary transport controls; a user pressing "↺15" expecting a 15s rewind gets a track change (or, per F-001, nothing).
What correct looks like: prev/next track glyphs (double-triangle ⏮/⏭ shapes in the same stroke grammar), or reserve skip-*-15 for an actual seek-15s control.
Confidence: HIGH — Verification: read of icons.tsx:510-540 (both glyph path sets embed outlined "15" text) + rg for other users (none).

### F-003 [S4]
Where: ui/src/screens/playlist/PlaylistPanel.tsx:208-222 (vs the honest-disabled pattern at :408, :424, :446)
Code:
```
          <IconButton
            ref={addTriggerRef}
            label={m("player.mini.queue.add")}
            size="sm"
            aria-haspopup="menu"
```
What is wrong: the "+" enqueue trigger is always enabled, but in the real desktop shell its menu can only ever contain the disabled empty row: `useEnqueueSources` returns [] when `isDesktopShell()` (DEMO_CLIPS gated off, enqueueSources.ts:25-27) and recentOpenings only exist on the demo transport (:31-36). The file's own no-seam standard — favorite/save/add-audio render `disabled: true` "never theater" (:406-446) — is not applied to a trigger whose entire menu is a single disabled row.
Why it matters: inconsistent honesty grammar; users get an enabled button that opens a dead menu in exactly the environment the honesty rules were written for.
What correct looks like: disable the trigger when `enqueueSources.length === 0` (the same seam the menu already reads).
Confidence: HIGH — Verification: read of enqueueSources.ts gating + enqueueEntries' empty branch (:56-57).

### F-004 [S4]
Where: ui/src/screens/playlist/PlaylistPanel.tsx:176-178
Code:
```
          {String(playlist.entries.length)} {m("playlist.panel.count-suffix")}
```
What is wrong: count rendered as number + space + a single "count-suffix" string — a pluralization pattern that cannot work beyond English ("3 items" vs Russian/Arabic plural forms). Compounding: the localization crate is orphaned and the UI hardcodes en-US (W11 evidence), so no locale ever exercises it, but the seam is being cemented into new UI.
Why it matters: i18n debt baked into a frozen public surface; every count site (also discovery.volumes.count-suffix, convert.queue.count-suffix) will need the same rewrite.
What correct looks like: an ICU plural message (`{count, plural, one {# item} other {# items}}`) at the messages layer.
Confidence: HIGH — Verification: read; `rg "count-suffix" src/i18n/messages.ts` → 3 sibling sites; W11 orphaned-localization evidence cited from wave prompt.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `useQueueMenuMsg` | 60-66 | REAL | — | catalog-first label resolution with lane fallback; correct |
| `NavigationSeam` / `RoutedNavigation` | 74-85 | REAL | — | router-presence guard prevents useNavigate crash in bare mounts; split keeps hooks unconditional — correct pattern |
| `openMediaDialog` | 91-93 | REAL | — | shell-wide CustomEvent; AppShell owns the listener (single-listener contract) |
| `PlaylistPanel` | 95-300 | SHALLOW | S2/S3/S4 | controls wired to protocol but unserved (F-001); outside-pointer dismiss effect :115-124 correct; enqueue anchor from getBoundingClientRect :216-217; VirtualList scrollToIndex deps safe (Virtual.tsx:158 keys on index/align, not identity) |
| `onRowDrop` | 149-165 | REAL | — | empty-payload guard :156-158 (Number("")===0 trap), NaN guard :161, entry existence :163 — robust |
| `closeAdd` | 125-128 | REAL | — | focus restore to trigger |
| `PlaylistRow` | 302-378 | SHALLOW | — | memo-free row inside virtualized window (fine — window bounds renders); play-next/play-later math verified correct; ContextMenu wraps ListRow — nested interactive handled by grid semantics (Virtual.tsx w26 Lead fix) |
| `queueRowMenu` | 386-456 | REAL | — | 8 frame rows in pinned order; 3 honestly disabled (favorite/save/add-audio) — no theater; shuffle checkbox `checked: playlist.shuffle === true` keeps null-honest face |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| 27 it-blocks in PlaylistPanel.test.tsx (baseline-passed) | YES for dispatch wiring, honest-disabled, drag math, windowing | MockTransport with dispatch spies asserting exact cmd/args per control — genuine behavioral coverage; mutation (e.g. dropping a dispatch) fails a spy assertion |
| same tests | NO for backend reality | mock completes every playlist command — cannot reveal that the engine refuses them all (F-001); the demo-transport lane is equally fake by design |
| "renders a bounded window for 10k entries" (:132) | YES | counts rendered rows vs window — real assertion |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Honest wiring per contract §5: real seams only — Play (playlist.play-item), Shuffle (playlist.set-shuffle, engine state)…" | PlaylistPanel.tsx:19-22 | SHALLOW | seams are real WIRE commands (generated/commands.ts) but unserved by every host — the "engine state" never arrives because the engine has no playlist model (events.rs:1986) |
| "Favorite / Save playlist / Add audio tracks have NO seam in this build … render disabled — never theater" | PlaylistPanel.tsx:23-25 | REAL | verified: :408/:424/:446 `disabled: true` + test :310 |
| "never a faked 'known off'" (shuffle/repeat honesty) | PlaylistPanel.tsx:11-13 | REAL | `aria-pressed={playlist.shuffle === true}` :191; null renders unpressed |
| (implied) the queue works in the shipped app | — | FALSE-at-HEAD | F-001: no host serves any playlist.* command; no GUI binary exists (W14) |

## 7. Dead / duplicated / unreachable code
None in-file. Cross-file: the entire `PlaylistActions` surface is unreachable-as-served at HEAD (F-001) — real protocol, zero implementations.

## 8. Dependencies & flags
Imports 14 local modules (all present); `react-router-dom` `useInRouterContext` guard; no feature flags. Depends on PlaylistProvider (error-code bug F-001), enqueueSources (demo-gated), AddSubtitlesDialog (file 8 of this agent).

## 9. Open questions for the next agent
- F-001's root fix lives in PlaylistProvider.tsx:142/:203 (not my file): confirm with the engine wave whether `unsupported-for-host` vs `feature-not-exposed` mapping was ever specified (doc 05 §5.2 "capability gaps are typed") — as written, EVERY typed refusal except feature-not-exposed bypasses the honesty note in every provider, not just playlist.
- PlaylistScreen.tsx (other agent) renders a saved-playlists store (playlistsStore.ts — localStorage?): reconcile with W11 "settings never persist" — two different persistence seams (localStorage works, settings don't)?
- The count-suffix pluralization debt (F-004) should be assigned to the i18n lane together with W11's en-US hardcoding.
